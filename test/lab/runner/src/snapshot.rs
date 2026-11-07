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
        // Only the exclusively created, generated direct child of our temp root.
        let safe_parent = self.0.parent() == std::env::temp_dir().canonicalize().ok().as_deref();
        let safe_name = self
            .0
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("hm-rust-lab-"));
        let safe_type =
            fs::symlink_metadata(&self.0).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
        if safe_parent && safe_name && safe_type {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
pub fn freeze(root: &Path, project: &str, evidence: &Path) -> Result<Snapshot> {
    let directory = std::env::temp_dir().canonicalize()?.join(project);
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
    let inventory = paths.clone();
    // Establish deletion state before copying. A disappearance after this point
    // is a source race, not a pre-existing working-tree deletion.
    let states = paths
        .iter()
        .map(|relative| match fs::symlink_metadata(root.join(relative)) {
            Ok(_) => Ok((relative.clone(), true)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok((relative.clone(), false))
            }
            Err(error) => Err(format!("source inventory {relative}: {error}")),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut manifest = Vec::new();
    for (relative, present) in states {
        let source = root.join(&relative);
        if !present {
            manifest.push(json!({"path":relative,"state":"deleted-working-source"}));
            continue;
        }
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) => return Err(format!("source metadata {relative}: {error}").into()),
        };
        if metadata.file_type().is_symlink() {
            return Err("symlink build input refused".into());
        }
        let bytes =
            fs::read(&source).map_err(|error| format!("source read {relative}: {error}"))?;
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
    fs::copy(
        snapshot.0.join("Cargo.lock"),
        evidence.join("built-cargo.lock"),
    )?;
    verify_source(root, &manifest)?;
    let after = Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "-z"])
        .current_dir(root)
        .output()?;
    let mut after = String::from_utf8(after.stdout)?
        .split('\0')
        .filter(|p| allowed(p))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    after.sort();
    after.dedup();
    if after != inventory {
        return Err("source inventory changed during immutable snapshot".into());
    }
    Ok(snapshot)
}
fn verify_source(root: &Path, manifest: &[Value]) -> Result<()> {
    for entry in manifest {
        let relative = entry["path"].as_str().ok_or("source path missing")?;
        let source = root.join(relative);
        if entry["state"] == "deleted-working-source" {
            if fs::symlink_metadata(&source).is_ok() {
                return Err(
                    format!("deleted source reappeared during snapshot: {relative}").into(),
                );
            }
        } else {
            let bytes = fs::read(&source)
                .map_err(|error| format!("source changed during snapshot {relative}: {error}"))?;
            if entry["sha256"] != digest(&bytes) {
                return Err(format!("source bytes changed during snapshot: {relative}").into());
            }
        }
    }
    Ok(())
}
fn allowed(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    (matches!(path, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml"))
        || matches!(
            path,
            "adapters/go/pilot-contract.json"
                | "adapters/go/compiler-input.json"
                | "adapters/go/upstream-fixtures.json"
                | "adapters/go/Dockerfile.fixture"
        )
        || ((path.starts_with("crates/") || path.starts_with("test/lab/"))
            && matches!(extension, "rs" | "toml" | "lock" | "json"))
        || (path.starts_with("deployments/lab/")
            && matches!(extension, "yaml" | "Dockerfile" | "dockerignore"))
        || (path.starts_with("docs/tooling/") && matches!(extension, "rs" | "toml"))
        || (path.starts_with("scripts/ci/") && matches!(extension, "rs" | "toml" | "lock"))
        || (path.starts_with("skills/agent-maintenance/")
            && matches!(extension, "rs" | "toml" | "lock"))
        || matches!(
            path,
            "build/rust/collector-browser.Dockerfile"
                | "build/rust/collector-browser.Dockerfile.dockerignore"
                | "build/rust/browser-product.Dockerfile"
                | "build/rust/browser-product.Dockerfile.dockerignore"
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
    fn source_changes_and_reappearing_deletions_are_refused() {
        let root =
            std::env::temp_dir().join(format!("humanymous-source-check-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("fixture.rs");
        fs::write(&source, b"before").unwrap();
        let manifest = vec![json!({"path":"fixture.rs","sha256":digest(b"before")})];
        assert!(verify_source(&root, &manifest).is_ok());
        fs::write(&source, b"after").unwrap();
        assert!(verify_source(&root, &manifest).is_err());
        let deleted = vec![json!({"path":"absent.rs","state":"deleted-working-source"})];
        assert!(verify_source(&root, &deleted).is_ok());
        fs::write(root.join("absent.rs"), b"returned").unwrap();
        assert!(verify_source(&root, &deleted).is_err());
        fs::remove_file(source).unwrap();
        fs::remove_file(root.join("absent.rs")).unwrap();
        fs::remove_dir(root).unwrap();
    }
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
            "crates/sdk/humanymous-runtime/src/gateway/main.rs",
            "test/lab/runner/Cargo.toml",
            "test/lab/mount_policy.rs",
            "test/lab/browser_expectations.rs",
            "test/lab/browser_outcome.rs",
            "test/lab/browser_wire.rs",
            "test/lab/browser-wire-contract.json",
            "test/lab/runner/browser-outcome-contract.json",
            "test/lab/runner/src/orchestration/browser_failure.rs",
            "test/lab/runner/src/orchestration/browser_failure/panic_site.rs",
            "test/lab/runner/mount-probe-contract.json",
            "deployments/lab/compose.rust.mount.yaml",
            "deployments/lab/images/product.Dockerfile",
        ] {
            assert!(allowed(path));
        }
    }
}
