//! Shadow counters for categories that must never reach scoring.
//!
//! Tor exits and blocklist hits are counted here and exposed in the operator
//! status only. Promotion of either category into scoring or verdicts is an
//! owner decision recorded in `sot:root/ip-reputation`, not a configuration.
use super::Classification;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic process-lifetime shadow hit counters.
#[derive(Debug, Default)]
pub struct ShadowCounters {
    tor_exit: AtomicU64,
    blocklist: AtomicU64,
    both: AtomicU64,
}

/// Point-in-time copy of [`ShadowCounters`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ShadowStats {
    /// Classified requests from a Tor exit.
    pub tor_exit_hits: u64,
    /// Classified requests from a blocklisted range.
    pub blocklist_hits: u64,
    /// Classified requests that were both.
    pub tor_exit_and_blocklist_hits: u64,
}

impl ShadowCounters {
    /// Count the shadow categories of one classification. The datacenter bit is
    /// deliberately ignored here; it is a scoring input counted by the store.
    pub fn record(&self, classification: &Classification) {
        if classification.tor_exit {
            self.tor_exit.fetch_add(1, Ordering::Relaxed);
        }
        if classification.blocklist {
            self.blocklist.fetch_add(1, Ordering::Relaxed);
        }
        if classification.tor_exit && classification.blocklist {
            self.both.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Current values.
    pub fn stats(&self) -> ShadowStats {
        ShadowStats {
            tor_exit_hits: self.tor_exit.load(Ordering::Relaxed),
            blocklist_hits: self.blocklist.load(Ordering::Relaxed),
            tor_exit_and_blocklist_hits: self.both.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests;
