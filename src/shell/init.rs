//! Pre-populates a `ShellSession` with a realistic fake CentOS+cPanel filesystem,
//! environment variables, and shell history.

use super::ShellSession;

pub fn populate(s: &mut ShellSession) {
    seed_env(s);
    seed_aliases(s);
    seed_dirs(s);
    seed_etc(s);
    seed_proc(s);
    seed_var(s);
    seed_root(s);
    seed_home(s);
    seed_usr(s);
    seed_cpanel(s);
    seed_history(s);
}

fn seed_env(s: &mut ShellSession) {
    let pairs: &[(&str, &str)] = &[
        ("HOME", "/root"),
        ("USER", "root"),
        ("LOGNAME", "root"),
        ("SHELL", "/bin/bash"),
        (
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/root/bin",
        ),
        ("PWD", "/root"),
        ("OLDPWD", "/root"),
        ("HOSTNAME", "cpanel.local"),
        ("HOSTTYPE", "x86_64"),
        ("OSTYPE", "linux-gnu"),
        ("MACHTYPE", "x86_64-redhat-linux-gnu"),
        ("TERM", "xterm-256color"),
        ("LANG", "en_US.UTF-8"),
        ("LC_ALL", "en_US.UTF-8"),
        ("MAIL", "/var/spool/mail/root"),
        ("HISTSIZE", "1000"),
        ("HISTFILE", "/root/.bash_history"),
        ("HISTFILESIZE", "1000"),
        ("PS1", "[\\u@\\h \\W]\\$ "),
        ("EDITOR", "vi"),
        ("SHLVL", "1"),
        ("_", "/usr/bin/env"),
        ("CPANEL", "11.118"),
        ("WHM", "11.118"),
    ];
    for (k, v) in pairs {
        s.env.insert((*k).into(), (*v).into());
    }
}

fn seed_aliases(s: &mut ShellSession) {
    let pairs: &[(&str, &str)] = &[
        ("ll", "ls -l"),
        ("la", "ls -A"),
        ("l", "ls -CF"),
        ("grep", "grep --color=auto"),
        ("egrep", "egrep --color=auto"),
        ("fgrep", "fgrep --color=auto"),
    ];
    for (k, v) in pairs {
        s.aliases.insert((*k).into(), (*v).into());
    }
}

fn seed_dirs(s: &mut ShellSession) {
    for d in [
        "/bin",
        "/sbin",
        "/etc",
        "/etc/cron.d",
        "/etc/cron.daily",
        "/etc/cron.hourly",
        "/etc/httpd",
        "/etc/httpd/conf",
        "/etc/httpd/conf.d",
        "/etc/init.d",
        "/etc/sysconfig",
        "/etc/security",
        "/etc/ssh",
        "/etc/ssl",
        "/etc/ssl/certs",
        "/etc/pki",
        "/etc/yum.repos.d",
        "/etc/postfix",
        "/etc/exim",
        "/etc/mysql",
        "/dev",
        "/dev/pts",
        "/proc",
        "/proc/sys",
        "/proc/sys/kernel",
        "/proc/sys/net",
        "/proc/net",
        "/sys",
        "/lib",
        "/lib64",
        "/media",
        "/mnt",
        "/opt",
        "/root",
        "/root/.ssh",
        "/run",
        "/srv",
        "/tmp",
        "/usr",
        "/usr/bin",
        "/usr/sbin",
        "/usr/lib",
        "/usr/lib64",
        "/usr/local",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/usr/local/lib",
        "/usr/local/cpanel",
        "/usr/local/cpanel/bin",
        "/usr/local/cpanel/etc",
        "/usr/local/cpanel/scripts",
        "/usr/local/cpanel/whostmgr",
        "/usr/local/cpanel/whostmgr/bin",
        "/usr/local/cpanel/whostmgr/docroot",
        "/usr/local/cpanel/Cpanel",
        "/usr/local/cpanel/3rdparty/bin",
        "/usr/share",
        "/usr/share/man",
        "/usr/share/doc",
        "/var",
        "/var/cache",
        "/var/lib",
        "/var/lib/mysql",
        "/var/lib/cpanel",
        "/var/log",
        "/var/log/apache2",
        "/var/log/httpd",
        "/var/log/mysql",
        "/var/spool",
        "/var/spool/cron",
        "/var/spool/mail",
        "/var/www",
        "/var/www/html",
        "/var/www/cgi-bin",
        "/home",
        "/home/cpanel",
        "/home/cpanel/public_html",
        "/home/cpanel/etc",
        "/home/cpanel/mail",
        "/home/cpanel/.cpanel",
        "/home/cpanel/.ssh",
        "/home/cpanel/logs",
    ] {
        let _ = s.vfs.mkdir_p(d, 0o755);
    }
}

fn seed_etc(s: &mut ShellSession) {
    let entries: &[(&str, &[u8], u16)] = &[
        ("/etc/hostname", b"cpanel.local\n", 0o644),
        ("/etc/hosts",
            b"127.0.0.1   localhost localhost.localdomain localhost4 localhost4.localdomain4\n::1         localhost localhost.localdomain localhost6 localhost6.localdomain6\n192.168.1.100 cpanel.local cpanel\n", 0o644),
        ("/etc/passwd",
            b"root:x:0:0:root:/root:/bin/bash\nbin:x:1:1:bin:/bin:/sbin/nologin\ndaemon:x:2:2:daemon:/sbin:/sbin/nologin\nadm:x:3:4:adm:/var/adm:/sbin/nologin\nlp:x:4:7:lp:/var/spool/lpd:/sbin/nologin\nsync:x:5:0:sync:/sbin:/bin/sync\nshutdown:x:6:0:shutdown:/sbin:/sbin/shutdown\nhalt:x:7:0:halt:/sbin:/sbin/halt\nmail:x:8:12:mail:/var/spool/mail:/sbin/nologin\noperator:x:11:0:operator:/root:/sbin/nologin\nftp:x:14:50:FTP User:/var/ftp:/sbin/nologin\nnobody:x:99:99:Nobody:/:/sbin/nologin\nsystemd-network:x:192:192:systemd Network Management:/:/sbin/nologin\ndbus:x:81:81:System message bus:/:/sbin/nologin\nsshd:x:74:74:Privilege-separated SSH:/var/empty/sshd:/sbin/nologin\npostfix:x:89:89::/var/spool/postfix:/sbin/nologin\nchrony:x:998:996::/var/lib/chrony:/sbin/nologin\nmailnull:x:47:47::/var/spool/mqueue:/sbin/nologin\nsmmsp:x:51:51::/var/spool/mqueue:/sbin/nologin\napache:x:48:48:Apache:/usr/share/httpd:/sbin/nologin\nmysql:x:27:27:MySQL Server:/var/lib/mysql:/bin/false\nnamed:x:25:25:Named:/var/named:/bin/false\ncpanel:x:1000:1000:cPanel User:/home/cpanel:/usr/local/cpanel/bin/jailshell\n", 0o644),
        ("/etc/shadow",
            b"root:$6$rounds=5000$7v8H4kK1$abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUV:19000:0:99999:7:::\nbin:*:18353:0:99999:7:::\ndaemon:*:18353:0:99999:7:::\ncpanel:$6$abcd1234$ZYXWVUTSRQPONMLKJIHGFEDCBA9876543210zyxwvutsrqponmlkjihgfedcba:19000:0:99999:7:::\n", 0o600),
        ("/etc/group",
            b"root:x:0:\nbin:x:1:\ndaemon:x:2:\nsys:x:3:\nadm:x:4:\ntty:x:5:\ndisk:x:6:\nwheel:x:10:root\nmail:x:12:postfix\nnobody:x:99:\napache:x:48:\nmysql:x:27:\ncpanel:x:1000:\n", 0o644),
        ("/etc/resolv.conf",
            b"; generated by NetworkManager\nnameserver 8.8.8.8\nnameserver 8.8.4.4\nsearch local\n", 0o644),
        ("/etc/os-release",
            b"NAME=\"CentOS Linux\"\nVERSION=\"7 (Core)\"\nID=\"centos\"\nID_LIKE=\"rhel fedora\"\nVERSION_ID=\"7\"\nPRETTY_NAME=\"CentOS Linux 7 (Core)\"\nANSI_COLOR=\"0;31\"\nCPE_NAME=\"cpe:/o:centos:centos:7\"\nHOME_URL=\"https://www.centos.org/\"\n", 0o644),
        ("/etc/redhat-release", b"CentOS Linux release 7.9.2009 (Core)\n", 0o644),
        ("/etc/fstab",
            b"# /etc/fstab\nUUID=12345678-aaaa-bbbb-cccc-dddddddddddd  /         ext4    defaults  1 1\nUUID=87654321-eeee-ffff-0000-111111111111  /home     ext4    defaults  1 2\nUUID=11223344-5566-7788-99aa-bbccddeeff00  swap      swap    defaults  0 0\n", 0o644),
        ("/etc/crontab",
            b"SHELL=/bin/bash\nPATH=/sbin:/bin:/usr/sbin:/usr/bin\nMAILTO=root\n0 * * * * root run-parts /etc/cron.hourly\n0 2 * * * root run-parts /etc/cron.daily\n0 4 * * 0 root run-parts /etc/cron.weekly\n", 0o644),
        ("/etc/sudoers", b"root  ALL=(ALL)   ALL\n%wheel  ALL=(ALL)  ALL\n", 0o440),
        ("/etc/nsswitch.conf",
            b"passwd:     files\nshadow:     files\ngroup:      files\nhosts:      files dns\nnetworks:   files dns\nservices:   files\n", 0o644),
        ("/etc/sysctl.conf",
            b"net.ipv4.ip_forward = 0\nnet.ipv4.conf.default.rp_filter = 1\nkernel.sysrq = 0\n", 0o644),
        ("/etc/ssh/sshd_config",
            b"Port 22\nProtocol 2\nPermitRootLogin yes\nPubkeyAuthentication yes\nPasswordAuthentication yes\nUsePAM yes\nSubsystem sftp /usr/libexec/openssh/sftp-server\n", 0o600),
        ("/etc/issue", b"\\S\nKernel \\r on an \\m\n\n", 0o644),
        ("/etc/motd", b"Welcome to cPanel.\n", 0o644),
        ("/etc/httpd/conf/httpd.conf",
            b"# Apache server config\nServerRoot \"/etc/httpd\"\nListen 80\nUser apache\nGroup apache\nServerAdmin root@cpanel.local\n", 0o644),
        ("/var/spool/cron/root",
            b"# crontab for root\n0 2 * * * /usr/local/cpanel/scripts/upcp\n0 1 * * * /scripts/cpbackup\n*/5 * * * * /usr/local/cpanel/whostmgr/bin/dnsadmin\n", 0o600),
    ];
    for (p, d, m) in entries {
        s.vfs.write(p, d.to_vec(), *m).ok();
    }
}

fn seed_proc(s: &mut ShellSession) {
    let entries: &[(&str, &[u8])] = &[
        ("/proc/version",
            b"Linux version 3.10.0-1160.el7.x86_64 (builder@kbuilder.bsys.centos.org) (gcc version 4.8.5 20150623 (Red Hat 4.8.5-44) (GCC) ) #1 SMP Mon Oct 19 16:18:59 UTC 2020\n"),
        ("/proc/cpuinfo",
            b"processor\t: 0\nvendor_id\t: GenuineIntel\ncpu family\t: 6\nmodel\t\t: 85\nmodel name\t: Intel(R) Xeon(R) Gold 6132 CPU @ 2.60GHz\nstepping\t: 4\ncpu MHz\t\t: 2593.992\ncache size\t: 19712 KB\nphysical id\t: 0\nsiblings\t: 4\ncore id\t\t: 0\ncpu cores\t: 2\nbogomips\t: 5187.98\nclflush size\t: 64\ncache_alignment\t: 64\naddress sizes\t: 46 bits physical, 48 bits virtual\n"),
        ("/proc/meminfo",
            b"MemTotal:       16384000 kB\nMemFree:         2048000 kB\nMemAvailable:    7372800 kB\nBuffers:          512000 kB\nCached:          5632000 kB\nSwapTotal:       4194304 kB\nSwapFree:        4194304 kB\n"),
        ("/proc/uptime", b"3888000.50 7776000.00\n"),
        ("/proc/loadavg", b"0.12 0.08 0.05 1/142 12345\n"),
        ("/proc/mounts",
            b"/dev/sda1 / ext4 rw,relatime,data=ordered 0 0\nproc /proc proc rw,nosuid,nodev,noexec,relatime 0 0\nsysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0\ntmpfs /dev/shm tmpfs rw,nosuid,nodev 0 0\n/dev/sdb1 /home ext4 rw,relatime,data=ordered 0 0\n"),
    ];
    for (p, d) in entries {
        s.vfs.write(p, d.to_vec(), 0o444).ok();
    }
}

fn seed_var(s: &mut ShellSession) {
    let entries: &[(&str, &[u8])] = &[
        ("/var/log/messages",
            b"May  8 10:00:01 cpanel systemd[1]: Started Daily activities.\nMay  8 10:00:23 cpanel kernel: [12345.678901] eth0: link up\nMay  8 10:30:15 cpanel sshd[1456]: Accepted publickey for root from 192.168.1.50 port 54321 ssh2\nMay  8 11:00:00 cpanel /USR/SBIN/CRON[5678]: (root) CMD (/usr/local/cpanel/scripts/upcp)\n"),
        ("/var/log/secure",
            b"May  8 10:30:15 cpanel sshd[1456]: Accepted publickey for root from 192.168.1.50 port 54321 ssh2\nMay  8 10:30:15 cpanel sshd[1456]: pam_unix(sshd:session): session opened for user root by (uid=0)\n"),
        ("/var/log/auth.log",
            b"May  8 10:30:15 cpanel sshd[1456]: Accepted publickey for root from 192.168.1.50 port 54321 ssh2\n"),
        ("/var/log/cron",
            b"May  8 10:00:01 cpanel CROND[1001]: (root) CMD (run-parts /etc/cron.hourly)\n"),
        ("/var/log/maillog",
            b"May  8 10:00:00 cpanel exim[2100]: 2026-05-08 10:00:00 Start queue run: pid=2100\n"),
        ("/var/log/httpd/access_log",
            b"192.168.1.50 - - [08/May/2026:10:00:00 +0000] \"GET / HTTP/1.1\" 200 1234 \"-\" \"Mozilla/5.0\"\n"),
        ("/var/log/httpd/error_log",
            b"[Sat May 08 10:00:00.123456 2026] [mpm_prefork:notice] [pid 1520] AH00163: Apache/2.4.6 (CentOS) configured\n"),
        ("/var/log/exim_mainlog",
            b"2026-05-08 10:00:00 Start queue run: pid=2100\n"),
        ("/var/www/html/index.html",
            b"<!DOCTYPE html><html><head><title>cPanel</title></head><body><h1>It works!</h1></body></html>\n"),
        ("/var/www/html/robots.txt", b"User-agent: *\nDisallow: /cpanel\nDisallow: /whm\n"),
    ];
    for (p, d) in entries {
        s.vfs.write(p, d.to_vec(), 0o644).ok();
    }
}

fn seed_root(s: &mut ShellSession) {
    let entries: &[(&str, &[u8], u16)] = &[
        ("/root/.bashrc",
            b"# .bashrc\nalias rm='rm -i'\nalias cp='cp -i'\nalias mv='mv -i'\nif [ -f /etc/bashrc ]; then\n\t. /etc/bashrc\nfi\n", 0o644),
        ("/root/.bash_profile",
            b"# .bash_profile\nif [ -f ~/.bashrc ]; then\n\t. ~/.bashrc\nfi\nPATH=$PATH:$HOME/bin\nexport PATH\n", 0o644),
        ("/root/.bash_logout", b"# ~/.bash_logout\nclear\n", 0o644),
        ("/root/.bash_history",
            b"ls -la\ncd /var/log\ntail -n 100 messages\nps aux | grep cpanel\nsystemctl status httpd\nyum update -y\nwhmapi1 version\nuname -a\nfree -m\ndf -h\nnetstat -tlnp\n", 0o600),
        ("/root/.ssh/authorized_keys",
            b"ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC9wQ2admin_key admin@workstation\n", 0o600),
        ("/root/.ssh/known_hosts",
            b"github.com,140.82.112.3 ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEAq2A7hRGmdnm9tUDbO9IDSwBK6TbQa+PXYPCPy6rbTrTtw==\n", 0o644),
        ("/root/.viminfo", b"# Vim viminfo file\n", 0o600),
        ("/root/.lesshst", b"", 0o600),
    ];
    for (p, d, m) in entries {
        s.vfs.write(p, d.to_vec(), *m).ok();
    }
}

fn seed_home(s: &mut ShellSession) {
    let entries: &[(&str, &[u8], u16, u32)] = &[
        ("/home/cpanel/.bashrc", b"# cpanel user .bashrc\nalias ll='ls -l'\n", 0o644, 1000),
        ("/home/cpanel/.bash_profile", b"# .bash_profile\n[[ -f ~/.bashrc ]] && . ~/.bashrc\n", 0o644, 1000),
        ("/home/cpanel/.bash_history",
            b"cd public_html\nls -la\ncat .htaccess\nphp -v\nmysql -u cpanel_db -p\nexit\n", 0o600, 1000),
        ("/home/cpanel/public_html/index.html",
            b"<!DOCTYPE html><html><head><title>Welcome</title></head><body><h1>Hello cPanel</h1></body></html>\n", 0o644, 1000),
        ("/home/cpanel/public_html/.htaccess",
            b"Options +FollowSymLinks\nRewriteEngine On\nRewriteCond %{HTTPS} off\nRewriteRule ^(.*)$ https://%{HTTP_HOST}%{REQUEST_URI} [L,R=301]\n", 0o644, 1000),
        ("/home/cpanel/public_html/wp-config.php",
            b"<?php\ndefine('DB_NAME', 'cpanel_wp');\ndefine('DB_USER', 'cpanel_dbuser');\ndefine('DB_PASSWORD', '************');\ndefine('DB_HOST', 'localhost');\n$table_prefix = 'wp_';\nrequire_once ABSPATH . 'wp-settings.php';\n", 0o600, 1000),
        ("/home/cpanel/.cpanel/datastore.yaml", b"---\nlast_login: 2026-05-08T10:00:00Z\nplan: cpanel-pro\n", 0o600, 1000),
        ("/home/cpanel/etc/main.conf",
            b"# cPanel user main configuration\nDOMAIN=example.com\nIP=192.168.1.100\n", 0o644, 1000),
    ];
    for (p, d, m, uid) in entries {
        s.vfs.write(p, d.to_vec(), *m).ok();
        s.vfs.chown(p, *uid, *uid).ok();
    }
}

fn seed_usr(s: &mut ShellSession) {
    let names: &[&str] = &[
        "bash", "sh", "ls", "cat", "head", "tail", "ps", "netstat", "ss", "ifconfig", "ip", "ping",
        "curl", "wget", "perl", "python", "python3", "ruby", "node", "php", "git", "vi", "vim",
        "nano", "make", "gcc", "tar", "gzip", "find", "grep", "sed", "awk", "wc", "sort", "uniq",
        "tr", "cut", "ln", "cp", "mv", "rm", "mkdir", "chmod", "chown", "stat", "file", "du", "df",
        "free", "uname", "hostname", "id", "whoami", "uptime", "date", "env",
    ];
    for n in names {
        let p = format!("/usr/bin/{}", n);
        s.vfs
            .write(&p, format!("\x7fELF{}", n).into_bytes(), 0o755)
            .ok();
    }
    let sbin: &[&str] = &["sshd", "httpd", "mysqld", "exim", "crond", "nologin"];
    for n in sbin {
        let p = format!("/usr/sbin/{}", n);
        s.vfs
            .write(&p, format!("\x7fELF{}", n).into_bytes(), 0o755)
            .ok();
    }
}

fn seed_cpanel(s: &mut ShellSession) {
    let entries: &[(&str, &[u8])] = &[
        ("/usr/local/cpanel/version", b"118.0.13\n"),
        (
            "/etc/cpupdate.conf",
            b"UPDATES=daily\nRPMUP=daily\nSARULESUP=daily\n",
        ),
        (
            "/var/cpanel/version",
            b"# This file is no longer used; see /usr/local/cpanel/version\n",
        ),
        (
            "/usr/local/cpanel/etc/whostmgrd.conf",
            b"port=2087\nlisten=0.0.0.0\nssl=1\n",
        ),
        (
            "/usr/local/cpanel/scripts/upcp",
            b"#!/bin/bash\n# upcp - cPanel update wrapper\n/usr/local/cpanel/bin/upcp \"$@\"\n",
        ),
        (
            "/usr/local/cpanel/whostmgr/docroot/index.html",
            b"<html><body>WHM</body></html>\n",
        ),
    ];
    for (p, d) in entries {
        s.vfs.write(p, d.to_vec(), 0o644).ok();
    }
}

fn seed_history(s: &mut ShellSession) {
    let cmds: &[&str] = &[
        "ls -la",
        "cd /var/log",
        "tail -n 100 messages",
        "ps aux | grep cpanel",
        "systemctl status httpd",
        "yum update -y",
        "whmapi1 version",
        "uname -a",
        "free -m",
        "df -h",
        "netstat -tlnp",
    ];
    for c in cmds {
        s.history.push((*c).to_string());
    }
}

/// Initialize total_bytes counter from pre-seeded content.
pub fn init_vfs_quota(s: &mut ShellSession) {
    s.vfs.total_bytes = s.vfs.walk().iter().map(|(_, _, size)| size).sum();
}
