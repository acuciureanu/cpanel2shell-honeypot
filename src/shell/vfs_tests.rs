#[cfg(test)]
mod vfs_tests {
    use super::super::vfs::{format_mode, EntryKind, Vfs, VfsError};

    #[test]
    fn new_vfs_has_root() {
        let vfs = Vfs::new();
        assert!(vfs.is_dir("/"));
        assert!(vfs.exists("/"));
    }

    #[test]
    fn write_and_read_file() {
        let mut vfs = Vfs::new();
        vfs.write("/test.txt", b"hello".to_vec(), 0o644).unwrap();
        let data = vfs.read("/test.txt").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn read_missing_file() {
        let vfs = Vfs::new();
        assert!(matches!(vfs.read("/missing.txt"), Err(VfsError::NotFound)));
    }

    #[test]
    fn mkdir_creates_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/foo", 0o755).unwrap();
        assert!(vfs.is_dir("/foo"));
    }

    #[test]
    fn mkdir_duplicate_fails() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/foo", 0o755).unwrap();
        assert!(matches!(vfs.mkdir("/foo", 0o755), Err(VfsError::Exists)));
    }

    #[test]
    fn mkdir_p_creates_nested() {
        let mut vfs = Vfs::new();
        vfs.mkdir_p("/a/b/c", 0o755).unwrap();
        assert!(vfs.is_dir("/a"));
        assert!(vfs.is_dir("/a/b"));
        assert!(vfs.is_dir("/a/b/c"));
    }

    #[test]
    fn unlink_removes_file() {
        let mut vfs = Vfs::new();
        vfs.write("/x.txt", b"data".to_vec(), 0o644).unwrap();
        vfs.unlink("/x.txt").unwrap();
        assert!(!vfs.exists("/x.txt"));
    }

    #[test]
    fn unlink_dir_fails() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/dir", 0o755).unwrap();
        assert!(matches!(vfs.unlink("/dir"), Err(VfsError::IsADir)));
    }

    #[test]
    fn rmdir_empty_succeeds() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/empty", 0o755).unwrap();
        vfs.rmdir("/empty", false).unwrap();
        assert!(!vfs.exists("/empty"));
    }

    #[test]
    fn rmdir_nonempty_fails() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/parent", 0o755).unwrap();
        vfs.write("/parent/child.txt", b"x".to_vec(), 0o644)
            .unwrap();
        assert!(matches!(vfs.rmdir("/parent", false), Err(VfsError::Exists)));
    }

    #[test]
    fn rmdir_recursive_removes_all() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/parent", 0o755).unwrap();
        vfs.write("/parent/child.txt", b"x".to_vec(), 0o644)
            .unwrap();
        vfs.rmdir("/parent", true).unwrap();
        assert!(!vfs.exists("/parent"));
    }

    #[test]
    fn rename_file() {
        let mut vfs = Vfs::new();
        vfs.write("/old.txt", b"content".to_vec(), 0o644).unwrap();
        vfs.rename("/old.txt", "/new.txt").unwrap();
        assert!(!vfs.exists("/old.txt"));
        assert_eq!(vfs.read("/new.txt").unwrap(), b"content");
    }

    #[test]
    fn copy_file() {
        let mut vfs = Vfs::new();
        vfs.write("/src.txt", b"data".to_vec(), 0o644).unwrap();
        vfs.copy("/src.txt", "/dst.txt").unwrap();
        assert_eq!(vfs.read("/src.txt").unwrap(), b"data");
        assert_eq!(vfs.read("/dst.txt").unwrap(), b"data");
    }

    #[test]
    fn symlink_create_and_read() {
        let mut vfs = Vfs::new();
        vfs.write("/target.txt", b"hello".to_vec(), 0o644).unwrap();
        vfs.symlink("/target.txt", "/link.txt").unwrap();
        assert!(vfs.lookup("/link.txt").unwrap().is_symlink());
        assert_eq!(vfs.read("/link.txt").unwrap(), b"hello");
    }

    #[test]
    fn chmod_changes_mode() {
        let mut vfs = Vfs::new();
        vfs.write("/file.txt", b"x".to_vec(), 0o644).unwrap();
        vfs.chmod("/file.txt", 0o755).unwrap();
        let node = vfs.lookup("/file.txt").unwrap();
        assert_eq!(node.mode, 0o755);
    }

    #[test]
    fn chown_changes_owner() {
        let mut vfs = Vfs::new();
        vfs.write("/file.txt", b"x".to_vec(), 0o644).unwrap();
        vfs.chown("/file.txt", 1000, 1000).unwrap();
        let node = vfs.lookup("/file.txt").unwrap();
        assert_eq!(node.uid, 1000);
        assert_eq!(node.gid, 1000);
    }

    #[test]
    fn list_directory() {
        let mut vfs = Vfs::new();
        vfs.write("/dir/a.txt", b"a".to_vec(), 0o644).unwrap();
        vfs.write("/dir/b.txt", b"b".to_vec(), 0o644).unwrap();
        let entries = vfs.list("/dir").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn walk_finds_all() {
        let mut vfs = Vfs::new();
        vfs.write("/a.txt", b"a".to_vec(), 0o644).unwrap();
        vfs.write("/dir/b.txt", b"b".to_vec(), 0o644).unwrap();
        let paths = vfs.walk();
        assert_eq!(paths.len(), 3); // a.txt, dir, dir/b.txt
    }

    #[test]
    fn glob_matching() {
        let mut vfs = Vfs::new();
        vfs.write("/test1.txt", b"1".to_vec(), 0o644).unwrap();
        vfs.write("/test2.txt", b"2".to_vec(), 0o644).unwrap();
        vfs.write("/other.log", b"l".to_vec(), 0o644).unwrap();
        let hits = vfs.glob("/", "/*.txt");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn canonicalize_absolute() {
        assert_eq!(Vfs::canonicalize("/root", "/etc/passwd"), "/etc/passwd");
    }

    #[test]
    fn canonicalize_relative() {
        assert_eq!(Vfs::canonicalize("/root", "file.txt"), "/root/file.txt");
    }

    #[test]
    fn canonicalize_dotdot() {
        assert_eq!(
            Vfs::canonicalize("/root/dir", "../file.txt"),
            "/root/file.txt"
        );
    }

    #[test]
    fn canonicalize_tilde() {
        assert_eq!(Vfs::canonicalize("/root", "~"), "/root");
        assert_eq!(Vfs::canonicalize("/root", "~/file"), "/root/file");
    }

    #[test]
    fn split_handles_dot() {
        assert_eq!(Vfs::split("/a/./b"), vec!["a", "b"]);
    }

    #[test]
    fn split_handles_dotdot() {
        assert_eq!(Vfs::split("/a/b/../c"), vec!["a", "c"]);
    }

    #[test]
    fn format_mode_file() {
        let s = format_mode(EntryKind::File, 0o755);
        assert_eq!(s, "-rwxr-xr-x");
    }

    #[test]
    fn format_mode_dir() {
        let s = format_mode(EntryKind::Dir, 0o755);
        assert_eq!(s, "drwxr-xr-x");
    }

    #[test]
    fn format_mode_symlink() {
        let s = format_mode(EntryKind::Symlink, 0o777);
        assert_eq!(s, "lrwxrwxrwx");
    }

    #[test]
    fn append_to_file() {
        let mut vfs = Vfs::new();
        vfs.write("/log.txt", b"line1\n".to_vec(), 0o644).unwrap();
        vfs.append("/log.txt", b"line2\n", 0o644).unwrap();
        assert_eq!(vfs.read("/log.txt").unwrap(), b"line1\nline2\n");
    }

    #[test]
    fn append_creates_new() {
        let mut vfs = Vfs::new();
        vfs.append("/new.txt", b"data".as_slice(), 0o644).unwrap();
        assert_eq!(vfs.read("/new.txt").unwrap(), b"data");
    }
}
