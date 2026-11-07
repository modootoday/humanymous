use crate::authority::configuration::Peer;
use std::{io, path::Path};

/// Refuse symlink/reparse paths and grants outside the provisioned principal.
/// The trusted supervisor must prevent hostile concurrent ancestor replacement.
pub fn private_directory(path: &Path, peer: &Peer) -> io::Result<()> {
    if !path.is_absolute() || path.canonicalize()? != path {
        return Err(io::Error::other("directory must be canonical and absolute"));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("private directory required"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let Peer::Unix { uid } = peer else {
            return Err(io::Error::other("Unix principal required"));
        };
        if metadata.uid() != *uid
            || metadata.mode() & 0o077 != 0
            || metadata.mode() & 0o700 != 0o700
        {
            return Err(io::Error::other("private owner-only directory required"));
        }
        // Every ancestor must resist replacement by other unprivileged users.
        // Sticky directories such as /tmp protect this owner's child entry.
        for parent in path.ancestors().skip(1) {
            let m = std::fs::symlink_metadata(parent)?;
            if !m.is_dir()
                || m.file_type().is_symlink()
                || (m.uid() != 0 && m.uid() != *uid)
                || (m.mode() & 0o022 != 0 && m.mode() & 0o1000 == 0)
            {
                return Err(io::Error::other("untrusted directory ancestor"));
            }
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        let Peer::Windows { sid, .. } = peer else {
            return Err(io::Error::other("Windows principal required"));
        };
        if !crate::authority::configuration::valid_sid(sid)
            || metadata.file_attributes() & 0x400 != 0
        {
            return Err(io::Error::other("private non-reparse directory required"));
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x02200000)
            .open(path)?; // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
        crate::audit::storage::permissions::verify_windows_privacy(
            &file,
            sid,
            crate::audit::storage::permissions::WindowsPrivacy::ProtectedDirectory,
        )?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = peer;
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "unsupported ACL platform",
        ));
    }
    Ok(())
}
