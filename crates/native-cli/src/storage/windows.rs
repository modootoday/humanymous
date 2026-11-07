//! Safe standard-library handles pin every non-reparse directory until publication.
use super::Receipt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, Prefix},
    sync::atomic::{AtomicU64, Ordering},
};

// Win32 FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT.
const DIRECTORY_FLAGS: u32 = 0x02000000 | 0x00200000;
const REPARSE: u32 = 0x400;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn pin(root: &Path) -> io::Result<Vec<File>> {
    let mut components = root.components();
    if !matches!(components.next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)))
        || !matches!(components.next(), Some(Component::RootDir))
        || !components.all(|c| matches!(c, Component::Normal(name) if !name.to_string_lossy().contains(':') && !name.to_string_lossy().ends_with(['.', ' '])))
        || root.as_os_str().len() > 4096 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "workspace must be an existing absolute local-drive directory without traversal/device/stream aliases"));
    }
    let mut ancestors: Vec<_> = root.ancestors().collect();
    ancestors.reverse();
    let mut pinned = Vec::with_capacity(ancestors.len());
    for path in ancestors {
        // A read-capable directory handle participates in delete-sharing checks.
        // READ|WRITE sharing permits
        // ordinary workspace activity, but omitting DELETE prevents rename/swap.
        // OPEN_REPARSE_POINT inspects the opened final object, never canonicalizes
        // a junction into an apparently safe directory.
        let handle = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(DIRECTORY_FLAGS)
            .open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & REPARSE != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "reparse/non-directory workspace ancestor refused",
            ));
        }
        pinned.push(handle);
    }
    let canonical = fs::canonicalize(root)?;
    let input_drive = match root.components().next() {
        Some(Component::Prefix(p)) => p.kind(),
        _ => unreachable!("validated drive prefix"),
    };
    if !matches!((input_drive, canonical.components().next()),
        (Prefix::Disk(expected), Some(Component::Prefix(actual)))
            if matches!(actual.kind(), Prefix::VerbatimDisk(drive) | Prefix::Disk(drive) if drive.eq_ignore_ascii_case(&expected)))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "network or substituted-drive workspace aliases are unsupported",
        ));
    }
    Ok(pinned)
}

pub(super) fn publish(root: &Path, output: &str, bytes: &[u8]) -> io::Result<Receipt> {
    publish_with(
        root,
        output,
        bytes,
        |file, bytes| {
            file.write_all(bytes)?;
            file.sync_all()
        },
        |path| fs::remove_file(path),
    )
}

fn publish_with(
    root: &Path,
    output: &str,
    bytes: &[u8],
    write: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    cleanup: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<Receipt> {
    let _ancestors = pin(root)?;
    let destination = root.join(output);
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "destination already exists; nothing overwritten",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    let mut staged = None;
    for _ in 0..16 {
        let name = format!(
            ".humanymous-native-{}-{}.tmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let path = root.join(&name);
        // READ-only sharing denies both other writers and entry replacement.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(1)
            .open(&path)
        {
            Ok(file) => {
                staged = Some((name, path, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error),
        }
    }
    let (name, path, mut file) = staged.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "temporary-name collision budget exhausted",
        )
    })?;
    let outcome = write(&mut file, bytes).and_then(|()| fs::hard_link(&path, &destination));
    // The original temporary entry cannot be replaced while the handle is open.
    // After close, cleanup assumes trusted/non-mutating workspace entries. A
    // hostile same-user writer is explicitly outside this tool's security model.
    drop(file);
    let cleanup_complete = cleanup(&path).is_ok();
    match outcome {
        Ok(()) => Ok(Receipt {
            published: true,
            cleanup_complete,
            temporary_file: (!cleanup_complete).then_some(name),
        }),
        Err(error) => Err(io::Error::new(
            error.kind(),
            format!(
                "publication failed before destination creation: {error}; staging_cleanup_complete={cleanup_complete}; temporary={name}"
            ),
        )),
    }
}

#[cfg(test)]
mod tests;
