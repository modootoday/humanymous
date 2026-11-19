//! Gate runtime configuration fields and their zero-means-default resolution
//! (legacy `internal/gate/config.go` `Config`, the `rlWindow`/`rlSoft`/
//! `rlHard`/`erasureHold` helpers and the `NewServer` defaults for the
//! control path and credential fan-out cap, at baseline 981d25c).
//!
//! Route presets and path resolution are [`crate::gateway::route_policy`];
//! this type only carries the route table beside the scalar fields. The Go
//! capability fields (origin and token keys, token epochs, ban ledger, agent
//! key directory, Privacy Pass issuers, WebAuthn registry) are owned by
//! their modules (`gateway::token`, `gateway::ban`, `gateway::credential`)
//! and are passed to their consumers directly, never through this value.
//! Nothing here carries a score, weight, threshold or signal ID.
//!
//! Kept from Go, pinned against the oracle golden
//! `tests/fixtures/gateway/control-plane-go-golden.json` (`config`):
//! an empty control path is `/__hmn/`; a zero erasure hold is five minutes;
//! a zero credential fan-out cap is eight; a zero body cap is unlimited; the
//! legacy rate defaults are a 10 s window with soft 60 and hard 120.
//!
//! Deliberate difference: Go `rlWindow`/`rlSoft`/`rlHard` replaced a zero (or
//! negative) rate setting with those defaults. Owner decision 2026-11-16
//! (`.agents/retirement/owner-decisions-2026-11-16.md`, rate configuration)
//! keeps the fail-closed Rust behavior: [`GateConfig::rate`] refuses a zero
//! setting instead. Negative Go values have no Rust representation.
//!
//! Second deliberate difference, not covered by that owner decision: Go
//! `config.go` accepted a boot soft cutpoint above the hard one, while the
//! oracle's own settings contract (`internal/gate/settings_runtime.go`
//! `validateSettingsRuntime` at 981d25c) refuses `RateHard < RateSoft` for
//! every effective setting it applies. [`GateConfig::rate`] applies that one
//! contract to the boot value too (`rate::Config::new`), so hard below soft
//! is refused instead of producing an inverted limiter (fail-closed,
//! AGENTS.md hard constraint 2).
use crate::gateway::operations::erasure_hold;
use crate::gateway::route_policy::{RoutePolicy, RouteTable};
use crate::network::history::Unavailable;
use crate::network::history::rate::{Config as RateConfig, Limits};
use std::num::NonZeroU64;
use std::time::Duration;

/// Reserved control-plane prefix when none is configured.
pub const DEFAULT_CONTROL_PATH: &str = "/__hmn/";
/// Go `DefaultCredFanoutCap`: distinct /24 subnets one WebAuthn credential may
/// fast-path from before its trust upgrade reverts to Pass.
pub const DEFAULT_CRED_FANOUT_CAP: usize = 8;
/// Go `DefaultRateWindow`; documentation of the legacy value only.
pub const LEGACY_RATE_WINDOW: Duration = Duration::from_secs(10);
/// Go `DefaultRateSoft`; documentation of the legacy value only.
pub const LEGACY_RATE_SOFT: usize = 60;
/// Go `DefaultRateHard`; documentation of the legacy value only.
pub const LEGACY_RATE_HARD: usize = 120;

/// Gate runtime configuration (Go `Config` without capability fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GateConfig {
    /// Upstream origin base URL.
    pub upstream: String,
    /// Node identity.
    pub node_id: String,
    /// Reserved control-plane prefix; empty means [`DEFAULT_CONTROL_PATH`].
    pub control_path: String,
    /// Rate-limit window; zero is refused by [`Self::rate`].
    pub rate_window: Duration,
    /// Rate-limit soft cutpoint; zero is refused by [`Self::rate`].
    pub rate_soft: usize,
    /// Rate-limit hard cutpoint; zero is refused by [`Self::rate`].
    pub rate_hard: usize,
    /// Cancellable crypto-shred hold; zero means the five minute default.
    pub erasure_hold: Duration,
    /// Path prefix presets and the global monitor switch.
    pub routes: RouteTable,
    /// Log-only anomaly shadow observer (never affects a verdict).
    pub anomaly_shadow: bool,
    /// Credential /24 fan-out cap; zero means [`DEFAULT_CRED_FANOUT_CAP`].
    pub cred_fanout_cap: usize,
    /// Proxy-path request body cap in bytes; zero means unlimited.
    pub max_body_bytes: u64,
    /// Add `Strict-Transport-Security` to edge responses.
    pub hsts: bool,
    /// Read-only virtual-USB result directory; empty disables the view.
    pub virtual_usb_results_dir: String,
}

impl GateConfig {
    /// Policy for a request path (Go `Config.resolve`).
    pub fn resolve(&self, raw_path: &str) -> RoutePolicy {
        self.routes.resolve(raw_path)
    }
    /// Effective control-plane prefix (Go `NewServer`).
    pub fn control_path(&self) -> &str {
        if self.control_path.is_empty() {
            DEFAULT_CONTROL_PATH
        } else {
            &self.control_path
        }
    }
    /// Effective erasure hold (Go `erasureHold`).
    pub fn erasure_hold(&self) -> Duration {
        if self.erasure_hold.is_zero() {
            erasure_hold::DEFAULT_HOLD
        } else {
            self.erasure_hold
        }
    }
    /// Effective credential fan-out cap (Go `NewServer`).
    pub fn cred_fanout_cap(&self) -> usize {
        if self.cred_fanout_cap == 0 {
            DEFAULT_CRED_FANOUT_CAP
        } else {
            self.cred_fanout_cap
        }
    }
    /// Proxy-path body cap; `None` is unlimited (Go `MaxBodyBytes <= 0`).
    pub fn max_body(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.max_body_bytes)
    }
    /// Rate-limit configuration. Unlike Go there are no defaults: a zero
    /// window or cutpoint is refused (owner decision 2026-11-16), and hard
    /// below soft is refused as Go `validateSettingsRuntime` refused it.
    pub fn rate(&self, limits: Limits) -> Result<RateConfig, Unavailable> {
        RateConfig::new(self.rate_window, self.rate_soft, self.rate_hard, limits)
    }
}

#[cfg(test)]
mod tests;
