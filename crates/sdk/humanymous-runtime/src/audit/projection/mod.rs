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
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

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

/// A TCP connection whose reads and writes all end by one absolute deadline,
/// like a Go `net.Conn` deadline. Before each socket call the socket timeout is
/// set to the time remaining, so a peer that trickles bytes cannot stretch the
/// operation past the deadline; once it has passed, every call fails with
/// [`io::ErrorKind::TimedOut`].
pub(super) struct DeadlineStream {
    stream: TcpStream,
    deadline: Instant,
}

impl DeadlineStream {
    /// Resolve `address` and connect, all before `deadline`.
    ///
    /// An IP literal needs no lookup. A host name is resolved by the system
    /// resolver on a helper thread, because std offers no bounded lookup and
    /// the crate takes no resolver dependency. The wait for that thread is
    /// bounded by the deadline; the lookup itself is not, so a hung resolver
    /// leaves the abandoned helper thread running until the resolver gives up.
    pub(super) fn connect(address: &str, deadline: Instant) -> io::Result<Self> {
        let resolved = resolve(address, deadline)?;
        let mut last = None;
        for candidate in resolved {
            match TcpStream::connect_timeout(&candidate, remaining(deadline)?) {
                Ok(stream) => return Ok(Self { stream, deadline }),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| io::Error::other("address resolved to nothing")))
    }

    /// Move the deadline for the calls that follow.
    pub(super) fn set_deadline(&mut self, deadline: Instant) {
        self.deadline = deadline;
    }

    /// The underlying socket, for options such as `TCP_NODELAY`.
    pub(super) fn socket(&self) -> &TcpStream {
        &self.stream
    }
}

impl Read for DeadlineStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream
            .set_read_timeout(Some(remaining(self.deadline)?))?;
        self.stream.read(buf)
    }
}

impl Write for DeadlineStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream
            .set_write_timeout(Some(remaining(self.deadline)?))?;
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Time left before `deadline`, or `TimedOut` once none is left. A zero
/// socket timeout would mean "no timeout", so it is never returned.
fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "projection deadline passed"))
}

fn resolve(address: &str, deadline: Instant) -> io::Result<Vec<SocketAddr>> {
    if let Ok(literal) = address.parse::<SocketAddr>() {
        return Ok(vec![literal]);
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let owned = address.to_owned();
    thread::Builder::new()
        .name("audit-projection-resolve".into())
        .spawn(move || {
            let result = owned
                .to_socket_addrs()
                .map(Iterator::collect::<Vec<SocketAddr>>);
            // The caller may have given up; a closed channel is fine.
            let _ = sender.send(result);
        })?;
    match receiver.recv_timeout(remaining(deadline)?) {
        Ok(result) => {
            let resolved = result?;
            remaining(deadline)?;
            Ok(resolved)
        }
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "projection address lookup exceeded the deadline",
        )),
    }
}

#[cfg(test)]
mod tests;
