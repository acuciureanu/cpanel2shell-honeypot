use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::to_bytes,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    capture::{log_request, save_capture},
    config::AppConfig,
    limits::{DEFAULT_MAX_POST_BODY, check_rate_limit, evict_sessions_if_needed},
    shell::ShellSession,
    events::EventType,
};

pub async fn handle_terminal(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(token): Path<String>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    body: axum::body::Body,
) -> Response {
    let path = uri.path();

    // Rate limiting
    if !check_rate_limit(&config, remote.ip()).await {
        tracing::warn!("[{}] Rate limit exceeded for {}", config.port, remote.ip());
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }

    log_request(&config, EventType::Command, remote, "POST", path, &headers, None).await;

    let bytes = match to_bytes(body, DEFAULT_MAX_POST_BODY).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to read body: {}", e);
            return (StatusCode::PAYLOAD_TOO_LARGE, "Too large").into_response();
        }
    };

    let cmd = String::from_utf8_lossy(&bytes);
    tracing::info!(
        "[{}] Terminal command from {}: {}",
        config.port,
        remote,
        cmd
    );

    // Check session cap
    {
        let sessions = config.sessions.read().await;
        if sessions.len() >= config.max_sessions {
            tracing::warn!("[{}] Session cap reached, rejecting {}", config.port, remote);
            return (StatusCode::SERVICE_UNAVAILABLE, "Server busy").into_response();
        }
    }

    let mut sessions = config.sessions.write().await;
    let session = sessions
        .entry(token.clone())
        .or_insert_with(|| ShellSession::new(config.max_vfs_bytes));
    let output = session.exec(&cmd);
    drop(sessions);

    // Evict old sessions if needed
    evict_sessions_if_needed(&config).await;

    let capture_data = format!("{}\n{}\n", cmd, output);
    let filepath = save_capture("cmd", remote, "terminal", capture_data.as_bytes(), "txt").await;
    tracing::info!("Command captured to {:?}", filepath);

    (StatusCode::OK, [("Content-Type", "text/plain")], output).into_response()
}
