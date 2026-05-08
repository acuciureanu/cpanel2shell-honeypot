use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::to_bytes,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    capture::{log_post_body, log_request},
    config::AppConfig,
    limits::DEFAULT_MAX_POST_BODY,
    events::EventType,
};

/// Serves any file that was previously uploaded into a fake session's filesystem.
pub async fn handle_file_serve(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path((token, subpath)): Path<(String, String)>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/cpsess{}/{}", token, subpath);
    log_request(&config, EventType::Request, remote, "GET", &path, &headers, None).await;

    let sessions = config.sessions.read().await;

    if let Some(session) = sessions.get(&token) {
        let file_path = crate::shell::vfs::Vfs::canonicalize(
            &session.current_dir,
            &format!("/cpsess{}/{}", token, subpath),
        );
        if let Ok(data) = session.vfs.read(&file_path) {
            tracing::info!(
                "[{}] Serving fake file {} to {}",
                config.port,
                file_path,
                remote
            );
            return (
                StatusCode::OK,
                [("Content-Type", "application/octet-stream")],
                data.to_vec(),
            )
                .into_response();
        }
    }

    tracing::warn!(
        "[{}] File not found {} for {}, returning 404",
        config.port,
        path,
        remote
    );
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

/// Captures the body of any POST request that did not match a specific route.
pub async fn handle_raw_post(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    body: axum::body::Body,
) -> Response {
    let path = uri.path();
    log_request(&config, EventType::RawPost, remote, "POST", path, &headers, None).await;

    let bytes = match to_bytes(body, DEFAULT_MAX_POST_BODY).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to read body: {}", e);
            return (StatusCode::PAYLOAD_TOO_LARGE, "Too large").into_response();
        }
    };

    log_post_body(remote, path, &headers, &bytes).await;

    (
        StatusCode::OK,
        [("Content-Type", "application/json")],
        r#"{"status":1,"msg":"OK"}"#,
    )
        .into_response()
}

/// Catch-all GET handler for any path that did not match a defined route.
pub async fn handle_fallback(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> impl IntoResponse {
    let path = uri.path();
    log_request(&config, EventType::Scan, remote, "GET", path, &headers, None).await;
    tracing::warn!(
        "[{}] Unhandled request from {} to {} - returning 404",
        config.port,
        remote,
        path
    );
    (StatusCode::NOT_FOUND, "Not Found")
}
