//! Bash-subset interpreter operating on an in-memory VFS.

mod builtins;
mod commands;
mod exec;
mod expand;
pub mod init;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vfs_tests;
pub mod vfs;

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use vfs::Vfs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    pub current_dir: String,
    pub vfs: Vfs,
    pub env: HashMap<String, String>,
    pub aliases: HashMap<String, String>,
    pub history: Vec<String>,
    pub last_status: i32,
    /// Synthetic shell PID — deterministic per session, used by $$ and ps.
    pub pid: u32,
    pub created_at: i64,
    pub last_seen_at: i64,
}

impl ShellSession {
    pub fn new(max_vfs_bytes: usize) -> Self {
        let mut vfs = Vfs::new();
        vfs.max_bytes = max_vfs_bytes;
        let mut s = Self {
            current_dir: "/root".into(),
            vfs,
            env: HashMap::new(),
            aliases: HashMap::new(),
            history: Vec::new(),
            last_status: 0,
            pid: rand::rng().random_range(2000..30000),
            created_at: chrono::Utc::now().timestamp(),
            last_seen_at: chrono::Utc::now().timestamp(),
        };
        init::populate(&mut s);
        init::init_vfs_quota(&mut s);
        s
    }

    /// Execute a (possibly compound) command line and return combined stdout+stderr.
    pub fn exec(&mut self, cmd: &str) -> String {
        self.last_seen_at = chrono::Utc::now().timestamp();
        exec::run(self, cmd)
    }
}
