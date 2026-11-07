//! Provisioned scope, complete route inventory and explicit capability loss.
use crate::authority::Rejection;
use humanymous_core::integrity::adapter::AdapterProfile;
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
        "python-asgi" => Ok(AdapterProfile::PythonAsgi),
        "python-wsgi" => Ok(AdapterProfile::PythonWsgi),
        "ruby-rack" => Ok(AdapterProfile::RubyRack),
        "apache" => Ok(AdapterProfile::Apache),
        "aspnetcore" => Ok(AdapterProfile::DotnetAspNetCore),
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
    /// Inline component key, zeroizing from deserialization, 32..256 bytes lower hex.
    #[serde(default)]
    pub hmac_key_hex: zeroize::Zeroizing<String>,
    /// Inline component seed, zeroizing from deserialization, 32 bytes lower hex.
    #[serde(default)]
    pub signing_seed_hex: zeroize::Zeroizing<String>,
    /// Existing sealed custody. Mutually exclusive with inline component keys.
    #[serde(default)]
    pub keys: Option<SealedKeys>,
    /// Require a broker-injected witness before opening any sealed state.
    #[serde(default)]
    pub require_external_witness: bool,
    /// Explicit first-install authorization. Not a rollback-proof anchor.
    pub allow_new: bool,
    /// Independently retained trusted head, null on explicitly unanchored startup.
    pub trusted_head: Option<crate::audit::checkpoint::Checkpoint>,
}

/// Private inherited startup credential, never Debug or Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedKeys {
    /// Existing private custody directory; startup never creates keys.
    pub directory: PathBuf,
    /// Explicit private input transport; no argv/env/file fallback.
    pub unlock: UnlockSource,
}
/// Unlock policy is independent of the adapter and its authenticated peer.
#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum UnlockSource {
    /// Controlled broker/hosted-lab input; external parent provenance is required.
    InheritedPrivatePipe {
        /// Consumed before listening, never included in readiness.
        passphrase: zeroize::Zeroizing<String>,
    },
    /// Existing protected native service store; unsupported platforms refuse.
    OsServiceStore {
        /// Exact owned service namespace.
        service_name: String,
        /// Exact nonsecret provider selector.
        credential_id: String,
    },
}
impl Audit {
    /// Reject ambiguous or missing sources before opening persistent state.
    pub fn validate_source(&self) -> Result<(), Rejection> {
        let inline = !self.hmac_key_hex.is_empty() && !self.signing_seed_hex.is_empty();
        let any_inline = !self.hmac_key_hex.is_empty() || !self.signing_seed_hex.is_empty();
        if match &self.keys {
            Some(keys) => {
                any_inline
                    || keys.directory.as_os_str().is_empty()
                    || match &keys.unlock {
                        UnlockSource::InheritedPrivatePipe { passphrase } => {
                            passphrase.is_empty() || passphrase.len() > 4096
                        }
                        UnlockSource::OsServiceStore {
                            service_name,
                            credential_id,
                        } => service_name.is_empty() || credential_id.is_empty(),
                    }
            }
            None => !inline || self.require_external_witness,
        } {
            return Err(Rejection::new(503, "invalid_audit_source"));
        }
        Ok(())
    }
}

fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
impl Configuration {
    /// Provisioned observation capabilities; this does not qualify an adapter runtime.
    /// This contains no keys, credentials or client observations. A bound IPC
    /// listener does not imply routes requiring absent planes are ready.
    pub fn capability_manifest(&self) -> serde_json::Value {
        let node = self.adapter == AdapterProfile::Node;
        let nginx = self.adapter == AdapterProfile::Nginx;
        let routes: Vec<_> = self
            .routes
            .iter()
            .map(|route| {
                let r = &route.required;
                let available = !r.raw_tls && !r.original_h2 && !r.collector && !r.rit;
                serde_json::json!({"method": route.method, "path": route.path,
                "node_raw": node && available, "fetch_normalized": node && available && !r.raw_header_order,
                "nginx_raw": nginx && available,"nginx_normalized":nginx && available && !r.raw_header_order,
                "asgi_normalized": self.adapter == AdapterProfile::PythonAsgi && available && !r.raw_header_order,
                "wsgi_normalized": self.adapter == AdapterProfile::PythonWsgi && available && !r.raw_header_order,
                "rack_normalized": self.adapter == AdapterProfile::RubyRack && available && !r.raw_header_order,
                "apache_normalized": self.adapter == AdapterProfile::Apache && available && !r.raw_header_order,
                "aspnetcore_normalized": self.adapter == AdapterProfile::DotnetAspNetCore && available && !r.raw_header_order})
            })
            .collect();
        serde_json::json!({"id":"application-http", "revision":self.scope.revision,
            "raw_tls":false, "original_h2":false, "collector":false, "rit":false,
            "complete_body":true, "raw_header_order":"node-raw-or-nginx-raw-only", "routes":routes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn provisioned_server_profiles_reject_mixed_wire_identity_and_mode() {
        for (name, id, mode) in [
            ("apache", "humanymous.apache", "apache-normalized"),
            ("aspnetcore", "humanymous.dotnet", "aspnetcore-normalized"),
        ] {
            let selected = adapter_profile(serde_json::Value::String(name.into())).unwrap();
            let mut request = serde_json::json!({
                "protocol":"humanymous.adapter/0.1","id":"01234567-89ab-4cde-8fab-0123456789ab",
                "tenant":"t","origin":"o","revision":"r","deadline":2000,"method":"POST",
                "target":"/api","authority":"owned.test","peer":null,
                "headers":[["host","owned.test"],["content-length","3"]],"headerMode":mode,
                "body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":3,
                    "sha256":hex::encode(sha2::Sha256::digest(b"abc")),"base64":"YWJj"},
                "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":false,"completeBody":true},
                "stream":null,"capture":null,"connectionId":null,
                "adapter":{"id":id,"version":"0.1.0-experimental.1","installation":"i"}
            });
            let valid = serde_json::to_vec(&request).unwrap();
            assert!(
                humanymous_core::integrity::adapter::parse_with_profile(&valid, 1000, selected)
                    .is_ok()
            );
            request["adapter"]["id"] = "humanymous.node".into();
            assert!(
                humanymous_core::integrity::adapter::parse_with_profile(
                    &serde_json::to_vec(&request).unwrap(),
                    1000,
                    selected
                )
                .is_err()
            );
            request["adapter"]["id"] = id.into();
            request["headerMode"] = "asgi-normalized".into();
            assert!(
                humanymous_core::integrity::adapter::parse_with_profile(
                    &serde_json::to_vec(&request).unwrap(),
                    1000,
                    selected
                )
                .is_err()
            );
        }
    }

    #[test]
    fn provisioning_selects_exact_profile_and_rejects_wire_names() {
        for (name, expected) in [
            ("node", AdapterProfile::Node),
            ("nginx", AdapterProfile::Nginx),
            ("python-asgi", AdapterProfile::PythonAsgi),
            ("python-wsgi", AdapterProfile::PythonWsgi),
            ("ruby-rack", AdapterProfile::RubyRack),
            ("apache", AdapterProfile::Apache),
            ("aspnetcore", AdapterProfile::DotnetAspNetCore),
        ] {
            let value = serde_json::Value::String(name.into());
            assert_eq!(adapter_profile(value).unwrap(), expected);
        }
        for name in [
            "python",
            "ruby",
            "humanymous.python",
            "asgi-normalized",
            "humanymous.apache",
            "apache-normalized",
            "dotnet",
            "aspnetcore-normalized",
            "",
        ] {
            assert!(adapter_profile(serde_json::Value::String(name.into())).is_err());
        }
    }

    #[test]
    fn normalized_readiness_never_inherits_another_profile_or_missing_plane() {
        for (profile, ready) in [
            ("python-asgi", "asgi_normalized"),
            ("python-wsgi", "wsgi_normalized"),
            ("ruby-rack", "rack_normalized"),
            ("apache", "apache_normalized"),
            ("aspnetcore", "aspnetcore_normalized"),
        ] {
            let value = serde_json::json!({
                "adapter":profile,"endpoint":"unused",
                "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"owned.test"},
                "peer":{"kind":"unix","uid":1},
                "routes":[{"method":"POST","path":"/api","required":{
                    "raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}],
                "audit":{"directory":"unused","node_id":"test","hmac_key_hex":"",
                    "signing_seed_hex":"","allow_new":false,"trusted_head":null},
                "capacity":1,"replay_capacity":1,"rate_per_second":1,"timeout_ms":1,"clock_skew_ms":0
            });
            for missing in [
                None,
                Some("raw_tls"),
                Some("original_h2"),
                Some("raw_header_order"),
                Some("collector"),
                Some("rit"),
            ] {
                let mut configured = value.clone();
                if let Some(required) = missing {
                    configured["routes"][0]["required"][required] = true.into();
                }
                let config: Configuration = serde_json::from_value(configured).unwrap();
                let manifest = config.capability_manifest();
                for mode in [
                    "node_raw",
                    "fetch_normalized",
                    "nginx_raw",
                    "nginx_normalized",
                    "asgi_normalized",
                    "wsgi_normalized",
                    "rack_normalized",
                    "apache_normalized",
                    "aspnetcore_normalized",
                ] {
                    assert_eq!(
                        manifest["routes"][0][mode],
                        mode == ready && missing.is_none()
                    );
                }
            }
        }
    }
}
