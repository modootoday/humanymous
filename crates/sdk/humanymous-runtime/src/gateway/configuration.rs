//! Trusted installation scope and an exact, capability-aware route inventory.
use crate::authority::configuration::{Peer, Required, Scope};
use crate::gateway::Rejection;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
pub mod admin;
mod audit;
pub mod credentials;
mod edge;
pub mod exposure;
pub mod inject;
mod ip_reputation;
mod operational;
pub mod recon_sweep;
pub use admin::{AdminListener, ModelSource};
pub use audit::{Audit, MAX_WITNESS_PROOF, SealedKeys, WitnessInput, WitnessInputDurability};
pub use edge::Edge;
pub use ip_reputation::{ClientAddressSource, IpReputation};
pub use operational::OperationalSettings;

/// Private inherited startup document; intentionally not Debug or Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Ingress address. Loopback unless `public_listen` is set (then a
    /// non-loopback address with a TLS listener). Port zero is available for
    /// pilots.
    pub listen: SocketAddr,
    /// Explicit public edge opt-in (owner decision B01 #1): a non-loopback
    /// `listen` served over TLS to non-loopback peers. Absent keeps the
    /// loopback-only listener. See [`exposure`].
    #[serde(default)]
    pub public_listen: bool,
    /// Concurrent receive bounds of the public listener; ignored on loopback.
    #[serde(default)]
    pub public_receive: exposure::PublicReceive,
    /// Transport-pass-through load balancers allowed to send a PROXY v2
    /// header to the public listener (Go `-trusted-proxies`, plan B26): CIDRs
    /// or bare IPs, at most 256, none broader than /8. Only with
    /// `public_listen`; empty keeps every peer's socket address as its client.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    /// Fixed owned loopback origin address, never taken from request headers.
    /// It stays loopback with a public edge: the origin hop is plaintext and
    /// a non-loopback upstream (Go `-upstream` URL) is plan B14.
    pub origin: SocketAddr,
    /// Installation/tenant/authority/policy binding.
    pub scope: Scope,
    /// Complete inventory of protected routes.
    pub routes: Vec<Route>,
    /// Provisioned durable audit identity and recovery head.
    pub audit: Audit,
    /// Local audit directory owner identity; never taken from HTTP.
    pub owner: Peer,
    /// Number of live sessions; no live eviction.
    pub session_capacity: usize,
    /// Fixed session lifetime in milliseconds.
    pub session_ttl_ms: u64,
    /// Fixed per-connection lifetime in milliseconds.
    pub timeout_ms: u64,
    /// Global accepted connections per second.
    pub rate_per_second: u32,
    /// Per authenticated volatile session resource quota; unrelated to scoring.
    #[serde(default = "default_session_rate")]
    pub session_rate_per_second: u32,
    /// Explicit browser contract; absent preserves native-only configuration.
    #[serde(default)]
    pub browser: Option<BrowserPolicy>,
    /// Optional operator IP reputation feed; absent means no classification.
    #[serde(default)]
    pub ip_reputation: Option<IpReputation>,
    /// B02 TLS listener request and its one certificate source; absent keeps
    /// the plaintext loopback listener. A request that selects no source is
    /// refused before custody, never served with a self-signed fallback.
    #[serde(default)]
    pub tls: Option<crate::gateway::tls::listener::ListenerTls>,
    /// Front-of-path edge gates (ban, spoof header), HSTS and origin cloak;
    /// absent keeps the Go defaults: both gates enforce, no HSTS, no cloak.
    #[serde(default)]
    pub edge: Edge,
    /// Gate operational logging (legacy `-log-*` flags); absent writes no
    /// operational records. Validated and opened before custody.
    #[serde(default)]
    pub logging: Option<crate::gateway::logging::InheritedLogging>,
    /// Separate loopback admin TCP listener serving `gateway::admin_plane`
    /// (owner decision B01 #1); absent serves no admin plane.
    #[serde(default)]
    pub admin: Option<AdminListener>,
    /// Credential trust-anchor files (Go `-agent-keys`, `-pat-issuers`);
    /// absent installs no credential gate. See [`credentials`].
    #[serde(default)]
    pub credentials: Option<credentials::Credentials>,
    /// Decision-probing sweep gate (HR-30, owner decision #2); absent
    /// installs none. See [`recon_sweep`].
    #[serde(default)]
    pub recon_sweep: Option<recon_sweep::ReconSweep>,
    /// Opt-in HTML loader injection (owner decision #7); absent changes no
    /// response. See [`inject`].
    #[serde(default)]
    pub inject: Option<inject::Inject>,
}

/// Distinct integrity contracts. A browser subset never claims full wire binding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RitProfile {
    /// Legacy native whole-header contract, unchanged by browser support.
    #[default]
    NativeFull,
    /// Browser-controllable subset with no automatic application cookies.
    BrowserAuthOnly,
}

/// Explicit entity delivery semantics; browser integrity uses complete bodies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseProfile {
    /// Complete bounded entity validated before writing its downstream head.
    #[default]
    Buffered,
    /// Native-only incremental entity with a fixed one MiB and request deadline cap.
    NativeStream,
}

/// Exact same-origin boundary for the initial cookie-free browser contract.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserPolicy {
    /// Exact serialized origin, currently http:// plus configured authority.
    pub origin: String,
}

fn default_session_rate() -> u32 {
    100
}

/// One exact route and its honest evidence prerequisites.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// Exact uppercase method.
    pub method: String,
    /// Exact unescaped path, without query.
    pub path: String,
    /// Missing required planes reject before origin.
    pub required: Required,
    /// Explicit profile opt-in; old route documents remain native-only.
    #[serde(default)]
    pub rit_profile: RitProfile,
    /// Explicit opt-in; no automatic streaming inference from content type.
    #[serde(default)]
    pub response_profile: ResponseProfile,
}

/// Only bounded complete HTTP request methods have an implementation.
pub fn method(value: &str) -> bool {
    matches!(value, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD")
}

/// Strict origin-form target; queries remain exact signed bytes.
pub fn target(value: &str) -> bool {
    let path = value.split('?').next().unwrap_or("");
    value.len() <= 8192
        && value.starts_with('/')
        && !value.starts_with("//")
        && value
            .bytes()
            .all(|b| (33..127).contains(&b) && !b"#\\".contains(&b))
        && !path.contains('%')
        && !path.split('/').any(|s| s == "." || s == "..")
}

impl Configuration {
    /// The parsed `trusted_proxies` allowlist; `None` when empty or refused.
    pub fn trusted_proxy_allowlist(&self) -> Option<crate::network::proxy::Allowlist> {
        if self.trusted_proxies.is_empty() {
            return None;
        }
        crate::network::proxy::Allowlist::parse(&self.trusted_proxies.join(",")).ok()
    }

    /// Nonsecret resource configuration, separate from frozen detection policy.
    pub fn operational(&self) -> OperationalSettings {
        OperationalSettings {
            session_capacity: self.session_capacity,
            session_ttl_ms: self.session_ttl_ms,
            timeout_ms: self.timeout_ms,
            rate_per_second: self.rate_per_second,
            session_rate_per_second: self.session_rate_per_second,
        }
    }
    /// Fail closed before binding a listener or opening an origin connection.
    pub fn validate(&self) -> Result<(), Rejection> {
        let bad = Rejection::new(503, "configuration");
        if !exposure::listen_permitted(self.listen, self.public_listen, self.tls.as_ref())
            // The loopback edge contract trusts a PROXY v2 header from every
            // peer; on a public listener only listed balancers may send one.
            || self.public_listen && self.ip_reputation.is_some() && self.trusted_proxies.is_empty()
            || !self.trusted_proxies.is_empty()
                && (!self.public_listen || self.trusted_proxy_allowlist().is_none())
            || !self.public_receive.valid()
            || self.admin.as_ref().is_some_and(|admin| {
                admin.validate().is_err()
                    || admin.listen.port() != 0
                        && (admin.listen == self.listen || admin.listen == self.origin)
            })
            || !self.origin.ip().is_loopback()
            || self.origin.port() == 0
            || self.origin == self.listen
            || self.operational().validate().is_err()
            || self.routes.is_empty()
            || self.routes.len() > 4096
            || !self.audit.witness_input_valid()
            || self
                .ip_reputation
                .as_ref()
                .is_some_and(|policy| policy.validate().is_err())
            || self.edge.origin_key().is_err()
            || self
                .credentials
                .as_ref()
                .is_some_and(|credentials| credentials.validate().is_err())
            || self
                .recon_sweep
                .as_ref()
                .is_some_and(|sweep| sweep.config().is_none())
            || self
                .inject
                .as_ref()
                .is_some_and(|inject| !inject.valid(&self.routes))
            || self.edge.hsts && self.tls.is_none()
        {
            return Err(bad);
        }
        for value in [
            &self.scope.installation,
            &self.scope.tenant,
            &self.scope.origin,
            &self.scope.revision,
        ] {
            if value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            {
                return Err(bad);
            }
        }
        let authority = &self.scope.authority;
        if authority.is_empty()
            || authority.len() > 1024
            || !authority
                .bytes()
                .all(|b| (33..127).contains(&b) && !b"@/#?\\,".contains(&b))
        {
            return Err(bad);
        }
        let mut seen = std::collections::BTreeSet::new();
        if self
            .browser
            .as_ref()
            .is_some_and(|policy| policy.origin != format!("http://{}", self.scope.authority))
        {
            return Err(bad);
        }
        for route in &self.routes {
            if !method(&route.method)
                || !target(&route.path)
                || route.path.contains('?')
                || crate::authority::configuration::reserved(&route.path)
                || !route.required.rit
                || (route.rit_profile == RitProfile::BrowserAuthOnly && self.browser.is_none())
                || (route.rit_profile == RitProfile::BrowserAuthOnly
                    && route.response_profile == ResponseProfile::NativeStream)
                || !seen.insert((&route.method, &route.path))
            {
                return Err(bad);
            }
        }
        Ok(())
    }

    /// Listener readiness is distinct from browser/product qualification. A
    /// public listener always has TLS, so its profile name comes from the
    /// service's TLS readiness.
    pub fn readiness(&self) -> serde_json::Value {
        let routes: Vec<_> = self
            .routes
            .iter()
            .map(|r| {
                serde_json::json!({
                    "method":r.method,"path":r.path,
                    "ready":!r.required.raw_tls && !r.required.original_h2 && !r.required.collector,
                    "rit_profile":r.rit_profile,
                    "response_profile":r.response_profile
                })
            })
            .collect();
        serde_json::json!({"profile":"experimental-loopback-http1", "raw_tls":false,
            "public_listen":self.public_listen,
            "admin_plane":self.admin.is_some(),
            "operational":self.operational(),
            "original_h2":false,"raw_header_order":true,"rit":true,
            "client_asserted_report_ingestion":true,"browser_collector":false,
            "browser_session":self.browser.is_some(),"browser_cookie_credentials":false,
            "browser_recovery":false,"administration":false,"product_qualified":false,
            "revision":self.scope.revision,"routes":routes,
            "ip_reputation":self.ip_reputation.as_ref().map(IpReputation::readiness),
            "edge":self.edge.readiness()})
    }
}
