//! Separate authenticated admin TCP listener (Go `cmd/gate` `-admin-addr`,
//! `HMN_ADMIN_TOKENS` and the in-memory `BanStore` thresholds at baseline
//! 981d25c), owner decision B01 #1.
//!
//! Kept from Go:
//! - One bearer token per operator, each bound to one of the four roles
//!   (auditor, operator, approver, DPO). Outside the local demo Go refused a
//!   shipped or placeholder token (an `e2e-` prefix, or `change`,
//!   `placeholder`, `example` or `your-` in any case) and one shorter than
//!   16 characters; that check is [`weak_token`].
//! - The ban store rate window and cutpoints default to Go
//!   `DefaultRateWindow`/`DefaultRateSoft`/`DefaultRateHard` (10 s, 60, 120).
//!
//! Deliberate differences, each pinned by tests:
//! - The listener is loopback only and no key widens it. Go defaulted to
//!   loopback and only warned on a wider bind; the warning case is refused.
//! - It serves plain HTTP on loopback. Go served the admin listener with a
//!   per-boot self-signed certificate and optional mutual TLS; the runtime
//!   has no production self-signed certificate (plan B02), so off-host access
//!   goes through an operator-run TLS or SSO front, as Go also recommended.
//! - Tokens come from the private inherited configuration, never from the
//!   environment, and there is no local-demo mode: weak tokens are always
//!   refused, and Go's per-boot random tokens for unlisted roles (which no
//!   one could present) are not minted. At least one operator is required.
//! - The ban store needs explicit history bounds; the rate history kernel has
//!   no production defaults, so absent bounds refuse the configuration.
//! - `models` carries Go's `-ml-bundle`, `-ml-pubkey`, `-ml-bundle-sig` and
//!   `-ml-bundle-digest` admission flags (cmd/server/ml_setup.go). Under
//!   owner decision #12 the bundle is only staged at startup; activation is
//!   the dual-controlled admin promote. See `gateway::service::admin`.
use crate::gateway::Rejection;
use crate::gateway::admin_approval::Role;
use crate::gateway::ban::Store;
use crate::network::history::rate;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroizing;

/// Most configured operators.
pub const MAX_OPERATORS: usize = 64;
/// Longest accepted bearer token.
pub const MAX_TOKEN: usize = 512;
/// Go `DefaultRateWindow` in seconds.
pub const DEFAULT_RATE_WINDOW_SEC: u64 = 10;
/// Go `DefaultRateSoft`.
pub const DEFAULT_RATE_SOFT: usize = 60;
/// Go `DefaultRateHard`.
pub const DEFAULT_RATE_HARD: usize = 120;

/// Admin listener section of the private startup document. Not Debug: it
/// carries bearer tokens.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminListener {
    /// Loopback admin address; port zero is available for pilots.
    pub listen: SocketAddr,
    /// Operators and their bearer tokens (Go `HMN_ADMIN_TOKENS`).
    pub operators: Vec<AdminOperator>,
    /// Bounds of the ban store the plane and the edge ban gate share.
    pub bans: BanStoreBounds,
    /// Directory of the runtime settings overlay store (Go `-settings-dir`).
    /// Absent, the settings routes answer 404 after authentication.
    #[serde(default)]
    pub settings_dir: Option<PathBuf>,
    /// Behavioral-model candidate staged at startup (Go `-ml-*` admission
    /// flags). Absent, the `models` routes answer 404 after authentication.
    #[serde(default)]
    pub models: Option<ModelSource>,
}

/// Startup model candidate (legacy `mlFlags` admission fields).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelSource {
    /// `-ml-bundle`: bundle JSON path.
    pub bundle: PathBuf,
    /// `-ml-pubkey`: PKIX Ed25519 PEM path; when set every stage needs a
    /// signature it accepts.
    #[serde(default)]
    pub public_key: Option<PathBuf>,
    /// `-ml-bundle-sig`: detached signature over the exact bundle bytes.
    #[serde(default)]
    pub signature: Option<PathBuf>,
    /// `-ml-bundle-digest`: lowercase hex SHA-256 pin; empty means unset.
    #[serde(default)]
    pub digest: String,
}

impl ModelSource {
    fn valid(&self) -> bool {
        let path = |path: &PathBuf| !path.as_os_str().is_empty();
        path(&self.bundle)
            && self.public_key.as_ref().is_none_or(path)
            && self.signature.as_ref().is_none_or(path)
            && (self.digest.is_empty()
                || (self.digest.len() == 64
                    && self
                        .digest
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))))
    }
}

/// One operator principal.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminOperator {
    /// `auditor`, `operator`, `approver` or `dpo`.
    pub role: String,
    /// Operator id recorded (pseudonymized) in audit records.
    pub id: String,
    /// Bearer token; never logged or echoed in readiness.
    pub token: Zeroizing<String>,
}

/// Explicit ban store bounds.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanStoreBounds {
    /// Active bans and strike records retained.
    pub capacity: usize,
    /// Rate window in seconds (Go default 10).
    #[serde(default = "default_window")]
    pub window_sec: u64,
    /// Soft cutpoint (Go default 60).
    #[serde(default = "default_soft")]
    pub soft: usize,
    /// Hard cutpoint (Go default 120).
    #[serde(default = "default_hard")]
    pub hard: usize,
    /// Rate history: tracked keys.
    pub max_keys: usize,
    /// Rate history: hits retained per key.
    pub max_hits_per_key: usize,
    /// Rate history: key bytes.
    pub max_key_bytes: usize,
    /// Rate history: retained payload bytes.
    pub max_payload_bytes: usize,
    /// Rate history: keys scanned per sweep (at most 4096).
    pub gc_scan_budget: usize,
}

fn default_window() -> u64 {
    DEFAULT_RATE_WINDOW_SEC
}
fn default_soft() -> usize {
    DEFAULT_RATE_SOFT
}
fn default_hard() -> usize {
    DEFAULT_RATE_HARD
}

/// Go `cmd/gate` boot refusal of a shipped, placeholder or short admin secret.
pub fn weak_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    token.starts_with("e2e-")
        || ["change", "placeholder", "example", "your-"]
            .iter()
            .any(|marker| lower.contains(marker))
        || token.chars().count() < 16
}

impl AdminOperator {
    /// The parsed role.
    pub fn role(&self) -> Option<Role> {
        Role::parse(&self.role)
    }
}

impl BanStoreBounds {
    /// Build the empty shared store; any refused bound is a configuration
    /// refusal.
    pub fn store(&self) -> Result<Store, Rejection> {
        let bad = || Rejection::new(503, "configuration");
        let window = Duration::from_secs(self.window_sec);
        let limits = rate::Limits::new(
            self.max_keys,
            self.max_hits_per_key,
            self.max_key_bytes,
            window,
            self.max_payload_bytes,
            self.gc_scan_budget,
        )
        .map_err(|_| bad())?;
        let config = rate::Config::new(window, self.soft, self.hard, limits).map_err(|_| bad())?;
        Store::new(config, self.capacity).map_err(|_| bad())
    }
}

impl AdminListener {
    /// Fail closed before any listener binds.
    pub fn validate(&self) -> Result<(), Rejection> {
        let bad = Rejection::new(503, "configuration");
        if !super::exposure::admin_listen_permitted(self.listen)
            || self.operators.is_empty()
            || self.operators.len() > MAX_OPERATORS
            || self
                .settings_dir
                .as_ref()
                .is_some_and(|dir| dir.as_os_str().is_empty())
            || self.models.as_ref().is_some_and(|models| !models.valid())
        {
            return Err(bad);
        }
        let mut tokens = std::collections::BTreeSet::new();
        let mut ids = std::collections::BTreeSet::new();
        for operator in &self.operators {
            let token = operator.token.as_str();
            if operator.role().is_none()
                || operator.id.is_empty()
                || operator.id.len() > 128
                || !operator.id.bytes().all(|b| b.is_ascii_graphic())
                || token.len() > MAX_TOKEN
                || !token.bytes().all(|b| b.is_ascii_graphic())
                || weak_token(token)
                || !tokens.insert(token)
                || !ids.insert(operator.id.as_str())
            {
                return Err(bad);
            }
        }
        self.bans.store().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn admin(listen: &str, token: &str) -> AdminListener {
        serde_json::from_value(json!({
            "listen": listen,
            "operators": [{"role":"operator","id":"op-1","token":token}],
            "bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,"max_key_bytes":128,
                "max_payload_bytes":1_048_576,"gc_scan_budget":64}
        }))
        .unwrap()
    }

    #[test]
    fn go_weak_admin_secrets_are_refused() {
        for token in [
            "e2e-operator-token-0123",
            "CHANGE-ME-0123456789",
            "placeholder-0123456789",
            "Example-0123456789abc",
            "your-token-0123456789",
            "short-token",
        ] {
            assert!(weak_token(token), "{token}");
            assert!(admin("127.0.0.1:0", token).validate().is_err(), "{token}");
        }
        assert!(!weak_token("operator-fixture-token"));
        assert!(
            admin("127.0.0.1:0", "operator-fixture-token")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn model_source_paths_and_digest_pin_are_checked() {
        let with = |models: serde_json::Value| {
            let mut admin = admin("127.0.0.1:0", "operator-fixture-token");
            admin.models = Some(serde_json::from_value(models).unwrap());
            admin.validate()
        };
        let hex = "0123456789abcdef".repeat(4);
        assert!(with(json!({"bundle": "/m/b.json"})).is_ok());
        assert!(
            with(json!({"bundle": "/m/b.json", "public_key": "/m/k.pem",
            "signature": "/m/b.sig", "digest": hex}))
            .is_ok()
        );
        assert!(with(json!({"bundle": ""})).is_err());
        assert!(with(json!({"bundle": "/m/b.json", "public_key": ""})).is_err());
        assert!(with(json!({"bundle": "/m/b.json", "signature": ""})).is_err());
        assert!(with(json!({"bundle": "/m/b.json", "digest": hex.to_uppercase()})).is_err());
        assert!(with(json!({"bundle": "/m/b.json", "digest": "abc"})).is_err());
        assert!(
            serde_json::from_value::<ModelSource>(json!({"bundle": "/m/b.json", "shadow": "/x"}))
                .is_err()
        );
    }

    #[test]
    fn non_loopback_admin_listen_is_refused() {
        let token = "operator-fixture-token";
        assert!(admin("127.0.0.1:8445", token).validate().is_ok());
        assert!(admin("[::1]:8445", token).validate().is_ok());
        assert!(admin("0.0.0.0:8445", token).validate().is_err());
        assert!(admin("192.0.2.10:8445", token).validate().is_err());
    }

    #[test]
    fn operator_table_and_ban_bounds_fail_closed() {
        let token = "operator-fixture-token";
        let mut config = admin("127.0.0.1:0", token);
        config.operators.clear();
        assert!(config.validate().is_err());
        let mut config = admin("127.0.0.1:0", token);
        config.operators[0].role = "root".into();
        assert!(config.validate().is_err());
        let mut config = admin("127.0.0.1:0", token);
        config.operators.push(AdminOperator {
            role: "approver".into(),
            id: "ap-1".into(),
            token: Zeroizing::new(token.into()),
        });
        assert!(config.validate().is_err(), "duplicate token");
        let mut config = admin("127.0.0.1:0", token);
        config.operators[0].token = Zeroizing::new("8f1c2a9b e04d7c11a5".into());
        assert!(config.validate().is_err(), "token outside a bearer header");
        let mut config = admin("127.0.0.1:0", token);
        config.bans.max_keys = 0;
        assert!(config.validate().is_err());
        let config = admin("127.0.0.1:0", token);
        assert_eq!(config.bans.window_sec, 10);
        assert_eq!((config.bans.soft, config.bans.hard), (60, 120));
    }
}
