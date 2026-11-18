//! Distribution seams for the edge's global-correctness stores (legacy
//! internal/gate/seams.go at baseline 981d25c, PLAN-07 R18).
//!
//! One node keeps bans and rate counts in memory. Across a fleet those are the
//! state that must be shared, so the enforcement path depends on these traits
//! and a shared backend can implement them without touching the gateway. The
//! local `ban::Store` and `rate::Limiter` implement them; the impls below are
//! the compile-time proof that the seam cannot silently drift.
//!
//! Dual-controlled bans (`ban::Store::add_controlled`) are deliberately not on
//! the seam: a shared backend must route them through audit control too.
use super::ban::{Entry, Key, Outcome, Refusal, Store};
use super::verdict_store::{Sticky, VerdictStore};
use crate::network::history::Unavailable;
use crate::network::history::rate::{ConfigurationEffect, Level, Limiter, Observation};
use std::time::Duration;

/// Ban store contract the edge enforces against (SoT-27). A shared
/// implementation must keep strike accounting atomic across nodes.
pub trait BanLedger {
    /// Active ban for `key`, if any.
    fn check(&mut self, key: &Key, now: Duration) -> Option<Entry>;
    /// Count a request and apply automatic escalation.
    fn observe(&mut self, key: &Key, now: Duration) -> Result<Outcome, Refusal>;
    /// Count a request without creating or enforcing a ban (monitor mode).
    fn observe_rate(&mut self, key: &Key, now: Duration) -> Result<Level, Refusal>;
    /// Single-operator temporary ban of a narrow key.
    fn add_temporary(
        &mut self,
        key: &Key,
        why: &str,
        incident: Option<&str>,
        term: Duration,
        now: Duration,
    ) -> Result<Entry, Refusal>;
    /// Lift a non-permanent ban.
    fn lift(&mut self, key: &Key) -> Result<bool, Refusal>;
    /// Active bans.
    fn list(&mut self, now: Duration) -> Vec<Entry>;
}

/// Sticky-verdict contract (SoT-21 §2). A shared implementation must treat
/// an absent or stale entry as `Unknown`, never as a grant.
pub trait VerdictLedger {
    /// Record or update a session's verdict; false when not stored.
    fn set(&self, sid: &str, sticky: Sticky) -> bool;
    /// The session's verdict, `Unknown` when absent or stale.
    fn get(&self, sid: &str, now: Duration) -> Sticky;
    /// Evict expired entries.
    fn gc(&self, now: Duration);
}

impl VerdictLedger for VerdictStore {
    fn set(&self, sid: &str, sticky: Sticky) -> bool {
        VerdictStore::set(self, sid, sticky)
    }
    fn get(&self, sid: &str, now: Duration) -> Sticky {
        VerdictStore::get(self, sid, now)
    }
    fn gc(&self, now: Duration) {
        VerdictStore::gc(self, now)
    }
}

/// Fingerprint-keyed rate counter that drives automatic bans (SoT-17).
pub trait RateLimiter {
    /// Count one request for `key`.
    fn observe(&mut self, key: &str, now: Duration) -> Result<Observation, Unavailable>;
    /// Classify a count.
    fn level(&self, count: usize) -> Level;
    /// Hot-apply a new window and cutpoints (SoT-39).
    fn configure(
        &mut self,
        window: Duration,
        soft: usize,
        hard: usize,
        now: Duration,
    ) -> Result<ConfigurationEffect, Unavailable>;
    /// Sweep expired local history (a shared backend expires its own keys).
    fn gc(&mut self, now: Duration) -> Result<(), Unavailable>;
}

impl<L: RateLimiter> BanLedger for Store<L> {
    fn check(&mut self, key: &Key, now: Duration) -> Option<Entry> {
        Store::check(self, key, now).cloned()
    }
    fn observe(&mut self, key: &Key, now: Duration) -> Result<Outcome, Refusal> {
        Store::observe(self, key, now)
    }
    fn observe_rate(&mut self, key: &Key, now: Duration) -> Result<Level, Refusal> {
        Store::observe_rate(self, key, now)
    }
    fn add_temporary(
        &mut self,
        key: &Key,
        why: &str,
        incident: Option<&str>,
        term: Duration,
        now: Duration,
    ) -> Result<Entry, Refusal> {
        Store::add_temporary(self, key, why, incident, term, now)
    }
    fn lift(&mut self, key: &Key) -> Result<bool, Refusal> {
        Store::lift(self, key)
    }
    fn list(&mut self, now: Duration) -> Vec<Entry> {
        Store::list(self, now)
    }
}

impl RateLimiter for Limiter {
    fn observe(&mut self, key: &str, now: Duration) -> Result<Observation, Unavailable> {
        Limiter::observe(self, key, now)
    }
    fn level(&self, count: usize) -> Level {
        Limiter::level(self, count)
    }
    fn configure(
        &mut self,
        window: Duration,
        soft: usize,
        hard: usize,
        now: Duration,
    ) -> Result<ConfigurationEffect, Unavailable> {
        Limiter::configure(self, window, soft, hard, now)
    }
    fn gc(&mut self, now: Duration) -> Result<(), Unavailable> {
        Limiter::gc(self, now).map(|_| ())
    }
}

#[cfg(test)]
mod tests;
