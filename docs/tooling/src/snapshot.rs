//! Private source commitments distinguish working bytes, LF text and Git storage.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

/// One source file's byte identities; no file contents are exported.
#[derive(Serialize)]
pub struct FileIdentity {
    /// Repository-relative path.
    pub source: String,
    /// Exact bytes read from the validated working tree.
    pub working_sha256: String,
    /// LF-normalized UTF-8 text, or unchanged binary bytes.
    pub canonical_sha256: String,
    /// SHA-256 of actual Git index blob bytes, if tracked there.
    pub git_index_sha256: Option<String>,
    /// Whether LF/exact canonical source equals Git stored bytes.
    pub canonical_matches_index: bool,
    /// Explicit byte treatment; binary data is never text-normalized.
    pub byte_policy: String,
}

/// Source-bound metadata, not an automatically approved documentation review.
#[derive(Serialize)]
pub struct Snapshot {
    /// Data schema version.
    pub schema: u32,
    /// Actual HEAD observed before and after the capture.
    pub source_sha: String,
    /// Whether any repository changes were present.
    pub dirty: bool,
    /// Every current tracked or nonignored source under the governed roots.
    pub files: Vec<FileIdentity>,
    /// Always false: inventory is not substantive review acceptance.
    pub documentation_review_accepted: bool,
}

fn git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("git source inspection failed: {}", output.status))
    }
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn canonical(bytes: &[u8], attribute: Option<&[u8]>) -> (Vec<u8>, &'static str) {
    let as_text = attribute != Some(b"unset".as_slice())
        && !bytes.iter().take(8000).any(|&b| b == 0)
        && std::str::from_utf8(bytes).is_ok();
    if as_text {
        (
            String::from_utf8_lossy(bytes)
                .replace("\r\n", "\n")
                .into_bytes(),
            "utf8-lf",
        )
    } else {
        (bytes.to_vec(), "exact-binary")
    }
}

fn index_digest(root: &Path) -> Result<String, String> {
    let path = String::from_utf8(git(root, &["rev-parse", "--git-path", "index"])?)
        .map_err(|e| e.to_string())?;
    let bytes = fs::read(root.join(path.trim())).map_err(|e| e.to_string())?;
    Ok(hash(&bytes))
}

/// Capture exact identities and reject a changing HEAD/index/source set.
/// This function never changes Git, review evidence, source bytes or permissions.
pub fn capture(root: &Path) -> Result<Snapshot, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let head = git(&root, &["rev-parse", "HEAD"])?;
    let index_before = index_digest(&root)?;
    let paths = git(
        &root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "crates",
            "adapters",
            "web",
            "build",
            "docs",
        ],
    )?;
    let mut child = Command::new("git")
        .args(["check-attr", "-z", "--stdin", "text"])
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("attribute stdin missing")?;
    let input_bytes = paths.clone();
    let writer = std::thread::spawn(move || input.write_all(&input_bytes));
    let attributes = child.wait_with_output().map_err(|e| e.to_string())?;
    writer
        .join()
        .map_err(|_| "attribute writer panicked")?
        .map_err(|e| e.to_string())?;
    if !attributes.status.success() {
        return Err("Git text-attribute query failed".into());
    }
    let parts = attributes.stdout.split(|&b| b == 0).collect::<Vec<_>>();
    let mut text = std::collections::BTreeMap::new();
    for fields in parts.chunks(3) {
        if fields.len() == 3 {
            text.insert(
                String::from_utf8_lossy(fields[0]).into_owned(),
                fields[2].to_vec(),
            );
        }
    }
    let mut files = Vec::new();
    for raw in paths.split(|&b| b == 0).filter(|p| !p.is_empty()) {
        let source = String::from_utf8(raw.to_vec()).map_err(|e| e.to_string())?;
        let path = root.join(&source);
        let metadata = fs::symlink_metadata(&path).map_err(|e| format!("{source}: {e}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("not a regular source file: {source}"));
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let (canonical, byte_policy) = canonical(&bytes, text.get(&source).map(Vec::as_slice));
        let object = format!(":{source}");
        let blob = git(&root, &["cat-file", "blob", &object]).ok();
        let canonical_sha256 = hash(&canonical);
        let git_index_sha256 = blob.as_deref().map(hash);
        files.push(FileIdentity {
            source,
            working_sha256: hash(&bytes),
            canonical_matches_index: git_index_sha256.as_ref() == Some(&canonical_sha256),
            canonical_sha256,
            git_index_sha256,
            byte_policy: byte_policy.into(),
        });
    }
    for file in &files {
        if hash(&fs::read(root.join(&file.source)).map_err(|e| e.to_string())?)
            != file.working_sha256
        {
            return Err(format!("source changed during snapshot: {}", file.source));
        }
    }
    if git(&root, &["rev-parse", "HEAD"])? != head
        || index_digest(&root)? != index_before
        || git(
            &root,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                "crates",
                "adapters",
                "web",
                "build",
                "docs",
            ],
        )? != paths
    {
        return Err("HEAD or source inventory changed during snapshot; retry explicitly".into());
    }
    Ok(Snapshot {
        schema: 1,
        source_sha: String::from_utf8(head)
            .map_err(|e| e.to_string())?
            .trim()
            .into(),
        dirty: !git(&root, &["status", "--porcelain=v1", "-z"])?.is_empty(),
        files,
        documentation_review_accepted: false,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_and_lf_digest_distinction_is_not_a_review_approval() {
        let (lf, policy) = super::canonical(b"alpha\r\nbeta\r\n", Some(b"auto"));
        assert_eq!(lf, b"alpha\nbeta\n");
        assert_eq!(policy, "utf8-lf");
        for (raw, attribute) in [
            (b"\0PNG\r\n".as_slice(), Some(b"auto".as_slice())),
            (b"\xff\r\n".as_slice(), Some(b"auto".as_slice())),
            (b"explicit binary\r\n".as_slice(), Some(b"unset".as_slice())),
        ] {
            let (bytes, policy) = super::canonical(raw, attribute);
            assert_eq!(bytes, raw);
            assert_eq!(policy, "exact-binary");
        }
    }
}
