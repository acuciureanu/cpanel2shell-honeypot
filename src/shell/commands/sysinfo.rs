use super::{CmdResult, ShellSession};
use chrono::Utc;

pub fn uname(argv: &[String]) -> CmdResult {
    let all = argv.iter().any(|a| a == "-a");
    let kernel = argv
        .iter()
        .any(|a| a == "-r" || a == "-a" || argv.len() == 1);
    let sysname = argv
        .iter()
        .any(|a| a == "-s" || a == "-a" || argv.len() == 1);
    let machine = argv.iter().any(|a| a == "-m" || a == "-a");
    let nodename = argv.iter().any(|a| a == "-n" || a == "-a");

    let mut parts = Vec::new();
    if sysname {
        parts.push("Linux");
    }
    if nodename {
        parts.push("cpanel.local");
    }
    if kernel {
        parts.push("5.15.0-105-generic");
    }
    if machine {
        parts.push("x86_64");
    }

    if all {
        CmdResult::ok(b"Linux cpanel.local 5.15.0-105-generic #115-Ubuntu SMP Mon Apr 15 09:52:04 UTC 2024 x86_64 x86_64 x86_64 GNU/Linux\n".to_vec())
    } else {
        CmdResult::ok(format!("{}\n", parts.join(" ")).into_bytes())
    }
}

pub fn hostname(session: &ShellSession, argv: &[String]) -> CmdResult {
    if argv.iter().any(|a| a == "-i" || a == "--ip-address") {
        return CmdResult::ok(b"192.168.1.100\n".to_vec());
    }
    CmdResult::ok(
        format!(
            "{}\n",
            session
                .env
                .get("HOSTNAME")
                .cloned()
                .unwrap_or_else(|| "cpanel.local".into())
        )
        .into_bytes(),
    )
}

pub fn uptime() -> CmdResult {
    CmdResult::ok(
        b" 10:00:00 up 2 days,  1:30,  1 user,  load average: 0.05, 0.02, 0.00\n".to_vec(),
    )
}

pub fn free(argv: &[String]) -> CmdResult {
    let human = argv.iter().any(|a| a == "-h" || a == "--human");
    if human {
        CmdResult::ok(b"              total        used        free      shared  buff/cache   available\nMem:          7.8Gi       876Mi       6.1Gi        45Mi       865Mi       6.7Gi\nSwap:         2.0Gi          0B       2.0Gi\n".to_vec())
    } else {
        CmdResult::ok(b"              total        used        free      shared  buff/cache   available\nMem:        7976000      876200     6234500       45000      865300     6899800\nSwap:       2048000           0     2048000\n".to_vec())
    }
}

pub fn df(argv: &[String]) -> CmdResult {
    let human = argv.iter().any(|a| a == "-h" || a == "--human-readable");
    if human {
        CmdResult::ok(b"Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1        98G   12G   82G  13% /\n/dev/sdb1       492G   45G  423G  10% /home\n".to_vec())
    } else {
        CmdResult::ok(b"Filesystem     1K-blocks     Used Available Use% Mounted on\n/dev/sda1      102400000 12345678  87654322  13% /\n/dev/sdb1      512000000 45000000 423000000  10% /home\n".to_vec())
    }
}

pub fn lscpu() -> CmdResult {
    CmdResult::ok(b"Architecture:            x86_64\nCPU op-mode(s):        32-bit, 64-bit\nAddress sizes:         39 bits physical, 48 bits virtual\nByte Order:            Little Endian\nCPU(s):                4\nOn-line CPU(s) list:   0-3\nVendor ID:             GenuineIntel\nModel name:            Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz\nCPU family:            6\nModel:                 79\nThread(s) per core:    2\nCore(s) per socket:    2\nSocket(s):             1\nStepping:              1\nBogoMIPS:              4800.00\nFlags:                 fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc arch_perfmon rep_good nopl xtopology cpuid tsc_known_freq pni pclmulqdq ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand hypervisor lahf_lm abm cpuid_fault invpcid_single pti ssbd ibrs ibpb stibp fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid xsaveopt arat umip md_clear arch_capabilities\n".to_vec())
}

pub fn lsb_release(argv: &[String]) -> CmdResult {
    let short = argv.iter().any(|a| a == "-s" || a == "--short");
    let id = argv.iter().any(|a| a == "-i" || a == "--id");
    let desc = argv.iter().any(|a| a == "-d" || a == "--description");
    let release = argv.iter().any(|a| a == "-r" || a == "--release");
    let codename = argv.iter().any(|a| a == "-c" || a == "--codename");

    if short {
        if id {
            return CmdResult::ok(b"Ubuntu\n".to_vec());
        }
        if desc {
            return CmdResult::ok(b"Ubuntu 22.04.4 LTS\n".to_vec());
        }
        if release {
            return CmdResult::ok(b"22.04\n".to_vec());
        }
        if codename {
            return CmdResult::ok(b"jammy\n".to_vec());
        }
    }

    CmdResult::ok(b"Distributor ID: Ubuntu\nDescription:    Ubuntu 22.04.4 LTS\nRelease:        22.04\nCodename:       jammy\n".to_vec())
}

pub fn w() -> CmdResult {
    CmdResult::ok(b" 10:00:00 up 2 days,  1:30,  1 user,  load average: 0.05, 0.02, 0.00\nUSER     TTY      FROM             LOGIN@   IDLE   JCPU   PCPU  WHAT\nroot     pts/0    192.168.1.50     09:30    0.00s  0.02s  0.00s -bash\n".to_vec())
}

pub fn who() -> CmdResult {
    CmdResult::ok(b"root     pts/0        2026-05-08 09:30 (192.168.1.50)\n".to_vec())
}

pub fn last() -> CmdResult {
    CmdResult::ok(b"root     pts/0        192.168.1.50     Sat May  8 09:30   still logged in\nroot     pts/0        192.168.1.50     Fri May  7 17:45 - 18:30  (00:45)\nreboot   system boot  5.15.0-105-generi Fri May  7 08:00 - still running\n".to_vec())
}

pub fn id(session: &ShellSession, _argv: &[String]) -> CmdResult {
    let user = session
        .env
        .get("USER")
        .cloned()
        .unwrap_or_else(|| "root".into());
    let uid = session
        .env
        .get("UID")
        .cloned()
        .unwrap_or_else(|| "0".into());
    CmdResult::ok(
        format!(
            "uid={}({}) gid={}({}) groups={}({})\n",
            uid, user, uid, user, uid, user
        )
        .into_bytes(),
    )
}

pub fn date(argv: &[String]) -> CmdResult {
    let now = Utc::now();
    if argv
        .iter()
        .any(|a| a == "-u" || a == "--utc" || a == "--universal")
    {
        CmdResult::ok(format!("{} UTC\n", now.format("%a %b %e %H:%M:%S UTC %Y")).into_bytes())
    } else {
        CmdResult::ok(format!("{}\n", now.format("%a %b %e %H:%M:%S %Z %Y")).into_bytes())
    }
}

pub fn lsmod() -> CmdResult {
    CmdResult::ok(b"Module                  Size  Used by\noverlay               131072  0\nbr_netfilter           28672  0\nxt_conntrack           16384  1\nxt_MASQUERADE          20480  1\nxt_addrtype            16384  2\nfuse                  139264  3\n".to_vec())
}

pub fn dmesg() -> CmdResult {
    CmdResult::ok(b"[    0.000000] Linux version 5.15.0-105-generic (buildd@lcy02-amd64-045) (gcc (Ubuntu 11.4.0-1ubuntu1~22.04) 11.4.0, GNU ld (GNU Binutils for Ubuntu) 2.38) #115-Ubuntu SMP Mon Apr 15 09:52:04 UTC 2024\n[    0.000000] Command line: BOOT_IMAGE=/boot/vmlinuz-5.15.0-105-generic root=UUID=abc123-def456-ghi789-jkl012 mpt quiet\n[    0.000000] KERNEL supported cpus:\n[    0.000000]   Intel GenuineIntel\n[    0.000000]   AMD AuthenticAMD\n[    0.000000]   Centaur CentaurHauls\n".to_vec())
}

pub fn mount() -> CmdResult {
    CmdResult::ok(b"/dev/sda1 on / type ext4 (rw,relatime)\n/dev/sdb1 on /home type ext4 (rw,relatime)\nproc on /proc type proc (rw,nosuid,nodev,noexec,relatime)\nsysfs on /sys type sysfs (rw,nosuid,nodev,noexec,relatime,seclabel)\ntmpfs on /dev/shm type tmpfs (rw,nosuid,nodev,seclabel)\n".to_vec())
}
