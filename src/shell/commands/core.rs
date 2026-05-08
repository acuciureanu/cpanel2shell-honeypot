use super::{CmdResult, ShellSession, read_stdin_or_files, ts_to_ls};
use super::super::vfs::{EntryKind, Vfs, format_mode, group_for_gid, user_for_uid};

pub fn ls(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let mut show_all = false;
    let mut long = false;
    let mut human = false;
    let mut one_per_line = false;
    let mut classify = false;
    let mut targets: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with("--") {
            match a.as_str() {
                "--all" => show_all = true,
                "--human-readable" => human = true,
                "--color" | "--color=auto" | "--color=always" | "--color=never" => {}
                _ => {}
            }
        } else if a.starts_with('-') {
            for c in a.chars().skip(1) {
                match c {
                    'a' | 'A' => show_all = true,
                    'l' => long = true,
                    'h' => human = true,
                    '1' => one_per_line = true,
                    'F' => classify = true,
                    'r' | 't' | 'S' | 'R' => {}
                    _ => {}
                }
            }
        } else {
            targets.push(a.clone());
        }
    }
    if targets.is_empty() {
        targets.push(session.current_dir.clone());
    }

    let mut out = String::new();
    let mut err = String::new();
    let multi = targets.len() > 1;
    for (i, t) in targets.iter().enumerate() {
        let path = Vfs::canonicalize(&session.current_dir, t);
        let entries_res = if session.vfs.is_dir(&path) {
            session.vfs.list(&path)
        } else if session.vfs.exists(&path) {
            // single file listing
            let n = session.vfs.lookup(&path).unwrap();
            let kind = if n.is_dir() {
                EntryKind::Dir
            } else if n.is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::File
            };
            Ok(vec![super::super::vfs::DirEntry {
                name: t.clone(),
                kind,
                mode: n.mode,
                uid: n.uid,
                gid: n.gid,
                size: n.size(),
                mtime: n.mtime,
                nlink: n.nlink,
            }])
        } else {
            Err(super::super::vfs::VfsError::NotFound)
        };
        let entries = match entries_res {
            Ok(e) => e,
            Err(e) => {
                err.push_str(&format!("ls: cannot access '{}': {}\n", t, e.message(t)));
                continue;
            }
        };
        if multi {
            out.push_str(&format!("{}:\n", t));
        }
        let mut entries = entries;
        if !show_all {
            entries.retain(|e| !e.name.starts_with('.'));
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        if long {
            let total: usize = entries.iter().map(|e| e.size / 1024 + 1).sum();
            out.push_str(&format!("total {}\n", total));
            for e in &entries {
                let mode_s = format_mode(e.kind, e.mode);
                let user = user_for_uid(e.uid);
                let group = group_for_gid(e.gid);
                let size = if human {
                    human_size(e.size)
                } else {
                    format!("{}", e.size)
                };
                let name = decorate(&e.name, e.kind, classify);
                out.push_str(&format!(
                    "{} {} {} {} {:>8} {} {}\n",
                    mode_s,
                    e.nlink,
                    user,
                    group,
                    size,
                    ts_to_ls(e.mtime),
                    name
                ));
            }
        } else if one_per_line {
            for e in &entries {
                out.push_str(&decorate(&e.name, e.kind, classify));
                out.push('\n');
            }
        } else {
            let names: Vec<String> = entries
                .iter()
                .map(|e| decorate(&e.name, e.kind, classify))
                .collect();
            if !names.is_empty() {
                out.push_str(&names.join("  "));
                out.push('\n');
            }
        }
        if multi && i + 1 < targets.len() {
            out.push('\n');
        }
    }

    let status = if err.is_empty() { 0 } else { 2 };
    CmdResult {
        stdout: out.into_bytes(),
        stderr: err.into_bytes(),
        status,
    }
}

fn decorate(name: &str, kind: EntryKind, classify: bool) -> String {
    if !classify {
        return name.to_string();
    }
    match kind {
        EntryKind::Dir => format!("{}/", name),
        EntryKind::Symlink => format!("{}@", name),
        EntryKind::File => name.to_string(),
    }
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

pub fn cat(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let files: Vec<String> = argv[1..]
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    if files.is_empty() {
        return CmdResult::ok(stdin.to_vec());
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut status = 0;
    for f in &files {
        let path = Vfs::canonicalize(&session.current_dir, f);
        match session.vfs.read(&path) {
            Ok(d) => out.extend_from_slice(d),
            Err(e) => {
                err.extend_from_slice(format!("cat: {}: {}\n", f, e.message(f)).as_bytes());
                status = 1;
            }
        }
    }
    CmdResult {
        stdout: out,
        stderr: err,
        status,
    }
}

pub fn head(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut n = 10usize;
    let mut files: Vec<String> = Vec::new();
    let mut iter = argv[1..].iter();
    while let Some(a) = iter.next() {
        if let Some(rest) = a.strip_prefix("-n") {
            if rest.is_empty() {
                if let Some(v) = iter.next() {
                    n = v.parse().unwrap_or(10);
                }
            } else {
                n = rest.parse().unwrap_or(10);
            }
        } else if a.starts_with('-') && a.len() > 1 {
            n = a[1..].parse().unwrap_or(10);
        } else {
            files.push(a.clone());
        }
    }
    let buf = match read_stdin_or_files(session, &files, stdin) {
        Ok(b) => b,
        Err(e) => return CmdResult::err(format!("head: {}\n", e).into_bytes(), 1),
    };
    let s = String::from_utf8_lossy(&buf);
    let out: String = s.lines().take(n).collect::<Vec<_>>().join("\n") + "\n";
    CmdResult::ok(out.into_bytes())
}

pub fn tail(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut n = 10usize;
    let mut files: Vec<String> = Vec::new();
    let mut iter = argv[1..].iter();
    while let Some(a) = iter.next() {
        if a == "-f" {
            // honeypot just ignores follow
            continue;
        }
        if let Some(rest) = a.strip_prefix("-n") {
            if rest.is_empty() {
                if let Some(v) = iter.next() {
                    n = v.trim_start_matches('+').parse().unwrap_or(10);
                }
            } else {
                n = rest.trim_start_matches('+').parse().unwrap_or(10);
            }
        } else if a.starts_with('-') && a.len() > 1 {
            n = a[1..].parse().unwrap_or(10);
        } else {
            files.push(a.clone());
        }
    }
    let buf = match read_stdin_or_files(session, &files, stdin) {
        Ok(b) => b,
        Err(e) => return CmdResult::err(format!("tail: {}\n", e).into_bytes(), 1),
    };
    let s = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    let out = lines[start..].join("\n") + "\n";
    CmdResult::ok(out.into_bytes())
}

pub fn wc(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut want_l = false;
    let mut want_w = false;
    let mut want_c = false;
    let mut files: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            for c in a.chars().skip(1) {
                match c {
                    'l' => want_l = true,
                    'w' => want_w = true,
                    'c' | 'm' => want_c = true,
                    _ => {}
                }
            }
        } else {
            files.push(a.clone());
        }
    }
    if !want_l && !want_w && !want_c {
        want_l = true;
        want_w = true;
        want_c = true;
    }
    let make_line = |buf: &[u8], label: &str| -> String {
        let s = String::from_utf8_lossy(buf);
        let lines = s.lines().count();
        let words = s.split_whitespace().count();
        let bytes = buf.len();
        let mut parts = Vec::new();
        if want_l {
            parts.push(format!("{:>7}", lines));
        }
        if want_w {
            parts.push(format!("{:>7}", words));
        }
        if want_c {
            parts.push(format!("{:>7}", bytes));
        }
        if !label.is_empty() {
            parts.push(label.to_string());
        }
        parts.join(" ") + "\n"
    };
    let mut out = String::new();
    if files.is_empty() {
        out.push_str(&make_line(stdin, ""));
    } else {
        for f in &files {
            let path = Vfs::canonicalize(&session.current_dir, f);
            match session.vfs.read(&path) {
                Ok(d) => out.push_str(&make_line(d, f)),
                Err(e) => {
                    return CmdResult::err(format!("wc: {}: {}\n", f, e.message(f)).into_bytes(), 1);
                }
            }
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn sort(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut reverse = false;
    let mut numeric = false;
    let mut unique = false;
    let mut files: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            for c in a.chars().skip(1) {
                match c {
                    'r' => reverse = true,
                    'n' => numeric = true,
                    'u' => unique = true,
                    _ => {}
                }
            }
        } else {
            files.push(a.clone());
        }
    }
    let buf = match read_stdin_or_files(session, &files, stdin) {
        Ok(b) => b,
        Err(e) => return CmdResult::err(format!("sort: {}\n", e).into_bytes(), 1),
    };
    let s = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = s.lines().map(|s| s.to_string()).collect();
    if numeric {
        lines.sort_by(|a, b| {
            let an: f64 = a.trim().parse().unwrap_or(f64::INFINITY);
            let bn: f64 = b.trim().parse().unwrap_or(f64::INFINITY);
            an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort();
    }
    if reverse {
        lines.reverse();
    }
    if unique {
        lines.dedup();
    }
    CmdResult::ok((lines.join("\n") + "\n").into_bytes())
}

pub fn uniq(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut count = false;
    let mut files: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            if a.contains('c') {
                count = true;
            }
        } else {
            files.push(a.clone());
        }
    }
    let buf = match read_stdin_or_files(session, &files, stdin) {
        Ok(b) => b,
        Err(e) => return CmdResult::err(format!("uniq: {}\n", e).into_bytes(), 1),
    };
    let s = String::from_utf8_lossy(&buf);
    let mut out = String::new();
    let mut prev: Option<String> = None;
    let mut n = 0usize;
    for line in s.lines() {
        match &prev {
            Some(p) if p == line => n += 1,
            _ => {
                if let Some(p) = &prev {
                    if count {
                        out.push_str(&format!("{:>7} {}\n", n, p));
                    } else {
                        out.push_str(p);
                        out.push('\n');
                    }
                }
                prev = Some(line.to_string());
                n = 1;
            }
        }
    }
    if let Some(p) = &prev {
        if count {
            out.push_str(&format!("{:>7} {}\n", n, p));
        } else {
            out.push_str(p);
            out.push('\n');
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn tr(argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut delete = false;
    let mut squeeze = false;
    let mut sets: Vec<&String> = Vec::new();
    for a in &argv[1..] {
        if a.starts_with('-') {
            if a.contains('d') {
                delete = true;
            }
            if a.contains('s') {
                squeeze = true;
            }
        } else {
            sets.push(a);
        }
    }
    let s = String::from_utf8_lossy(stdin).to_string();
    let from = sets.first().map(|s| s.as_str()).unwrap_or("");
    let to = sets.get(1).map(|s| s.as_str()).unwrap_or("");
    let from_chars: Vec<char> = expand_tr_set(from);
    let to_chars: Vec<char> = expand_tr_set(to);
    let mut out = String::new();
    let mut last: Option<char> = None;
    for c in s.chars() {
        if let Some(idx) = from_chars.iter().position(|x| *x == c) {
            if delete {
                continue;
            }
            let mapped = to_chars
                .get(idx)
                .copied()
                .unwrap_or(*to_chars.last().unwrap_or(&c));
            if squeeze && last == Some(mapped) {
                continue;
            }
            out.push(mapped);
            last = Some(mapped);
        } else {
            out.push(c);
            last = Some(c);
        }
    }
    CmdResult::ok(out.into_bytes())
}

fn expand_tr_set(s: &str) -> Vec<char> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let start = chars[i] as u32;
            let end = chars[i + 2] as u32;
            if end >= start {
                for c in start..=end {
                    if let Some(ch) = char::from_u32(c) {
                        out.push(ch);
                    }
                }
            }
            i += 3;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

pub fn cut(argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut delim = '\t';
    let mut fields: Vec<usize> = Vec::new();
    let mut iter = argv[1..].iter();
    while let Some(a) = iter.next() {
        if a == "-d" {
            if let Some(v) = iter.next()
                && let Some(c) = v.chars().next()
            {
                delim = c;
            }
        } else if let Some(rest) = a.strip_prefix("-d") {
            if let Some(c) = rest.chars().next() {
                delim = c;
            }
        } else if a == "-f" {
            if let Some(v) = iter.next() {
                fields = parse_fields(v);
            }
        } else if let Some(rest) = a.strip_prefix("-f") {
            fields = parse_fields(rest);
        }
    }
    let s = String::from_utf8_lossy(stdin);
    let mut out = String::new();
    for line in s.lines() {
        let parts: Vec<&str> = line.split(delim).collect();
        let mut sel: Vec<&str> = Vec::new();
        for f in &fields {
            if *f >= 1 && *f - 1 < parts.len() {
                sel.push(parts[*f - 1]);
            }
        }
        out.push_str(&sel.join(&delim.to_string()));
        out.push('\n');
    }
    CmdResult::ok(out.into_bytes())
}

fn parse_fields(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in s.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            let a: usize = a.parse().unwrap_or(1);
            let b: usize = b.parse().unwrap_or(a);
            for i in a..=b {
                out.push(i);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            out.push(n);
        }
    }
    out
}

pub fn grep(session: &ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut case_insensitive = false;
    let mut invert = false;
    let mut count = false;
    let mut line_nums = false;
    let mut quiet = false;
    let mut pattern: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut iter = argv[1..].iter();
    while let Some(a) = iter.next() {
        if a == "-e" {
            pattern = iter.next().cloned();
        } else if a.starts_with("--") {
            // long options: --color, --color=auto, --include, --exclude, etc. — ignore most
            if a == "--invert-match" {
                invert = true;
            } else if a == "--count" {
                count = true;
            } else if a == "--line-number" {
                line_nums = true;
            } else if a == "--quiet" || a == "--silent" {
                quiet = true;
            } else if a == "--ignore-case" {
                case_insensitive = true;
            }
            // everything else (--color, --color=auto, --include=, etc.) silently ignored
        } else if a.starts_with('-') && a.len() > 1 {
            for c in a.chars().skip(1) {
                match c {
                    'i' => case_insensitive = true,
                    'v' => invert = true,
                    'c' => count = true,
                    'n' => line_nums = true,
                    'q' => quiet = true,
                    'r' | 'R' => {}
                    'E' | 'F' | 'H' | 's' | 'a' | 'w' | 'x' | 'o' | 'l' | 'L' | 'm' | 'e' => {}
                    _ => {}
                }
            }
        } else if pattern.is_none() {
            pattern = Some(a.clone());
        } else {
            files.push(a.clone());
        }
    }
    let pat = pattern.unwrap_or_default();
    let pat_l = pat.to_lowercase();

    let do_match = |line: &str| -> bool {
        let m = if case_insensitive {
            line.to_lowercase().contains(&pat_l)
        } else {
            line.contains(&pat)
        };
        if invert { !m } else { m }
    };

    let process = |buf: &[u8], label: Option<&str>| -> (String, usize) {
        let s = String::from_utf8_lossy(buf);
        let mut out = String::new();
        let mut hits = 0usize;
        for (i, line) in s.lines().enumerate() {
            if do_match(line) {
                hits += 1;
                if quiet {
                    continue;
                }
                if count {
                    continue;
                }
                if let Some(lbl) = label {
                    out.push_str(lbl);
                    out.push(':');
                }
                if line_nums {
                    out.push_str(&format!("{}:", i + 1));
                }
                out.push_str(line);
                out.push('\n');
            }
        }
        (out, hits)
    };

    let mut combined = String::new();
    let mut total = 0usize;
    if files.is_empty() {
        let (o, h) = process(stdin, None);
        combined.push_str(&o);
        total += h;
    } else {
        let multi = files.len() > 1;
        for f in &files {
            let path = Vfs::canonicalize(&session.current_dir, f);
            match session.vfs.read(&path) {
                Ok(d) => {
                    let (o, h) = process(d, if multi { Some(f) } else { None });
                    combined.push_str(&o);
                    total += h;
                }
                Err(e) => {
                    return CmdResult::err(
                        format!("grep: {}: {}\n", f, e.message(f)).into_bytes(),
                        2,
                    );
                }
            }
        }
    }
    if count {
        combined = format!("{}\n", total);
    }
    let status = if total > 0 { 0 } else { 1 };
    if quiet {
        combined.clear();
    }
    CmdResult {
        stdout: combined.into_bytes(),
        stderr: vec![],
        status,
    }
}

pub fn sed(argv: &[String], stdin: &[u8]) -> CmdResult {
    // Simple sed: supports `s/a/b/[g]` and `-n` `p` forms only.
    let mut script: Option<String> = None;
    let mut iter = argv[1..].iter();
    while let Some(a) = iter.next() {
        if a == "-e" {
            script = iter.next().cloned();
        } else if !a.starts_with('-') && script.is_none() {
            script = Some(a.clone());
        }
    }
    let s = String::from_utf8_lossy(stdin);
    let script = script.unwrap_or_default();
    if let Some(rest) = script.strip_prefix("s") {
        let delim = rest.chars().next().unwrap_or('/');
        let body: Vec<&str> = rest[delim.len_utf8()..].splitn(3, delim).collect();
        if body.len() >= 2 {
            let from = body[0];
            let to = body[1];
            let global = body.get(2).map(|f| f.contains('g')).unwrap_or(false);
            let mut out = String::new();
            for line in s.lines() {
                let new_line = if global {
                    line.replace(from, to)
                } else {
                    line.replacen(from, to, 1)
                };
                out.push_str(&new_line);
                out.push('\n');
            }
            return CmdResult::ok(out.into_bytes());
        }
    }
    CmdResult::ok(stdin.to_vec())
}

pub fn awk(argv: &[String], stdin: &[u8]) -> CmdResult {
    // Minimal awk: supports `'{print $N}'` and `'{print}'` only.
    let prog = argv
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_default();
    let print_field: Option<usize> = if let Some(start) = prog.find("$") {
        let rest = &prog[start + 1..];
        let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        n.parse().ok()
    } else {
        None
    };
    let mut delim = ' ';
    if let Some(idx) = argv.iter().position(|a| a == "-F")
        && let Some(v) = argv.get(idx + 1)
        && let Some(c) = v.chars().next()
    {
        delim = c;
    }
    let s = String::from_utf8_lossy(stdin);
    let mut out = String::new();
    for line in s.lines() {
        let fields: Vec<&str> = if delim == ' ' {
            line.split_whitespace().collect()
        } else {
            line.split(delim).collect()
        };
        match print_field {
            Some(0) => {
                out.push_str(line);
                out.push('\n');
            }
            Some(n) if n >= 1 && n - 1 < fields.len() => {
                out.push_str(fields[n - 1]);
                out.push('\n');
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    CmdResult::ok(out.into_bytes())
}

pub fn find(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    // find <path> [-name PAT] [-type f|d] [-iname PAT]
    let mut path = ".".to_string();
    let mut name_pat: Option<String> = None;
    let mut iname_pat: Option<String> = None;
    let mut type_filter: Option<char> = None;
    let mut iter = argv[1..].iter();
    if let Some(a) = iter.clone().next()
        && !a.starts_with('-')
    {
        path = iter.next().unwrap().clone();
    }
    while let Some(a) = iter.next() {
        match a.as_str() {
            "-name" => name_pat = iter.next().cloned(),
            "-iname" => iname_pat = iter.next().cloned(),
            "-type" => {
                if let Some(t) = iter.next() {
                    type_filter = t.chars().next();
                }
            }
            "-maxdepth" | "-mindepth" => {
                iter.next();
            }
            _ => {}
        }
    }
    let abs = Vfs::canonicalize(&session.current_dir, &path);
    let walk = session.vfs.walk();
    let mut hits: Vec<String> = Vec::new();
    if abs == "/" || session.vfs.exists(&abs) {
        if abs != "/" {
            hits.push(abs.clone());
        } else {
            hits.push("/".into());
        }
    }
    for (p, kind, _) in walk {
        if !p.starts_with(&abs) && abs != "/" {
            continue;
        }
        let basename = p.rsplit('/').next().unwrap_or("");
        if let Some(ref pat) = name_pat
            && let Ok(pattern) = glob::Pattern::new(pat)
            && !pattern.matches(basename)
        {
            continue;
        }
        if let Some(ref pat) = iname_pat
            && let Ok(pattern) = glob::Pattern::new(&pat.to_lowercase())
            && !pattern.matches(&basename.to_lowercase())
        {
            continue;
        }
        if let Some(t) = type_filter {
            let want_dir = t == 'd';
            let want_file = t == 'f';
            let want_link = t == 'l';
            let is_dir = matches!(kind, EntryKind::Dir);
            let is_link = matches!(kind, EntryKind::Symlink);
            let is_file = matches!(kind, EntryKind::File);
            if want_dir && !is_dir {
                continue;
            }
            if want_file && !is_file {
                continue;
            }
            if want_link && !is_link {
                continue;
            }
        }
        hits.push(p);
    }
    hits.sort();
    hits.dedup();
    let out = if hits.is_empty() {
        String::new()
    } else {
        hits.join("\n") + "\n"
    };
    CmdResult::ok(out.into_bytes())
}

pub fn xargs(session: &mut ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    // Simple form: feed each line as args to argv[1..]
    if argv.len() < 2 {
        return CmdResult::ok(stdin.to_vec());
    }
    let s = String::from_utf8_lossy(stdin);
    let base: Vec<String> = argv[1..].to_vec();
    let mut combined_out = Vec::new();
    let mut combined_err = Vec::new();
    let mut last_status = 0;
    for line in s.split_whitespace() {
        let mut full = base.clone();
        full.push(line.to_string());
        let r = super::dispatch(session, &full, &[]);
        combined_out.extend_from_slice(&r.stdout);
        combined_err.extend_from_slice(&r.stderr);
        last_status = r.status;
    }
    CmdResult {
        stdout: combined_out,
        stderr: combined_err,
        status: last_status,
    }
}

pub fn tee(session: &mut ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    let mut append = false;
    let mut files: Vec<String> = Vec::new();
    for a in &argv[1..] {
        if a == "-a" {
            append = true;
        } else if !a.starts_with('-') {
            files.push(a.clone());
        }
    }
    for f in &files {
        let path = Vfs::canonicalize(&session.current_dir, f);
        if append {
            session.vfs.append(&path, stdin, 0o644).ok();
        } else {
            session.vfs.write(&path, stdin.to_vec(), 0o644).ok();
        }
    }
    CmdResult::ok(stdin.to_vec())
}

pub fn rev(stdin: &[u8]) -> CmdResult {
    let s = String::from_utf8_lossy(stdin);
    let mut out = String::new();
    for line in s.lines() {
        out.push_str(&line.chars().rev().collect::<String>());
        out.push('\n');
    }
    CmdResult::ok(out.into_bytes())
}

pub fn yes(argv: &[String]) -> CmdResult {
    let s = if argv.len() > 1 {
        argv[1..].join(" ")
    } else {
        "y".into()
    };
    let mut out = String::new();
    for _ in 0..1024 {
        out.push_str(&s);
        out.push('\n');
    }
    CmdResult::ok(out.into_bytes())
}

pub fn basename(argv: &[String]) -> CmdResult {
    let p = argv.get(1).cloned().unwrap_or_default();
    let suffix = argv.get(2).cloned().unwrap_or_default();
    let mut name = p.rsplit('/').next().unwrap_or("").to_string();
    if !suffix.is_empty() && name.ends_with(&suffix) {
        name = name[..name.len() - suffix.len()].to_string();
    }
    CmdResult::ok(format!("{}\n", name).into_bytes())
}

pub fn dirname(argv: &[String]) -> CmdResult {
    let p = argv.get(1).cloned().unwrap_or_default();
    let d = if let Some(idx) = p.rfind('/') {
        if idx == 0 {
            "/".to_string()
        } else {
            p[..idx].to_string()
        }
    } else {
        ".".to_string()
    };
    CmdResult::ok(format!("{}\n", d).into_bytes())
}

pub fn readlink(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.len() < 2 {
        return CmdResult::empty();
    }
    let path = Vfs::canonicalize(&session.current_dir, &argv[1]);
    if let Ok(node) = session.vfs.lookup(&path)
        && let super::super::vfs::NodeKind::Symlink(t) = &node.kind
    {
        return CmdResult::ok(format!("{}\n", t).into_bytes());
    }
    CmdResult::err(vec![], 1)
}

pub fn realpath(session: &ShellSession, argv: &[String]) -> CmdResult {
    let p = argv.get(1).cloned().unwrap_or_default();
    let abs = Vfs::canonicalize(&session.current_dir, &p);
    CmdResult::ok(format!("{}\n", abs).into_bytes())
}
