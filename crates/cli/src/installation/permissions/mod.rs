use super::err;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use humanymous_native_security::storage::InstallationObject;
#[cfg(windows)]
pub(super) use windows::private as private_windows;
pub(crate) fn plain(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let drive = match path.components().next() {
            Some(Component::Prefix(p)) => match p.kind() {
                std::path::Prefix::Disk(letter) => format!("{}:\\", char::from(letter)),
                _ => return Err(err("only local drive-letter paths are supported")),
            },
            _ => return Err(err("absolute local drive-letter path required")),
        };
        humanymous_native_security::volume::require_fixed(&drive)?;
    }
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(err("absolute path without traversal required"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(err("reparse point rejected"));
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err(err("symlink rejected"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && current == path => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub(crate) fn private_parent(path: &Path) -> std::io::Result<()> {
    if !fs::metadata(path)?.is_dir() {
        return Err(err("parent must exist"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = humanymous_native_security::process::current_uid();
        let meta = fs::metadata(path)?;
        if !uid_is_allowed(uid) || meta.uid() != uid || meta.mode() & 0o777 != 0o700 {
            return Err(err("private non-root owned parent required"));
        }
    }
    #[cfg(windows)]
    private_windows(path, InstallationObject::Parent)?;
    #[cfg(not(any(unix, windows)))]
    return Err(err("unsupported OS permission boundary"));
    Ok(())
}

pub(crate) fn private_file(path: &Path) -> std::io::Result<()> {
    plain(path)?;
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(err("private regular file required"));
    }
    #[cfg(windows)]
    private_windows(path, InstallationObject::File)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path)?;
        if metadata.uid() != humanymous_native_security::process::current_uid()
            || metadata.mode() & 0o777 != 0o600
        {
            return Err(err("private owner-only managed file required"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn uid_is_allowed(uid: u32) -> bool {
    uid != 0
}
