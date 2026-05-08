//! Shell executor.

use super::expand::expand_word;
use super::lexer::tokenize;
use super::parser::{parse, Ast, Command, Op, Redirect};
use super::ShellSession;

pub struct CmdResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

impl CmdResult {
    pub fn ok(stdout: Vec<u8>) -> Self {
        Self {
            stdout,
            stderr: vec![],
            status: 0,
        }
    }
    pub fn err(stderr: Vec<u8>, status: i32) -> Self {
        Self {
            stdout: vec![],
            stderr,
            status,
        }
    }
    pub fn empty() -> Self {
        Self {
            stdout: vec![],
            stderr: vec![],
            status: 0,
        }
    }
}

pub fn run(session: &mut ShellSession, input: &str) -> String {
    let trimmed = input.trim_end_matches(['\r', '\n']);
    if trimmed.trim().is_empty() {
        return String::new();
    }
    session.history.push(trimmed.to_string());

    let tokens = match tokenize(trimmed) {
        Ok(t) => t,
        Err(e) => {
            session.last_status = 2;
            return format!("bash: syntax error: {}\n", e.0);
        }
    };
    let ast = match parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            session.last_status = 2;
            return format!("bash: {}\n", e.0);
        }
    };
    let res = exec_ast(session, &ast, &[]);
    session.last_status = res.status;
    let mut out = res.stdout;
    out.extend_from_slice(&res.stderr);
    String::from_utf8_lossy(&out).to_string()
}

fn exec_ast(session: &mut ShellSession, ast: &Ast, stdin: &[u8]) -> CmdResult {
    match ast {
        Ast::Empty => CmdResult::empty(),
        Ast::Seq(items) => {
            let mut last = CmdResult::empty();
            let mut combined_out = Vec::new();
            let mut combined_err = Vec::new();
            for item in items {
                last = exec_ast(session, item, stdin);
                combined_out.extend_from_slice(&last.stdout);
                combined_err.extend_from_slice(&last.stderr);
            }
            CmdResult {
                stdout: combined_out,
                stderr: combined_err,
                status: last.status,
            }
        }
        Ast::AndOr(l, op, r) => {
            let lr = exec_ast(session, l, stdin);
            let mut out = lr.stdout.clone();
            let mut err = lr.stderr.clone();
            let take_right = match op {
                Op::And => lr.status == 0,
                Op::Or => lr.status != 0,
            };
            if take_right {
                let rr = exec_ast(session, r, stdin);
                out.extend_from_slice(&rr.stdout);
                err.extend_from_slice(&rr.stderr);
                CmdResult {
                    stdout: out,
                    stderr: err,
                    status: rr.status,
                }
            } else {
                CmdResult {
                    stdout: out,
                    stderr: err,
                    status: lr.status,
                }
            }
        }
        Ast::Pipeline(cmds) => exec_pipeline(session, cmds, stdin),
        Ast::If {
            condition,
            then_body,
            elif_clauses,
            else_body,
        } => {
            let cond_res = exec_ast(session, condition, stdin);
            if cond_res.status == 0 {
                return exec_ast(session, then_body, stdin);
            }
            for (elif_cond, elif_body) in elif_clauses {
                let elif_res = exec_ast(session, elif_cond, stdin);
                if elif_res.status == 0 {
                    return exec_ast(session, elif_body, stdin);
                }
            }
            if let Some(else_b) = else_body {
                exec_ast(session, else_b, stdin)
            } else {
                CmdResult::empty()
            }
        }
        Ast::For { var, items, body } => {
            let mut combined_out = Vec::new();
            let mut combined_err = Vec::new();
            let mut last_status = 0;
            let items_expanded: Vec<String> = if items.is_empty() {
                // for i in $@ (default to positional params, but we don't have those)
                vec![]
            } else {
                items
                    .iter()
                    .flat_map(|parts| expand_word(session, parts))
                    .collect()
            };
            for item in items_expanded {
                session.env.insert(var.clone(), item);
                let res = exec_ast(session, body, stdin);
                combined_out.extend_from_slice(&res.stdout);
                combined_err.extend_from_slice(&res.stderr);
                last_status = res.status;
            }
            CmdResult {
                stdout: combined_out,
                stderr: combined_err,
                status: last_status,
            }
        }
        Ast::While { condition, body } => {
            let mut combined_out = Vec::new();
            let mut combined_err = Vec::new();
            let mut last_status = 0;
            let mut iterations = 0;
            const MAX_ITERATIONS: usize = 10_000;
            loop {
                iterations += 1;
                if iterations > MAX_ITERATIONS {
                    combined_err
                        .extend_from_slice(b"bash: while loop exceeded maximum iterations\n");
                    last_status = 1;
                    break;
                }
                let cond_res = exec_ast(session, condition, stdin);
                if cond_res.status != 0 {
                    break;
                }
                let res = exec_ast(session, body, stdin);
                combined_out.extend_from_slice(&res.stdout);
                combined_err.extend_from_slice(&res.stderr);
                last_status = res.status;
            }
            CmdResult {
                stdout: combined_out,
                stderr: combined_err,
                status: last_status,
            }
        }
    }
}

fn exec_pipeline(session: &mut ShellSession, cmds: &[Command], stdin: &[u8]) -> CmdResult {
    let mut piped_in: Vec<u8> = stdin.to_vec();
    let mut last_status = 0i32;
    let mut accumulated_err = Vec::new();
    let last_idx = cmds.len() - 1;
    let mut final_out = Vec::new();

    for (i, cmd) in cmds.iter().enumerate() {
        let res = exec_command(session, cmd, &piped_in);
        last_status = res.status;
        accumulated_err.extend_from_slice(&res.stderr);
        if i == last_idx {
            final_out = res.stdout;
        } else {
            piped_in = res.stdout;
        }
    }
    CmdResult {
        stdout: final_out,
        stderr: accumulated_err,
        status: last_status,
    }
}

fn exec_command(session: &mut ShellSession, cmd: &Command, stdin: &[u8]) -> CmdResult {
    if let Some(inner) = &cmd.subshell {
        return exec_ast(session, inner, stdin);
    }
    if cmd.argv.is_empty() {
        return CmdResult::empty();
    }

    let mut argv: Vec<String> = Vec::new();
    for word in &cmd.argv {
        let expanded = expand_word(session, word);
        argv.extend(expanded);
    }
    if argv.is_empty() {
        return CmdResult::empty();
    }

    // Env-assignment prefix
    while !argv.is_empty() && is_env_assignment(&argv[0]) {
        let kv = argv.remove(0);
        if let Some((k, v)) = kv.split_once('=') {
            session.env.insert(k.to_string(), v.to_string());
        }
        if argv.is_empty() {
            return CmdResult::empty();
        }
    }

    // Input redirection
    let mut effective_stdin: Vec<u8> = stdin.to_vec();
    if let Some(body) = &cmd.heredoc {
        effective_stdin = body.clone();
    }
    for r in &cmd.redirects {
        if let Redirect::In(parts) = r {
            let p = expand_word(session, parts).join(" ");
            let path = super::vfs::Vfs::canonicalize(&session.current_dir, &p);
            match session.vfs.read(&path) {
                Ok(d) => effective_stdin = d.to_vec(),
                Err(e) => {
                    return CmdResult::err(
                        format!("bash: {}: {}\n", p, e.message(&p)).into_bytes(),
                        1,
                    );
                }
            }
        }
    }

    let mut res = super::commands::dispatch(session, &argv, &effective_stdin);

    if cmd.merge_stderr {
        let mut s = res.stdout;
        s.extend_from_slice(&res.stderr);
        res = CmdResult {
            stdout: s,
            stderr: vec![],
            status: res.status,
        };
    }

    // Output redirects
    for r in &cmd.redirects {
        match r {
            Redirect::Out(parts) => {
                let p = expand_word(session, parts).join(" ");
                let path = super::vfs::Vfs::canonicalize(&session.current_dir, &p);
                session.vfs.write(&path, res.stdout.clone(), 0o644).ok();
                res.stdout.clear();
            }
            Redirect::Append(parts) => {
                let p = expand_word(session, parts).join(" ");
                let path = super::vfs::Vfs::canonicalize(&session.current_dir, &p);
                session.vfs.append(&path, &res.stdout, 0o644).ok();
                res.stdout.clear();
            }
            Redirect::In(_) => {}
        }
    }

    if cmd.background {
        let mut bg_msg = format!("[1] {}\n", session.pid + 1).into_bytes();
        bg_msg.append(&mut res.stdout);
        res.stdout = bg_msg;
    }

    res
}

fn is_env_assignment(s: &str) -> bool {
    if let Some((k, _)) = s.split_once('=') {
        !k.is_empty()
            && k.chars()
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false)
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    } else {
        false
    }
}
