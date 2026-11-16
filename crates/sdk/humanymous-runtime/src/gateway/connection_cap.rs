//! Concurrent accepted-connection cap (shipping row B04; legacy
//! cmd/server/maxconns_test.go and the `netutil.LimitListener` wrap in both
//! mains at baseline 981d25c).
//!
//! A slow-header or slow-body flood accumulates open connections at
//! open-rate times hold-time even when each one is eventually timed out; a
//! per-connection deadline does not bound concurrency. The accept loop takes a
//! permit before each accept and moves it into the connection task, so at
//! most `limit` accepted connections are live and further peers wait in the
//! kernel backlog (back-pressure) instead of consuming process resources.
//!
//! This is a resource bound, never a client identity, verdict or detection
//! signal. Differences from Go: a zero or oversized limit is refused instead
//! of meaning "unlimited"/accepted, and [`ConnectionCap::try_acquire`] offers a
//! non-blocking path that `LimitListener` did not.
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

/// Largest accepted limit.
pub const MAX_CONNECTION_CAP: usize = 1 << 16;

/// Static refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Limit is zero or above [`MAX_CONNECTION_CAP`].
    Limit,
    /// Cap was closed for shutdown.
    Closed,
}

/// Shared counted admission for accepted connections.
#[derive(Debug, Clone)]
pub struct ConnectionCap {
    slots: Arc<Semaphore>,
    limit: usize,
}

/// One live accepted connection; dropping it frees the slot.
#[derive(Debug)]
pub struct ConnectionPermit {
    _slot: OwnedSemaphorePermit,
}

impl ConnectionCap {
    /// Cap of `limit` concurrent connections, 1..=[`MAX_CONNECTION_CAP`].
    pub fn new(limit: usize) -> Result<Self, Refusal> {
        if limit == 0 || limit > MAX_CONNECTION_CAP {
            return Err(Refusal::Limit);
        }
        Ok(Self {
            slots: Arc::new(Semaphore::new(limit)),
            limit,
        })
    }

    /// Wait for a free slot; call before `accept`.
    pub async fn acquire(&self) -> Result<ConnectionPermit, Refusal> {
        Arc::clone(&self.slots)
            .acquire_owned()
            .await
            .map(|slot| ConnectionPermit { _slot: slot })
            .map_err(|_| Refusal::Closed)
    }

    /// Take a slot only if one is free now.
    pub fn try_acquire(&self) -> Result<Option<ConnectionPermit>, Refusal> {
        match Arc::clone(&self.slots).try_acquire_owned() {
            Ok(slot) => Ok(Some(ConnectionPermit { _slot: slot })),
            Err(TryAcquireError::NoPermits) => Ok(None),
            Err(TryAcquireError::Closed) => Err(Refusal::Closed),
        }
    }

    /// Live permits.
    pub fn in_use(&self) -> usize {
        self.limit - self.slots.available_permits()
    }

    /// Configured limit.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Refuse all waiting and future acquisitions; live permits stay valid.
    pub fn close(&self) {
        self.slots.close();
    }
}

#[cfg(test)]
mod tests;
