//! Owned flat temporary directories for native CLI tests only.
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Directory(pub PathBuf);
impl Directory {
    pub fn new() -> Self {
        let base = std::env::var_os("HUMANYMOUS_NATIVE_CLI_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = base.join(format!(
            "humanymous-native-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn count(&self) -> usize {
        std::fs::read_dir(&self.0).unwrap().count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        // No recursive traversal/deletion: only files or empty child dirs in this fresh root.
        for file in std::fs::read_dir(&self.0).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(file.path()).or_else(|_| std::fs::remove_dir(file.path()));
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
