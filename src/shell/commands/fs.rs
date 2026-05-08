use super::{CmdResult, ShellSession};
use super::super::vfs::{EntryKind, Vfs};
use super::super::vfs::{format_mode, user_for_uid, group_for_gid};
use super::ts_to_ls;

pub fn cp(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let positional: Vec<&String> = argv[1..].iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() < 2 {
        return CmdResult::err(b"cp: missing file operand\n".to_vec(), 1);
    }
    let dst = positional.last().unwrap();
    let dst_abs = Vfs::canonicalize(&session.current_dir, dst);
    for src in &positional[..positional.len() - 1] {
        let src_abs = Vfs::canonicalize(&session.current_dir, src);
        let final_dst = if session.vfs.is_dir(&dst_abs) {
            let name = src.rsplit('/').next().unwrap_or(src);
            format!("{}/{}", dst_abs.trim_end_matches('/'), name)
        } else {
            dst_abs.clone()
        };
        if let Err(e) = session.vfs.copy(&src_abs, &final_dst) {
            return CmdResult::err(format!("cp: {}: {}\n", src, e.message(src)).into_bytes(), 1);
        }
    }
    CmdResult::empty()
}

pub fn mv(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let positional: Vec<&String> = argv[1..].iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() < 2 {
        return CmdResult::err(b"mv: missing file operand\n".to_vec(), 1);
    }
    let dst = positional.last().unwrap();
    let dst_abs = Vfs::canonicalize(&session.current_dir, dst);
    for src in &positional[..positional.len() - 1] {
        let src_abs = Vfs::canonicalize(&session.current_dir, src);
        let final_dst = if session.vfs.is_dir(&dst_abs) {
            let name = src.rsplit('/').next().unwrap_or(src);
            format!("{}/{}", dst_abs.trim_end_matches('/'), name)
        } else {
            dst_abs.clone()
        };
        if let Err(e) = session.vfs.rename(&src_abs, &final_dst) {
            return CmdResult::err(format!("mv: {}: {}\n", src, e.message(src)).into_bytes(), 1);
        }
    }
    CmdResult::empty()
}

pub fn rm(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let mut recursive = false;
    let mut force = false;
    let mut targets: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            for c in a.chars().skip(1) {
                match c {
                    'r' | 'R' => recursive = true,
                    'f' => force = true,
                    'v' => {}
                    _ => {}
                }
            }
        } else {
            targets.push(a.clone());
        }
    }
    let mut err = String::new();
    let mut status = 0;
    for t in &targets {
        let path = Vfs::canonicalize(&session.current_dir, t);
        if session.vfs.is_dir(&path) {
            if recursive {
                let _ = session.vfs.rmdir(&path, true);
            } else {
                err.push_str(&format!("rm: cannot remove '{}': Is a directory\n", t));
                status = 1;
            }
        } else if let Err(e) = session.vfs.unlink(&path)
            && !force
        {
            err.push_str(&format!("rm: cannot remove '{}': {}\n", t, e.message(t)));
            status = 1;
        }
    }
    CmdResult {
        stdout: vec![],
        stderr: err.into_bytes(),
        status,
    }
}

pub fn ln(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let mut symbolic = false;
    let mut positional: Vec<&String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            if a.contains('s') {
                symbolic = true;
            }
        } else {
            positional.push(a);
        }
    }
    if positional.len() < 2 {
        return CmdResult::err(b"ln: missing file operand\n".to_vec(), 1);
    }
    let target = positional[0];
    let link = Vfs::canonicalize(&session.current_dir, positional[1]);
    if symbolic {
        session.vfs.symlink(target, &link).ok();
    } else {
        let abs_target = Vfs::canonicalize(&session.current_dir, target);
        session.vfs.copy(&abs_target, &link).ok();
    }
    CmdResult::empty()
}

pub fn mkdir(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let mut parents = false;
    let mut targets: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            if a.contains('p') {
                parents = true;
            }
        } else {
            targets.push(a.clone());
        }
    }
    let mut err = String::new();
    let mut status = 0;
    for t in &targets {
        let path = Vfs::canonicalize(&session.current_dir, t);
        let res = if parents {
            session.vfs.mkdir_p(&path, 0o755)
        } else {
            session.vfs.mkdir(&path, 0o755)
        };
        if let Err(e) = res {
            err.push_str(&format!(
                "mkdir: cannot create directory '{}': {}\n",
                t,
                e.message(t)
            ));
            status = 1;
        }
    }
    CmdResult {
        stdout: vec![],
        stderr: err.into_bytes(),
        status,
    }
}

pub fn rmdir(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    for t in &argv[1..] {
        if t.starts_with('-') {
            continue;
        }
        let path = Vfs::canonicalize(&session.current_dir, t);
        session.vfs.rmdir(&path, false).ok();
    }
    CmdResult::empty()
}

pub fn touch(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    for t in &argv[1..] {
        if t.starts_with('-') {
            continue;
        }
        let path = Vfs::canonicalize(&session.current_dir, t);
        if !session.vfs.exists(&path) {
            session.vfs.write(&path, vec![], 0o644).ok();
        } else if let Ok(n) = session.vfs.lookup_mut(&path) {
            n.mtime = chrono::Utc::now().timestamp();
            n.atime = n.mtime;
        }
    }
    CmdResult::empty()
}

pub fn chmod(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 3 {
        return CmdResult::err(b"chmod: missing operand\n".to_vec(), 1);
    }
    let mode_s = &argv[1];
    let mode = u16::from_str_radix(mode_s, 8).unwrap_or(0o644);
    for t in &argv[2..] {
        let path = Vfs::canonicalize(&session.current_dir, t);
        session.vfs.chmod(&path, mode).ok();
    }
    CmdResult::empty()
}

pub fn chown(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 3 {
        return CmdResult::err(b"chown: missing operand\n".to_vec(), 1);
    }
    // we accept user[:group]
    let owner = &argv[1];
    let (uid, gid) = match owner.split_once(':') {
        Some((u, g)) => (uid_for(u), gid_for(g)),
        None => (uid_for(owner), 0),
    };
    for t in &argv[2..] {
        let path = Vfs::canonicalize(&session.current_dir, t);
        session.vfs.chown(&path, uid, gid).ok();
    }
    CmdResult::empty()
}

fn uid_for(name: &str) -> u32 {
    match name {
        "root" => 0,
        "cpanel" => 1000,
        "mysql" => 27,
        "apache" => 48,
        "nobody" => 99,
        _ => name.parse().unwrap_or(1000),
    }
}
fn gid_for(name: &str) -> u32 {
    uid_for(name)
}

pub fn stat(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::err(b"stat: missing operand\n".to_vec(), 1);
    }
    let mut out = String::new();
    for t in &argv[1..] {
        if t.starts_with('-') {
            continue;
        }
        let path = Vfs::canonicalize(&session.current_dir, t);
        match session.vfs.lookup(&path) {
            Ok(n) => {
                let kind = if n.is_dir() {
                    "directory"
                } else if n.is_symlink() {
                    "symbolic link"
                } else {
                    "regular file"
                };
                out.push_str(&format!(
                    "  File: '{}'\n  Size: {}\t\tBlocks: {}\tIO Block: 4096   {}\nDevice: 802h/2050d\tInode: 1234567 Links: {}\nAccess: ({:o}/{})  Uid: ({:>5}/{:>8})   Gid: ({:>5}/{:>8})\nAccess: {}\nModify: {}\nChange: {}\n",
                    path, n.size(), n.size().div_ceil(512), kind, n.nlink, n.mode, format_mode(if n.is_dir() {EntryKind::Dir} else if n.is_symlink() {EntryKind::Symlink} else {EntryKind::File}, n.mode), n.uid, user_for_uid(n.uid), n.gid, group_for_gid(n.gid), ts_to_ls(n.atime), ts_to_ls(n.mtime), ts_to_ls(n.ctime),
                ));
            }
            Err(e) => {
                return CmdResult::err(
                    format!("stat: cannot stat '{}': {}\n", t, e.message(t)).into_bytes(),
                    1,
                );
            }
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn file(session: &ShellSession, argv: &[String]) -> CmdResult {
    let mut out = String::new();
    for t in &argv[1..] {
        if t.starts_with('-') {
            continue;
        }
        let path = Vfs::canonicalize(&session.current_dir, t);
        match session.vfs.lookup(&path) {
            Ok(n) => {
                let desc = match &n.kind {
                    super::super::vfs::NodeKind::Dir(_) => "directory".to_string(),
                    super::super::vfs::NodeKind::Symlink(target) => format!("symbolic link to {}", target),
                    super::super::vfs::NodeKind::File(d) => detect_file_type(d),
                };
                out.push_str(&format!("{}: {}\n", t, desc));
            }
            Err(_) => out.push_str(&format!("{}: cannot open\n", t)),
        }
    }
    CmdResult::ok(out.into_bytes())
}

fn detect_file_type(d: &[u8]) -> String {
    if d.is_empty() {
        return "empty".to_string();
    }
    if d.starts_with(b"\x7fELF") {
        return "ELF 64-bit LSB executable, x86-64".to_string();
    }
    if d.starts_with(b"#!/") {
        let line: String = d.iter().take(80).map(|&b| b as char).collect();
        if line.contains("python") {
            return "Python script, ASCII text executable".to_string();
        }
        if line.contains("perl") {
            return "Perl script, ASCII text executable".to_string();
        }
        if line.contains("bash") || line.contains("/sh") {
            return "POSIX shell script, ASCII text executable".to_string();
        }
        return "a script, ASCII text executable".to_string();
    }
    if d.starts_with(b"PK\x03\x04") {
        return "Zip archive data".to_string();
    }
    if d.starts_with(b"\x1f\x8b") {
        return "gzip compressed data".to_string();
    }
    if d.iter()
        .all(|&b| b == b'\n' || b == b'\t' || b == b'\r' || (0x20..0x7f).contains(&b))
    {
        return "ASCII text".to_string();
    }
    "data".to_string()
}

pub fn du(session: &ShellSession, argv: &[String]) -> CmdResult {
    let mut human = false;
    let mut targets: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            if a.contains('h') {
                human = true;
            }
        } else {
            targets.push(a.clone());
        }
    }
    if targets.is_empty() {
        targets.push(session.current_dir.clone());
    }
    let mut out = String::new();
    for t in &targets {
        let abs = Vfs::canonicalize(&session.current_dir, t);
        let mut total = 0usize;
        for (p, _, sz) in session.vfs.walk() {
            if p.starts_with(&abs) {
                total += sz;
            }
        }
        let s = if human {
            human_size(total)
        } else {
            format!("{}", total / 1024 + 1)
        };
        out.push_str(&format!("{}\t{}\n", s, t));
    }
    CmdResult::ok(out.into_bytes())
}

fn human_size(n: usize) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < UNITS.len() {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}{}", n, UNITS[0])
    } else {
        format!("{:.1}{}", v, UNITS[i])
    }
}
