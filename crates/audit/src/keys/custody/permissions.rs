use super::{Owner, invalid};
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Component, Path},
};

pub(crate) fn directory(path: &Path, owner: &Owner) -> io::Result<()> {
    let mut component_path = std::path::PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err(invalid("custody path aliases refused"));
        }
        component_path.push(component);
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&component_path)?;
        no_alias(&metadata)?;
        if !metadata.is_dir() {
            return Err(invalid("custody directory component is not a directory"));
        }
    }
    let mut options = OpenOptions::new();
    options.read(true);
    native_options(&mut options);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x02200000); // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
    }
    let file = options.open(path)?;
    check(&file, owner, true)
}
pub(crate) fn create(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    native_options(&mut options);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
pub(crate) fn existing(path: &Path, write: bool) -> io::Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    no_alias(&metadata)?;
    if !metadata.is_file() {
        return Err(invalid("custody entry is not a regular file"));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(write);
    native_options(&mut options);
    options.open(path)
}
fn native_options(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // NONBLOCK avoids blocking on a FIFO substituted by a same-owner actor.
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // OPEN_REPARSE_POINT
    }
}
fn no_alias(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.file_type().is_symlink() {
        return Err(invalid("custody symlink refused"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(invalid("custody reparse point refused"));
        }
    }
    Ok(())
}
pub(crate) fn file(file: &File, owner: &Owner) -> io::Result<()> {
    check(file, owner, false)
}
fn check(file: &File, owner: &Owner, directory: bool) -> io::Result<()> {
    let metadata = file.metadata()?;
    no_alias(&metadata)?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(invalid("unexpected custody entry type"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let Owner::CurrentUser = owner;
        let expected = if directory { 0o700 } else { 0o600 };
        if metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != expected
            || (!directory && metadata.nlink() != 1)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "custody requires private owner-only permissions",
            ));
        }
    }
    #[cfg(windows)]
    {
        use crate::storage::permissions::{WindowsPrivacy, verify_windows_privacy};
        let Owner::WindowsSid(sid) = owner;
        verify_windows_privacy(
            file,
            sid,
            if directory {
                WindowsPrivacy::InheritableDirectory
            } else {
                WindowsPrivacy::File
            },
        )?;
    }
    Ok(())
}
