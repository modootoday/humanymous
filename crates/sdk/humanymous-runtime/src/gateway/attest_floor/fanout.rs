//! WebAuthn credential fan-out cap (ceiling-guard #2; legacy `webauthnGate`
//! in internal/gate/gates.go and `NewServer` in gate.go at baseline 981d25c).
//!
//! A verified possession proof normally buys the trust-upgrade fast path. If
//! one credential is seen from more than the cap of distinct /24 (or /48)
//! subnets, it is a shared or stolen passkey fronting a proxy farm: on an
//! attested route the upgrade is withheld and the request falls through to
//! the floor, which challenges (Pass), never denies. Fan-out is counted on
//! every route, so a farm cannot warm up on unmarked routes, but the upgrade
//! is withheld only on attested routes, so a roaming human on one passkey is
//! never newly challenged on ordinary traffic.
//!
//! Kept from Go: the `cred:` key, counting before the decision, the strict
//! `subnets > cap` test, the default cap of 8 for a non-positive setting and
//! the audit note. The caller supplies the correlation registry (Go
//! `correlation.New(time.Hour)`; [`WINDOW`] is that TTL) and verifies the
//! assertion first.
//!
//! Deliberate fail-closed difference, pinned by a test: when the registry
//! cannot admit the observation (a resource bound, a clock regression or an
//! over-long field), the upgrade is withheld on every route and the request
//! takes the ordinary pipeline. Go's unbounded map never refused.
use crate::network::correlation::{FleetInput, Registry, Stats};
use crate::network::history::Unavailable;
use std::time::Duration;

/// Go `DefaultCredFanoutCap`.
pub const DEFAULT_CAP: usize = 8;
/// Go credential correlation TTL.
pub const WINDOW: Duration = Duration::from_secs(60 * 60);

/// Whether the verified credential keeps its fast path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Trust-upgrade (forward without a behavioral fight).
    Upgrade,
    /// Over the cap on an attested route: no fast path; audit `note`.
    Withheld {
        /// Distinct subnets seen for this credential.
        subnets: usize,
        /// Go audit fail reason for the withheld upgrade.
        note: String,
    },
    /// The registry refused the observation; no fast path on any route.
    Unavailable,
}

/// Per-credential subnet fan-out tracker.
pub struct CredentialFanout {
    registry: Registry,
    cap: usize,
}

impl CredentialFanout {
    /// `configured` is the operator cap; zero or negative selects
    /// [`DEFAULT_CAP`] (Go `cfg.CredFanoutCap <= 0`).
    pub fn new(registry: Registry, configured: i64) -> Self {
        let cap = usize::try_from(configured)
            .ok()
            .filter(|cap| *cap > 0)
            .unwrap_or(DEFAULT_CAP);
        Self { registry, cap }
    }

    /// Effective cap.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Evict credentials whose window has passed (Go `credCorr.GC` in
    /// `Server.GC`).
    pub fn gc(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.registry.gc(now)
    }

    /// Count this verified assertion and decide the fast path.
    pub fn observe(
        &mut self,
        credential: &str,
        subnet: &str,
        sid: &str,
        attest_floor: bool,
        now: Duration,
    ) -> Decision {
        let key = format!("cred:{credential}");
        if self
            .registry
            .observe(&key, subnet, sid, now, FleetInput::LocalOnly)
            .is_err()
        {
            return Decision::Unavailable;
        }
        let Stats::Observed { subnets, .. } = self.registry.stats(&key) else {
            return Decision::Unavailable;
        };
        if attest_floor && subnets > self.cap {
            return Decision::Withheld {
                subnets,
                note: format!(
                    "webauthn credential {} fan-out {subnets} subnets > cap {} \u{2014} trust-upgrade withheld on attested route, routed to Pass",
                    short(credential),
                    self.cap
                ),
            };
        }
        Decision::Upgrade
    }
}

/// Go `credID[:16]`: the first 16 bytes, cut back to a character boundary.
fn short(credential: &str) -> &str {
    let mut end = credential.len().min(16);
    while !credential.is_char_boundary(end) {
        end -= 1;
    }
    &credential[..end]
}
