//! Structured JSONL event logging for SIEM ingestion.

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

use chrono::Utc;
use serde::Serialize;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    sync::Mutex,
};

/// Types of honeypot events.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    AuthProbe,
    Command,
    Upload,
    Scan,
    RawPost,
    Request,
}

/// A structured honeypot event.
#[derive(Debug, Clone, Serialize)]
pub struct HoneypotEvent {
    pub timestamp: String,
    pub event_type: EventType,
    pub source_ip: String,
    pub source_port: u16,
    pub server_port: u16,
    pub user_agent: Option<String>,
    pub method: String,
    pub path: String,
    pub session_token: Option<String>,
    pub payload: Option<String>,
    pub payload_hash: Option<String>,
    pub response_status: u16,
    pub extra: Option<HashMap<String, String>>,
}

impl HoneypotEvent {
    pub fn new(
        event_type: EventType,
        remote: SocketAddr,
        server_port: u16,
        method: &str,
        path: &str,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type,
            source_ip: remote.ip().to_string(),
            source_port: remote.port(),
            server_port,
            user_agent: None,
            method: method.to_string(),
            path: path.to_string(),
            session_token: None,
            payload: None,
            payload_hash: None,
            response_status: 200,
            extra: None,
        }
    }
}

/// Event logger that appends JSON lines to a file.
pub struct EventLogger {
    file: Mutex<File>,
    count: Mutex<u64>,
}

impl EventLogger {
    pub async fn new(path: &str) -> Result<Self, std::io::Error> {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        Ok(Self {
            file: Mutex::new(file),
            count: Mutex::new(0),
        })
    }

    /// Append an event to the JSONL log.
    pub async fn log(&self, event: &HoneypotEvent) -> Result<(), std::io::Error> {
        let line = serde_json::to_string(event)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;

        let mut count = self.count.lock().await;
        *count += 1;

        // Flush every 100 events
        if *count % 100 == 0 {
            file.flush().await?;
        }

        Ok(())
    }
    pub async fn flush(&self) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.flush().await
    }
}

/// Shared event logger handle.
pub type EventLogHandle = Arc<EventLogger>;
