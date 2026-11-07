//! Exact local build-input commitment, including theme configuration and tokens.
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn excluded(name: &str) -> bool {
    matches!(
        name,
        "tooling"
            | "scripts"
            | "vendor"
            | "node_modules"
            | ".git"
            | ".bundle"
            | "_site"
            | ".jekyll-cache"
            | ".sass-cache"
            | "AGENTS.md"
            | "INTEGRATION.md"
            | "reviews.json"
    )
}

fn visit(root: &Path, path: &Path, entries: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("symbolic build input: {}", path.display()));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !excluded(&crate::files::relative(root, &entry.path())) {
                visit(root, &entry.path(), entries)?;
            }
        }
    } else if metadata.is_file() {
        entries.push((
            crate::files::relative(root, path),
            fs::read(path).map_err(|e| e.to_string())?,
        ));
    } else {
        return Err(format!("non-regular build input: {}", path.display()));
    }
    Ok(())
}

pub(crate) fn digest(root: &Path) -> Result<String, String> {
    let mut entries = Vec::new();
    visit(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = Sha256::new();
    hash.update(b"humanymous.pages.inputs.v1\0");
    for (path, bytes) in entries {
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    Ok(hex::encode(hash.finalize()))
}
