//! Opt-in append-only JSONL writer for oracle-confirmed training records
//! (legacy `internal/mltrain/sink.go` at baseline 981d25c; SoT
//! model-training-correction, row "mltrain sink").
//!
//! Construction is the opt-in: a composition root builds a sink only for an
//! explicitly configured lab or training path and otherwise holds `None`, which
//! replaces Go's nil-receiver no-op. Writes carry aggregate [`Record`] values
//! only, never raw event streams. A sink error is non-fatal to callers by
//! contract; it never changes a decision.
//!
//! Deliberate fail-closed differences from Go:
//! - The path must be absolute without `..` components; Go accepted any path.
//! - An existing target that is a symlink or not a regular file is refused.
//!   The check runs before open; a concurrent swap is outside this lab-only
//!   threat model.
//! - Each record passes the shared [`Record::sample`] admission and the
//!   default line budget before it is written, so the sink never emits a line
//!   that [`super::record::read_jsonl`] would reject. Go wrote any record.
use super::record::{Limits, Record, RecordError};
use crate::behavior::features::schema_hash;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};

/// Stable failure categories; they carry no path or OS diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkError {
    /// Path is relative or contains a parent component.
    Path,
    /// Existing target is a symlink, directory or other non-regular file.
    NotRegular,
    /// Parent directory creation or file open failed.
    Open,
    /// Record failed the shared training admission.
    Invalid(RecordError),
    /// Encoded record exceeds the reader's physical line budget.
    TooLarge,
    /// Write or final sync failed.
    Write,
}

impl std::fmt::Display for SinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "training sink: {self:?}")
    }
}

impl std::error::Error for SinkError {}

/// Append-only JSONL writer. Exclusive access (`&mut self`) replaces Go's mutex.
#[derive(Debug)]
pub struct TraceSink {
    file: File,
    count: u64,
}

impl TraceSink {
    /// Open `path` for appending, creating it owner-only (0600) and any missing
    /// parent directories owner-only (0700) on Unix. Existing lines are kept.
    pub fn open(path: &Path) -> Result<Self, SinkError> {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(SinkError::Path);
        }
        if let Some(parent) = path.parent() {
            let mut builder = DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            builder.create(parent).map_err(|_| SinkError::Open)?;
        }
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_file() => return Err(SinkError::NotRegular),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(SinkError::Open),
        }
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(path).map_err(|_| SinkError::Open)?;
        if !file.metadata().map_err(|_| SinkError::Open)?.is_file() {
            return Err(SinkError::NotRegular);
        }
        Ok(Self { file, count: 0 })
    }

    /// Validate and append one record as a single JSONL line, unbuffered, so a
    /// returned `Ok` means the line reached the operating system.
    pub fn append(&mut self, record: &Record) -> Result<(), SinkError> {
        let limits = Limits::default();
        record
            .clone()
            .sample(&schema_hash(), limits)
            .map_err(SinkError::Invalid)?;
        let mut line = serde_json::to_vec(record).map_err(|_| SinkError::Write)?;
        line.push(b'\n');
        if line.len() > limits.line_bytes {
            return Err(SinkError::TooLarge);
        }
        self.file.write_all(&line).map_err(|_| SinkError::Write)?;
        self.count += 1;
        Ok(())
    }

    /// Records appended by this handle (observability).
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Sync and close. Consuming `self` makes a second close unrepresentable.
    pub fn close(self) -> Result<(), SinkError> {
        self.file.sync_all().map_err(|_| SinkError::Write)
    }
}

#[cfg(test)]
mod tests;
