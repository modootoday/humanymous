//! Immutable source, dependency and copied-artifact closure helpers.
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
type Result<T> = std::result::Result<T, String>;
pub(super) fn harness_path(raw: &[u8]) -> Result<PathBuf> {
    let paths: Vec<_> = String::from_utf8_lossy(raw)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| {
            row["reason"] == "compiler-artifact" && row["target"]["name"] == "wsgi_server"
        })
        .filter_map(|row| row["executable"].as_str().map(PathBuf::from))
        .collect();
    if paths.len() != 1 {
        return Err("one exact Cargo WSGI executable required".into());
    }
    Ok(paths[0].clone())
}
pub(super) fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut block = [0; 65536];
    loop {
        let size = file.read(&mut block).map_err(|e| e.to_string())?;
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub(super) fn directory_manifest(directory: &Path) -> Result<String> {
    fn visit(base: &Path, path: &Path, rows: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                visit(base, &entry.path(), rows)?;
            } else if kind.is_file() {
                rows.push(format!(
                    "{} {}",
                    entry.path().strip_prefix(base).unwrap().display(),
                    digest(&entry.path())?
                ));
            } else if kind.is_symlink() {
                let path = entry.path();
                let target = fs::read_link(&path).map_err(|e| e.to_string())?;
                let content = if path.is_file() {
                    digest(&path)?
                } else {
                    String::from("directory-link")
                };
                rows.push(format!(
                    "{} symlink:{} {content}",
                    path.strip_prefix(base).unwrap().display(),
                    target.display()
                ));
            } else {
                return Err("unexpected dependency filesystem entry".into());
            }
        }
        Ok(())
    }
    let mut rows = Vec::new();
    visit(directory, directory, &mut rows)?;
    rows.sort();
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}
pub(super) fn directory_manifest_excluding_venv(bundle: &Path) -> Result<String> {
    let mut rows = Vec::new();
    for name in [
        "module/humanymous_python.so",
        "humanymous-authority",
        "wsgi_server",
    ] {
        rows.push(format!("{name} {}", digest(&bundle.join(name))?));
    }
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}
pub(super) fn source_manifest(root: &Path) -> Result<String> {
    let mut command = Command::new("git");
    command
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(root);
    let clean = capture_with_stderr(&mut command, b"", Duration::from_secs(15), 1 << 20)
        .map_err(|_| "source status failed")?;
    if !clean.status.success() || !clean.stdout.is_empty() {
        return Err("tracked source changed".into());
    }
    let mut command = Command::new("git");
    command.args(["ls-files", "-z"]).current_dir(root);
    let files = capture_with_stderr(&mut command, b"", Duration::from_secs(15), 4 << 20)
        .map_err(|_| "source inventory failed")?;
    if !files.status.success() {
        return Err("source inventory failed".into());
    }
    let mut rows = Vec::new();
    for name in files
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name = std::str::from_utf8(name).map_err(|_| "source path encoding")?;
        rows.push(format!("{name} {}", digest(&root.join(name))?));
    }
    rows.sort();
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}
