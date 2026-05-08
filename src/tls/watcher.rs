//! Config file watcher: debounces filesystem events and reloads TLS config.

use std::{path::Path, sync::Arc, time::Duration};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::{config::TlsConfig, resolver::DynamicResolver};

/// Spawn a background task that watches `config_path` and reloads the resolver on change.
pub fn watch_config(config_path: String, resolver: Arc<DynamicResolver>) {
    let (tx, mut rx) = mpsc::channel::<()>(1);

    let path_clone = config_path.clone();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(ev) = res {
                match ev.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        tx.try_send(()).ok();
                    }
                    _ => {}
                }
            }
        })
        .expect("failed to create file watcher");

    let watch_dir = Path::new(&config_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
        warn!("TLS config watcher failed to start: {}", e);
        return;
    }

    tokio::spawn(async move {
        // Keep watcher alive.
        let _watcher = watcher;
        loop {
            if rx.recv().await.is_none() {
                break;
            }
            // Debounce: wait 500ms for any subsequent rapid events.
            tokio::time::sleep(Duration::from_millis(500)).await;
            // Drain remaining queued signals.
            while rx.try_recv().is_ok() {}

            info!("TLS config changed, reloading: {}", path_clone);
            match TlsConfig::load(&path_clone) {
                Ok(new_cfg) => {
                    resolver.reload(new_cfg);
                    info!("TLS config reloaded successfully");
                }
                Err(e) => {
                    warn!("TLS config reload failed (keeping old config): {}", e);
                }
            }
        }
    });
}
