//! Trusted installation scope and an exact, capability-aware route inventory.
use crate::Rejection;
use humanymous_authority::configuration::{Audit, Peer, Required, Scope};
use serde::Deserialize;
use std::net::SocketAddr;

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
    /// Fail closed before binding a listener or opening an origin connection.
    pub fn validate(&self) -> Result<(), Rejection> {
        let bad = Rejection::new(503, "configuration");
        if !self.listen.ip().is_loopback()
            || !self.origin.ip().is_loopback()
            || self.origin.port() == 0
            || self.origin == self.listen
            || !(1..=4096).contains(&self.session_capacity)
            || !(1000..=3_600_000).contains(&self.session_ttl_ms)
            || !(1..=30_000).contains(&self.timeout_ms)
            || !(1..=10_000).contains(&self.rate_per_second)
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
        for route in &self.routes {
            if !method(&route.method)
                || !target(&route.path)
                || route.path.contains('?')
                || humanymous_authority::configuration::reserved(&route.path)
                || !route.required.rit
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
                    "ready":!r.required.raw_tls && !r.required.original_h2 && !r.required.collector
                })
            })
            .collect();
        serde_json::json!({"profile":"experimental-loopback-http1", "raw_tls":false,
            "original_h2":false,"raw_header_order":true,"rit":true,
            "client_asserted_report_ingestion":true,"browser_collector":false,
            "browser_recovery":false,"administration":false,"product_qualified":false,
            "revision":self.scope.revision,"routes":routes})
    }
}
