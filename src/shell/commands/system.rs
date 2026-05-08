use super::{CmdResult, ShellSession};

pub fn ps(session: &ShellSession, argv: &[String]) -> CmdResult {
    let aux = argv
        .iter()
        .any(|a| a.contains('a') && a.contains('u') && a.contains('x'));
    let ef = argv.iter().any(|a| a.contains('e') && a.contains('f'));
    if aux || ef {
        CmdResult::ok(format!(
            "USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND\n\
             root         1  0.0  0.1  21532  7640 ?        Ss   May08   0:01 /sbin/init\n\
             root       500  0.0  0.2  98752 12340 ?        Ss   May08   0:00 sshd: /usr/sbin/sshd\n\
             root      1234  0.0  0.3 152340 23456 ?        Ssl  May08   0:05 /usr/local/cpanel/whostmgr/bin/whostmgrd\n\
             root      2345  0.0  0.1  45678  8765 pts/0    S+   10:00   0:00 -bash\n\
             {}      3456  0.0  0.0  12345  4321 pts/0    R+   10:01   0:00 ps {}\n",
            session.env.get("USER").cloned().unwrap_or_else(|| "root".into()),
            if aux { "aux" } else { "-ef" }
        ).into_bytes())
    } else {
        CmdResult::ok(
            format!(
                "  PID TTY          TIME CMD\n\
             2345 pts/0    00:00:00 bash\n\
             3456 pts/0    00:00:00 ps\n"
            )
            .into_bytes(),
        )
    }
}

pub fn top() -> CmdResult {
    CmdResult::ok(b"top - 10:00:00 up 2 days,  1:30,  1 user,  load average: 0.05, 0.02, 0.00\nTasks: 120 total,   1 running, 119 sleeping,   0 stopped,   0 zombie\n%Cpu(s):  0.3 us,  0.2 sy,  0.0 ni, 99.5 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st\nMiB Mem :   7976.0 total,   6234.5 free,    876.2 used,    865.3 buff/cache\nMiB Swap:   2048.0 total,   2048.0 free,      0.0 used.   6899.8 avail Mem\n\n  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND\n 1234 root      20   0  152340  23456  12345 S   0.0   0.3   0:05.12 whostmgrd\n  500 root      20   0   98752  12340   8765 S   0.0   0.2   0:00.45 sshd\n".to_vec())
}

pub fn kill(argv: &[String]) -> CmdResult {
    let sig = argv
        .iter()
        .find(|a| a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "-15".into());
    let pid = argv
        .iter()
        .find(|a| !a.starts_with('-') && a.parse::<i32>().is_ok())
        .cloned();
    if let Some(p) = pid {
        CmdResult::ok(format!("Killed process {} (signal {})\n", p, sig).into_bytes())
    } else {
        CmdResult::err(b"kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ... or kill -l [sigspec]\n".to_vec(), 1)
    }
}

pub fn pgrep(argv: &[String]) -> CmdResult {
    let pattern = argv.get(1).cloned().unwrap_or_default();
    if pattern.is_empty() {
        return CmdResult::err(b"pgrep: no matching criteria specified\n".to_vec(), 2);
    }
    CmdResult::ok(b"1234\n5678\n".to_vec())
}

pub fn netstat() -> CmdResult {
    CmdResult::ok(b"Active Internet connections (w/o servers)\nProto Recv-Q Send-Q Local Address           Foreign Address         State\ntcp        0      0 192.168.1.100:2083      192.168.1.50:54321      ESTABLISHED\ntcp        0      0 192.168.1.100:2087      192.168.1.50:54322      ESTABLISHED\ntcp        0      0 192.168.1.100:22        192.168.1.50:54320      ESTABLISHED\n".to_vec())
}

pub fn ss(argv: &[String]) -> CmdResult {
    let t = argv.iter().any(|a| a == "-t" || a == "--tcp");
    let a = argv.iter().any(|a| a == "-a" || a == "--all");
    if t || a {
        CmdResult::ok(b"Netid State  Recv-Q Send-Q Local Address:Port  Peer Address:PortProcess\ntcp   ESTAB  0      0      192.168.1.100:2083 192.168.1.50:54321\ntcp   ESTAB  0      0      192.168.1.100:2087 192.168.1.50:54322\ntcp   ESTAB  0      0      192.168.1.100:22   192.168.1.50:54320\n".to_vec())
    } else {
        CmdResult::ok(
            b"Netid State  Recv-Q Send-Q Local Address:Port  Peer Address:PortProcess\n".to_vec(),
        )
    }
}

pub fn ifconfig() -> CmdResult {
    CmdResult::ok("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500\n        inet 192.168.1.100  netmask 255.255.255.0  broadcast 192.168.1.255\n        inet6 fe80::20c:29ff:fe12:3456  prefixlen 64  scopeid 0x20<link>\n        ether 00:0c:29:12:34:56  txqueuelen 1000  (Ethernet)\n        RX packets 123456  bytes 123456789 (117.7 MiB)\n        RX errors 0  dropped 0  overruns 0  frame 0\n        TX packets 654321  bytes 987654321 (941.6 MiB)\n        TX errors 0  dropped 0 overruns 0  carrier 0  collisions 0\n\nlo: flags=73<UP,LOOPBACK,RUNNING>  mtu 65536\n        inet 127.0.0.1  netmask 255.0.0.0\n        inet6 ::1  prefixlen 128  scopeid 0x10<host>\n        loop  txqueuelen 1000  (Local Loopback)\n        RX packets 999999  bytes 99999999 (95.3 MiB)\n        RX errors 0  dropped 0  overruns 0  frame 0\n        TX packets 999999  bytes 99999999 (95.3 MiB)\n        TX errors 0  dropped 0 overruns 0  carrier 0  collisions 0\n".to_string().into_bytes())
}

pub fn ip(argv: &[String]) -> CmdResult {
    let sub = argv.get(1).map(String::as_str).unwrap_or("");
    match sub {
        "addr" | "a" => CmdResult::ok(b"1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN group default qlen 1000\n    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00\n    inet 127.0.0.1/8 scope host lo\n       valid_lft forever preferred_lft forever\n2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP group default qlen 1000\n    link/ether 00:0c:29:12:34:56 brd ff:ff:ff:ff:ff:ff\n    inet 192.168.1.100/24 brd 192.168.1.255 scope global dynamic eth0\n       valid_lft 86394sec preferred_lft 86394sec\n".to_vec()),
        "link" | "l" => CmdResult::ok(b"1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN mode DEFAULT group default qlen 1000\n    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00\n2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP mode DEFAULT group default qlen 1000\n    link/ether 00:0c:29:12:34:56 brd ff:ff:ff:ff:ff:ff\n".to_vec()),
        "route" | "r" => CmdResult::ok(b"default via 192.168.1.1 dev eth0 proto dhcp metric 100\n192.168.1.0/24 dev eth0 proto kernel scope link src 192.168.1.100 metric 100\n".to_vec()),
        _ => CmdResult::err(format!("Object \"{}\" is unknown, try \"ip help\".\n", sub).into_bytes(), 1),
    }
}

pub fn ping(argv: &[String]) -> CmdResult {
    let host = argv.get(1).cloned().unwrap_or_default();
    if host.is_empty() {
        return CmdResult::err(
            b"ping: usage error: Destination address required\n".to_vec(),
            2,
        );
    }
    CmdResult::ok(format!("PING {} (192.0.2.1) 56(84) bytes of data.\n64 bytes from 192.0.2.1: icmp_seq=1 ttl=64 time=0.123 ms\n64 bytes from 192.0.2.1: icmp_seq=2 ttl=64 time=0.145 ms\n64 bytes from 192.0.2.1: icmp_seq=3 ttl=64 time=0.132 ms\n\n--- {} ping statistics ---\n3 packets transmitted, 3 received, 0% packet loss, time 2003ms\nrtt min/avg/max/mdev = 0.123/0.133/0.145/0.012 ms\n", host, host).into_bytes())
}

pub fn traceroute(argv: &[String]) -> CmdResult {
    let host = argv.get(1).cloned().unwrap_or_default();
    if host.is_empty() {
        return CmdResult::err(b"traceroute: missing host operand\n".to_vec(), 1);
    }
    CmdResult::ok(format!("traceroute to {} (192.0.2.1), 30 hops max, 60 byte packets\n 1  192.168.1.1 (192.168.1.1)  0.456 ms  0.389 ms  0.412 ms\n 2  10.0.0.1 (10.0.0.1)  1.234 ms  1.198 ms  1.245 ms\n 3  192.0.2.1 (192.0.2.1)  2.567 ms  2.534 ms  2.589 ms\n", host).into_bytes())
}

pub fn dig(argv: &[String]) -> CmdResult {
    let domain = argv.get(1).cloned().unwrap_or_default();
    if domain.is_empty() {
        return CmdResult::err(b"dig: '@server' is not a valid address\n".to_vec(), 1);
    }
    CmdResult::ok(format!("; <<>> DiG 9.18.1 <<>> {}\n;; global options: +cmd\n;; Got answer:\n;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 12345\n;; flags: qr rd ra; QUERY: 1, ANSWER: 1, AUTHORITY: 0, ADDITIONAL: 1\n\n;; QUESTION SECTION:\n;{}\t\t\t\tIN\tA\n\n;; ANSWER SECTION:\n{}\t\t3600\tIN\tA\t192.0.2.1\n\n;; Query time: 23 msec\n;; SERVER: 192.168.1.1#53(192.168.1.1)\n;; WHEN: Sat May 08 10:00:00 UTC 2026\n;; MSG SIZE  rcvd: 56\n", domain, domain, domain).into_bytes())
}

pub fn host(argv: &[String]) -> CmdResult {
    let domain = argv.get(1).cloned().unwrap_or_default();
    if domain.is_empty() {
        return CmdResult::err(b"host: '' is not a legal name (empty label)\n".to_vec(), 1);
    }
    CmdResult::ok(
        format!(
            "{} has address 192.0.2.1\n{} mail is handled by 10 mail.{}.\n",
            domain, domain, domain
        )
        .into_bytes(),
    )
}

pub fn arp() -> CmdResult {
    CmdResult::ok(b"Address                  HWtype  HWaddress           Flags Mask            Iface\n192.168.1.1              ether   00:50:56:c0:00:08   C                     eth0\n192.168.1.50             ether   00:0c:29:ab:cd:ef   C                     eth0\n".to_vec())
}

pub fn route() -> CmdResult {
    CmdResult::ok(b"Kernel IP routing table\nDestination     Gateway         Genmask         Flags Metric Ref    Use Iface\ndefault         192.168.1.1     0.0.0.0         UG    100    0        0 eth0\n192.168.1.0     0.0.0.0         255.255.255.0   U     100    0        0 eth0\n".to_vec())
}
