use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
};
use rand::Rng;

use crate::{
    capture::log_request,
    config::{AppConfig, cookie_name, random_id},
    events::EventType,
};

pub async fn handle_login(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> impl IntoResponse {
    let path = uri.path();
    log_request(&config, EventType::AuthProbe, remote, "GET", path, &headers, None).await;

    let cn = cookie_name(config.port, path, &config.whm_ports);
    let cookie_value = format!("{},%25{}", random_id(16), random_id(32));
    let cookie = format!("{}={}; Path=/; HttpOnly; Secure", cn, cookie_value);

    (
        StatusCode::OK,
        AppendHeaders([("Set-Cookie", cookie)]),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>cPanel Login</title>
    <style>
        body { background: #f0f0f0; font-family: "Open Sans", sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
        .login-container { background: #fff; padding: 40px; border-radius: 4px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); width: 320px; text-align: center; }
        .logo { font-size: 24px; font-weight: bold; color: #f68e21; margin-bottom: 20px; }
        input { width: 100%; padding: 10px; margin-bottom: 15px; border: 1px solid #ccc; border-radius: 3px; box-sizing: border-box; }
        button { width: 100%; padding: 10px; background: #f68e21; color: #fff; border: none; border-radius: 3px; cursor: pointer; font-size: 16px; font-weight: bold; }
        button:hover { background: #e07d1a; }
        .footer { margin-top: 20px; font-size: 12px; color: #777; }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="logo">cPanel</div>
        <form action="/login" method="POST">
            <input type="text" name="user" placeholder="Username" required>
            <input type="password" name="pass" placeholder="Password" required>
            <button type="submit">Log in</button>
        </form>
        <div class="footer">Copyright &copy; 2026 cPanel, L.L.C.</div>
    </div>
</body>
</html>"#,
    )
}

pub async fn handle_root(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> Response {
    let path = uri.path();
    log_request(&config, EventType::AuthProbe, remote, "GET", path, &headers, None).await;

    let cn = cookie_name(config.port, path, &config.whm_ports);
    let has_cookie = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|c| c.contains(cn))
        .unwrap_or(false);

    if !has_cookie {
        tracing::warn!(
            "[{}] Missing {} cookie from {}, redirecting to login",
            config.port,
            cn,
            remote
        );
        return (
            StatusCode::FOUND,
            AppendHeaders([("Location", "/login".to_string())]),
            "Redirecting to login...",
        )
            .into_response();
    }

    let token = format!(
        "cpsess{}",
        rand::rng().random_range(100_000_000u32..999_999_999u32)
    );
    let base_path = if path == "/" {
        ""
    } else {
        path.trim_end_matches('/')
    };
    let location = format!("{}/{}/", base_path, token);

    tracing::info!(
        "[{}] Redirecting {} to {} (auth bypass attempt)",
        config.port,
        remote,
        location
    );

    (
        StatusCode::FOUND,
        AppendHeaders([("Location", location)]),
        "Redirecting...",
    )
        .into_response()
}

pub async fn handle_cpsess_get(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    axum::extract::Path(_token): axum::extract::Path<String>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> impl IntoResponse {
    let path = uri.path();
    log_request(&config, EventType::AuthProbe, remote, "GET", path, &headers, None).await;

    tracing::info!(
        "[{}] Serving expired_session marker to {} (scanner confirmed vulnerability check)",
        config.port,
        remote
    );

    (
        StatusCode::OK,
        [("Content-Type", "text/html; charset=utf-8")],
        "msg_code:[expired_session]\n<html><body>Session expired</body></html>",
    )
}
