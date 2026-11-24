//! Opt-in verdict fast path (legacy internal/gate `verdictTokenGate` with the
//! `hmn_vt` issuer of control_collect.go at baseline 981d25c, SoT-21 §3,
//! HR-28; owner decision #4 with transport option 1, issue #75, 2026-11-24).
//!
//! Go needed a token key and was off without one. Here the gate is off unless
//! this object is configured. Present, a scored ALLOW on an enforcing route
//! keeps the session's verdict on the server for `ttl_ms` (Go's 30 minutes
//! by default), bound to the client fingerprint; within it the session's
//! requests take the audited fast path to the origin without re-scoring. No
//! token or cookie is sent to the client. The mode is the boot mode of
//! `gate.verdict_token`; an active settings overlay that names the gate
//! replaces it per request (owner decision #3).
use crate::gateway::ban::gate::Mode;
use serde::Deserialize;

/// Go's verdict-token lifetime (`now.Add(30*time.Minute)`), also the maximum.
pub const DEFAULT_TTL_MS: u64 = 30 * 60 * 1000;
/// Upper bound on `max_sessions`.
pub const MAX_SESSIONS: usize = 1 << 20;

/// `verdict_fast_path` section of the private startup document.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictFastPath {
    /// Boot mode of `gate.verdict_token`; default `enforce`.
    #[serde(default = "enforce")]
    pub mode: Mode,
    /// Verdict lifetime; default and maximum Go's 30 minutes, zero refused.
    #[serde(default = "ttl")]
    pub ttl_ms: u64,
    /// Sessions whose verdict is kept at once (no default).
    pub max_sessions: usize,
}

fn enforce() -> Mode {
    Mode::Enforce
}

fn ttl() -> u64 {
    DEFAULT_TTL_MS
}

impl VerdictFastPath {
    /// Whether the bounds are usable.
    pub fn valid(&self) -> bool {
        (1..=DEFAULT_TTL_MS).contains(&self.ttl_ms)
            && (1..=MAX_SESSIONS).contains(&self.max_sessions)
    }

    /// Nonsecret readiness view.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"mode":super::edge::mode_name(self.mode),"ttl_ms":self.ttl_ms,
            "max_sessions":self.max_sessions})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_go_and_bounds_are_validated() {
        let fast: VerdictFastPath = serde_json::from_value(json!({"max_sessions":64})).unwrap();
        assert_eq!((fast.mode, fast.ttl_ms), (Mode::Enforce, 1_800_000));
        assert!(fast.valid());
        assert_eq!(
            fast.readiness(),
            json!({"mode":"enforce","ttl_ms":1_800_000,"max_sessions":64})
        );
        for bad in [
            json!({"max_sessions":0}),
            json!({"max_sessions":1,"ttl_ms":0}),
            json!({"max_sessions":1,"ttl_ms":DEFAULT_TTL_MS + 1}),
            json!({"max_sessions":MAX_SESSIONS + 1}),
        ] {
            let fast: VerdictFastPath = serde_json::from_value(bad.clone()).unwrap();
            assert!(!fast.valid(), "{bad}");
        }
        assert!(serde_json::from_value::<VerdictFastPath>(json!({})).is_err());
        assert!(
            serde_json::from_value::<VerdictFastPath>(json!({"max_sessions":1,"key":"x"})).is_err()
        );
    }
}
