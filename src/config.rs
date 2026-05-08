use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Instant};

use rand::{distr::Alphanumeric, Rng};
use tokio::sync::RwLock;

use crate::{events::EventLogHandle, shell::ShellSession};

/// Directory where captured payloads are written.
pub const CAPTURES_DIR: &str = "captures";

pub type SessionMap = Arc<RwLock<HashMap<String, ShellSession>>>;

/// Shared application state cloned into every Axum handler.
#[derive(Clone)]
pub struct AppConfig {
    /// The port this server instance is bound to.
    pub port: u16,
    /// Ports that should use the `whostmgrsession` cookie instead of `cpsession`.
    pub whm_ports: Vec<u16>,
    /// Per-token fake shell sessions.
    pub sessions: SessionMap,
    /// Per-IP rate limit tracking: (last_request_time, request_count).
    pub rate_limits: Arc<RwLock<HashMap<IpAddr, (Instant, u32)>>>,
    /// Structured event logger.
    pub event_log: Option<EventLogHandle>,
    /// Maximum concurrent sessions
    pub max_sessions: usize,
    /// Maximum VFS bytes per session
    pub max_vfs_bytes: usize,
    /// Rate limit (requests per minute)
    pub rate_limit: u32,
    /// Maximum disk usage for captures in MB
    pub max_captures_disk_mb: u64,
}

/// Returns a cryptographically-random alphanumeric string of `len` characters.
pub fn random_id(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Determines the correct session-cookie name for a given port / path combination.
pub fn cookie_name(port: u16, path: &str, whm_ports: &[u16]) -> &'static str {
    if path.starts_with("/___proxy_subdomain_whm") {
        "whostmgrsession"
    } else if path.starts_with("/___proxy_subdomain_cpanel") {
        "cpsession"
    } else if whm_ports.contains(&port) {
        "whostmgrsession"
    } else {
        "cpsession"
    }
}
