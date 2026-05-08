//! Shell builtins: cd, export, set, unset, alias, history, pwd, exit, true/false, etc.

use super::exec::CmdResult;
use super::vfs::Vfs;
use super::ShellSession;

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "pwd"
            | "export"
            | "unset"
            | "set"
            | "alias"
            | "unalias"
            | "history"
            | "exit"
            | "logout"
            | "true"
            | "false"
            | "echo"
            | "printf"
            | "test"
            | "["
            | "type"
            | "command"
            | ":"
            | "source"
            | "."
    )
}

pub fn run(session: &mut ShellSession, argv: &[String], stdin: &[u8]) -> Option<CmdResult> {
    let cmd = argv[0].as_str();
    match cmd {
        "cd" => Some(cd(session, argv)),
        "pwd" => Some(CmdResult::ok(
            format!("{}\n", session.current_dir).into_bytes(),
        )),
        "export" => Some(export(session, argv)),
        "unset" => Some(unset(session, argv)),
        "set" => Some(set(session, argv)),
        "alias" => Some(alias(session, argv)),
        "unalias" => Some(unalias(session, argv)),
        "history" => Some(history(session, argv)),
        "exit" | "logout" => Some(CmdResult::ok(b"logout\n".to_vec())),
        "true" => Some(CmdResult::empty()),
        "false" => Some(CmdResult::err(vec![], 1)),
        "echo" => Some(echo(argv)),
        "printf" => Some(printf(argv)),
        "test" | "[" => Some(test(session, argv)),
        "type" => Some(type_builtin(session, argv)),
        "command" => {
            // `command foo args` => bypass aliases; we just strip and re-dispatch
            if argv.len() > 1 {
                let rest: Vec<String> = argv[1..].to_vec();
                Some(super::commands::dispatch_with_alias(
                    session, &rest, stdin, false,
                ))
            } else {
                Some(CmdResult::empty())
            }
        }
        ":" => Some(CmdResult::empty()),
        "source" | "." => Some(CmdResult::empty()),
        _ => None,
    }
}

fn cd(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let target = argv.get(1).map(String::as_str).unwrap_or_else(|| {
        session
            .env
            .get("HOME")
            .map(String::as_str)
            .unwrap_or("/root")
    });
    let target = if target == "-" {
        session
            .env
            .get("OLDPWD")
            .cloned()
            .unwrap_or_else(|| "/root".into())
    } else {
        target.to_string()
    };
    let resolved = Vfs::canonicalize(&session.current_dir, &target);
    if session.vfs.is_dir(&resolved) || resolved == "/" {
        session
            .env
            .insert("OLDPWD".into(), session.current_dir.clone());
        session.current_dir = resolved.clone();
        session.env.insert("PWD".into(), resolved);
        CmdResult::empty()
    } else {
        CmdResult::err(
            format!("bash: cd: {}: No such file or directory\n", target).into_bytes(),
            1,
        )
    }
}

fn export(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() == 1 {
        let mut out = String::new();
        let mut keys: Vec<&String> = session.env.keys().collect();
        keys.sort();
        for k in keys {
            out.push_str(&format!("declare -x {}=\"{}\"\n", k, session.env[k]));
        }
        return CmdResult::ok(out.into_bytes());
    }
    for a in &argv[1..] {
        if let Some((k, v)) = a.split_once('=') {
            session.env.insert(k.to_string(), v.to_string());
        }
    }
    CmdResult::empty()
}

fn unset(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    for a in &argv[1..] {
        session.env.remove(a);
    }
    CmdResult::empty()
}

fn set(_session: &mut ShellSession, _argv: &[String]) -> CmdResult {
    // We accept `set -e`, `set -x`, etc. silently.
    CmdResult::empty()
}

fn alias(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() == 1 {
        let mut out = String::new();
        let mut keys: Vec<&String> = session.aliases.keys().collect();
        keys.sort();
        for k in keys {
            out.push_str(&format!("alias {}='{}'\n", k, session.aliases[k]));
        }
        return CmdResult::ok(out.into_bytes());
    }
    for a in &argv[1..] {
        if let Some((k, v)) = a.split_once('=') {
            let v = v.trim_matches('\'').trim_matches('"');
            session.aliases.insert(k.to_string(), v.to_string());
        }
    }
    CmdResult::empty()
}

fn unalias(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    for a in &argv[1..] {
        session.aliases.remove(a);
    }
    CmdResult::empty()
}

fn history(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let limit: Option<usize> = argv.get(1).and_then(|s| s.parse().ok());
    let n = session.history.len();
    let start = match limit {
        Some(l) if l < n => n - l,
        _ => 0,
    };
    let mut out = String::new();
    for (i, h) in session.history.iter().enumerate().skip(start) {
        out.push_str(&format!("{:5}  {}\n", i + 1, h));
    }
    CmdResult::ok(out.into_bytes())
}

fn echo(argv: &[String]) -> CmdResult {
    let mut newline = true;
    let mut interpret = false;
    let mut start = 1;
    while start < argv.len() {
        match argv[start].as_str() {
            "-n" => newline = false,
            "-e" => interpret = true,
            "-ne" | "-en" => {
                newline = false;
                interpret = true;
            }
            _ => break,
        }
        start += 1;
    }
    let joined = argv[start..].join(" ");
    let body = if interpret {
        interpret_escapes(&joined)
    } else {
        joined
    };
    let mut bytes = body.into_bytes();
    if newline {
        bytes.push(b'\n');
    }
    CmdResult::ok(bytes)
}

fn interpret_escapes(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('0') => out.push('\0'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn printf(argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::empty();
    }
    let fmt = interpret_escapes(&argv[1]);
    // Very limited %s / %d substitution
    let mut out = String::new();
    let mut args = argv[2..].iter();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('s') => {
                    if let Some(a) = args.next() {
                        out.push_str(a);
                    }
                }
                Some('d') | Some('i') => {
                    if let Some(a) = args.next() {
                        out.push_str(a);
                    }
                }
                Some('%') => out.push('%'),
                Some(other) => {
                    out.push('%');
                    out.push(other);
                }
                None => out.push('%'),
            }
        } else {
            out.push(c);
        }
    }
    CmdResult::ok(out.into_bytes())
}

fn test(session: &ShellSession, argv: &[String]) -> CmdResult {
    // strip trailing ] for `[ ... ]`
    let mut args: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
    if argv[0] == "[" {
        if args.last() == Some(&"]") {
            args.pop();
        } else {
            return CmdResult::err(b"[: missing `]'\n".to_vec(), 2);
        }
    }
    let ok = match args.as_slice() {
        [] => false,
        [a] => !a.is_empty(),
        ["-z", a] => a.is_empty(),
        ["-n", a] => !a.is_empty(),
        ["-e", p] | ["-f", p] => session.vfs.is_file(p),
        ["-d", p] => session.vfs.is_dir(p),
        [a, "=", b] | [a, "==", b] => a == b,
        [a, "!=", b] => a != b,
        [a, "-eq", b] => a.parse::<i64>().ok() == b.parse::<i64>().ok(),
        [a, "-ne", b] => a.parse::<i64>().ok() != b.parse::<i64>().ok(),
        _ => false,
    };
    if ok {
        CmdResult::empty()
    } else {
        CmdResult::err(vec![], 1)
    }
}

fn type_builtin(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::empty();
    }
    let mut out = String::new();
    for name in &argv[1..] {
        if is_builtin(name) {
            out.push_str(&format!("{} is a shell builtin\n", name));
        } else if let Some(v) = session.aliases.get(name) {
            out.push_str(&format!("{} is aliased to `{}'\n", name, v));
        } else {
            let mut found = false;
            for prefix in &[
                "/usr/local/sbin/",
                "/usr/local/bin/",
                "/usr/sbin/",
                "/usr/bin/",
                "/sbin/",
                "/bin/",
            ] {
                let path = format!("{}{}", prefix, name);
                if session.vfs.is_file(&path) {
                    out.push_str(&format!("{} is {}\n", name, path));
                    found = true;
                    break;
                }
            }
            if !found && super::commands::is_known(name) {
                out.push_str(&format!("{} is hashed (/usr/bin/{})\n", name, name));
            } else if !found {
                out.push_str(&format!("bash: type: {}: not found\n", name));
            }
        }
    }
    CmdResult::ok(out.into_bytes())
}
