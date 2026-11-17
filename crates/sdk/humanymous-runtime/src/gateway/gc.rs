//! Composite garbage collection over the gateway's in-memory detection
//! kernels (legacy internal/gate/gate.go `Server.GC` and gc_test.go at
//! baseline 981d25c, PLAN-08 deployment-review ship-blocker).
//!
//! Go swept every map it owned in one call so fingerprint/IP churn could not
//! grow them without bound, and skipped optional kernels that were off. This
//! is the same fan-out: each kernel the caller owns is swept, absent kernels
//! are skipped, and one kernel's refusal never stops the others. Unlike Go,
//! a kernel refusal (for example a regressed clock) is reported instead of
//! being silent, and the sweep only counts as complete when every present
//! kernel succeeded.
//!
//! Out of scope here: the settings overlay expiry Go ran first is part of the
//! scoring/route overlay model, which Rust deliberately does not have (detection
//! freeze); the periodic ticker that calls this belongs to the serving
//! composition.
use super::ban;
use super::seams::VerdictLedger;
use crate::network::correlation;
use crate::network::history::{Unavailable, anomaly, sweep};
use std::time::Duration;

/// Idle TTL for the anomaly observer's per-key series (Go anomaly_shadow.go
/// `a.det.GC(now, 10*time.Minute)`).
pub const ANOMALY_IDLE_TTL: Duration = Duration::from_secs(10 * 60);

/// The kernels one sweep covers; `None` means that kernel is off.
#[derive(Default)]
pub struct Kernels<'a> {
    /// Sticky verdicts.
    pub verdicts: Option<&'a dyn VerdictLedger>,
    /// Bans, strikes and their rate windows.
    pub bans: Option<&'a mut ban::Store>,
    /// Decision-probing sweep bindings.
    pub sweep: Option<&'a mut sweep::Detector>,
    /// Anomaly observer series.
    pub anomaly: Option<&'a mut anomaly::Detector>,
    /// Per-credential fan-out correlation.
    pub credentials: Option<&'a mut correlation::Registry>,
}

/// Per-kernel result of one sweep; `None` means the kernel was absent.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Whether the verdict ledger was swept (it cannot refuse).
    pub verdicts: bool,
    /// Ban store result.
    pub bans: Option<Result<(), ban::Refusal>>,
    /// Sweep detector result: bindings evicted.
    pub sweep: Option<Result<usize, Unavailable>>,
    /// Anomaly detector result: series evicted.
    pub anomaly: Option<Result<usize, Unavailable>>,
    /// Credential correlation result.
    pub credentials: Option<Result<(), Unavailable>>,
}

impl Report {
    /// True when every present kernel swept without refusal.
    pub fn complete(&self) -> bool {
        self.bans.as_ref().is_none_or(Result::is_ok)
            && self.sweep.as_ref().is_none_or(Result::is_ok)
            && self.anomaly.as_ref().is_none_or(Result::is_ok)
            && self.credentials.as_ref().is_none_or(Result::is_ok)
    }
}

/// Sweep every present kernel at `now`, in Go's order.
pub fn sweep(kernels: Kernels<'_>, now: Duration) -> Report {
    let verdicts = kernels.verdicts.map(|v| v.gc(now)).is_some();
    Report {
        verdicts,
        bans: kernels.bans.map(|b| b.gc(now)),
        sweep: kernels.sweep.map(|s| s.gc(now)),
        anomaly: kernels.anomaly.map(|a| a.gc(now, ANOMALY_IDLE_TTL)),
        credentials: kernels.credentials.map(|c| c.gc(now)),
    }
}

#[cfg(test)]
mod tests;
