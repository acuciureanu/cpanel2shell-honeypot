//! Session persistence: periodic snapshots to disk + reload on startup.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    limits::{MAX_CAPTURES_DISK_MB, check_disk_quota},
    shell::ShellSession,
};

const SESSION_DIR: &str = "captures/sessions";
const ARCHIVE_DIR: &str = "captures/sessions/archive";

pub type SessionMap = Arc<RwLock<HashMap<String, ShellSession>>>;

/// Load all unexpired session snapshots from disk into `sessions`.
pub async fn load_sessions(sessions: &SessionMap, ttl_days: u32) {
    let path = Path::new(SESSION_DIR);
    if !path.exists() {
        return;
    }
    let cutoff = chrono::Utc::now().timestamp() - (ttl_days as i64 * 86400);
    let mut loaded = 0usize;
    let mut archived = 0usize;

    let dir = match tokio::fs::read_dir(path).await {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut dir = dir;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if p.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.contains("archive"))
            .unwrap_or(false)
        {
            continue;
        }
        let data = match tokio::fs::read(&p).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let session: ShellSession = match serde_json::from_slice(&data) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to deserialize session {:?}: {}", p, e);
                continue;
            }
        };
        if session.last_seen_at < cutoff {
            tokio::fs::create_dir_all(ARCHIVE_DIR).await.ok();
            if let Some(name) = p.file_name() {
                let archive_path = Path::new(ARCHIVE_DIR).join(name);
                tokio::fs::rename(&p, &archive_path).await.ok();
            }
            archived += 1;
            continue;
        }
        let token = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        sessions.write().await.insert(token, session);
        loaded += 1;
    }
    if loaded > 0 || archived > 0 {
        info!(
            "Sessions loaded: {} active, {} archived (ttl={}d)",
            loaded, archived, ttl_days
        );
    }
}

/// Spawn a background task that flushes dirty sessions every `interval`.
pub fn start_flush_task(sessions: SessionMap, interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            flush_all(&sessions).await;
        }
    });
}

async fn flush_all(sessions: &SessionMap) {
    // Check disk quota before flushing
    if !check_disk_quota().await {
        warn!("Disk quota exceeded ({} MB), skipping session flush", MAX_CAPTURES_DISK_MB);
        return;
    }

    tokio::fs::create_dir_all(SESSION_DIR).await.ok();
    let snapshot = {
        let map = sessions.read().await;
        map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, ShellSession)>>()
    };
    for (token, session) in snapshot {
        if let Err(e) = flush_session(&token, &session).await {
            warn!("Failed to flush session {}: {}", token, e);
        }
    }
}

pub async fn flush_session(
    token: &str,
    session: &ShellSession,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(SESSION_DIR).await?;
    let json = serde_json::to_vec_pretty(session)?;

    // Atomic write: tempfile → rename
    let final_path = PathBuf::from(SESSION_DIR).join(format!("{}.json", token));
    let tmp_path = final_path.with_extension("json.tmp");
    tokio::fs::write(&tmp_path, &json).await?;
    tokio::fs::rename(&tmp_path, &final_path).await?;
    Ok(())
}
