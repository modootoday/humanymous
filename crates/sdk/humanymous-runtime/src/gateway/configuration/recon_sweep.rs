//! Decision-probing sweep gate provisioning (legacy internal/gate gate.go
//! `NewSweepDetector(30*time.Second, 8)` and gates.go `sweepGate` at baseline
//! 981d25c, HR-30; owner decision #2, plan `plan:root/20261121201500`).
//!
//! Go always built the detector. Here it is configuration-driven: without a
//! `recon_sweep` object no sweep gate is installed. Present, the window and
//! threshold default to Go's (30 s, more than 8 sessions per fingerprint) and
//! the binding bound, which Go did not have, is explicit. The mode is the
//! boot mode of `gate.recon_sweep`; an active settings overlay that names the
//! gate replaces it on the request path (owner decision #3). Window and
//! threshold are the detector's measurement bounds, not scoring weights,
//! thresholds or verdict bands.
use crate::gateway::ban::gate::Mode;
use crate::network::history::sweep::{Config, Detector};
use serde::Deserialize;
use std::time::Duration;

/// Go's sliding window in milliseconds.
pub const DEFAULT_WINDOW_MS: u64 = 30_000;
/// Go's threshold: a binding with more distinct sessions is sweeping.
pub const DEFAULT_MAX_SESSIONS: usize = 8;
/// Upper bound on `window_ms` (one hour).
pub const MAX_WINDOW_MS: u64 = 3_600_000;
/// Upper bound on `max_sessions`.
pub const MAX_SESSIONS: usize = 4096;
/// Upper bound on `max_bindings`.
pub const MAX_BINDINGS: usize = 1 << 20;

/// `recon_sweep` section of the private startup document.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconSweep {
    /// Boot mode of `gate.recon_sweep`; default `enforce` (Go's default).
    #[serde(default = "enforce")]
    pub mode: Mode,
    /// Sliding window; default Go's 30 s. Zero or above one hour is refused.
    #[serde(default = "window")]
    pub window_ms: u64,
    /// Distinct sessions per binding the window tolerates; default Go's 8.
    #[serde(default = "sessions")]
    pub max_sessions: usize,
    /// Bindings retained at once (no default; Go was unbounded). A new
    /// binding beyond it cannot be counted: an enforcing gate answers 503
    /// `sweep_unavailable`, a non-enforcing one continues unrecorded.
    pub max_bindings: usize,
}

fn enforce() -> Mode {
    Mode::Enforce
}

fn window() -> u64 {
    DEFAULT_WINDOW_MS
}

fn sessions() -> usize {
    DEFAULT_MAX_SESSIONS
}

impl ReconSweep {
    /// The detector bounds; `None` when a bound is zero or out of range.
    pub fn config(&self) -> Option<Config> {
        if self.window_ms > MAX_WINDOW_MS
            || self.max_sessions > MAX_SESSIONS
            || self.max_bindings > MAX_BINDINGS
        {
            return None;
        }
        Config::new(
            Duration::from_millis(self.window_ms),
            self.max_sessions,
            self.max_bindings,
        )
        .ok()
    }

    /// A fresh detector, when the bounds are valid.
    pub fn detector(&self) -> Option<Detector> {
        self.config().map(Detector::new)
    }

    /// Nonsecret readiness view.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"mode":super::edge::mode_name(self.mode),"window_ms":self.window_ms,
            "max_sessions":self.max_sessions,"max_bindings":self.max_bindings})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_go_and_bounds_are_validated() {
        let sweep: ReconSweep = serde_json::from_value(json!({"max_bindings":1024})).unwrap();
        assert_eq!(
            (sweep.mode, sweep.window_ms, sweep.max_sessions),
            (Mode::Enforce, 30_000, 8)
        );
        assert!(sweep.config().is_some());
        assert_eq!(
            sweep.readiness(),
            json!({"mode":"enforce","window_ms":30000,"max_sessions":8,"max_bindings":1024})
        );
        let monitor: ReconSweep =
            serde_json::from_value(json!({"mode":"monitor","max_bindings":1})).unwrap();
        assert_eq!(monitor.mode, Mode::Monitor);
        assert!(serde_json::from_value::<ReconSweep>(json!({})).is_err());
        assert!(serde_json::from_value::<ReconSweep>(json!({"max_bindings":1,"extra":1})).is_err());
        assert!(
            serde_json::from_value::<ReconSweep>(json!({"max_bindings":1,"mode":"block"})).is_err()
        );
        for bad in [
            json!({"max_bindings":0}),
            json!({"max_bindings":1,"window_ms":0}),
            json!({"max_bindings":1,"max_sessions":0}),
            json!({"max_bindings":1,"window_ms":MAX_WINDOW_MS + 1}),
            json!({"max_bindings":1,"max_sessions":MAX_SESSIONS + 1}),
            json!({"max_bindings":MAX_BINDINGS + 1}),
        ] {
            let sweep: ReconSweep = serde_json::from_value(bad.clone()).unwrap();
            assert!(sweep.config().is_none(), "{bad}");
        }
    }
}
