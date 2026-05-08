//! TLS certificate configuration schema.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Default behaviour when no [[host]] entry matches: "auto" or "none".
    #[serde(default = "default_mode")]
    pub default: DefaultMode,

    /// Per-SNI host entries.
    #[serde(default)]
    pub host: Vec<HostEntry>,

    /// Global auto-generation settings (used as fallback).
    #[serde(default)]
    pub auto: AutoSettings,
}

fn default_mode() -> DefaultMode {
    DefaultMode::Auto
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultMode {
    Auto,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    /// SNI hostname; supports simple `*.example.com` wildcards.
    pub sni: String,

    /// If set, load cert/key from these PEM files.
    pub cert: Option<String>,
    pub key: Option<String>,

    /// If set to "auto" (or cert/key absent), auto-generate.
    #[serde(default)]
    pub mode: HostMode,

    /// Extra SANs to add when auto-generating.
    #[serde(default)]
    pub sans: Vec<String>,

    /// Override validity period (days).
    pub validity_days: Option<u32>,

    /// Key type: "rsa-2048", "rsa-4096", "ecdsa-p256", "ed25519".
    pub key_type: Option<String>,

    /// Spoof issuer CN in the generated cert.
    pub issuer_cn: Option<String>,

    /// Backdate not_before for realism (RFC 3339).
    pub not_before: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostMode {
    #[default]
    Auto,
    Pinned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSettings {
    /// Where to cache generated certs on disk.
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde(default = "default_key_type")]
    pub key_type: String,

    #[serde(default = "default_validity")]
    pub validity_days: u32,

    #[serde(default = "default_issuer_cn")]
    pub issuer_cn: String,
}

fn default_cache_dir() -> String {
    "./captures/certs".into()
}
fn default_key_type() -> String {
    "rsa-2048".into()
}
fn default_validity() -> u32 {
    365
}
fn default_issuer_cn() -> String {
    "Let's Encrypt Authority X3".into()
}

impl Default for AutoSettings {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            key_type: default_key_type(),
            validity_days: default_validity(),
            issuer_cn: default_issuer_cn(),
        }
    }
}

impl TlsConfig {
    pub fn load(path: &str) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        toml::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Find the best matching HostEntry for the given SNI (exact > wildcard > None).
    pub fn match_sni(&self, sni: &str) -> Option<&HostEntry> {
        let mut wildcard_match: Option<&HostEntry> = None;
        for h in &self.host {
            if h.sni == sni {
                return Some(h);
            }
            if h.sni.starts_with("*.") {
                let suffix = &h.sni[1..]; // ".example.com"
                if sni.ends_with(suffix) {
                    wildcard_match = Some(h);
                }
            }
        }
        wildcard_match
    }
}
