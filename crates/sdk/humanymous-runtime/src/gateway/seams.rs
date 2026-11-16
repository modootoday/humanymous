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
//! the seam: a shared backend must route them through audit control too. The
//! sticky-verdict ledger has no Rust store yet.
use super::ban::{Entry, Key, Outcome, Refusal, Store};
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
}

impl BanLedger for Store {
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
}

#[cfg(test)]
mod tests;
