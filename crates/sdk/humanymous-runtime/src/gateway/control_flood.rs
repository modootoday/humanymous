//! Per-source control-plane flood guard (SoT-28 WS7).
//!
//! Port of legacy `internal/gate/control.go` `cpGuard` and
//! `WithControlLimiter` at baseline 981d25c. The ban gate does not cover the
//! control plane, so the collect and session endpoints meter their own callers
//! and drop a flooding source with 429 before any session or scoring work.
//!
//! Kept from Go:
//! - The source key is `cp:<client ip>`; each source has its own rolling
//!   window, so one flooding address never limits another.
//! - Defaults are 150 (soft) and 300 (hard) requests per 10 s window.
//! - Every request is counted, including dropped ones, and a source is dropped
//!   once its rolling count reaches the hard cutpoint (Go `Level(count) >= 2`).
//! - The flood is metered, not chained per request: exactly one
//!   `ratelimit.hard_exceeded` record is produced, when the count equals hard
//!   plus one, with `control-plane flood: <endpoint>` as its reason.
//! - The refusal is `429` with `Retry-After: 10` and the body
//!   `control plane rate limited`.
//! - The record verdict is Go `string(VerdictDeny)`, `deny`.
//!
//! Deliberate differences, each pinned by tests:
//! - The guard is bounded by [`Limits`] (keys, hits per key, key bytes and
//!   payload). Any limiter refusal (a full table, a source whose count exceeds
//!   the per-key bound, or a clock regression) fails closed as a drop without a
//!   record instead of growing without bound.
//!
//! The guard is a pure kernel: the caller supplies the socket peer address and
//! a monotonic clock, appends the returned record and writes the refusal. It is
//! not yet composed in front of a served control-plane collect route.
use crate::audit::record::{Actor, Record};
use crate::network::history::rate::{Config, GcResult, Limiter, Limits, Observation, Unavailable};
use std::net::IpAddr;
use std::time::Duration;

/// Legacy default rolling window.
pub const WINDOW: Duration = Duration::from_secs(10);
/// Legacy default soft cutpoint.
pub const SOFT: usize = 150;
/// Legacy default hard cutpoint.
pub const HARD: usize = 300;
/// Refusal status.
pub const STATUS: u16 = 429;
/// `Retry-After` value in seconds.
pub const RETRY_AFTER: &str = "10";
/// Refusal body (Go `http.Error` adds the trailing newline).
pub const BODY: &str = "control plane rate limited\n";
/// Audit event for the sampled hard breach.
pub const EVENT: &str = "ratelimit.hard_exceeded";

/// Default resource bounds: 4096 sources, 512 hits per source, a 10 minute
/// largest window and the Go GC scan budget of 4096.
pub fn default_limits() -> Result<Limits, Unavailable> {
    Limits::new(4096, 512, 64, Duration::from_secs(600), 64 << 20, 4096)
}

/// Outcome of metering one control-plane request.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Serve the request.
    Pass,
    /// Answer [`STATUS`] with [`RETRY_AFTER`] and [`BODY`]; append `breach`
    /// when present.
    Drop {
        /// The single sampled hard-breach record of this flood.
        breach: Option<Box<Record>>,
    },
}

/// Bounded per-source flood guard.
pub struct Guard {
    limiter: Limiter,
    hard: usize,
}

impl Guard {
    /// Guard with the legacy defaults ([`WINDOW`], [`SOFT`], [`HARD`]).
    pub fn new() -> Result<Self, Unavailable> {
        Self::with_thresholds(WINDOW, SOFT, HARD, default_limits()?)
    }

    /// Guard with explicit thresholds (Go `WithControlLimiter`).
    pub fn with_thresholds(
        window: Duration,
        soft: usize,
        hard: usize,
        limits: Limits,
    ) -> Result<Self, Unavailable> {
        let config = Config::new(window, soft, hard, limits)?;
        Ok(Self {
            limiter: Limiter::new(config),
            hard,
        })
    }

    /// Meter one request from `client` to `endpoint` at monotonic `now`.
    pub fn admit(&mut self, client: IpAddr, endpoint: &str, now: Duration) -> Decision {
        let key = format!("cp:{client}");
        let count = match self.limiter.observe(&key, now) {
            Ok(Observation::Counted { count, .. }) => count,
            // The key always carries the `cp:` prefix, so it is never empty;
            // refuse rather than serve an uncounted request.
            Ok(Observation::IgnoredEmptyKey) | Err(_) => return Decision::Drop { breach: None },
        };
        if count < self.hard {
            return Decision::Pass;
        }
        let breach = (count == self.hard + 1).then(|| Box::new(breach_record(endpoint)));
        Decision::Drop { breach }
    }

    /// Bounded eviction of idle sources (Go `Limiter.GC`).
    pub fn gc(&mut self, now: Duration) -> Result<GcResult, Unavailable> {
        self.limiter.gc(now)
    }
}

fn breach_record(endpoint: &str) -> Record {
    Record {
        event_type: EVENT.into(),
        actor: Actor {
            kind: "system".into(),
            id_pseudonym: String::new(),
        },
        tenant_id: "control".into(),
        route_class: "control".into(),
        verdict: "deny".into(),
        enforcement_action: "rate_limit".into(),
        enforcement_mode: "enforce".into(),
        fail_reason: format!("control-plane flood: {endpoint}"),
        key_id: "k1".into(),
        ..Record::default()
    }
}

#[cfg(test)]
mod tests;
