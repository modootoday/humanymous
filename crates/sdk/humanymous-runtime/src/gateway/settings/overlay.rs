//! Legacy runtime overlay (SoT-39 RuntimeOverlay), ported Go-identically from
//! the frozen oracle `internal/gate/settings` at 981d25c under owner decision B1
//! (issue #75 comment 6979694428).
//!
//! This is a scoring-input model: its defaults, bounds, mutation classes and
//! catalogs equal the oracle exactly and must not be tuned. An empty overlay
//! resolves to the built-in policy, so freeze goldens stay behavior-identical.
//! It is a pure library owner; the operational CAS snapshot in the parent module
//! remains the only settings path wired into the hosted gateway. The admin
//! write plane over it (propose, dry-run, rollback, approval-backed commit and
//! expiry) is `gateway::admin_plane::settings`, served only when attached.
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt::Write as _;

mod classify;
mod resolve;
mod store;
mod timestamp;
mod validate;
mod wire;

pub use classify::classify;
pub use resolve::{BootInput, Effective, configure_engine, resolve};
pub use store::{OverlayFileStore, OverlayLoadError, OverlayStoreError, schema_json};
pub use timestamp::{Timestamp, TimestampError};
pub use validate::{OverlayValidationError, validate};
pub use wire::{OverlayDecodeError, OverlayEncodeError};

#[cfg(test)]
mod tests;

/// Enforcement mode of a gate, hard rule or residual network class. Unknown
/// spellings are kept verbatim so validation can refuse them like the oracle.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// `enforce`.
    Enforce,
    /// `monitor`: observe and audit without enforcing.
    Monitor,
    /// `shadow`.
    Shadow,
    /// `off`.
    Off,
    /// Any other spelling, including the empty string. Only [`Mode::parse`]
    /// and [`Mode::default`] build it, so it never holds a known spelling.
    Other(UnknownMode),
}

/// A mode spelling outside the known set, kept verbatim for refusal messages.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownMode(String);

impl UnknownMode {
    /// Exact wire spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Mode {
    /// Classify a wire spelling without trimming or case folding.
    pub fn parse(value: &str) -> Self {
        match value {
            "enforce" => Self::Enforce,
            "monitor" => Self::Monitor,
            "shadow" => Self::Shadow,
            "off" => Self::Off,
            other => Self::Other(UnknownMode(other.into())),
        }
    }
    /// Exact wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Enforce => "enforce",
            Self::Monitor => "monitor",
            Self::Shadow => "shadow",
            Self::Off => "off",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl Default for Mode {
    /// The oracle's zero value is the empty string, which is not a valid mode.
    fn default() -> Self {
        Self::Other(UnknownMode(String::new()))
    }
}

/// Server-computed change class; a client-supplied class is never trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationClass {
    /// `A`: tighten.
    A,
    /// `B`: loosen a non-integrity control.
    B,
    /// `C`: neutralize or weaken an integrity control.
    C,
    /// `D`: weaken a route preset.
    D,
}

impl MutationClass {
    /// Wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

/// Versioned operator delta. Field names, optionality and wire order follow
/// the oracle `Overlay` struct exactly.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    /// `schemaVersion`; empty or `1.0.0`.
    pub schema_version: String,
    /// `overlayId`; empty means no configuration-version digest.
    pub overlay_id: String,
    /// `createdAt`; zero time when absent.
    pub created_at: Timestamp,
    /// `createdBy`.
    pub created_by: String,
    /// `approvedBy`.
    pub approved_by: String,
    /// `status`: pending, active, rolled_back or expired; empty counts as active.
    pub status: String,
    /// `expiresAt`.
    pub expires_at: Option<Timestamp>,
    /// `parentConfigVersion`.
    pub parent_config_version: String,
    /// `mutationClass` as stored; informational only.
    pub mutation_class: String,
    /// Gate module modes.
    pub gates: BTreeMap<String, Mode>,
    /// Engine hard-rule modes.
    pub hard_rules: BTreeMap<String, Mode>,
    /// Policy threshold overrides.
    pub scoring: Option<ScoringPatch>,
    /// Per-signal score multipliers.
    pub weight_multipliers: BTreeMap<String, f64>,
    /// Residual network class modes.
    pub net_policy: BTreeMap<String, Mode>,
    /// Route prefix presets.
    pub routes: BTreeMap<String, String>,
    /// Rate-limit overrides.
    pub rate_limit: Option<RateLimitPatch>,
    /// Global monitor posture override.
    pub global_monitor: Option<bool>,
}

/// Policy knob overrides; `None` keeps the code default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScoringPatch {
    /// `challengeAt`.
    pub challenge_at: Option<f64>,
    /// `denyAt`.
    pub deny_at: Option<f64>,
    /// `layerCap`.
    pub layer_cap: Option<f64>,
}

/// Rate threshold overrides; `None` keeps the boot value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RateLimitPatch {
    /// `windowSec`.
    pub window_sec: Option<i64>,
    /// `soft`.
    pub soft: Option<i64>,
    /// `hard`.
    pub hard: Option<i64>,
}

/// Admin propose and dry-run body (the oracle's `settingsProposeBody` in
/// internal/gate/admin_settings.go): allow-listed overlay content, the CAS
/// parent, the class C confirmation phrase and the requested lifetime. A
/// client-supplied mutation class or overlay identity is never read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProposeBody {
    /// `parentConfigVersion`: the configuration version the change is based on.
    pub parent_config_version: String,
    /// `hardRules`.
    pub hard_rules: BTreeMap<String, Mode>,
    /// `gates`.
    pub gates: BTreeMap<String, Mode>,
    /// `scoring`.
    pub scoring: Option<ScoringPatch>,
    /// `weightMultipliers`.
    pub weight_multipliers: BTreeMap<String, f64>,
    /// `netPolicy`.
    pub net_policy: BTreeMap<String, Mode>,
    /// `routes`.
    pub routes: BTreeMap<String, String>,
    /// `rateLimit`.
    pub rate_limit: Option<RateLimitPatch>,
    /// `globalMonitor`.
    pub global_monitor: Option<bool>,
    /// `integrityConfirm`: must be `DISABLE-INTEGRITY` for class C.
    pub integrity_confirm: String,
    /// `expiresInSec`: requested lifetime; zero or negative asks for the
    /// class default.
    pub expires_in_sec: i64,
}

impl ProposeBody {
    /// Decode the first JSON value of a request body with the oracle's
    /// `json.Decoder.Decode` semantics for this struct.
    pub fn from_json(bytes: &[u8]) -> Result<Self, OverlayDecodeError> {
        wire::decode_propose(bytes)
    }
}

/// Decode the first JSON value of a rollback body (the oracle's anonymous
/// `struct{ ParentConfigVersion string }`) and return the parent version.
pub fn decode_rollback_parent(bytes: &[u8]) -> Result<String, OverlayDecodeError> {
    wire::decode_parent(bytes)
}

/// Code defaults `(challengeAt, denyAt, layerCap)` of policy version 1.0.0.
pub const fn scoring_defaults() -> (f64, f64, f64) {
    (30.0, 70.0, 60.0)
}

/// Engine hard rules whose demotion to monitor is class C.
pub const INTEGRITY_CRITICAL_ENGINE_HRS: [&str; 5] = ["HR-1", "HR-16", "HR-18", "HR-19", "HR-2"];

/// Gates that cannot be off and whose demotion below enforce is class C.
pub const INTEGRITY_CRITICAL_GATES: [&str; 3] =
    ["gate.smuggle", "gate.spoof_header", "gate.verdict_token"];

/// Version 1 gate module catalog, in oracle order.
pub const KNOWN_GATES: [&str; 7] = [
    "gate.smuggle",
    "gate.spoof_header",
    "gate.ban",
    "gate.verdict_token",
    "gate.recon_sweep",
    "gate.inject",
    "gate.attest_floor",
];

/// Residual network policy classes, in oracle order.
pub const NET_POLICY_CLASSES: [&str; 14] = [
    "net.proxy.hop",
    "net.proxy.anon",
    "net.proxy.spoof",
    "net.vpn",
    "net.tor",
    "net.tcp",
    "net.correlation",
    "net.h2.spoof",
    "net.header.order",
    "net.tls.pq",
    "net.tls.alps",
    "net.tls.certcomp",
    "net.http.h1",
    "net.tls.ech",
];

impl Overlay {
    /// Stable signing JSON (`CanonicalJSON`): a fixed field subset in oracle
    /// order with sorted maps. `None` encodes as `{}`.
    pub fn canonical_json(overlay: Option<&Self>) -> Result<Vec<u8>, OverlayEncodeError> {
        match overlay {
            None => Ok(b"{}".to_vec()),
            Some(overlay) => wire::canonical(overlay).map(String::into_bytes),
        }
    }
    /// Go `json.MarshalIndent(overlay, "", "  ")` bytes, the persisted file form.
    pub fn to_json_indent(&self) -> Result<Vec<u8>, OverlayEncodeError> {
        wire::indented(self).map(String::into_bytes)
    }
    /// Go `json.Marshal(overlay)` bytes (the pending approval body and the
    /// admin overlay list).
    pub fn to_json(&self) -> Result<Vec<u8>, OverlayEncodeError> {
        wire::compact(self).map(String::into_bytes)
    }
    /// Decode with Go `encoding/json` semantics for this struct: case-insensitive
    /// keys, merged duplicates, ignored unknown fields and top-level `null`.
    pub fn from_json(bytes: &[u8]) -> Result<Self, OverlayDecodeError> {
        wire::decode(bytes)
    }
}

type HmacSha256 = Hmac<Sha256>;

fn keyed(key: &[u8]) -> HmacSha256 {
    HmacSha256::new_from_slice(key).expect("HMAC accepts any key length")
}

/// Short keyed digest of the canonical body; empty without an overlay ID and
/// `err` when the canonical body cannot be encoded.
pub fn overlay_digest(overlay: Option<&Overlay>, key: &[u8]) -> String {
    let Some(overlay) = overlay.filter(|overlay| !overlay.overlay_id.is_empty()) else {
        return String::new();
    };
    let Ok(body) = Overlay::canonical_json(Some(overlay)) else {
        return "err".into();
    };
    let mut mac = keyed(key);
    mac.update(&body);
    hex::encode(&mac.finalize().into_bytes()[..6])
}

/// Configuration version `cfg-…` binding posture, routes, rate and overlay digest.
#[allow(clippy::too_many_arguments)]
pub fn hash_config_version(
    key: &[u8],
    global_monitor: bool,
    kill_switch: bool,
    routes: &BTreeMap<String, String>,
    window_sec: i64,
    soft: i64,
    hard: i64,
    overlay: Option<&Overlay>,
) -> String {
    let mut text = format!("monitor={global_monitor}|kill={kill_switch}");
    for (prefix, preset) in routes {
        write!(text, "|{prefix}={preset}").expect("string write");
    }
    write!(text, "|rl={window_sec}/{soft}/{hard}").expect("string write");
    let digest = overlay_digest(overlay, key);
    if let Some(overlay) = overlay.filter(|_| !digest.is_empty()) {
        write!(text, "|ovl={digest}|id={}", overlay.overlay_id).expect("string write");
    }
    let mut mac = keyed(key);
    mac.update(text.as_bytes());
    format!("cfg-{}", hex::encode(&mac.finalize().into_bytes()[..6]))
}
