//! N2 (SoT dotnet-protected-host-launch 4.1, owner decision D3): one
//! read-only `GetFileType` query on a private duplicate of standard input.
//! Pipe type is a necessary creator-channel fact, never producer proof.
use super::{BridgeError, os_failure};
use std::os::windows::io::{AsHandle, AsRawHandle};
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_PIPE, FILE_TYPE_UNKNOWN, GetFileType};

/// Report whether this process's standard input is a pipe object.
/// A console, disk file, character device or absent handle reports `false`.
pub fn stdin_is_pipe() -> Result<bool, BridgeError> {
    // Safe duplicate: the query never borrows the process-global std handle
    // slot, and the duplicate closes on drop without touching fd 0.
    let owned = std::io::stdin()
        .as_handle()
        .try_clone_to_owned()
        .map_err(|_| BridgeError::OsFailure)?;
    Ok(classify(query(&owned)?))
}

#[allow(unsafe_code)]
fn query(owned: &std::os::windows::io::OwnedHandle) -> Result<u32, BridgeError> {
    // SAFETY: `owned` is a live handle this function's caller owns for the
    // entire synchronous call; GetFileType only reads object type metadata,
    // takes no ownership and writes no caller memory.
    let kind = unsafe { GetFileType(owned.as_handle().as_raw_handle()) };
    if kind == FILE_TYPE_UNKNOWN && std::io::Error::last_os_error().raw_os_error() != Some(0) {
        return Err(os_failure());
    }
    Ok(kind)
}

fn classify(kind: u32) -> bool {
    kind == FILE_TYPE_PIPE
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_CHAR, FILE_TYPE_DISK};
    #[test]
    fn only_pipe_type_is_a_creator_channel_candidate() {
        assert!(classify(FILE_TYPE_PIPE));
        for kind in [FILE_TYPE_DISK, FILE_TYPE_CHAR, FILE_TYPE_UNKNOWN, 0x8000] {
            assert!(!classify(kind));
        }
    }
}
