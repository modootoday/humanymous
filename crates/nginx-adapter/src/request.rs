//! Safe ingress projection and shared-schema prevalidation; no decision policy.
#![forbid(unsafe_code)]

use base64::{Engine, engine::general_purpose::STANDARD};
use humanymous_authority::configuration::Peer;
use humanymous_integrity::adapter::{AdapterProfile, parse_with_profile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use zeroize::Zeroize;

/// Maximum complete entity body accepted by the protocol.
pub const BODY_LIMIT: usize = 1_048_576;

/// Exact protected application route; no prefix matching or rewrite expansion.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// Uppercase method.
    pub method: String,
    /// Exact ASCII path, excluding query.
    pub path: String,
}

/// Private supervisor provisioning. Deliberately does not implement Debug.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Canonical absolute filesystem Unix socket; no TCP fallback.
    pub endpoint: String,
    /// Fresh credential delivered from the authority's private readiness pipe.
    pub credential: String,
    /// Actual expected server principal; Linux requires Unix.
    pub server: Peer,
    /// Provisioned installation.
    pub installation: String,
    /// Provisioned tenant.
    pub tenant: String,
    /// Provisioned origin.
    pub origin: String,
    /// Provisioned revision.
    pub revision: String,
    /// Exact Host/authority.
    pub authority: String,
    /// Single monotonic budget, including body reception and IPC.
    pub timeout_ms: u64,
    /// Complete method/path inventory.
    pub routes: Vec<Route>,
}
impl Drop for Configuration {
    fn drop(&mut self) {
        self.credential.zeroize();
    }
}
impl Configuration {
    /// Reject incomplete/unbounded provisioning before enabling a listener.
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 65_536 {
            return Err("configuration limit");
        }
        let config: Self = serde_json::from_slice(bytes).map_err(|_| "configuration schema")?;
        let identity = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        };
        if [
            &config.installation,
            &config.tenant,
            &config.origin,
            &config.revision,
        ]
        .iter()
        .any(|s| !identity(s))
            || !(1..=30_000).contains(&config.timeout_ms)
            || !(32..=256).contains(&config.credential.len())
            || !config
                .credential
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !config.endpoint.starts_with('/')
            || config.endpoint.starts_with("//")
            || config.endpoint.len() > 100
            || config.endpoint.as_bytes().contains(&0)
            || !matches!(config.server, Peer::Unix { .. })
            || config.authority.is_empty()
            || config.authority.len() > 1024
            || !config
                .authority
                .bytes()
                .all(|b| (33..=126).contains(&b) && !b"@/#?\\".contains(&b))
            || config.routes.is_empty()
            || config.routes.len() > 1024
        {
            return Err("configuration binding");
        }
        let mut seen = HashSet::new();
        for route in &config.routes {
            if route.method.is_empty()
                || route.method.len() > 32
                || !route
                    .method
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b == b'-')
                || route.method == "CONNECT"
                || !route.path.starts_with('/')
                || route.path.starts_with("//")
                || reserved(&route.path)
                || route
                    .path
                    .bytes()
                    .any(|b| b <= 32 || b >= 127 || b"?#%\\".contains(&b))
                || route.path.split('/').any(|s| s == "." || s == "..")
                || !seen.insert((&route.method, &route.path))
            {
                return Err("route inventory");
            }
        }
        Ok(config)
    }
}

/// Internal security namespace never belongs to an application route.
pub fn reserved(path: &str) -> bool {
    path == "/_humanymous" || path.starts_with("/_humanymous/")
}

/// Headers that cannot assert client provenance or internal authority.
pub fn strip_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("x-humanymous-")
        || name.starts_with("x-forwarded-")
        || [
            "forwarded",
            "cf-connecting-ip",
            "true-client-ip",
            "x-real-ip",
        ]
        .contains(&name.as_str())
}

/// Immutable owned snapshot copied on the nginx thread, never native pointers.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Ingress {
    /// Uppercase received method.
    pub method: String,
    /// Untouched origin-form target, including query.
    pub target: String,
    /// Current nginx location path; used to reject rewrite drift.
    pub path: String,
    /// Ordered header pairs after spoofed-header removal.
    pub headers: Vec<[String; 2]>,
    /// Complete transfer-decoded/content-encoded bytes.
    pub body: Vec<u8>,
}
impl Ingress {
    /// Produce a protocol envelope only after shared schema/body validation.
    /// Caller injects a fresh server nonce and the original total deadline.
    pub fn envelope(
        &self,
        config: &Configuration,
        id: &str,
        deadline: u64,
        now: u64,
    ) -> Result<Vec<u8>, &'static str> {
        if self.body.len() > BODY_LIMIT
            || reserved(&self.path)
            || self.target.split('?').next() != Some(self.path.as_str())
            || !config
                .routes
                .iter()
                .any(|r| r.method == self.method && r.path == self.path)
        {
            return Err("route or complete body");
        }
        let mut hosts = 0;
        for [name, value] in &self.headers {
            let lower = name.to_ascii_lowercase();
            if strip_header(name)
                || [
                    "upgrade",
                    "trailer",
                    "transfer-encoding",
                    "te",
                    "expect",
                    "proxy-connection",
                ]
                .contains(&lower.as_str())
                || (lower == "connection"
                    && !["close", "keep-alive"].contains(&value.to_ascii_lowercase().as_str()))
                || (lower == "content-type"
                    && value.to_ascii_lowercase().starts_with("application/grpc"))
            {
                return Err("unsupported ingress provenance or stream");
            }
            if lower == "host" {
                hosts += 1;
                if value != &config.authority {
                    return Err("authority mismatch");
                }
            }
        }
        if hosts != 1 {
            return Err("one Host required");
        }
        let wire = serde_json::json!({
            "protocol":"humanymous.adapter/0.1",
            "adapter":{"id":"humanymous.nginx","version":"0.1.0-experimental.1","installation":config.installation},
            "connectionId":null,"capture":null,"id":id,"tenant":config.tenant,
            "origin":config.origin,"revision":config.revision,"deadline":deadline,
            "method":self.method,"target":self.target,"authority":config.authority,
            "peer":null,"headers":self.headers,"headerMode":"nginx-raw",
            "body":{"layer":"transfer-decoded-content-encoded","complete":true,
                "length":self.body.len(),"sha256":hex::encode(Sha256::digest(&self.body)),"base64":STANDARD.encode(&self.body)},
            "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":true,"completeBody":true},"stream":null
        });
        let bytes = serde_json::to_vec(&wire).map_err(|_| "envelope serialization")?;
        parse_with_profile(&bytes, now, AdapterProfile::Nginx)?;
        Ok(bytes)
    }
}

/// Generate a lowercase canonical UUIDv4 from OS randomness, never client data.
pub fn nonce() -> Result<String, &'static str> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "random unavailable")?;
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let h = hex::encode(bytes);
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}

/// Read supervisor provisioning from a canonical private no-follow Linux file.
/// The supervisor owns all ancestor mounts and deletes obsolete secret files.
#[cfg(target_os = "linux")]
pub fn read_private_configuration(path: &std::path::Path) -> Result<Configuration, &'static str> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let uid = rustix::process::geteuid().as_raw();
    if !path.is_absolute() || path.canonicalize().map_err(|_| "configuration path")? != path {
        return Err("configuration path");
    }
    let parent = path.parent().ok_or("configuration parent")?;
    humanymous_authority::transport::private_directory(parent, &Peer::Unix { uid })
        .map_err(|_| "configuration parent privacy")?;
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| "configuration open")?;
    let file = std::fs::File::from(fd);
    let meta = file.metadata().map_err(|_| "configuration metadata")?;
    if !meta.is_file()
        || meta.uid() != uid
        || meta.permissions().mode() & 0o777 != 0o600
        || meta.len() > 65536
    {
        return Err("configuration file privacy");
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    file.take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|_| "configuration read")?;
    let config = Configuration::parse(&bytes)?;
    if !matches!(config.server, Peer::Unix { uid: expected } if expected == uid) {
        return Err("configuration server identity");
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration::parse(serde_json::to_string(&serde_json::json!({
            "endpoint":"/run/private/decision.sock","credential":"a".repeat(32),"server":{"kind":"unix","uid":1000},
            "installation":"nginx","tenant":"t","origin":"o","revision":"r","authority":"example.test","timeout_ms":1000,
            "routes":[{"method":"POST","path":"/api"}]
        })).unwrap().as_bytes()).unwrap()
    }
    fn ingress() -> Ingress {
        Ingress {
            method: "POST".into(),
            target: "/api?q=1".into(),
            path: "/api".into(),
            headers: vec![
                ["Host".into(), "example.test".into()],
                ["Content-Length".into(), "3".into()],
                ["X-Dup".into(), "one".into()],
                ["x-dup".into(), "two".into()],
            ],
            body: vec![0, 255, 128],
        }
    }
    fn envelope(r: &Ingress) -> Result<Vec<u8>, &'static str> {
        r.envelope(
            &config(),
            "01234567-89ab-4def-8123-456789abcdef",
            2000,
            1000,
        )
    }
    #[test]
    fn binary_body_and_duplicate_order_roundtrip() {
        let bytes = envelope(&ingress()).unwrap();
        let parsed = parse_with_profile(&bytes, 1000, AdapterProfile::Nginx).unwrap();
        assert_eq!(parsed.body(), [0, 255, 128]);
        assert_eq!(parsed.headers(), ingress().headers);
        assert_eq!(parsed.connection_id(), None);
        assert_eq!(parsed.header_mode(), "nginx-raw");
    }
    #[test]
    fn spoofed_headers_are_explicitly_removed_on_both_legs() {
        for name in [
            "Forwarded",
            "X-Forwarded-For",
            "x-humanymous-allow",
            "CF-Connecting-IP",
            "True-Client-IP",
            "X-Real-IP",
        ] {
            assert!(strip_header(name));
            let mut r = ingress();
            r.headers.push([name.into(), "forged".into()]);
            assert!(envelope(&r).is_err());
        }
        assert!(!strip_header("Cookie"));
    }
    #[test]
    fn mutation_route_and_reserved_namespace_reject() {
        let mut r = ingress();
        r.body.push(0);
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.path = "/other".into();
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.target = "/api/../api".into();
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.path = "/_humanymous/admin".into();
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.headers[0][1] = "evil.test".into();
        assert!(envelope(&r).is_err());
    }
    #[test]
    fn missing_duplicate_host_framing_and_stream_reject() {
        let mut r = ingress();
        r.headers.remove(0);
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.headers.push(r.headers[0].clone());
        assert!(envelope(&r).is_err());
        let mut r = ingress();
        r.headers.push(r.headers[1].clone());
        assert!(envelope(&r).is_err());
        for name in ["Upgrade", "Trailer", "Transfer-Encoding", "Expect", "TE"] {
            let mut r = ingress();
            r.headers.push([name.into(), "x".into()]);
            assert!(envelope(&r).is_err());
        }
        let mut r = ingress();
        r.headers
            .push(["Connection".into(), "keep-alive, x-security".into()]);
        assert!(envelope(&r).is_err());
    }
    #[test]
    fn unsafe_targets_and_expiry_reject_via_shared_parser() {
        for target in [
            "/api?x=%2f",
            "/api?x=%00",
            "/api#x",
            "/api?q= ",
            "/api?x=%GG",
        ] {
            let mut r = ingress();
            r.target = target.into();
            assert!(envelope(&r).is_err());
        }
        assert!(
            ingress()
                .envelope(
                    &config(),
                    "01234567-89ab-4def-8123-456789abcdef",
                    1000,
                    1000
                )
                .is_err()
        );
    }
    #[test]
    fn nonce_is_fresh_and_shared_schema_validates_it() {
        let a = nonce().unwrap();
        let b = nonce().unwrap();
        assert_ne!(a, b);
        assert!(ingress().envelope(&config(), &a, 2000, 1000).is_ok());
    }
    #[test]
    fn invalid_config_is_rejected_before_native_registration() {
        assert!(Configuration::parse(b"{}").is_err());
        let c = config();
        assert_eq!(c.routes.len(), 1);
        assert!(reserved("/_humanymous"));
        assert!(!reserved("/_humanymous-app"));
        let bytes = serde_json::json!({"endpoint":"tcp://localhost","credential":"a".repeat(32),"server":{"kind":"unix","uid":1000},
            "installation":"nginx","tenant":"t","origin":"o","revision":"r","authority":"example.test","timeout_ms":1000,"routes":[]});
        assert!(Configuration::parse(&serde_json::to_vec(&bytes).unwrap()).is_err());
    }
}
