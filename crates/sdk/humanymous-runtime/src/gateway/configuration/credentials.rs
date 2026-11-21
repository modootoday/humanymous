//! Credential trust anchors of the served gate (shipping row B25; Go
//! `cmd/gate -agent-keys` and `-pat-issuers` at baseline 981d25c).
//!
//! Each optional path names an operator file the service reads once at
//! startup, before readiness:
//! - `agent_keys`: Web Bot Auth allowlist, `keyid base64url-ed25519-key` lines
//!   (`gateway::credential::web_bot_auth::KeyDirectory`);
//! - `pat_issuers`: PEM `PUBLIC KEY` blocks of trusted Privacy Pass issuers
//!   (`gateway::credential::privacy_pass::Verifier`);
//! - `webauthn`: the registered-credential allowlist, `credentialId
//!   base64url-spki-ecdsa-p256-key` lines (Go `-webauthn-creds`,
//!   `gateway::credential::webauthn::Registry`), its optional origin and RP ID
//!   binding (Go `HMN_WEBAUTHN_ORIGIN`, `HMN_WEBAUTHN_RPID`; empty leaves that
//!   check off, as in Go), the fan-out cap (Go `CredFanoutCap`, default 8)
//!   and explicit fan-out history bounds: the correlation registry has no
//!   production defaults, as the ban store's rate history has none.
//!
//! As in Go, an unreadable or invalid file refuses startup; the parsers are
//! stricter than Go's (see `gateway::credential`). A verified credential only
//! upgrades over scoring after request integrity (owner decision #11); it
//! never changes a score, threshold or signal.
use crate::gateway::Rejection;
use serde::Deserialize;
use std::path::PathBuf;

/// Optional credential trust-anchor files.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    /// Web Bot Auth trusted-key allowlist (Go `-agent-keys`).
    #[serde(default)]
    pub agent_keys: Option<PathBuf>,
    /// Trusted Privacy Pass issuer public keys (Go `-pat-issuers`).
    #[serde(default)]
    pub pat_issuers: Option<PathBuf>,
    /// WebAuthn possession verification (Go `-webauthn-creds`).
    #[serde(default)]
    pub webauthn: Option<WebAuthn>,
}

/// WebAuthn registered credentials, binding and fan-out bounds.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebAuthn {
    /// Registered-credential allowlist file.
    pub credentials: PathBuf,
    /// Expected client-data origin (Go `HMN_WEBAUTHN_ORIGIN`); empty is off.
    #[serde(default)]
    pub origin: String,
    /// Expected relying-party ID (Go `HMN_WEBAUTHN_RPID`); empty is off.
    #[serde(default)]
    pub rp_id: String,
    /// Distinct subnets one credential may come from before its upgrade is
    /// withheld on attested routes (Go `CredFanoutCap`); zero selects Go's 8.
    #[serde(default)]
    pub fanout_cap: u32,
    /// Fan-out history bounds.
    pub fanout: FanoutBounds,
}

/// Explicit credential fan-out history bounds. An observation beyond them
/// withholds the upgrade (fail closed), never admits it.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanoutBounds {
    /// Credentials tracked at once.
    pub max_credentials: usize,
    /// Distinct subnets and sessions retained per credential.
    pub max_distinct: usize,
    /// Longest credential key, subnet or session field in bytes.
    pub max_field_bytes: usize,
    /// Retained history bytes.
    pub max_payload_bytes: usize,
}

impl Credentials {
    /// Empty paths and an object naming no file are configuration refusals.
    pub fn validate(&self) -> Result<(), Rejection> {
        let empty =
            |path: &Option<PathBuf>| path.as_ref().is_some_and(|p| p.as_os_str().is_empty());
        let webauthn_bad = self.webauthn.as_ref().is_some_and(|webauthn| {
            webauthn.credentials.as_os_str().is_empty()
                || webauthn.origin.len() > 512
                || webauthn.rp_id.len() > 255
                || webauthn.fanout.limits().is_none()
        });
        if empty(&self.agent_keys)
            || empty(&self.pat_issuers)
            || webauthn_bad
            || self.agent_keys.is_none() && self.pat_issuers.is_none() && self.webauthn.is_none()
        {
            return Err(Rejection::new(503, "configuration"));
        }
        Ok(())
    }
}

/// One population series of the correlation registry's velocity measurement
/// (`network::correlation::Config` fixes its 256-sample window); these are
/// that series' own bounds, not credential history bounds.
const POPULATION: (usize, usize, usize, usize) = (16, 256, 256, 1 << 20);

impl FanoutBounds {
    /// The correlation registry configuration, or `None` for a zero bound.
    pub fn limits(&self) -> Option<crate::network::correlation::Config> {
        use crate::network::{correlation, history::anomaly};
        let limits = correlation::Limits::new(
            self.max_credentials,
            1,
            self.max_distinct,
            self.max_field_bytes,
            self.max_payload_bytes,
        )
        .ok()?;
        let (keys, window, key_bytes, payload) = POPULATION;
        let population = anomaly::Limits::new(keys, window, key_bytes, payload).ok()?;
        correlation::Config::new(
            crate::gateway::attest_floor::fanout::WINDOW,
            limits,
            population,
        )
        .ok()
    }
}
