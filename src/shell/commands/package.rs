use super::CmdResult;

pub fn apt(argv: &[String]) -> CmdResult {
    let sub = argv.get(1).map(String::as_str).unwrap_or("");
    match sub {
        "update" => CmdResult::ok(b"Hit:1 http://archive.ubuntu.com/ubuntu jammy InRelease\nReading package lists... Done\n".to_vec()),
        "install" => {
            let pkg = argv.get(2).cloned().unwrap_or_default();
            CmdResult::ok(format!("Reading package lists... Done\nBuilding dependency tree... Done\nThe following NEW packages will be installed:\n  {}\n0 upgraded, 1 newly installed, 0 to remove and 0 not upgraded.\nNeed to get 12.3 kB of archives.\nAfter this operation, 45.2 kB of additional disk space will be used.\nGet:1 http://archive.ubuntu.com/ubuntu jammy/main amd64 {} amd64 1.0.0 [12.3 kB]\nFetched 12.3 kB in 0s (123 kB/s)\nSelecting previously unselected package {}.\nPreparing to unpack .../{}_1.0.0_amd64.deb ...\nUnpacking {} (1.0.0) ...\nSetting up {} (1.0.0) ...\n", pkg, pkg, pkg, pkg, pkg, pkg).into_bytes())
        }
        _ => CmdResult::ok(b"Usage: apt [options] command\n".to_vec()),
    }
}

pub fn yum(argv: &[String]) -> CmdResult {
    let sub = argv.get(1).map(String::as_str).unwrap_or("");
    match sub {
        "install" => {
            let pkg = argv.get(2).cloned().unwrap_or_default();
            CmdResult::ok(format!("Loaded plugins: fastestmirror\nResolving Dependencies\n--> Running transaction check\n---> Package {}.x86_64 0:1.0.0-1.el7 will be installed\n--> Finished Dependency Resolution\n\nDependencies Resolved\n\n================================================================================\n Package          Arch            Version                Repository        Size\n================================================================================\nInstalling:\n {}               x86_64          1.0.0-1.el7            base              12 k\n\nTransaction Summary\n================================================================================\nInstall  1 Package\n\nTotal download size: 12 k\nInstalled size: 45 k\nDownloading packages:\n {}-1.0.0-1.el7.x86_64.rpm                                                                               |  12 kB  00:00:00\nRunning transaction check\nRunning transaction test\nTransaction test succeeded\nRunning transaction\n  Installing : {}-1.0.0-1.el7.x86_64                                                                 1/1\n  Verifying  : {}-1.0.0-1.el7.x86_64                                                                 1/1\n\nInstalled:\n  {}.x86_64 0:1.0.0-1.el7\n\nComplete!\n", pkg, pkg, pkg, pkg, pkg, pkg).into_bytes())
        }
        _ => CmdResult::ok(b"Loaded plugins: fastestmirror\n".to_vec()),
    }
}

pub fn rpm(_argv: &[String]) -> CmdResult {
    CmdResult::ok(b"package-1.0.0-1.el7.x86_64\n".to_vec())
}

pub fn dpkg(_argv: &[String]) -> CmdResult {
    CmdResult::ok(b"ii  package  1.0.0  amd64  Description\n".to_vec())
}
