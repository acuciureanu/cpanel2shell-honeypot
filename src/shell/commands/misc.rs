use super::super::vfs::Vfs;
use super::{CmdResult, ShellSession};

pub fn editor(name: &str) -> CmdResult {
    CmdResult::err(format!("{}: command not found\n", name).into_bytes(), 127)
}

pub fn env(session: &ShellSession) -> CmdResult {
    let mut out = String::new();
    for (k, v) in &session.env {
        out.push_str(&format!("{}={}\n", k, v));
    }
    CmdResult::ok(out.into_bytes())
}

pub fn which(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::err(b"which: no command specified\n".to_vec(), 1);
    }
    let mut out = String::new();
    for cmd in &argv[1..] {
        let paths = session.env.get("PATH").cloned().unwrap_or_else(|| {
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into()
        });
        let mut found = false;
        for p in paths.split(':') {
            let full = format!("{}/{}", p.trim_end_matches('/'), cmd);
            if session.vfs.exists(&full) {
                out.push_str(&format!("{}\n", full));
                found = true;
                break;
            }
        }
        if !found {
            return CmdResult::err(
                format!("which: no {} in ({})\n", cmd, paths).into_bytes(),
                1,
            );
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn whereis(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::err(b"whereis: not enough arguments\n".to_vec(), 1);
    }
    let mut out = String::new();
    for cmd in &argv[1..] {
        let bin = format!("/usr/bin/{}", cmd);
        if session.vfs.exists(&bin) {
            out.push_str(&format!("{}: {}\n", cmd, bin));
        } else {
            out.push_str(&format!("{}:\n", cmd));
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn locate(_session: &ShellSession, argv: &[String]) -> CmdResult {
    let pattern = argv.get(1).cloned().unwrap_or_default();
    if pattern.is_empty() {
        return CmdResult::err(b"locate: no pattern to search for specified\n".to_vec(), 1);
    }
    let files = vec![
        "/etc/passwd",
        "/etc/shadow",
        "/etc/hosts",
        "/etc/resolv.conf",
        "/usr/bin/python3",
        "/usr/bin/bash",
        "/usr/bin/ls",
        "/usr/bin/cat",
        "/home/user/.bashrc",
        "/home/user/.ssh/id_rsa",
        "/var/log/syslog",
    ];
    let mut out = String::new();
    for f in files {
        if f.contains(&pattern) {
            out.push_str(&format!("{}\n", f));
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn hashsum(cmd: &str, argv: &[String], stdin: &[u8], session: &ShellSession) -> CmdResult {
    let files: Vec<String> = argv
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    let mut out = String::new();

    if files.is_empty() {
        let data = if stdin.is_empty() {
            b"".to_vec()
        } else {
            stdin.to_vec()
        };
        let hash = fake_hash(cmd, &data);
        out.push_str(&format!("{}  -\n", hash));
    } else {
        for f in files {
            let path = Vfs::canonicalize(&session.current_dir, &f);
            let data = session.vfs.read(&path).unwrap_or_default();
            let hash = fake_hash(cmd, &data);
            out.push_str(&format!("{}  {}\n", hash, f));
        }
    }

    CmdResult::ok(out.into_bytes())
}

fn fake_hash(cmd: &str, data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let v = hasher.finish();

    let hex_len = match cmd {
        "md5sum" => 32,
        "sha1sum" => 40,
        "sha256sum" => 64,
        _ => 64,
    };

    let mut hex = format!("{:016x}", v);
    while hex.len() < hex_len {
        hex.push_str(&format!("{:016x}", v.wrapping_mul(0x9E3779B97F4A7C15)));
    }
    hex.truncate(hex_len);
    hex
}

pub fn tar(argv: &[String]) -> CmdResult {
    let verbose = argv.iter().any(|a| a.contains('v'));
    if verbose {
        CmdResult::ok(b"./\n./file1\n./file2\n".to_vec())
    } else {
        CmdResult::empty()
    }
}

pub fn crontab(argv: &[String], session: &ShellSession) -> CmdResult {
    if argv.iter().any(|a| a == "-l") {
        if let Ok(d) = session.vfs.read("/var/spool/cron/root") {
            return CmdResult::ok(d.to_vec());
        }
        return CmdResult::ok(b"# Edit this file to introduce tasks to be run by cron.\n0 2 * * * /usr/local/cpanel/scripts/upcp\n0 1 * * * /scripts/cpbackup\n*/5 * * * * /usr/local/cpanel/whostmgr/bin/dnsadmin\n".to_vec());
    }
    CmdResult::empty()
}

pub fn systemctl(argv: &[String]) -> CmdResult {
    let sub = argv.get(1).map(String::as_str).unwrap_or("");
    match sub {
        "status" => {
            let svc = argv.get(2).cloned().unwrap_or_default();
            let body = format!(
                "\u{25cf} {svc}.service - {svc}\n\
                 \x20  Loaded: loaded (/usr/lib/systemd/system/{svc}.service; enabled; vendor preset: enabled)\n\
                 \x20  Active: active (running) since Sat 2026-05-08 10:00:00 UTC; 1h 30min ago\n\
                 \x20    Docs: man:{svc}(8)\n\
                 \x20Main PID: 1234 ({svc})\n\
                 \x20   Tasks: 10\n\
                 \x20  Memory: 45.2M\n\
                 \x20  CGroup: /system.slice/{svc}.service\n");
            CmdResult::ok(body.into_bytes())
        }
        "list-units" => CmdResult::ok(b"UNIT                LOAD   ACTIVE SUB     DESCRIPTION\nsshd.service        loaded active running OpenSSH server daemon\nhttpd.service       loaded active running The Apache HTTP Server\ncpanel.service      loaded active running cPanel & WHM Server\n".to_vec()),
        "is-active" => CmdResult::ok(b"active\n".to_vec()),
        "is-enabled" => CmdResult::ok(b"enabled\n".to_vec()),
        "start" | "stop" | "restart" | "reload" | "enable" | "disable" => CmdResult::empty(),
        _ => CmdResult::empty(),
    }
}

pub fn service(argv: &[String]) -> CmdResult {
    if argv.len() >= 3 && argv[2] == "status" {
        return CmdResult::ok(format!("{} is running\n", argv[1]).into_bytes());
    }
    CmdResult::empty()
}

pub fn journalctl() -> CmdResult {
    CmdResult::ok(b"-- Logs begin at Tue 2026-03-24 09:15:00 UTC, end at Sat 2026-05-08 11:30:00 UTC. --\nMay 08 10:00:01 cpanel.local systemd[1]: Started Daily apt activities.\nMay 08 10:05:23 cpanel.local sshd[1456]: Accepted publickey for root from 192.168.1.50\n".to_vec())
}

pub fn history(session: &ShellSession) -> CmdResult {
    let mut out = String::new();
    for (i, h) in session.history.iter().enumerate() {
        out.push_str(&format!("{:5}  {}\n", i + 1, h));
    }
    CmdResult::ok(out.into_bytes())
}

pub fn base64(argv: &[String], stdin: &[u8]) -> CmdResult {
    let decode = argv.iter().any(|a| a == "-d" || a == "--decode");
    if decode {
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, stdin) {
            Ok(data) => CmdResult::ok(data),
            Err(_) => CmdResult::err(b"base64: invalid input\n".to_vec(), 1),
        }
    } else {
        let data = if stdin.is_empty() {
            if let Some(_f) = argv.iter().find(|a| !a.starts_with('-')) {
                // Would need to read file, but we'll just return empty for now
                vec![]
            } else {
                vec![]
            }
        } else {
            stdin.to_vec()
        };
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
        CmdResult::ok(format!("{}\n", encoded).into_bytes())
    }
}

pub fn dd(_argv: &[String]) -> CmdResult {
    CmdResult::ok(b"0+0 records in\n0+0 records out\n0 bytes copied, 0.0 s, 0.0 kB/s\n".to_vec())
}

pub fn nohup(argv: &[String]) -> CmdResult {
    if argv.len() <= 1 {
        return CmdResult::err(b"nohup: missing operand\n".to_vec(), 1);
    }
    CmdResult::ok(b"".to_vec())
}
