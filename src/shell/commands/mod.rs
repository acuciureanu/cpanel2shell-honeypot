//! External-command catalog. Every "external" command operates on the VFS
//! and returns a CmdResult (stdout/stderr/exit). Grouped by category.

use super::ShellSession;
use super::exec::CmdResult;
use super::vfs::Vfs;

mod core;
mod cpanel;
mod dev;
mod fs;
mod misc;
mod net;
mod package;
mod sysinfo;
mod system;

/// Master dispatch. Tries builtin first, then external table, then "command not found".
pub fn dispatch(session: &mut ShellSession, argv: &[String], stdin: &[u8]) -> CmdResult {
    dispatch_with_alias(session, argv, stdin, true)
}

/// Dispatch with optional alias resolution. Used by `command` builtin to bypass aliases.
pub fn dispatch_with_alias(
    session: &mut ShellSession,
    argv: &[String],
    stdin: &[u8],
    resolve_aliases: bool,
) -> CmdResult {
    if argv.is_empty() {
        return CmdResult::empty();
    }

    // Alias substitution (only at first arg, and only once)
    let resolved_argv = if resolve_aliases {
        if let Some(repl) = session.aliases.get(&argv[0]).cloned() {
            let mut new_argv: Vec<String> = shell_words::split(&repl).unwrap_or_default();
            new_argv.extend_from_slice(&argv[1..]);
            new_argv
        } else {
            argv.to_vec()
        }
    } else {
        argv.to_vec()
    };
    if resolved_argv.is_empty() {
        return CmdResult::empty();
    }

    if let Some(r) = super::builtins::run(session, &resolved_argv, stdin) {
        return r;
    }

    let cmd = resolved_argv[0].as_str();
    let cmd_name = cmd.rsplit('/').next().unwrap_or(cmd);

    match cmd_name {
        // coreutils
        "ls" => core::ls(session, &resolved_argv),
        "cat" => core::cat(session, &resolved_argv, stdin),
        "head" => core::head(session, &resolved_argv, stdin),
        "tail" => core::tail(session, &resolved_argv, stdin),
        "wc" => core::wc(session, &resolved_argv, stdin),
        "sort" => core::sort(session, &resolved_argv, stdin),
        "uniq" => core::uniq(session, &resolved_argv, stdin),
        "tr" => core::tr(&resolved_argv, stdin),
        "cut" => core::cut(&resolved_argv, stdin),
        "grep" | "egrep" | "fgrep" => core::grep(session, &resolved_argv, stdin),
        "sed" => core::sed(&resolved_argv, stdin),
        "awk" | "gawk" | "mawk" => core::awk(&resolved_argv, stdin),
        "find" => core::find(session, &resolved_argv),
        "xargs" => core::xargs(session, &resolved_argv, stdin),
        "tee" => core::tee(session, &resolved_argv, stdin),
        "rev" => core::rev(stdin),
        "yes" => core::yes(&resolved_argv),
        "basename" => core::basename(&resolved_argv),
        "dirname" => core::dirname(&resolved_argv),
        "readlink" => core::readlink(session, &resolved_argv),
        "realpath" => core::realpath(session, &resolved_argv),

        // fileops
        "cp" => fs::cp(session, &resolved_argv),
        "mv" => fs::mv(session, &resolved_argv),
        "rm" => fs::rm(session, &resolved_argv),
        "ln" => fs::ln(session, &resolved_argv),
        "mkdir" => fs::mkdir(session, &resolved_argv),
        "rmdir" => fs::rmdir(session, &resolved_argv),
        "touch" => fs::touch(session, &resolved_argv),
        "chmod" => fs::chmod(session, &resolved_argv),
        "chown" => fs::chown(session, &resolved_argv),
        "chgrp" => CmdResult::empty(),
        "stat" => fs::stat(session, &resolved_argv),
        "file" => fs::file(session, &resolved_argv),
        "du" => fs::du(session, &resolved_argv),

        // process / network
        "ps" => system::ps(session, &resolved_argv),
        "top" | "htop" => system::top(),
        "kill" => system::kill(&resolved_argv),
        "killall" | "pkill" => CmdResult::empty(),
        "pgrep" => system::pgrep(&resolved_argv),
        "netstat" => system::netstat(),
        "ss" => system::ss(&resolved_argv),
        "ifconfig" => system::ifconfig(),
        "ip" => system::ip(&resolved_argv),
        "ping" | "ping6" => system::ping(&resolved_argv),
        "traceroute" | "tracepath" => system::traceroute(&resolved_argv),
        "dig" => system::dig(&resolved_argv),
        "host" | "nslookup" => system::host(&resolved_argv),
        "arp" => system::arp(),
        "route" => system::route(),

        // sysinfo
        "uname" => sysinfo::uname(&resolved_argv),
        "hostname" => sysinfo::hostname(session, &resolved_argv),
        "uptime" => sysinfo::uptime(),
        "free" => sysinfo::free(&resolved_argv),
        "df" => sysinfo::df(&resolved_argv),
        "lscpu" => sysinfo::lscpu(),
        "lsb_release" => sysinfo::lsb_release(&resolved_argv),
        "w" => sysinfo::w(),
        "who" => sysinfo::who(),
        "last" => sysinfo::last(),
        "id" => sysinfo::id(session, &resolved_argv),
        "whoami" => CmdResult::ok(
            format!(
                "{}\n",
                session
                    .env
                    .get("USER")
                    .cloned()
                    .unwrap_or_else(|| "root".into())
            )
            .into_bytes(),
        ),
        "groups" => CmdResult::ok(b"root bin daemon sys adm disk wheel\n".to_vec()),
        "tty" => CmdResult::ok(b"/dev/pts/0\n".to_vec()),
        "date" => sysinfo::date(&resolved_argv),
        "uname-r" => sysinfo::uname(&["uname".into(), "-r".into()]),
        "lsmod" => sysinfo::lsmod(),
        "dmesg" => sysinfo::dmesg(),
        "mount" => sysinfo::mount(),
        "umount" => CmdResult::empty(),

        // package managers (stubs)
        "apt" | "apt-get" | "aptitude" => package::apt(&resolved_argv),
        "yum" | "dnf" => package::yum(&resolved_argv),
        "rpm" => package::rpm(&resolved_argv),
        "dpkg" => package::dpkg(&resolved_argv),

        // network downloads
        "wget" => net::wget(session, &resolved_argv),
        "curl" => net::curl(&resolved_argv),
        "scp" | "rsync" | "sftp" => CmdResult::ok(b"".to_vec()),

        // dev tools
        "gcc" | "cc" | "g++" => dev::gcc(&resolved_argv),
        "make" => dev::make(),
        "python" | "python2" => dev::python(&resolved_argv, "2.7.18"),
        "python3" => dev::python(&resolved_argv, "3.9.18"),
        "perl" => dev::perl(&resolved_argv),
        "ruby" => dev::ruby(&resolved_argv),
        "node" | "nodejs" => dev::node(&resolved_argv),
        "php" => dev::php(&resolved_argv),
        "git" => dev::git(&resolved_argv),

        // editors
        "vi" | "vim" | "nano" | "emacs" | "ed" => misc::editor(cmd_name),

        // misc
        "clear" => CmdResult::ok(b"\x1b[2J\x1b[H".to_vec()),
        "reset" => CmdResult::ok(b"\x1b[2J\x1b[H".to_vec()),
        "env" => misc::env(session),
        "sleep" => CmdResult::empty(),
        "true" => CmdResult::empty(),
        "false" => CmdResult::err(vec![], 1),
        "which" => misc::which(session, &resolved_argv),
        "whereis" => misc::whereis(session, &resolved_argv),
        "locate" => misc::locate(session, &resolved_argv),
        "md5sum" | "sha1sum" | "sha256sum" => misc::hashsum(cmd_name, &resolved_argv, stdin, session),
        "tar" => misc::tar(&resolved_argv),
        "gzip" | "gunzip" | "bzip2" | "xz" | "zstd" => CmdResult::empty(),
        "unzip" => CmdResult::ok(b"Archive:  archive.zip\n".to_vec()),
        "crontab" => misc::crontab(&resolved_argv, session),
        "systemctl" => misc::systemctl(&resolved_argv),
        "service" => misc::service(&resolved_argv),
        "journalctl" => misc::journalctl(),
        "history" => misc::history(session),
        "base64" => misc::base64(&resolved_argv, stdin),
        "dd" => misc::dd(&resolved_argv),
        "nohup" => misc::nohup(session, &resolved_argv, stdin),

        // cPanel specific
        "whmapi1" | "whmapi0" => cpanel::whmapi(&resolved_argv),
        "uapi" => cpanel::uapi(&resolved_argv),
        "cpapi2" => cpanel::cpapi2(&resolved_argv),

        // dangerous (block but pretend not found)
        "reboot" | "poweroff" | "shutdown" | "halt" | "init" => CmdResult::err(
            format!("bash: {}: command not found\n", cmd_name).into_bytes(),
            127,
        ),

        unknown => CmdResult::err(
            format!("bash: {}: command not found\n", unknown).into_bytes(),
            127,
        ),
    }
}

pub fn is_known(name: &str) -> bool {
    // Used by `type` builtin.
    matches!(
        name,
        "ls" | "cat"
            | "head"
            | "tail"
            | "wc"
            | "sort"
            | "uniq"
            | "tr"
            | "cut"
            | "grep"
            | "sed"
            | "awk"
            | "find"
            | "xargs"
            | "cp"
            | "mv"
            | "rm"
            | "ln"
            | "mkdir"
            | "rmdir"
            | "touch"
            | "chmod"
            | "chown"
            | "stat"
            | "ps"
            | "kill"
            | "netstat"
            | "ss"
            | "ifconfig"
            | "ip"
            | "ping"
            | "uname"
            | "hostname"
            | "uptime"
            | "free"
            | "df"
            | "id"
            | "whoami"
            | "wget"
            | "curl"
            | "gcc"
            | "python"
            | "python3"
            | "perl"
            | "git"
            | "vi"
            | "vim"
            | "nano"
            | "which"
            | "whereis"
            | "base64"
            | "dd"
            | "nohup"
    )
}

// ─────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────

pub fn read_stdin_or_files(
    session: &ShellSession,
    args: &[String],
    stdin: &[u8],
) -> Result<Vec<u8>, String> {
    if args.is_empty() || args == ["-"] {
        return Ok(stdin.to_vec());
    }
    let mut buf = Vec::new();
    for f in args {
        let path = Vfs::canonicalize(&session.current_dir, f);
        match session.vfs.read(&path) {
            Ok(d) => buf.extend_from_slice(d),
            Err(e) => return Err(e.message(f)),
        }
    }
    Ok(buf)
}

pub fn ts_to_ls(t: i64) -> String {
    use chrono::{DateTime, TimeZone, Utc};
    let dt: DateTime<Utc> = Utc.timestamp_opt(t, 0).single().unwrap_or_else(Utc::now);
    dt.format("%b %e %H:%M").to_string()
}
