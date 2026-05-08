//! Variable / tilde / glob / command-substitution expansion.

use super::ShellSession;
use super::lexer::WordPart;

/// Expand a single word into zero or more strings (globbing may produce many).
pub fn expand_word(session: &mut ShellSession, parts: &[WordPart]) -> Vec<String> {
    let mut buf = String::new();
    let mut allow_glob = true;
    let mut allow_tilde = true;

    for (idx, part) in parts.iter().enumerate() {
        match part {
            WordPart::Lit(s) => {
                let s = if idx == 0 && allow_tilde {
                    expand_tilde(s, session)
                } else {
                    s.clone()
                };
                buf.push_str(&expand_dollars(&s, session));
                allow_tilde = false;
            }
            WordPart::DoubleQuoted(s) => {
                buf.push_str(&expand_dollars(s, session));
                allow_glob = false;
                allow_tilde = false;
            }
            WordPart::SingleQuoted(s) => {
                buf.push_str(s);
                allow_glob = false;
                allow_tilde = false;
            }
            WordPart::Escaped(s) => {
                buf.push_str(s);
                allow_glob = false;
                allow_tilde = false;
            }
        }
    }

    if allow_glob && (buf.contains('*') || buf.contains('?') || buf.contains('[')) {
        let cwd = session.current_dir.clone();
        let hits = session.vfs.glob(&cwd, &buf);
        if !hits.is_empty() {
            return hits;
        }
    }
    vec![buf]
}

fn expand_tilde(s: &str, session: &ShellSession) -> String {
    if s == "~" {
        session
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/root".into())
    } else if let Some(rest) = s.strip_prefix("~/") {
        let home = session
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/root".into());
        format!("{}/{}", home, rest)
    } else {
        s.to_string()
    }
}

/// Expand $VAR, ${VAR}, ${VAR:-default}, $?, $$, $(cmd), `cmd`.
pub fn expand_dollars(s: &str, session: &mut ShellSession) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && i + 1 < chars.len() {
            let n = chars[i + 1];
            if n == '(' {
                // command substitution
                let (body, ni) = read_balanced(&chars, i + 2, '(', ')');
                i = ni;
                let stdout = run_subshell(session, &body);
                out.push_str(stdout.trim_end_matches('\n'));
                continue;
            } else if n == '{' {
                let (body, ni) = read_balanced(&chars, i + 2, '{', '}');
                i = ni;
                out.push_str(&expand_braced_var(&body, session));
                continue;
            } else if n == '?' {
                out.push_str(&session.last_status.to_string());
                i += 2;
                continue;
            } else if n == '$' {
                out.push_str(&session.pid.to_string());
                i += 2;
                continue;
            } else if n.is_ascii_alphabetic() || n == '_' {
                let mut name = String::new();
                let mut j = i + 1;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    name.push(chars[j]);
                    j += 1;
                }
                if let Some(v) = session.env.get(&name) {
                    out.push_str(v);
                }
                i = j;
                continue;
            }
        } else if c == '`' {
            let (body, ni) = read_until(&chars, i + 1, '`');
            i = ni;
            let stdout = run_subshell(session, &body);
            out.push_str(stdout.trim_end_matches('\n'));
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn expand_braced_var(body: &str, session: &mut ShellSession) -> String {
    if let Some((name, default)) = body.split_once(":-") {
        match session.env.get(name) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => expand_dollars(default, session),
        }
    } else if let Some((name, alt)) = body.split_once(":+") {
        match session.env.get(name) {
            Some(v) if !v.is_empty() => expand_dollars(alt, session),
            _ => String::new(),
        }
    } else {
        session.env.get(body).cloned().unwrap_or_default()
    }
}

fn read_balanced(chars: &[char], start: usize, open: char, close: char) -> (String, usize) {
    let mut depth = 1;
    let mut s = String::new();
    let mut i = start;
    while i < chars.len() {
        let c = chars[i];
        if c == open {
            depth += 1;
            s.push(c);
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return (s, i + 1);
            }
            s.push(c);
        } else {
            s.push(c);
        }
        i += 1;
    }
    (s, i)
}

fn read_until(chars: &[char], start: usize, end: char) -> (String, usize) {
    let mut s = String::new();
    let mut i = start;
    while i < chars.len() && chars[i] != end {
        s.push(chars[i]);
        i += 1;
    }
    if i < chars.len() {
        i += 1;
    }
    (s, i)
}

fn run_subshell(session: &mut ShellSession, cmd: &str) -> String {
    // Use a clone of the session so subshell mutations don't leak (matches bash semantics).
    let mut sub = session.clone();
    let out = super::exec::run(&mut sub, cmd);
    // Propagate $? so callers can chain reliably.
    session.last_status = sub.last_status;
    out
}
