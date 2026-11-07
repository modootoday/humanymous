//! One native nonblocking lock owner for custody and WAL, with explicit release.
use std::{fs::File, io};

pub(in crate::audit) struct ExclusiveFile {
    file: File,
}
impl ExclusiveFile {
    pub(in crate::audit) fn acquire(file: File) -> io::Result<Self> {
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                io::Error::new(io::ErrorKind::WouldBlock, "audit writer already locked")
            }
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(Self { file })
    }
    #[cfg(all(test, unix))]
    pub(in crate::audit) fn duplicate_description(&self) -> io::Result<File> {
        self.file.try_clone()
    }
}
impl Drop for ExclusiveFile {
    fn drop(&mut self) {
        // Close alone retains flock while a duplicated/inherited description lives.
        // If the OS rejects unlock, closing the owned File is the final fallback;
        // a subsequent acquire must still respect any remaining kernel lock.
        let _ = self.file.unlock();
    }
}
