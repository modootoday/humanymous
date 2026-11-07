//! Local durable storage. Parsing alone never authenticates an audit chain.
#[cfg(any(unix, windows))]
mod exclusive_file;
mod framing;
#[cfg(any(unix, windows))]
pub(crate) use exclusive_file::ExclusiveFile;
#[cfg(any(unix, windows))]
mod private_state;
#[cfg(any(unix, windows))]
pub(crate) use private_state::PrivateState;
#[cfg(windows)]
pub mod permissions;
mod recovery;
pub use recovery::{RecoveryPolicy, VerifiedHistory, verify_history};
#[cfg(any(unix, windows))]
mod wal;
#[cfg(any(unix, windows))]
pub use wal::{Wal, WalOptions};

use crate::record::Record;
use std::{
    fs::{self, OpenOptions},
    io::{self, BufReader},
    path::Path,
};

/// Parsed segment and explicit crash-recovery accounting. Callers must verify
/// hashes, sequence, HMAC and checkpoints before accepting these records.
pub struct RecoveredSegment {
    /// Complete newline-committed records, in disk order.
    pub records: Vec<Record>,
    /// Durable bytes discarded from an incomplete terminal frame.
    pub truncated_bytes: u64,
}

/// Read a segment, optionally truncating its incomplete terminal frame and
/// syncing the repaired file. Requires exclusive writer ownership of a trusted,
/// service-owned directory: this function does not acquire that ownership.
/// Symlinks and non-files are rejected; this is not a defense against a hostile
/// directory owner replacing entries concurrently.
pub fn recover_segment(
    path: &Path,
    terminal: bool,
    max_frame: usize,
) -> io::Result<RecoveredSegment> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "audit segment must be a regular file",
        ));
    }
    let file = OpenOptions::new().read(true).write(terminal).open(path)?;
    let original_len = file.metadata()?.len();
    let frames = framing::decode::<Record>(BufReader::new(&file), max_frame, terminal)?;
    let truncated_bytes = if frames.torn_tail {
        file.set_len(frames.complete_bytes)?;
        file.sync_all()?;
        original_len
            .checked_sub(frames.complete_bytes)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "segment changed during recovery",
                )
            })?
    } else {
        0
    };
    Ok(RecoveredSegment {
        records: frames.values,
        truncated_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn repair_is_persisted_and_reopen_is_clean() {
        let path = std::env::temp_dir().join(format!(
            "humanymous-segment-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        use std::io::Write;
        file.write_all(b"{\"seq\":1}\n{\"seq\":").unwrap();
        file.sync_all().unwrap();
        drop(file);
        assert!(recover_segment(&path, false, 1024).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{\"seq\":1}\n{\"seq\":");
        let recovered = recover_segment(&path, true, 1024).unwrap();
        assert_eq!(recovered.records[0].seq, 1);
        assert_eq!(recovered.truncated_bytes, 7);
        assert_eq!(fs::read(&path).unwrap(), b"{\"seq\":1}\n");
        assert_eq!(
            recover_segment(&path, true, 1024).unwrap().truncated_bytes,
            0
        );
        fs::remove_file(path).unwrap();
    }
}
