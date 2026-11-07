//! Actual Git and working-byte commitments. Text is not normalized after reading.
use crate::{Result, command, require};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
pub const SCOPES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates",
    "adapters",
    "scripts/ci",
    "scripts/migration",
    "skills/agent-maintenance",
    "build/rust",
    "test/lab",
    "test/rust-wargame",
    "deployments/lab",
    "docs/tooling",
    "web/js",
    "docs/reviews.json",
    ".github/workflows",
];
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn file(path: &Path) -> Result<String> {
    Ok(sha(&std::fs::read(path).map_err(|e| e.to_string())?))
}
pub fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
pub fn head(root: &Path) -> Result<String> {
    Ok(command::text(root, "git", &["rev-parse", "HEAD"])?
        .trim()
        .into())
}
pub fn snapshot(root: &Path) -> Result<Value> {
    let head = head(root)?;
    let mut canonical_args = vec!["ls-tree", "-r", "--full-tree", &head, "--"];
    canonical_args.extend_from_slice(SCOPES);
    let canonical = command::text(root, "git", &canonical_args)?;
    let canonical_git_objects: Vec<_> = canonical
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(metadata, path)| {
            metadata
                .split_whitespace()
                .nth(2)
                .map(|id| json!({"path":path,"git_object_id":id}))
        })
        .collect();
    let mut args = vec![
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
        "--",
    ];
    args.extend_from_slice(SCOPES);
    let listed = command::text(root, "git", &args)?;
    let paths: BTreeSet<_> = listed.split('\0').filter(|path| !path.is_empty()).collect();
    let mut files = Vec::new();
    for path in paths {
        require(
            safe_relative(path),
            format!("unsafe governed source path: {path}"),
        )?;
        let full = root.join(path);
        if !full.exists() {
            files.push(json!({"path":path,"state":"deleted"}));
            continue;
        }
        let metadata = std::fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
        require(
            !metadata.file_type().is_symlink(),
            format!("governed symlink rejected: {path}"),
        )?;
        let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
        files.push(json!({"path":path,"sha256":sha(&bytes),"bytes":bytes.len()}));
    }
    let manifest_sha256 = sha(&serde_json::to_vec(&files).map_err(|e| e.to_string())?);
    let mut status_args = vec!["status", "--porcelain=v1", "--untracked-files=all", "--"];
    status_args.extend_from_slice(SCOPES);
    Ok(
        json!({"schema":"humanymous.delivery-source/1","source_sha":head,"manifest_sha256":manifest_sha256,"dirty":!command::text(root,"git",&status_args)?.trim().is_empty(),"files":files,"canonical_git_objects":canonical_git_objects,"canonical_git_tree_sha256":sha(canonical.as_bytes()),"hash_meaning":"files SHA256 hashes actual executed working bytes; canonical Git object IDs identify stored Git bytes and are not interchangeable hashes"}),
    )
}
pub fn lf_files(root: &Path, paths: impl Iterator<Item = String>) -> Result<()> {
    for path in paths {
        let bytes = std::fs::read(root.join(&path)).map_err(|e| e.to_string())?;
        require(
            !bytes.contains(&b'\r'),
            format!("authored source must contain LF Git bytes: {path}"),
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_refuse_cross_platform_escape() {
        for path in ["../a", "/a", "C:/a", "a\\b", "a/../b", ""] {
            assert!(!safe_relative(path));
        }
        assert!(safe_relative("crates/a/src/lib.rs"));
    }
    #[test]
    fn hashes_bind_exact_line_endings() {
        assert_ne!(sha(b"x\n"), sha(b"x\r\n"));
        assert_eq!(
            sha(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
