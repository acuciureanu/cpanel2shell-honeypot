#![forbid(unsafe_code)]

mod capture;
mod cli;
mod config;
mod events;
mod handlers;
mod limits;
mod router;
mod sessions;
mod shell;
mod tls;

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use clap::Parser;
use tokio::{net::TcpListener, sync::RwLock};
use tracing::{Level, info, warn};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use capture::ensure_captures_dir;
use cli::Args;
use config::{AppConfig, CAPTURES_DIR, cookie_name};
use router::make_router;

fn main() {
    rustls::crypto::ring::default_provider().install_default().ok();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
        .block_on(async_main());
}

async fn async_main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .with_span_events(FmtSpan::CLOSE)
        .init();

    info!("Starting cPanel2Shell honeypot (CVE-2026-41940)");
    ensure_captures_dir().await;
    info!("Captures will be saved to ./{}/", CAPTURES_DIR);

    // Shared session map (all ports share it so uploads are visible to the terminal).
    let shared_sessions = Arc::new(RwLock::new(HashMap::new()));

    // Load persisted sessions.
    if !args.no_session_persist {
        sessions::load_sessions(&shared_sessions, args.session_ttl_days).await;
        sessions::start_flush_task(shared_sessions.clone(), Duration::from_secs(5));
    }

    let tls_config = if !args.no_tls {
        Some(
            tls::build(
                args.certs_config.as_deref(),
                args.cert.as_deref(),
                args.key.as_deref(),
            )
            .await,
        )
    } else {
        info!("TLS disabled, running plain HTTP");
        None
    };

    // Initialize event logger
    let event_log = match crate::events::EventLogger::new("captures/events.jsonl").await {
        Ok(logger) => {
            info!("Event logging enabled: captures/events.jsonl");
            let logger_arc = Arc::new(logger);
            let logger_clone = logger_arc.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    if let Err(e) = logger_clone.flush().await {
                        warn!("Failed to flush event log: {}", e);
                    }
                }
            });
            Some(logger_arc)
        }
        Err(e) => {
            warn!("Failed to initialize event logger: {}", e);
            None
        }
    };

    let mut handles = vec![];
    let ports = args.ports.clone();
    let whm_ports = args.whm_ports.clone();

    for port in ports {
        let config = Arc::new(AppConfig {
            port,
            whm_ports: whm_ports.clone(),
            sessions: shared_sessions.clone(),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            event_log: event_log.clone(),
            max_sessions: args.max_sessions,
            max_vfs_bytes: args.max_vfs_mb * 1024 * 1024,
            rate_limit: args.rate_limit,
        });
        let router = make_router(config);
        let addr = format!("{}:{}", args.bind, port);
        let bind_addr: SocketAddr = addr.parse().expect("invalid bind address");
        let use_tls = tls_config.clone();
        let whm_ports_clone = whm_ports.clone();

        let handle = tokio::spawn(async move {
            if let Some(tls) = use_tls {
                let cn = cookie_name(port, "/", &whm_ports_clone);
                info!("Listening on https://{} (default cookie: {})", addr, cn);
                let app = router.into_make_service_with_connect_info::<SocketAddr>();
                if let Err(e) = axum_server::bind_rustls(bind_addr, tls).serve(app).await {
                    warn!("Server error on {}: {}", addr, e);
                }
            } else {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        warn!("Failed to bind {}: {}", addr, e);
                        return;
                    }
                };
                let cn = cookie_name(port, "/", &whm_ports_clone);
                info!("Listening on http://{} (default cookie: {})", addr, cn);
                let app = router.into_make_service_with_connect_info::<SocketAddr>();
                if let Err(e) = axum::serve(listener, app).await {
                    warn!("Server error on {}: {}", addr, e);
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.ok();
    }
}
