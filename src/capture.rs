use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

use axum::http::HeaderMap;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::Local;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use tracing::info;

use crate::config::{AppConfig, CAPTURES_DIR};
use crate::events::{EventType, HoneypotEvent};

pub async fn ensure_captures_dir() {
    fs::create_dir_all(CAPTURES_DIR).await.ok();
}

pub async fn save_capture(
    prefix: &str,
    remote: SocketAddr,
    path: &str,
    data: &[u8],
    ext: &str,
) -> PathBuf {
    ensure_captures_dir().await;
    let ts = Local::now().format("%Y%m%d_%H%M%S_%3f");
    let safe_path = path.replace(['/', '\\'], "_");
    let filename = format!(
        "{}_{}_{}_{}.{}",
        prefix,
        ts,
        remote.ip().to_string().replace(':', "_"),
        &safe_path[..safe_path.len().min(40)],
        ext,
    );
    let filepath = PathBuf::from(CAPTURES_DIR).join(&filename);
    if let Ok(mut f) = File::create(&filepath).await {
        f.write_all(data).await.ok();
    }
    filepath
}

pub async fn log_request(
    config: &AppConfig,
    event_type: EventType,
    remote: SocketAddr,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    extra: Option<HashMap<&str, String>>,
) {
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let mut log = format!(
        "[{}] port={} remote={} method={} path={}",
        ts, config.port, remote, method, path
    );

    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        log.push_str(&format!(" auth=\"{}\"", auth));
        let maybe_decoded = auth
            .strip_prefix("Basic ")
            .and_then(|b64| BASE64.decode(b64).ok());
        if let Some(decoded) = maybe_decoded {
            let text = String::from_utf8_lossy(&decoded);
            let safe = text.replace('\n', "\\n").replace('\u{FF}', "\\xff");
            log.push_str(&format!(" auth_decoded=\"{}\"", safe));
        }
    }

    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        log.push_str(&format!(" cookie=\"{}\"", cookie));
    }
    if let Some(ua) = headers.get("user-agent").and_then(|v| v.to_str().ok()) {
        log.push_str(&format!(" ua=\"{}\"", ua));
    }
    if let Some(ct) = headers.get("content-type").and_then(|v| v.to_str().ok()) {
        log.push_str(&format!(" ct=\"{}\"", ct));
    }

    if let Some(ref ex) = extra {
        for (k, v) in ex {
            log.push_str(&format!(" {}=\"{}\"", k, v));
        }
    }

    info!("{}", log);

    // Also log to JSONL
    if let Some(ref event_log) = config.event_log {
        let mut event = HoneypotEvent::new(
            event_type,
            remote,
            config.port,
            method,
            path,
        );
        if let Some(ua) = headers.get("user-agent").and_then(|v| v.to_str().ok()) {
            event.user_agent = Some(ua.to_string());
        }
        if let Some(ref ex) = extra {
            let mut extra_map = HashMap::new();
            for (k, v) in ex {
                extra_map.insert(k.to_string(), v.to_string());
            }
            event.extra = Some(extra_map);
        }
        event_log.log(&event).await.ok();
    }
}

pub async fn log_post_body(remote: SocketAddr, path: &str, headers: &HeaderMap, body: &[u8]) {
    let mut preamble = format!("POST from {} to {}\n", remote, path);
    for (k, v) in headers.iter() {
        if let Ok(vs) = v.to_str() {
            preamble.push_str(&format!("{}: {}\n", k, vs));
        }
    }
    preamble.push_str("\n--- BODY ---\n");
    let mut full = preamble.into_bytes();
    full.extend_from_slice(body);

    let filepath = save_capture("post", remote, path, &full, "txt").await;
    info!("Captured POST body to {:?}", filepath);
}


