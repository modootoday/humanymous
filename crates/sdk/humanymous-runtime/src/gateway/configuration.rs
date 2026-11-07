//! Trusted installation scope and an exact, capability-aware route inventory.
use crate::authority::configuration::{Peer, Required, Scope};
use crate::gateway::Rejection;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
mod audit;
mod operational;
pub use audit::{Audit, SealedKeys};
pub use operational::OperationalSettings;

/// Private inherited startup document; intentionally not Debug or Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Explicit loopback ingress address. Port zero is available for pilots.
    pub listen: SocketAddr,
    /// Fixed owned loopback origin address, never taken from request headers.
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
        if !self.listen.ip().is_loopback()
            || !self.origin.ip().is_loopback()
            || self.origin.port() == 0
            || self.origin == self.listen
            || self.operational().validate().is_err()
            || self.routes.is_empty()
            || self.routes.len() > 4096
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

    /// Listener readiness is distinct from browser/product qualification.
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
            "operational":self.operational(),
            "original_h2":false,"raw_header_order":true,"rit":true,
            "client_asserted_report_ingestion":true,"browser_collector":false,
            "browser_session":self.browser.is_some(),"browser_cookie_credentials":false,
            "browser_recovery":false,"administration":false,"product_qualified":false,
            "revision":self.scope.revision,"routes":routes})
    }
}
