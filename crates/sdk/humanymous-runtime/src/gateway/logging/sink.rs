//! Append-only regular-file sinks (legacy internal/logger sink.go at baseline
//! 981d25c). A path sink refuses symlinks, non-regular targets and a path that
//! changes identity between the pre-open check and the opened handle.
use super::Refusal;
use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};

/// Canonical absolute path used for duplicate detection. Relative paths and
/// `..` components are refused (Go resolved them against the working
/// directory; the kernel never consults ambient process state).
pub(super) fn canonical(raw: &str) -> Result<PathBuf, Refusal> {
    let path = Path::new(raw);
    if raw.trim().is_empty()
        || !path.is_absolute()
        || path.components().any(|c| c == Component::ParentDir)
    {
        return Err(Refusal::SinkPath);
    }
    Ok(path.components().collect())
}

pub(super) fn duplicate_key(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.into_owned()
    }
}

pub(super) fn open_append_regular(path: &Path) -> Result<File, Refusal> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.file_type().is_file() => {
            return Err(Refusal::SinkTarget);
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Refusal::SinkOpen),
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o640);
    }
    let file = options.open(path).map_err(|_| Refusal::SinkOpen)?;
    let opened = file.metadata().map_err(|_| Refusal::SinkOpen)?;
    if !opened.is_file() {
        return Err(Refusal::SinkTarget);
    }
    // Re-check the name after open against the open handle: a symlink or
    // file swapped in between the first check and the open is refused.
    let named = fs::symlink_metadata(path).map_err(|_| Refusal::SinkOpen)?;
    if named.file_type().is_symlink() || !same_file(&opened, &named) {
        return Err(Refusal::SinkTarget);
    }
    Ok(file)
}

#[cfg(unix)]
pub(super) fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

/// Without a stable file identity API only the type is compared; duplicate
/// paths are still refused by canonical name before opening.
#[cfg(not(unix))]
pub(super) fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.is_file() && right.is_file()
}
