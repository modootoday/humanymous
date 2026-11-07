//! Provisioned scope, complete route inventory and explicit capability loss.
use crate::Rejection;
use humanymous_integrity::adapter::AdapterProfile;
use serde::Deserialize;
use std::path::PathBuf;

/// One IPC service instance, read from a private inherited configuration pipe.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Credential-bound known producer. Omission preserves the Node profile.
    #[serde(default = "node_profile", deserialize_with = "adapter_profile")]
    pub adapter: AdapterProfile,
    /// Absolute filesystem UDS or local Windows pipe path.
    pub endpoint: String,
    /// Required installation, tenant, origin and active revision.
    pub scope: Scope,
    /// Actual OS principal/process, independent from request peer headers.
    pub peer: Peer,
    /// Complete exact method/path inventory.
    pub routes: Vec<Route>,
    /// Durable shared audit identity and recovery state.
    pub audit: Audit,
    /// Maximum simultaneous admitted IPC connections (1..128).
    pub capacity: usize,
    /// Maximum retained nonces (1..100000).
    pub replay_capacity: usize,
    /// Per-peer/global accepted connections per second (1..10000).
    pub rate_per_second: u32,
    /// Whole connection read/decision/write deadline (1..30000 milliseconds).
    pub timeout_ms: u64,
    /// Nonce retention beyond request deadline (0..30000 milliseconds).
    pub clock_skew_ms: u64,
}
fn node_profile() -> AdapterProfile {
    AdapterProfile::Node
}
fn adapter_profile<'de, D: serde::Deserializer<'de>>(d: D) -> Result<AdapterProfile, D::Error> {
    match String::deserialize(d)?.as_str() {
        "node" => Ok(AdapterProfile::Node),
        "nginx" => Ok(AdapterProfile::Nginx),
        _ => Err(serde::de::Error::custom("unsupported provisioned adapter")),
    }
}
/// Independently provisioned decision credential scope.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Node installation identity.
    pub installation: String,
    /// Tenant isolation identity.
    pub tenant: String,
    /// Origin identity.
    pub origin: String,
    /// Current configuration revision.
    pub revision: String,
    /// Exact origin Host/authority (case and port included).
    pub authority: String,
}
/// Native IPC peer requirements; platform-mismatched variants reject startup.
#[derive(Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Peer {
    /// Kernel-authenticated Unix effective UID.
    #[serde(rename = "unix")]
    Unix {
        /// Sole authorized Unix user.
        uid: u32,
    },
    /// Windows pipe DACL principal plus actual connected client process.
    #[serde(rename = "windows")]
    Windows {
        /// Sole permitted local SID for generated protected DACL.
        sid: String,
        /// Actual client PID supplied by a trusted supervisor.
        pid: u32,
    },
}
/// Exact protected origin route; queries remain part of request binding.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// Uppercase HTTP method.
    pub method: String,
    /// Exact origin path, without query or prefix matching.
    pub path: String,
    /// Required observations; absent observations never become defaults.
    pub required: Required,
}
/// Capability requirements are mandatory, explicit booleans.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Required {
    /// Original client TLS capture (not provided by Node protocol 0.1).
    pub raw_tls: bool,
    /// Original client HTTP/2 frames (not provided by Node protocol 0.1).
    pub original_h2: bool,
    /// Raw application header ordering (node-raw only).
    pub raw_header_order: bool,
    /// Authenticated browser collector report (not provided by protocol 0.1).
    pub collector: bool,
    /// Rust-owned browser/session request integrity (not provided by 0.1).
    pub rit: bool,
}
/// Keys are supplied through private stdin and never serialized or Debug-logged.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    /// Existing private WAL directory.
    pub directory: PathBuf,
    /// Stable audit node ID.
    pub node_id: String,
    /// Stable independent HMAC key, 32..256 bytes lower hex.
    pub hmac_key_hex: String,
    /// Stable 32-byte Ed25519 seed, lower hex.
    pub signing_seed_hex: String,
    /// Explicit first-install authorization. Not a rollback-proof anchor.
    pub allow_new: bool,
    /// Independently retained trusted head, null on explicitly unanchored startup.
    pub trusted_head: Option<humanymous_audit::checkpoint::Checkpoint>,
}

fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
impl Configuration {
    /// Honest readiness inventory for the only executable observation profile.
    /// This contains no keys, credentials or client observations. A bound IPC
    /// listener does not imply routes requiring absent planes are ready.
    pub fn capability_manifest(&self) -> serde_json::Value {
        let node = self.adapter == AdapterProfile::Node;
        let routes: Vec<_> = self
            .routes
            .iter()
            .map(|route| {
                let r = &route.required;
                let available = !r.raw_tls && !r.original_h2 && !r.collector && !r.rit;
                serde_json::json!({"method": route.method, "path": route.path,
                "node_raw": node && available, "fetch_normalized": node && available && !r.raw_header_order,
                "nginx_raw": !node && available,"nginx_normalized":!node && available && !r.raw_header_order})
            })
            .collect();
        serde_json::json!({"id":"application-http", "revision":self.scope.revision,
            "raw_tls":false, "original_h2":false, "collector":false, "rit":false,
            "complete_body":true, "raw_header_order":"node-raw-only", "routes":routes,
            "administration":false, "browser_recovery":false,
            "adapter":self.adapter.expected().id,"adapter_version":self.adapter.expected().version})
    }

    /// Validate trusted configuration before opening any public decision surface.
    pub fn validate(&self) -> Result<(), Rejection> {
        let bad = Rejection::new(503, "invalid_configuration");
        if !(1..=128).contains(&self.capacity)
            || !(1..=100_000).contains(&self.replay_capacity)
            || !(1..=10_000).contains(&self.rate_per_second)
            || !(1..=30_000).contains(&self.timeout_ms)
            || self.clock_skew_ms > 30_000
            || self.routes.is_empty()
            || self.routes.len() > 4096
            || [
                &self.scope.installation,
                &self.scope.tenant,
                &self.scope.origin,
                &self.scope.revision,
                &self.audit.node_id,
            ]
            .iter()
            .any(|s| !identity(s))
            || self.scope.authority.is_empty()
            || self.scope.authority.len() > 1024
            || !self
                .scope
                .authority
                .bytes()
                .all(|b| (33..=126).contains(&b) && !b"@/#?\\".contains(&b))
        {
            return Err(bad);
        }
        let mut seen = std::collections::BTreeSet::new();
        for route in &self.routes {
            if route.method.is_empty()
                || route.method.len() > 32
                || !route
                    .method
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b"!#$%&'*+-.^_`|~0123456789".contains(&b))
                || route.method == "CONNECT"
                || !route.path.starts_with('/')
                || route.path.starts_with("//")
                || route.path.len() > 8192
                || route
                    .path
                    .bytes()
                    .any(|b| b <= 32 || b >= 127 || b"?%#\\".contains(&b))
                || route.path.split('/').any(|s| s == "." || s == "..")
                || reserved(&route.path)
                || !seen.insert((&route.method, &route.path))
            {
                return Err(bad);
            }
        }
        match &self.peer {
            Peer::Unix { .. } if cfg!(unix) => {}
            Peer::Windows { sid, pid } if cfg!(windows) && *pid > 0 && valid_sid(sid) => {}
            _ => return Err(bad),
        }
        Ok(())
    }
}
/// Reserved data-plane namespace is unavailable in protocol 0.1.
pub fn reserved(path: &str) -> bool {
    path == "/_humanymous" || path.starts_with("/_humanymous/")
}
/// SID grammar restricted before generating a security descriptor string.
pub fn valid_sid(sid: &str) -> bool {
    sid.starts_with("S-1-5-")
        && sid.len() <= 184
        && sid[6..].split('-').all(|n| {
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && n.parse::<u32>().is_ok()
        })
}
