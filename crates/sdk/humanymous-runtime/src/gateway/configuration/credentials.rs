//! Credential trust anchors of the served gate (shipping row B25; Go
//! `cmd/gate -agent-keys` and `-pat-issuers` at baseline 981d25c).
//!
//! Each optional path names an operator file the service reads once at
//! startup, before readiness:
//! - `agent_keys`: Web Bot Auth allowlist, `keyid base64url-ed25519-key` lines
//!   (`gateway::credential::web_bot_auth::KeyDirectory`);
//! - `pat_issuers`: PEM `PUBLIC KEY` blocks of trusted Privacy Pass issuers
//!   (`gateway::credential::privacy_pass::Verifier`).
//!
//! As in Go, an unreadable or invalid file refuses startup; the parsers are
//! stricter than Go's (see `gateway::credential`). A verified credential only
//! upgrades over scoring after request integrity (owner decision #11); it
//! never changes a score, threshold or signal. WebAuthn credentials (Go
//! `-webauthn-creds`) need explicit fan-out bounds and are not configured here.
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
}

impl Credentials {
    /// Empty paths and an object naming no file are configuration refusals.
    pub fn validate(&self) -> Result<(), Rejection> {
        let empty =
            |path: &Option<PathBuf>| path.as_ref().is_some_and(|p| p.as_os_str().is_empty());
        if empty(&self.agent_keys)
            || empty(&self.pat_issuers)
            || self.agent_keys.is_none() && self.pat_issuers.is_none()
        {
            return Err(Rejection::new(503, "configuration"));
        }
        Ok(())
    }
}
