//! Dynamic TLS system: per-SNI cert resolution, auto-generation, config file, hot reload.

pub mod config;
pub mod generator;
pub mod resolver;
pub mod watcher;

use std::sync::Arc;

use rustls::ServerConfig;
use tracing::info;

use config::TlsConfig;
use resolver::DynamicResolver;

/// Build an `axum_server::tls_rustls::RustlsConfig` backed by our dynamic resolver.
///
/// * If `config_path` is `Some(p)`, load cert settings from a TOML file at `p`.
/// * If both `legacy_cert` and `legacy_key` are `Some`, use them as a pinned PEM pair
///   (backwards-compatible with the old `--cert`/`--key` flags).
/// * Otherwise generate a self-signed cert for `cpanel.local` + `localhost`.
pub async fn build(
    config_path: Option<&str>,
    legacy_cert: Option<&str>,
    legacy_key: Option<&str>,
) -> axum_server::tls_rustls::RustlsConfig {
    // Legacy shortcut: --cert / --key flags still work.
    if let (Some(cert), Some(key)) = (legacy_cert, legacy_key) {
        info!("Loading custom TLS certificate from {} and {}", cert, key);
        let cert_pem = tokio::fs::read(cert).await.expect("read cert failed");
        let key_pem = tokio::fs::read(key).await.expect("read key failed");
        return axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem)
            .await
            .expect("TLS config from legacy PEM failed");
    }

    let (tls_cfg, watch_path) = match config_path {
        Some(p) => {
            info!("Loading TLS config from {}", p);
            match TlsConfig::load(p) {
                Ok(c) => (c, Some(p.to_string())),
                Err(e) => {
                    tracing::warn!("Failed to load TLS config {}: {} — using defaults", p, e);
                    (default_tls_config(), None)
                }
            }
        }
        None => {
            info!("No TLS config; generating self-signed cert for cpanel.local + localhost");
            (default_tls_config(), None)
        }
    };

    let resolver = Arc::new(DynamicResolver::new(tls_cfg));

    // Pre-warm: eagerly generate and cache the default no-SNI cert.
    let g = generator::generate_simple(vec!["cpanel.local".into(), "localhost".into()]);
    resolver.preload_default(&g.cert_pem, &g.key_pem);

    // Start file watcher if config_path was provided.
    if let Some(p) = watch_path {
        watcher::watch_config(p, resolver.clone());
    }

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver);

    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(server_config))
}

fn default_tls_config() -> TlsConfig {
    TlsConfig {
        default: config::DefaultMode::Auto,
        host: vec![],
        auto: config::AutoSettings::default(),
    }
}
