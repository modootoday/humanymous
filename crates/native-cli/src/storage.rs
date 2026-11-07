//! Explicit single-manifest publication; no adapter installation or implicit mutation.
use crate::arguments::Init;
use std::io;

/// Publication and temporary-link cleanup are independent receipts.
#[derive(Debug, serde::Serialize)]
pub struct Receipt {
    /// True only after atomic no-clobber destination creation succeeds.
    pub published: bool,
    /// False means publication succeeded but its temporary link remains.
    pub cleanup_complete: bool,
    /// Workspace-relative temporary basename to recover when cleanup is incomplete.
    pub temporary_file: Option<String>,
}

/// Publish an honest manifest inside an explicitly selected workspace.
///
/// Only local Windows drive directories currently support writes. Trusted,
/// non-mutating workspace entries are required, particularly during cleanup.
/// Atomic publication does not establish power-loss/directory durability.
pub fn publish(init: &Init) -> io::Result<Receipt> {
    if !init.write {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write not explicitly requested",
        ));
    }
    crate::arguments::validate_output(&init.output).map_err(io::Error::other)?;
    let root = init
        .workspace
        .as_deref()
        .ok_or_else(|| io::Error::other("explicit workspace required"))?;
    let bytes = crate::manifest::bytes(init).map_err(io::Error::other)?;
    #[cfg(windows)]
    {
        windows::publish(root, &init.output, &bytes)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, bytes);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "manifest writes currently require Windows; use dry-run",
        ))
    }
}

#[cfg(windows)]
mod windows;
