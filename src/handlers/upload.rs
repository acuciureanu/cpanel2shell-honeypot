use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{
    capture::{log_request, save_capture},
    config::AppConfig,
    limits::{DEFAULT_MAX_POST_BODY, check_rate_limit, evict_sessions_if_needed},
    shell::ShellSession,
    events::EventType,
};

pub async fn handle_upload(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(token): Path<String>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let path = uri.path();

    // Rate limiting
    if !check_rate_limit(&config, remote.ip()).await {
        tracing::warn!("[{}] Rate limit exceeded for {}", config.port, remote.ip());
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }

    log_request(&config, EventType::Upload, remote, "POST", path, &headers, None).await;
    tracing::info!(
        "[{}] Multipart upload from {} to {}",
        config.port,
        remote,
        path
    );

    // Check session cap
    {
        let sessions = config.sessions.read().await;
        if sessions.len() >= config.max_sessions {
            tracing::warn!("[{}] Session cap reached, rejecting {}", config.port, remote);
            return (StatusCode::SERVICE_UNAVAILABLE, "Server busy").into_response();
        }
    }

    let mut saved: Vec<String> = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("unknown").to_string();
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.unwrap_or_default();

        // Reject if field data exceeds 10 MiB
        if data.len() > DEFAULT_MAX_POST_BODY {
            tracing::warn!("[{}] Upload too large from {}", config.port, remote);
            return (StatusCode::PAYLOAD_TOO_LARGE, "Upload too large").into_response();
        }

        let safe_name = filename.replace(['\\', '/', ':'], "_");

        let filepath = save_capture("upload", remote, &safe_name, &data, "bin").await;
        tracing::info!(
            "[{}] Uploaded file '{}' ({} bytes) from {} saved to {:?}",
            config.port,
            filename,
            data.len(),
            remote,
            filepath
        );
        saved.push(format!("{} -> {:?}", safe_name, filepath));

        let meta = format!(
            "Upload from: {}\nPath: {}\nField: {}\nFilename: {}\nSize: {} bytes\n",
            remote,
            path,
            field_name,
            filename,
            data.len()
        );
        let meta_path =
            save_capture("upload_meta", remote, &safe_name, meta.as_bytes(), "txt").await;
        tracing::info!("Upload metadata saved to {:?}", meta_path);

        // Insert uploaded file into the session filesystem
        let upload_path = format!(
            "/cpsess{}/uploads/{}",
            token.trim_start_matches("cpsess"),
            safe_name
        );
        let mut sessions = config.sessions.write().await;
        let session = sessions
            .entry(token.clone())
            .or_insert_with(|| ShellSession::new(config.max_vfs_bytes));
        session.vfs.write(&upload_path, data.to_vec(), 0o644).ok();
    }

    // Evict old sessions if needed
    evict_sessions_if_needed(&config).await;

    (
        StatusCode::OK,
        [("Content-Type", "application/json")],
        format!(
            r#"{{"status":1,"msg":"Upload complete","files":["{}"]}}"#,
            saved.join("\",\""))
    ).into_response()
}
