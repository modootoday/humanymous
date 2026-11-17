//! Asynchronous audit projections (SoT-32 Tier 1 hot tail, Tier 2 columnar
//! store), ported from Go `internal/audit/sink_record.go`, `sink_redis.go` and
//! `sink_clickhouse.go` (oracle 981d25c).
//!
//! The local WAL is the only durability authority: its error fails the seal
//! closed. A projection is best-effort and replayable from the WAL. It must
//! never block or fail the seal, so a full backlog or an unreachable server
//! drops the record from that projection and counts it.
use crate::audit::record::Record;
use std::io;

#[cfg(feature = "gateway")]
pub mod clickhouse;
#[cfg(feature = "gateway")]
pub mod redis;

/// Destination for sealed records, fed in sequence order (Go `RecordSink` for
/// projection sinks).
pub trait Projection: Send + Sync {
    /// Accept one sealed record without blocking; never fails the seal.
    fn append(&self, record: &Record);
    /// Ship what is buffered; an error means this projection lost records.
    fn flush(&self) -> io::Result<()>;
    /// Records dropped from this projection under load or outage.
    fn dropped(&self) -> u64;
}

#[cfg(all(test, feature = "gateway"))]
mod tests;
