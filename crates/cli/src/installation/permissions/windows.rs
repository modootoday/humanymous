use humanymous_native_security::{
    process::current_identity,
    storage::{InstallationObject, WindowsPrivacy, verify_windows_privacy},
};
use std::{fs, io, path::Path};

pub(in crate::installation) fn private(path: &Path, object: InstallationObject) -> io::Result<()> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    let identity = current_identity()?;
    if !identity.installer_allowed() {
        return Err(io::Error::other(
            "non-elevated non-administrator token required",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(io::Error::other("reparse point rejected"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x00200000 | if metadata.is_dir() { 0x02000000 } else { 0 })
        .open(path)?;
    verify_windows_privacy(&file, identity.sid(), WindowsPrivacy::Installation(object))
}
