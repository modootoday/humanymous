//! Best-effort projections of sealed audit records, ported from Go
//! `internal/audit/sink_record.go`, `sink_redis.go` and `sink_clickhouse.go`
//! (oracle 981d25c).
//!
//! The local WAL behind [`crate::audit::log::Log`] stays the only authority:
//! its failure fails the append closed and withholds the enforcement effect.
//! A projection is a lossy, replayable copy for a hot tail (Redis Streams) or a
//! columnar store (ClickHouse). It never blocks or fails the seal path, and a
//! record it could not deliver is counted in its `dropped` counter instead of
//! being reported as shipped. A projection is never evidence of audit coverage;
//! the authenticated WAL history and its checkpoints are.
pub mod clickhouse;
pub mod redis_stream;

use crate::audit::record::Record;
use std::io;

/// Destination for sealed records (Go `RecordSink`). The authority WAL is not
/// pluggable through this trait; only projections implement it.
///
/// Implementations receive records in the sequence order the log seals them,
/// must return promptly (enqueue, do not ship inline) and must count every
/// record they accept but cannot deliver.
pub trait RecordSink: Send + Sync {
    /// Accept one sealed record. A projection enqueues it and returns; a full
    /// backlog drops and counts it rather than blocking the writer.
    fn append_record(&self, record: &Record) -> io::Result<()>;
    /// Deliver what was accepted so far, where the projection supports it.
    fn flush(&self) -> io::Result<()>;
    /// Flush where supported and stop the background shipper.
    fn close(&self) -> io::Result<()>;
}

#[cfg(test)]
mod tests;
