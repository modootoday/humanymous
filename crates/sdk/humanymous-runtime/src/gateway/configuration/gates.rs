//! Config-driven gate provisioning (owner decision #2, issue #75 comment
//! 7017558784): the ban store, Web Bot Auth key directory, Privacy Pass
//! issuers, WebAuthn credentials and the decision-probing sweep detector of
//! legacy `cmd/gate/main.go` (`-agent-keys`, `-pat-issuers`,
//! `-webauthn-creds`, `HMN_WEBAUTHN_ORIGIN`, `HMN_WEBAUTHN_RPID`) and
//! internal/gate `gate.go` `NewServer` at baseline 981d25c.
//!
//! Absent, nothing is installed (the served path skips each gate, as Go did
//! with a nil directory, verifier or registry). Present, the source must load:
//! an unreadable, oversized, empty or invalid file, or a refused bound, fails
//! startup before audit custody opens, as Go's `gateFatalf` did. Source
//! formats are Go's, parsed by the ported `gateway::credential` kernels.
//!
//! Deliberate differences, each pinned by tests:
//! - Go built the sweep detector (30 s, 8 sessions) and an in-memory ban store
//!   unconditionally; here both are opt-in, and the sweep needs an explicit
//!   binding bound (no unbounded map).
//! - The WebAuthn fan-out tracker needs explicit resource bounds; Go's map was
//!   unbounded. Its cap keeps Go's `CredFanoutCap <= 0` default.
//! - The WebAuthn origin and RP ID come from this document, not the process
//!   environment.
//! - A standalone ban store is refused when the admin listener is configured:
//!   the admin plane owns the one store its ban routes write.
use crate::gateway::Rejection;
use crate::gateway::attest_floor::fanout::{self, CredentialFanout};
use crate::gateway::ban::gate::Mode;
use crate::gateway::configuration::admin::BanStoreBounds;
use crate::gateway::credential::{MAX_CONFIG, privacy_pass, web_bot_auth, webauthn};
use crate::network::correlation;
use crate::network::history::anomaly;
use crate::network::history::sweep::{Config as SweepConfig, Detector};
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Gate provisioning section of the private startup document.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gates {
    /// In-memory ban store for the `gate.ban` gate (no admin listener).
    #[serde(default)]
    pub bans: Option<BanStoreBounds>,
    /// Web Bot Auth trusted-key allowlist file: `keyid base64url-ed25519`
    /// lines (Go `-agent-keys`).
    #[serde(default)]
    pub agent_keys: Option<PathBuf>,
    /// PEM file of trusted Privacy Pass issuer public keys (Go `-pat-issuers`).
    #[serde(default)]
    pub pat_issuers: Option<PathBuf>,
    /// WebAuthn registered-credential allowlist (Go `-webauthn-creds`).
    #[serde(default)]
    pub webauthn: Option<WebAuthn>,
    /// Decision-probing sweep detector (HR-30).
    #[serde(default)]
    pub sweep: Option<Sweep>,
}

/// WebAuthn credentials, binding and the credential fan-out tracker bounds.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebAuthn {
    /// `credentialId base64-spki` lines (Go `NewWebAuthnRegistry`).
    pub credentials: PathBuf,
    /// Expected client-data origin; empty leaves the check off (Go
    /// `HMN_WEBAUTHN_ORIGIN`).
    #[serde(default)]
    pub origin: String,
    /// Expected RP ID; empty leaves the check off (Go `HMN_WEBAUTHN_RPID`).
    #[serde(default)]
    pub rp_id: String,
    /// Distinct /24s per credential before its fast path reverts (Go
    /// `CredFanoutCap`); zero or negative selects the default.
    #[serde(default)]
    pub fanout_cap: i64,
    /// Fan-out tracker resource bounds.
    pub fanout: FanoutBounds,
}

/// Explicit resource bounds of the credential fan-out tracker.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanoutBounds {
    /// Retained credential keys.
    pub max_keys: usize,
    /// Distinct subnets retained per credential.
    pub max_distinct: usize,
    /// Bytes per retained field.
    pub max_field_bytes: usize,
    /// Payload bytes across the tracker.
    pub max_payload_bytes: usize,
    /// Population series retained.
    pub population_keys: usize,
    /// Population samples per series.
    pub population_window: usize,
    /// Population key bytes.
    pub population_key_bytes: usize,
    /// Population payload bytes.
    pub population_payload_bytes: usize,
}

/// Sweep detector settings (Go `NewSweepDetector(30*time.Second, 8)`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sweep {
    /// Boot `gate.recon_sweep` mode; an active settings overlay overrides it.
    #[serde(default = "enforce")]
    pub mode: Mode,
    /// Observation window in seconds.
    #[serde(default = "default_window")]
    pub window_sec: u64,
    /// Sessions per fingerprint inside the window before it is a sweep.
    #[serde(default = "default_sessions")]
    pub max_sessions: usize,
    /// Retained fingerprint bindings.
    pub max_bindings: usize,
}

fn enforce() -> Mode {
    Mode::Enforce
}
fn default_window() -> u64 {
    30
}
fn default_sessions() -> usize {
    8
}

/// Loaded capabilities, ready to install on the application.
#[derive(Default)]
pub struct Provisioned {
    /// Standalone ban store.
    pub bans: Option<crate::gateway::ban::Store>,
    /// Web Bot Auth directory.
    pub agent_keys: Option<web_bot_auth::KeyDirectory>,
    /// Privacy Pass issuers.
    pub pat_issuers: Option<privacy_pass::Verifier>,
    /// WebAuthn registry and its fan-out tracker.
    pub webauthn: Option<(webauthn::Registry, CredentialFanout)>,
    /// Sweep detector and its boot mode.
    pub sweep: Option<(Detector, Mode)>,
}

impl Provisioned {
    /// Nonsecret readiness: which capabilities are installed.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"bans":self.bans.is_some(),"agent_keys":self.agent_keys.is_some(),
            "pat_issuers":self.pat_issuers.is_some(),"webauthn":self.webauthn.is_some(),
            "sweep":self.sweep.is_some()})
    }
}

fn bad() -> Rejection {
    Rejection::new(503, "configuration")
}

/// Read a provisioning source of at most `MAX_CONFIG` bytes as UTF-8.
fn read(path: &Path) -> Result<String, Rejection> {
    let file = std::fs::File::open(path).map_err(|_| bad())?;
    let mut text = String::new();
    file.take(MAX_CONFIG as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|_| bad())?;
    if text.len() > MAX_CONFIG {
        return Err(bad());
    }
    Ok(text)
}

impl FanoutBounds {
    fn tracker(&self, cap: i64) -> Result<CredentialFanout, Rejection> {
        let limits = correlation::Limits::new(
            self.max_keys,
            1,
            self.max_distinct,
            self.max_field_bytes,
            self.max_payload_bytes,
        )
        .map_err(|_| bad())?;
        let population = anomaly::Limits::new(
            self.population_keys,
            self.population_window,
            self.population_key_bytes,
            self.population_payload_bytes,
        )
        .map_err(|_| bad())?;
        let config =
            correlation::Config::new(fanout::WINDOW, limits, population).map_err(|_| bad())?;
        Ok(CredentialFanout::new(
            correlation::Registry::new(config),
            cap,
        ))
    }
}

impl Gates {
    /// Load every configured source; any failure refuses startup.
    /// `admin` is whether the admin listener (which owns a ban store) is on.
    pub fn load(&self, admin: bool) -> Result<Provisioned, Rejection> {
        let bans = match &self.bans {
            Some(_) if admin => return Err(bad()),
            Some(bounds) => Some(bounds.store()?),
            None => None,
        };
        let agent_keys = match &self.agent_keys {
            Some(path) => Some(web_bot_auth::KeyDirectory::parse(&read(path)?).map_err(|_| bad())?),
            None => None,
        };
        let pat_issuers = match &self.pat_issuers {
            Some(path) => Some(privacy_pass::Verifier::from_pem(&read(path)?).map_err(|_| bad())?),
            None => None,
        };
        let webauthn = match &self.webauthn {
            Some(config) => {
                let mut registry =
                    webauthn::Registry::parse(&read(&config.credentials)?).map_err(|_| bad())?;
                registry.set_binding(&config.origin, &config.rp_id);
                Some((registry, config.fanout.tracker(config.fanout_cap)?))
            }
            None => None,
        };
        let sweep = match &self.sweep {
            Some(config) => {
                let detector = SweepConfig::new(
                    Duration::from_secs(config.window_sec),
                    config.max_sessions,
                    config.max_bindings,
                )
                .map_err(|_| bad())?;
                Some((Detector::new(detector), config.mode))
            }
            None => None,
        };
        Ok(Provisioned {
            bans,
            agent_keys,
            pat_issuers,
            webauthn,
            sweep,
        })
    }
}
