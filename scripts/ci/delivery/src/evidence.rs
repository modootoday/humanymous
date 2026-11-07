//! Private evidence covers and exact attachment hashes, separate from reader prose.
use crate::{Result, require, source};
use std::{
    fs,
    path::{Path, PathBuf},
};
/// Call only with an owning formatter's public allowlist, never a raw report.
pub fn emit_public_summary(title: &str, summary: &serde_json::Value) -> Result<()> {
    use std::io::Write;
    require(
        !title.contains(['\n', '\r', '`']),
        "plain summary title required",
    )?;
    let text = serde_json::to_string_pretty(summary).map_err(|e| e.to_string())?;
    println!("{text}");
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let mut output = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        writeln!(output, "### {title}\n\n```json\n{text}\n```").map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn bundle(root: &Path) -> Result<PathBuf> {
    let parent = root.join(".spec.local/root/evidence");
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    require(
        parent
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(root.canonicalize().map_err(|e| e.to_string())?),
        "private evidence escaped repository",
    )?;
    loop {
        let now = time::OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}",
            now.year(),
            u8::from(now.month()),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        // One timestamp ID per bundle; atomic directory reservation avoids a
        // second runner accepting stale files or duplicating a Specfold ID.
        let directory = parent.join(format!("{stamp}-delivery-pilot"));
        match fs::create_dir(&directory) {
            Ok(()) => {
                register(&directory, "Source-bound delivery pilot")?;
                return Ok(directory);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                std::thread::sleep(std::time::Duration::from_millis(25))
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}
fn files(root: &Path, path: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        require(
            !metadata.file_type().is_symlink(),
            "evidence symlink refused",
        )?;
        if path.is_dir() {
            files(root, &path, result)?;
        } else if path != root.join("index.evidence.md") {
            result.push(path);
        }
    }
    Ok(())
}
pub fn register(directory: &Path, title: &str) -> Result<()> {
    require(
        !title.contains(['\n', '\r', ':', '"']),
        "plain evidence title required",
    )?;
    let directory = directory.canonicalize().map_err(|e| e.to_string())?;
    let name = directory
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("evidence bundle name missing")?;
    let stamp = name
        .get(..14)
        .ok_or("UTC timestamp evidence bundle required")?;
    require(
        stamp.bytes().all(|c| c.is_ascii_digit()),
        "UTC timestamp evidence bundle required",
    )?;
    let date = format!(
        "{}-{}-{}T{}:{}:{}Z",
        &stamp[..4],
        &stamp[4..6],
        &stamp[6..8],
        &stamp[8..10],
        &stamp[10..12],
        &stamp[12..14]
    );
    let mut paths = Vec::new();
    files(&directory, &directory, &mut paths)?;
    paths.sort();
    let mut text = format!(
        "---\nid: evidence:root/{stamp}\ntitle: {title}\nstatus: active\ncreated: {date}\nupdated: {date}\nfiles:"
    );
    if paths.is_empty() {
        text.push_str(" []\n");
    } else {
        text.push('\n');
    }
    for path in paths {
        let relative = path
            .strip_prefix(&directory)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        require(
            source::safe_relative(&relative),
            "unsafe evidence attachment path",
        )?;
        let bytes = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        text.push_str(&format!(
            "  - {{path: {relative}, sha256: {}, bytes: {bytes}, store: repo}}\n",
            source::file(&path)?
        ));
    }
    text.push_str("---\n\n# Source-bound delivery evidence\n\nAttachments retain actual process exits, source manifests and component scope.\nA blocked or failed full-product gate is not relabeled as successful.\nRaw private execution evidence is not public capability qualification.\n");
    fs::write(directory.join("index.evidence.md"), text).map_err(|e| e.to_string())
}
