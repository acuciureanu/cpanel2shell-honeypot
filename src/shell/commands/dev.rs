use super::CmdResult;

pub fn gcc(argv: &[String]) -> CmdResult {
    let has_version = argv.iter().any(|a| a == "--version" || a == "-v");
    if has_version {
        return CmdResult::ok(b"gcc (GCC) 11.3.1 20220421 (Red Hat 11.3.1-2)\nCopyright (C) 2021 Free Software Foundation, Inc.\n".to_vec());
    }
    let out = argv.get(2).cloned().unwrap_or_else(|| "a.out".into());
    CmdResult::ok(format!("Compilation successful: {}\n", out).into_bytes())
}

pub fn make() -> CmdResult {
    CmdResult::ok(b"make: Nothing to be done for 'all'.\n".to_vec())
}

pub fn python(argv: &[String], version: &str) -> CmdResult {
    if argv.iter().any(|a| a == "--version" || a == "-V") {
        return CmdResult::ok(format!("Python {}\n", version).into_bytes());
    }
    if argv.len() > 1 {
        CmdResult::ok(b"".to_vec())
    } else {
        CmdResult::ok(format!("Python {} (default, May  8 2026, 10:00:00)\n[GCC 11.3.1] :: Anaconda, Inc. on linux\nType \"help\", \"copyright\", \"credits\" or \"license\" for more information.\n>>> ", version).into_bytes())
    }
}

pub fn perl(argv: &[String]) -> CmdResult {
    if argv.iter().any(|a| a == "--version" || a == "-v") {
        return CmdResult::ok(b"This is perl 5, version 34, subversion 0 (v5.34.0) built for x86_64-linux-thread-multi\n".to_vec());
    }
    CmdResult::ok(b"".to_vec())
}

pub fn ruby(argv: &[String]) -> CmdResult {
    if argv.iter().any(|a| a == "--version" || a == "-v") {
        return CmdResult::ok(b"ruby 3.0.4p208 (2022-04-12 revision 3) [x86_64-linux]\n".to_vec());
    }
    CmdResult::ok(b"".to_vec())
}

pub fn node(argv: &[String]) -> CmdResult {
    if argv.iter().any(|a| a == "--version" || a == "-v") {
        return CmdResult::ok(b"v18.16.0\n".to_vec());
    }
    CmdResult::ok(b"> ".to_vec())
}

pub fn php(argv: &[String]) -> CmdResult {
    if argv.iter().any(|a| a == "--version" || a == "-v") {
        return CmdResult::ok(
            b"PHP 8.1.2 (cli) (built: May  8 2026 10:00:00) (NTS)\nCopyright (c) The PHP Group\n"
                .to_vec(),
        );
    }
    CmdResult::ok(b"".to_vec())
}

pub fn git(argv: &[String]) -> CmdResult {
    let sub = argv.get(1).map(String::as_str).unwrap_or("");
    match sub {
        "--version" => CmdResult::ok(b"git version 2.34.1\n".to_vec()),
        "status" => CmdResult::ok(b"On branch main\nnothing to commit, working tree clean\n".to_vec()),
        "log" => CmdResult::ok(b"commit abc123 (HEAD -> main)\nAuthor: Admin <admin@localhost>\nDate:   Sat May 8 10:00:00 2026 +0000\n\n    Initial commit\n".to_vec()),
        _ => CmdResult::ok(b"".to_vec()),
    }
}
