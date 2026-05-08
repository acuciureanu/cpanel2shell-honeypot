//! SNI-aware dynamic certificate resolver implementing `rustls::server::ResolvesServerCert`.

use std::collections::HashMap;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use rustls::pki_types::CertificateDer;
use rustls::server::ClientHello;
use rustls::server::ResolvesServerCert;
use rustls::sign::{CertifiedKey, SigningKey};
use tracing::{info, warn};

use super::config::{DefaultMode, TlsConfig};
use super::generator;

pub struct DynamicResolver {
    config: ArcSwap<TlsConfig>,
    /// in-memory cert cache: SNI → CertifiedKey
    cache: Mutex<HashMap<String, Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for DynamicResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicResolver").finish_non_exhaustive()
    }
}

impl DynamicResolver {
    pub fn new(config: TlsConfig) -> Self {
        Self {
            config: ArcSwap::new(Arc::new(config)),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn reload(&self, new_config: TlsConfig) {
        info!("TLS config reloaded");
        self.config.store(Arc::new(new_config));
        // Flush in-memory cache; disk-cached certs survive (loaded on next miss).
        if let Ok(mut c) = self.cache.lock() {
            c.clear();
        }
    }

    fn get_or_build(&self, sni: &str) -> Option<Arc<CertifiedKey>> {
        // Cache hit
        if let Ok(c) = self.cache.lock()
            && let Some(ck) = c.get(sni)
        {
            return Some(ck.clone());
        }

        let cfg = self.config.load();

        // 1. Pinned host entry?
        let entry = cfg.match_sni(sni);
        let ck = if let Some(e) = entry {
            if let (Some(cert_path), Some(key_path)) = (&e.cert, &e.key) {
                load_pem_files(cert_path, key_path)
            } else {
                // auto-generate for this specific entry
                let g = generator::generate_for_sni(sni, Some(e), &cfg.auto);
                build_certified_key(&g.cert_pem, &g.key_pem)
            }
        } else {
            // No matching host entry — use default
            match cfg.default {
                DefaultMode::None => return None,
                DefaultMode::Auto => {
                    // Try disk cache first
                    let cache_dir = &cfg.auto.cache_dir;
                    let safe = sni.replace(['/', '\\', ':'], "_");
                    let cert_path = format!("{}/{}.cert.pem", cache_dir, safe);
                    let key_path = format!("{}/{}.key.pem", cache_dir, safe);
                    if std::path::Path::new(&cert_path).exists()
                        && std::path::Path::new(&key_path).exists()
                    {
                        load_pem_files(&cert_path, &key_path)
                    } else {
                        let g = generator::generate_for_sni(sni, None, &cfg.auto);
                        std::fs::create_dir_all(cache_dir).ok();
                        std::fs::write(&cert_path, &g.cert_pem).ok();
                        std::fs::write(&key_path, &g.key_pem).ok();
                        build_certified_key(&g.cert_pem, &g.key_pem)
                    }
                }
            }
        }?;

        let ck = Arc::new(ck);
        if let Ok(mut c) = self.cache.lock() {
            // LRU bound: drop oldest when over 1024
            if c.len() >= 1024
                && let Some(k) = c.keys().next().cloned()
            {
                c.remove(&k);
            }
            c.insert(sni.to_string(), ck.clone());
        }
        Some(ck)
    }
}

impl ResolvesServerCert for DynamicResolver {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = hello.server_name().unwrap_or("default");
        self.get_or_build(sni)
    }
}

impl DynamicResolver {
    /// Pre-load a cert as the "default" (no-SNI) fallback.
    pub fn preload_default(&self, cert_pem: &[u8], key_pem: &[u8]) {
        if let Some(ck) = build_certified_key(cert_pem, key_pem)
            && let Ok(mut c) = self.cache.lock()
        {
            c.insert("default".to_string(), Arc::new(ck));
        }
    }
}

fn load_pem_files(cert_path: &str, key_path: &str) -> Option<CertifiedKey> {
    let cert_bytes = std::fs::read(cert_path).ok()?;
    let key_bytes = std::fs::read(key_path).ok()?;
    build_certified_key(&cert_bytes, &key_bytes)
}

pub fn build_certified_key(cert_pem: &[u8], key_pem: &[u8]) -> Option<CertifiedKey> {
    let mut cert_reader = BufReader::new(cert_pem);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .filter_map(|c| c.ok())
        .map(|c| c.into_owned())
        .collect();
    if certs.is_empty() {
        warn!("No certificates found in PEM");
        return None;
    }

    let mut key_reader = BufReader::new(key_pem);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .ok()
        .flatten()?;

    let signing_key: Arc<dyn SigningKey> =
        rustls::crypto::ring::sign::any_supported_type(&key).ok()?;

    Some(CertifiedKey::new(certs, signing_key))
}
