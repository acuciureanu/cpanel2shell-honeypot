use super::{CmdResult, ShellSession};

pub fn wget(session: &mut ShellSession, argv: &[String]) -> CmdResult {
    let url = argv.get(1).cloned().unwrap_or_default();
    if url.is_empty() {
        return CmdResult::err(b"wget: missing URL\n".to_vec(), 1);
    }

    // Extract filename from URL
    let filename = url.rsplit('/').next().unwrap_or("index.html");
    let path = if filename.is_empty() {
        "index.html"
    } else {
        filename
    };
    let abs_path = format!("{}/{}", session.current_dir.trim_end_matches('/'), path);

    let content = format!(
        "<!-- Downloaded from {} -->\n<html><head><title>Downloaded</title></head><body><h1>It works!</h1></body></html>\n",
        url
    );
    let content_len = content.len();
    session
        .vfs
        .write(&abs_path, content.into_bytes(), 0o755)
        .ok();

    let out = format!(
        "--2026-05-08 10:00:00--  {}\nResolving {}... 192.0.2.1\nConnecting to {}|192.0.2.1|:80... connected.\nHTTP request sent, awaiting response... 200 OK\nLength: {} (1.2K)\nSaving to: '{}'\n\n{}          100%[===================>]   1.20K  --.-KB/s    in 0s\n\n2026-05-08 10:00:00 (50.2 MB/s) - '{}' saved [{}]\n",
        url, url, url, content_len, path, path, path, content_len
    );
    CmdResult::ok(out.into_bytes())
}

pub fn curl(argv: &[String]) -> CmdResult {
    let url = argv.iter().find(|a| a.starts_with("http")).cloned();
    if let Some(u) = url {
        if u.starts_with("https://") {
            CmdResult::ok(format!("{{\"url\":\"{}\"}}\n", u).into_bytes())
        } else {
            CmdResult::ok(
                format!(
                    "<html><head><title>{}</title></head><body><h1>It works!</h1></body></html>\n",
                    u
                )
                .into_bytes(),
            )
        }
    } else {
        CmdResult::err(
            b"curl: try 'curl --help' or 'curl --manual' for more information\n".to_vec(),
            2,
        )
    }
}
