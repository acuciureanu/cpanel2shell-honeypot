//! Resource limits and rate limiting for DoS protection.

use std::{
    net::IpAddr,
    time::{Duration, Instant},
};



/// Maximum VFS bytes per session (50 MiB).
pub const DEFAULT_MAX_VFS_BYTES_PER_SESSION: usize = 50 * 1024 * 1024;

pub fn default_max_vfs_bytes() -> usize {
    DEFAULT_MAX_VFS_BYTES_PER_SESSION
}




/// Maximum POST body size in bytes (10 MiB).
pub const DEFAULT_MAX_POST_BODY: usize = 10 * 1024 * 1024;



/// Check if a request from `ip` is within rate limits.
/// Returns `true` if the request should be allowed.
pub async fn check_rate_limit(config: &crate::config::AppConfig, ip: IpAddr) -> bool {
    let mut map = config.rate_limits.write().await;
    let now = Instant::now();
    let window = Duration::from_secs(60);

    // Clean up stale entries (older than 2 minutes)
    map.retain(|_, (t, _)| now.duration_since(*t) < Duration::from_secs(120));

    match map.get_mut(&ip) {
        Some((last, count)) => {
            if now.duration_since(*last) > window {
                // Window expired, reset
                *last = now;
                *count = 1;
                true
            } else {
                *count += 1;
                *count <= config.rate_limit
            }
        }
        None => {
            map.insert(ip, (now, 1));
            true
        }
    }
}

/// Check if the captures directory is within disk quota.
pub async fn check_disk_quota(max_mb: u64) -> bool {
    let captures_dir = std::path::Path::new(crate::config::CAPTURES_DIR);
    if !captures_dir.exists() {
        return true;
    }
    match dir_size_mb(captures_dir).await {
        Ok(size) => size < max_mb,
        Err(_) => true, // If we can't check, allow it
    }
}

/// Calculate directory size in bytes recursively.
async fn dir_size_mb(path: &std::path::Path) -> Result<u64, std::io::Error> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(current_path) = stack.pop() {
        let mut entries = tokio::fs::read_dir(current_path).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let meta = entry.metadata().await?;
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    Ok(total / (1024 * 1024))
}

/// Evict oldest sessions if we're over the soft limit (90% of max).
pub async fn evict_sessions_if_needed(config: &crate::config::AppConfig) {
    let mut map = config.sessions.write().await;
    let soft_limit = (config.max_sessions as f64 * 0.9) as usize;
    if map.len() <= soft_limit {
        return;
    }

    // Collect tokens sorted by last_seen_at (oldest first)
    let mut tokens: Vec<(String, i64)> = map
        .iter()
        .map(|(k, v)| (k.clone(), v.last_seen_at))
        .collect();
    tokens.sort_by_key(|(_, ts)| *ts);

    // Evict 10% of max sessions (the oldest ones)
    let evict_count = (config.max_sessions as f64 * 0.1) as usize;
    for (token, _) in tokens.into_iter().take(evict_count) {
        map.remove(&token);
    }
}
