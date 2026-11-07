//! Allowlisted immutable build bytes, bound separately from a dirty checkout SHA.
use crate::docker::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err("Git workspace required".into());
    }
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()).canonicalize()?)
}
pub struct Snapshot(pub PathBuf);
impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn freeze(root: &Path, project: &str, evidence: &Path) -> Result<Snapshot> {
    let directory = std::env::temp_dir().join(project);
    fs::create_dir(&directory)?;
    let snapshot = Snapshot(directory);
    let output = Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "-z"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("source inventory failed".into());
    }
    let mut paths = String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|p| allowed(p))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let mut manifest = Vec::new();
    for relative in paths {
        let source = root.join(&relative);
        if fs::symlink_metadata(&source)?.file_type().is_symlink() {
            return Err("symlink build input refused".into());
        }
        let bytes = fs::read(&source)?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("source input over budget".into());
        }
        let destination = snapshot.0.join(&relative);
        fs::create_dir_all(destination.parent().ok_or("source parent missing")?)?;
        fs::write(destination, &bytes)?;
        manifest.push(json!({"path":relative,"sha256":digest(&bytes),"bytes":bytes.len()}));
    }
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;
    write(
        evidence,
        "source-manifest.json",
        &json!({"head":String::from_utf8(head.stdout)?.trim(),"dirty":String::from_utf8(dirty.stdout)?,"files":manifest,"meaning":"exact immutable image source; HEAD alone is not its identity"}),
    )?;
    Ok(snapshot)
}
fn allowed(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    (matches!(path, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml"))
        || ((path.starts_with("crates/") || path.starts_with("test/lab/"))
            && matches!(extension, "rs" | "toml" | "lock" | "json"))
        || (path.starts_with("deployments/lab/")
            && matches!(extension, "yaml" | "Dockerfile" | "dockerignore"))
        || (path.starts_with("docs/tooling/") && matches!(extension, "rs" | "toml"))
        || matches!(
            path,
            "build/rust/collector-browser.Dockerfile"
                | "build/rust/collector-browser.Dockerfile.dockerignore"
        )
        || matches!(
            path,
            "web/js/features/mouse.js" | "web/js/features/keystroke.js" | "web/js/advanced.js"
        )
}
pub fn write(directory: &Path, name: &str, value: &Value) -> Result<()> {
    fs::write(directory.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_allowlist_excludes_private_and_foreign_execution() {
        for path in [
            ".spec.local/key.json",
            "deployments/lab/authority/pilot.mjs",
            "test/lab/run.py",
            "crates/x/secret.pem",
            ".git/config",
        ] {
            assert!(!allowed(path));
        }
        for path in [
            "Cargo.lock",
            "crates/runtime/src/main.rs",
            "test/lab/runner/Cargo.toml",
            "deployments/lab/images/product.Dockerfile",
        ] {
            assert!(allowed(path));
        }
    }
}
