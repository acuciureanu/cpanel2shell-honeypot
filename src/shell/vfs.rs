//! Virtual filesystem.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    File(Vec<u8>),
    Dir(BTreeMap<String, Inode>),
    Symlink(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub kind: NodeKind,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub mtime: i64,
    pub atime: i64,
    pub ctime: i64,
    pub nlink: u32,
}

impl Inode {
    pub fn dir(mode: u16, uid: u32, gid: u32) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            kind: NodeKind::Dir(BTreeMap::new()),
            mode,
            uid,
            gid,
            mtime: now,
            atime: now,
            ctime: now,
            nlink: 2,
        }
    }
    pub fn file(data: Vec<u8>, mode: u16, uid: u32, gid: u32) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            kind: NodeKind::File(data),
            mode,
            uid,
            gid,
            mtime: now,
            atime: now,
            ctime: now,
            nlink: 1,
        }
    }
    pub fn symlink(target: String, uid: u32, gid: u32) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            kind: NodeKind::Symlink(target),
            mode: 0o777,
            uid,
            gid,
            mtime: now,
            atime: now,
            ctime: now,
            nlink: 1,
        }
    }
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, NodeKind::Dir(_))
    }
    pub fn is_file(&self) -> bool {
        matches!(self.kind, NodeKind::File(_))
    }
    pub fn is_symlink(&self) -> bool {
        matches!(self.kind, NodeKind::Symlink(_))
    }
    pub fn size(&self) -> usize {
        match &self.kind {
            NodeKind::File(d) => d.len(),
            NodeKind::Dir(_) => 4096,
            NodeKind::Symlink(t) => t.len(),
        }
    }
}

#[derive(Debug)]
pub enum VfsError {
    NotFound,
    NotADir,
    IsADir,
    Exists,
    InvalidPath,
    QuotaExceeded,
}

impl VfsError {
    pub fn message(&self, path: &str) -> String {
        match self {
            VfsError::NotFound => format!("{}: No such file or directory", path),
            VfsError::NotADir => format!("{}: Not a directory", path),
            VfsError::IsADir => format!("{}: Is a directory", path),
            VfsError::Exists => format!("{}: File exists", path),
            VfsError::InvalidPath => format!("{}: Invalid path", path),
            VfsError::QuotaExceeded => format!("{}: Disk quota exceeded", path),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vfs {
    pub root: Inode,
    #[serde(default)]
    pub total_bytes: usize,
    #[serde(default = "crate::limits::default_max_vfs_bytes")]
    pub max_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: usize,
    pub mtime: i64,
    pub nlink: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
}

impl Default for Vfs {
    fn default() -> Self {
        Self {
            root: Inode::dir(0o755, 0, 0),
            total_bytes: 0,
            max_bytes: crate::limits::default_max_vfs_bytes(),
        }
    }
}

impl Vfs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn split(path: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for seg in path.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    out.pop();
                }
                s => out.push(s.to_string()),
            }
        }
        out
    }

    pub fn canonicalize(cwd: &str, path: &str) -> String {
        let abs = if path.starts_with('/') {
            path.to_string()
        } else if path == "~" {
            "/root".to_string()
        } else if let Some(rest) = path.strip_prefix("~/") {
            format!("/root/{}", rest)
        } else {
            format!("{}/{}", cwd.trim_end_matches('/'), path)
        };
        let parts = Self::split(&abs);
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }

    pub fn lookup(&self, path: &str) -> Result<&Inode, VfsError> {
        let parts = Self::split(path);
        let mut node = &self.root;
        for p in &parts {
            match &node.kind {
                NodeKind::Dir(map) => {
                    node = map.get(p).ok_or(VfsError::NotFound)?;
                }
                _ => return Err(VfsError::NotADir),
            }
        }
        Ok(node)
    }

    pub fn lookup_mut(&mut self, path: &str) -> Result<&mut Inode, VfsError> {
        let parts = Self::split(path);
        let mut node = &mut self.root;
        for p in &parts {
            let entry = match &mut node.kind {
                NodeKind::Dir(map) => map.get_mut(p).ok_or(VfsError::NotFound)?,
                _ => return Err(VfsError::NotADir),
            };
            node = entry;
        }
        Ok(node)
    }

    pub fn exists(&self, path: &str) -> bool {
        self.lookup(path).is_ok()
    }

    pub fn is_dir(&self, path: &str) -> bool {
        self.lookup(path).map(|n| n.is_dir()).unwrap_or(false)
    }

    pub fn is_file(&self, path: &str) -> bool {
        self.lookup(path).map(|n| n.is_file()).unwrap_or(false)
    }

    pub fn read(&self, path: &str) -> Result<&[u8], VfsError> {
        let node = self.lookup(path)?;
        match &node.kind {
            NodeKind::File(d) => Ok(d.as_slice()),
            NodeKind::Dir(_) => Err(VfsError::IsADir),
            NodeKind::Symlink(target) => {
                let resolved = Self::canonicalize("/", target);
                self.read_owned(&resolved)
            }
        }
    }

    fn read_owned(&self, path: &str) -> Result<&[u8], VfsError> {
        let node = self.lookup(path)?;
        match &node.kind {
            NodeKind::File(d) => Ok(d.as_slice()),
            NodeKind::Dir(_) => Err(VfsError::IsADir),
            NodeKind::Symlink(_) => Err(VfsError::InvalidPath),
        }
    }

    pub fn list(&self, path: &str) -> Result<Vec<DirEntry>, VfsError> {
        let node = self.lookup(path)?;
        match &node.kind {
            NodeKind::Dir(map) => {
                let mut out = Vec::with_capacity(map.len());
                for (name, child) in map {
                    let kind = if child.is_dir() {
                        EntryKind::Dir
                    } else if child.is_symlink() {
                        EntryKind::Symlink
                    } else {
                        EntryKind::File
                    };
                    out.push(DirEntry {
                        name: name.clone(),
                        kind,
                        mode: child.mode,
                        uid: child.uid,
                        gid: child.gid,
                        size: child.size(),
                        mtime: child.mtime,
                        nlink: child.nlink,
                    });
                }
                Ok(out)
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn write(&mut self, path: &str, data: Vec<u8>, mode: u16) -> Result<(), VfsError> {
        let new_bytes = data.len();
        if self.total_bytes + new_bytes > self.max_bytes {
            return Err(VfsError::QuotaExceeded);
        }
        let parts = Self::split(path);
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        let (filename, dir_parts) = parts.split_last().unwrap();
        let mut node = &mut self.root;
        for p in dir_parts {
            let next = match &mut node.kind {
                NodeKind::Dir(map) => map
                    .entry(p.clone())
                    .or_insert_with(|| Inode::dir(0o755, 0, 0)),
                _ => return Err(VfsError::NotADir),
            };
            node = next;
        }
        match &mut node.kind {
            NodeKind::Dir(map) => {
                // If overwriting, subtract old size
                if let Some(old) = map.get(filename) {
                    self.total_bytes -= old.size();
                }
                let inode = Inode::file(data, mode, 0, 0);
                self.total_bytes += new_bytes;
                map.insert(filename.clone(), inode);
                Ok(())
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn append(&mut self, path: &str, data: &[u8], mode: u16) -> Result<(), VfsError> {
        if let Ok(existing) = self.read(path) {
            let mut combined = existing.to_vec();
            combined.extend_from_slice(data);
            self.write(path, combined, mode)
        } else {
            self.write(path, data.to_vec(), mode)
        }
    }

    pub fn mkdir(&mut self, path: &str, mode: u16) -> Result<(), VfsError> {
        let parts = Self::split(path);
        if parts.is_empty() {
            return Err(VfsError::Exists);
        }
        let (name, dir_parts) = parts.split_last().unwrap();
        let mut node = &mut self.root;
        for p in dir_parts {
            let next = match &mut node.kind {
                NodeKind::Dir(map) => map.get_mut(p).ok_or(VfsError::NotFound)?,
                _ => return Err(VfsError::NotADir),
            };
            node = next;
        }
        match &mut node.kind {
            NodeKind::Dir(map) => {
                if map.contains_key(name) {
                    return Err(VfsError::Exists);
                }
                map.insert(name.clone(), Inode::dir(mode, 0, 0));
                Ok(())
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn mkdir_p(&mut self, path: &str, mode: u16) -> Result<(), VfsError> {
        let parts = Self::split(path);
        let mut node = &mut self.root;
        for p in &parts {
            let next = match &mut node.kind {
                NodeKind::Dir(map) => {
                    if !map.contains_key(p) {
                        map.insert(p.clone(), Inode::dir(mode, 0, 0));
                    }
                    map.get_mut(p).unwrap()
                }
                _ => return Err(VfsError::NotADir),
            };
            node = next;
        }
        Ok(())
    }

    pub fn unlink(&mut self, path: &str) -> Result<(), VfsError> {
        let parts = Self::split(path);
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        let (name, dir_parts) = parts.split_last().unwrap();
        let mut node = &mut self.root;
        for p in dir_parts {
            let next = match &mut node.kind {
                NodeKind::Dir(map) => map.get_mut(p).ok_or(VfsError::NotFound)?,
                _ => return Err(VfsError::NotADir),
            };
            node = next;
        }
        match &mut node.kind {
            NodeKind::Dir(map) => {
                let removed = map.remove(name).ok_or(VfsError::NotFound)?;
                if removed.is_dir() {
                    map.insert(name.clone(), removed);
                    return Err(VfsError::IsADir);
                }
                self.total_bytes = self.total_bytes.saturating_sub(removed.size());
                Ok(())
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn rmdir(&mut self, path: &str, recursive: bool) -> Result<(), VfsError> {
        let parts = Self::split(path);
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        let (name, dir_parts) = parts.split_last().unwrap();
        let mut node = &mut self.root;
        for p in dir_parts {
            let next = match &mut node.kind {
                NodeKind::Dir(map) => map.get_mut(p).ok_or(VfsError::NotFound)?,
                _ => return Err(VfsError::NotADir),
            };
            node = next;
        }
        match &mut node.kind {
            NodeKind::Dir(map) => {
                let entry = map.get(name).ok_or(VfsError::NotFound)?;
                if let NodeKind::Dir(children) = &entry.kind {
                    if !recursive && !children.is_empty() {
                        return Err(VfsError::Exists);
                    }
                } else {
                    return Err(VfsError::NotADir);
                }
                let removed = map.remove(name).unwrap();
                self.total_bytes = self.total_bytes.saturating_sub(removed.size());
                Ok(())
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), VfsError> {
        let node = self.lookup(from)?.clone();
        // Remove source
        let parts = Self::split(from);
        let (name, dir_parts) = parts.split_last().ok_or(VfsError::InvalidPath)?;
        let mut cur = &mut self.root;
        for p in dir_parts {
            cur = match &mut cur.kind {
                NodeKind::Dir(m) => m.get_mut(p).ok_or(VfsError::NotFound)?,
                _ => return Err(VfsError::NotADir),
            };
        }
        if let NodeKind::Dir(m) = &mut cur.kind {
            m.remove(name);
        }
        // Insert at dest
        self.insert_inode(to, node)
    }

    pub fn copy(&mut self, from: &str, to: &str) -> Result<(), VfsError> {
        let node = self.lookup(from)?.clone();
        self.insert_inode(to, node)
    }

    pub fn symlink(&mut self, target: &str, linkpath: &str) -> Result<(), VfsError> {
        self.insert_inode(linkpath, Inode::symlink(target.to_string(), 0, 0))
    }

    pub fn chmod(&mut self, path: &str, mode: u16) -> Result<(), VfsError> {
        let n = self.lookup_mut(path)?;
        n.mode = mode;
        Ok(())
    }

    pub fn chown(&mut self, path: &str, uid: u32, gid: u32) -> Result<(), VfsError> {
        let n = self.lookup_mut(path)?;
        n.uid = uid;
        n.gid = gid;
        Ok(())
    }

    fn insert_inode(&mut self, path: &str, node: Inode) -> Result<(), VfsError> {
        let parts = Self::split(path);
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        let (name, dir_parts) = parts.split_last().unwrap();
        let mut cur = &mut self.root;
        for p in dir_parts {
            cur = match &mut cur.kind {
                NodeKind::Dir(m) => m
                    .entry(p.clone())
                    .or_insert_with(|| Inode::dir(0o755, 0, 0)),
                _ => return Err(VfsError::NotADir),
            };
        }
        match &mut cur.kind {
            NodeKind::Dir(m) => {
                m.insert(name.clone(), node);
                Ok(())
            }
            _ => Err(VfsError::NotADir),
        }
    }

    pub fn walk(&self) -> Vec<(String, EntryKind, usize)> {
        let mut out = Vec::new();
        Self::walk_inner(&self.root, "", &mut out);
        out
    }

    fn walk_inner(node: &Inode, prefix: &str, out: &mut Vec<(String, EntryKind, usize)>) {
        if let NodeKind::Dir(map) = &node.kind {
            for (name, child) in map {
                let path = if prefix.is_empty() {
                    format!("/{}", name)
                } else {
                    format!("{}/{}", prefix, name)
                };
                let kind = if child.is_dir() {
                    EntryKind::Dir
                } else if child.is_symlink() {
                    EntryKind::Symlink
                } else {
                    EntryKind::File
                };
                out.push((path.clone(), kind, child.size()));
                if child.is_dir() {
                    Self::walk_inner(child, &path, out);
                }
            }
        }
    }

    pub fn glob(&self, cwd: &str, pattern: &str) -> Vec<String> {
        let abs_pattern = if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            format!("{}/{}", cwd.trim_end_matches('/'), pattern)
        };
        let pat = match glob::Pattern::new(&abs_pattern) {
            Ok(p) => p,
            Err(_) => return vec![],
        };
        let mut hits: Vec<String> = self
            .walk()
            .into_iter()
            .map(|(p, _, _)| p)
            .filter(|p| pat.matches(p))
            .collect();
        hits.sort();
        hits
    }
}

pub fn format_mode(kind: EntryKind, mode: u16) -> String {
    let type_char = match kind {
        EntryKind::Dir => 'd',
        EntryKind::Symlink => 'l',
        EntryKind::File => '-',
    };
    let mut s = String::with_capacity(10);
    s.push(type_char);
    let bits = [
        (mode & 0o400 != 0, 'r'),
        (mode & 0o200 != 0, 'w'),
        (mode & 0o100 != 0, 'x'),
        (mode & 0o040 != 0, 'r'),
        (mode & 0o020 != 0, 'w'),
        (mode & 0o010 != 0, 'x'),
        (mode & 0o004 != 0, 'r'),
        (mode & 0o002 != 0, 'w'),
        (mode & 0o001 != 0, 'x'),
    ];
    for (on, c) in bits {
        s.push(if on { c } else { '-' });
    }
    s
}

pub fn user_for_uid(uid: u32) -> &'static str {
    match uid {
        0 => "root",
        1000 => "cpanel",
        27 => "mysql",
        48 => "apache",
        99 => "nobody",
        _ => "user",
    }
}
pub fn group_for_gid(gid: u32) -> &'static str {
    match gid {
        0 => "root",
        1000 => "cpanel",
        27 => "mysql",
        48 => "apache",
        99 => "nobody",
        _ => "user",
    }
}
