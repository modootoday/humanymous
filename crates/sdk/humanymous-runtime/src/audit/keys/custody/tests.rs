use super::*;
use std::{
    os::unix::fs::{DirBuilderExt, MetadataExt},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct PrivateDirectory(PathBuf);
impl PrivateDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-description-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        fs::remove_file(self.0.join("keys.lock")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn refused(path: &Path) {
    assert!(
        matches!(Custody::acquire(path,Owner::CurrentUser),Err(e) if e.kind()==io::ErrorKind::WouldBlock)
    );
}

#[test]
fn guard_drop_unlocks_with_duplicate_description_still_alive() {
    let dir = PrivateDirectory::new();
    let first = Custody::acquire(&dir.0, Owner::CurrentUser).unwrap();
    let duplicate = first._lock.duplicate_description().unwrap();
    let identity = |path: &Path| {
        let m = fs::metadata(path.join("keys.lock")).unwrap();
        (m.dev(), m.ino())
    };
    let initial = identity(&dir.0);
    refused(&dir.0);
    drop(first);
    assert_eq!(
        identity(&dir.0),
        initial,
        "guard release must not unlink/replace the lock inode"
    );
    let second = Custody::acquire(&dir.0, Owner::CurrentUser);
    assert!(
        second.is_ok(),
        "stale description retained released custody: {:?}",
        second.as_ref().err()
    );
    let second = second.unwrap();
    refused(&dir.0);
    drop(duplicate);
    refused(&dir.0);
    drop(second);
    let final_guard = Custody::acquire(&dir.0, Owner::CurrentUser).unwrap();
    assert_eq!(identity(&dir.0), initial);
    drop(final_guard);
}
