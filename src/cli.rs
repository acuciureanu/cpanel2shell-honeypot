use clap::Parser;

/// CLI arguments for the honeypot server.
#[derive(Parser, Debug)]
#[command(name = "cpanel2shell-honeypot")]
#[command(about = "Honeypot for CVE-2026-41940 (cPanel2Shell) scanners")]
pub struct Args {
    /// Ports to listen on (comma-separated)
    #[arg(short, long, value_delimiter = ',', default_values = &["2087", "2083"])]
    pub ports: Vec<u16>,

    /// Address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Disable TLS and use plain HTTP
    #[arg(long)]
    pub no_tls: bool,

    /// Ports that use the whostmgrsession cookie (comma-separated)
    #[arg(long, value_delimiter = ',', default_values = &["2087"])]
    pub whm_ports: Vec<u16>,

    /// Path to TLS certificate PEM file (legacy; prefer --certs-config)
    #[arg(long)]
    pub cert: Option<String>,

    /// Path to TLS private key PEM file (legacy; prefer --certs-config)
    #[arg(long)]
    pub key: Option<String>,

    /// Path to TOML TLS config file (SNI-aware, hot-reload, per-host certs)
    #[arg(long)]
    pub certs_config: Option<String>,

    /// Session TTL in days; older snapshots are archived (default: 30)
    #[arg(long, default_value = "30")]
    pub session_ttl_days: u32,

    /// Disable session persistence (in-memory only, lost on restart)
    #[arg(long)]
    pub no_session_persist: bool,

    /// Maximum concurrent sessions (default: 10000)
    #[arg(long, default_value = "10000")]
    pub max_sessions: usize,

    /// Maximum VFS size per session in MB (default: 50)
    #[arg(long, default_value = "50")]
    pub max_vfs_mb: usize,

    /// Rate limit: max requests per IP per minute (default: 120)
    #[arg(long, default_value = "120")]
    pub rate_limit: u32,

    /// Path to JSONL event log file (default: captures/events.jsonl)
    #[arg(long, default_value = "captures/events.jsonl")]
    pub event_log: String,

    /// Maximum disk usage for captures in MB (default: 5000)
    #[arg(long, default_value = "5000")]
    pub max_captures_disk_mb: u64,
}
