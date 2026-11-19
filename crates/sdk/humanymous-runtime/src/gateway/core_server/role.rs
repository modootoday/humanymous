//! Core binary role (legacy `cmd/server` main.go and acme.go at baseline
//! 981d25c; owner decision B01 #1). `humanymous-runtime core` reads one
//! private [`CoreRole`] document from its inherited stdin, prepares the
//! optional TLS source with the B02 `tls::listener` (ACME included), mounts
//! the Pass flow through [`super::pass::PassMount`], binds the listener,
//! writes one readiness line and serves [`super::serve::serve`].
//!
//! Kept from Go:
//! - A fresh 32-byte master key per process (Go `rand.Read(masterKey)`).
//! - `rit` defaults on (Go `-rit=true`); the connection cap defaults to 1024
//!   (Go `-max-conns`/`HMN_MAX_CONNS`).
//! - The step-up token key is the shared Gate key as hex (Go `HMN_TOKEN_KEY`);
//!   absent, the Core issues no step-up receipt.
//! - The certificate sources are Go's `-tls-cert`/`-tls-key` and `-acme-*`,
//!   as the Gate `tls` object; the readiness line carries `tls_mode`.
//!
//! Deliberate differences, each pinned by tests:
//! - Loopback only by default. A non-loopback listen needs `public_listen`
//!   and TLS from a provisioned pair or ACME (Go listened on `:8443` with a
//!   self-signed fallback; there is no public plaintext or self-signed Core).
//! - Settings come from the private document, not flags or environment; a
//!   connection cap of zero (Go: disabled) is refused.
//! - The Core serves HTTP/1.1 only, so `h2` is removed from the ALPN offer
//!   and readiness reports `original_h2: false`.
//! - The step-up key needs [`crate::gateway::token::MIN_KEY`] (32) bytes, the
//!   existing `TokenKey` floor; Go accepted 16.
//! - Every history kernel needs explicit [`CoreLimits`]; none has a
//!   production default.
//! - Not ported here: the ML bundle, Redis correlation, playground, lab
//!   receipt directory and operational logging flags.
use super::{Clock, Config, CoreServer, Limits};
use crate::gateway::configuration::exposure;
use crate::gateway::connection_cap::ConnectionCap;
use crate::gateway::tls::listener::{ListenerTls, PreparedTls};
use crate::network::correlation;
use crate::network::history::{anomaly, rate};
use crate::network::proxy::Cidr;
use crate::network::traffic;
use humanymous_core::watermark::frame;
use humanymous_core::watermark::issuance::Fallback;
use rustls::ServerConfig;
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use zeroize::Zeroizing;

/// Go `-max-conns` default.
pub const DEFAULT_MAX_CONNS: usize = 1024;
/// Whole-connection budget default: Go's 30 s read and write timeouts.
pub const DEFAULT_BUDGET_MS: u64 = 30_000;

/// Private Core role document. Not Debug: it carries the step-up key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreRole {
    /// Listen address (Go `-addr`); loopback unless `public_listen`.
    pub listen: SocketAddr,
    /// Explicit public opt-in; needs TLS from a pair or ACME.
    #[serde(default)]
    pub public_listen: bool,
    /// Optional TLS listener and its one certificate source.
    #[serde(default)]
    pub tls: Option<ListenerTls>,
    /// Web root (Go `-web`).
    pub web_dir: String,
    /// Live RIT enforcement (Go `-rit`, default on).
    #[serde(default = "default_rit")]
    pub rit: bool,
    /// Concurrent connection cap (Go `-max-conns`, default 1024).
    #[serde(default = "default_max_conns")]
    pub max_conns: usize,
    /// Whole-connection I/O budget in milliseconds (100..=300 000).
    #[serde(default = "default_budget")]
    pub budget_ms: u64,
    /// Peers whose forwarding headers are trusted (owner decision C1).
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    /// Shared Gate step-up key as hex (Go `HMN_TOKEN_KEY`).
    #[serde(default)]
    pub token_key_hex: Option<Zeroizing<String>>,
    /// Explicit history bounds.
    pub limits: CoreLimits,
}

fn default_rit() -> bool {
    true
}
fn default_max_conns() -> usize {
    DEFAULT_MAX_CONNS
}
fn default_budget() -> u64 {
    DEFAULT_BUDGET_MS
}

/// Explicit bounds for every retained Core history.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreLimits {
    /// Traffic log: sessions, addresses, field bytes, payload bytes.
    pub traffic: [usize; 4],
    /// Correlation registry: keys, fan-out, distinct, field bytes, payload bytes.
    pub correlation: [usize; 5],
    /// Correlation population: keys, window, key bytes, payload bytes.
    pub correlation_population: [usize; 4],
    /// Rate limiter: keys, hits per key, key bytes, payload bytes, GC scan budget.
    pub rate: [usize; 5],
    /// Watermark ledger capacity.
    pub watermark_capacity: usize,
    /// Watermark codec: input bytes, output bytes, candidates.
    pub watermark_codec: [usize; 3],
    /// Fingerprint-churn sessions.
    pub fingerprint_sessions: usize,
}

impl CoreLimits {
    fn limits(&self) -> Result<Limits, &'static str> {
        let refused = |_| "core limits invalid";
        let [sessions, addresses, field, payload] = self.traffic;
        let [keys, fanout, distinct, c_field, c_payload] = self.correlation;
        let [p_keys, window, key_bytes, p_payload] = self.correlation_population;
        let [r_keys, hits, r_key_bytes, r_payload, scan] = self.rate;
        let [input, output, candidates] = self.watermark_codec;
        Ok(Limits {
            traffic: traffic::Limits::new(sessions, addresses, field, payload).map_err(refused)?,
            correlation: correlation::Limits::new(keys, fanout, distinct, c_field, c_payload)
                .map_err(refused)?,
            correlation_population: anomaly::Limits::new(p_keys, window, key_bytes, p_payload)
                .map_err(refused)?,
            rate: rate::Limits::new(
                r_keys,
                hits,
                r_key_bytes,
                super::RATE_WINDOW,
                r_payload,
                scan,
            )
            .map_err(refused)?,
            watermark_capacity: self.watermark_capacity,
            watermark_codec: frame::Limits::new(input, output, candidates)
                .map_err(|_| "core limits invalid")?,
            fingerprint_sessions: self.fingerprint_sessions,
        })
    }
}

impl CoreRole {
    /// Fail closed before any TLS source, key or listener is touched.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !exposure::listen_permitted(self.listen, self.public_listen, self.tls.as_ref()) {
            return Err("core listen refused: loopback only unless public_listen with TLS");
        }
        if self.web_dir.is_empty()
            || self.max_conns == 0
            || !(100..=300_000).contains(&self.budget_ms)
            || self.trusted_proxies.len() > 256
            || self.limits.watermark_capacity == 0
            || self.limits.fingerprint_sessions == 0
        {
            return Err("core configuration invalid");
        }
        self.trusted()?;
        self.token_key()?;
        self.limits.limits().map(|_| ())
    }

    fn trusted(&self) -> Result<Vec<Cidr>, &'static str> {
        self.trusted_proxies
            .iter()
            .map(|cidr| Cidr::parse(cidr).map_err(|_| "core trusted proxy invalid"))
            .collect()
    }

    fn token_key(&self) -> Result<Option<crate::gateway::token::TokenKey>, &'static str> {
        self.token_key_hex
            .as_ref()
            .map(|text| {
                let bytes = hex::decode(text.as_bytes()).map_err(|_| "core token key invalid")?;
                crate::gateway::token::TokenKey::new(bytes).map_err(|_| "core token key invalid")
            })
            .transpose()
    }
}

/// Read and validate one private role document (64 KiB budget).
pub fn read_role(input: impl Read) -> io::Result<CoreRole> {
    let mut bytes = Zeroizing::new(Vec::new());
    input.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(io::Error::other("core configuration budget"));
    }
    let role: CoreRole =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("core configuration JSON"))?;
    role.validate().map_err(io::Error::other)?;
    Ok(role)
}

/// The Core TLS offer: the prepared source without `h2`, which the Core does
/// not serve. ACME keeps its `acme-tls/1` challenge protocol.
pub fn core_tls(prepared: &PreparedTls) -> Arc<ServerConfig> {
    let mut config = (*prepared.server_config()).clone();
    config.alpn_protocols.retain(|protocol| protocol != b"h2");
    Arc::new(config)
}

/// A bound Core listener and its server.
pub struct CoreService {
    server: Arc<CoreServer>,
    listener: TcpListener,
    options: super::serve::Options,
    readiness: Value,
}

impl CoreService {
    /// Build the server and bind its listener. `tls` must be the prepared
    /// source of `role.tls`; a TLS request without it is refused.
    pub async fn bind(
        role: &CoreRole,
        tls: Option<Arc<ServerConfig>>,
        clock: Clock,
    ) -> io::Result<Self> {
        role.validate().map_err(io::Error::other)?;
        if role.tls.is_some() != tls.is_some() {
            return Err(io::Error::other("core TLS request without its source"));
        }
        let mut master_key = Zeroizing::new(vec![0u8; 32]);
        getrandom::fill(&mut master_key)
            .map_err(|_| io::Error::other("core master key entropy unavailable"))?;
        let config = Config {
            web_dir: role.web_dir.clone(),
            master_key,
            rit_enabled: role.rit,
            ops_token: String::new(),
            trusted_proxies: role.trusted().map_err(io::Error::other)?,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            watermark_fallback: Fallback::OpaqueTrailer,
            limits: role.limits.limits().map_err(io::Error::other)?,
        };
        let server = CoreServer::new(config, clock)
            .map_err(|_| io::Error::other("core startup refused"))?
            .with_pass_flow(role.token_key().map_err(io::Error::other)?)
            .map_err(|_| io::Error::other("core pass mount refused"))?;
        let listener = TcpListener::bind(role.listen).await?;
        let alpn: Vec<String> = tls.as_ref().map_or_else(Vec::new, |tls| {
            tls.alpn_protocols
                .iter()
                .map(|protocol| String::from_utf8_lossy(protocol).into_owned())
                .collect()
        });
        let profile = match (role.public_listen, tls.is_some()) {
            (true, _) => "experimental-core-public-tls-h1",
            (false, true) => "experimental-core-loopback-tls-h1",
            (false, false) => "experimental-core-loopback-http1",
        };
        let readiness = json!({
            "role": "core",
            "listen": listener.local_addr()?,
            "profile": {
                "profile": profile,
                "public_listen": role.public_listen,
                "raw_tls": tls.is_some(),
                "original_h2": false,
                "alpn": alpn,
                "rit": role.rit,
                "pass": true,
                "step_up_receipts": role.token_key_hex.is_some(),
                "max_conns": role.max_conns,
                "product_qualified": false,
            },
        });
        let options = super::serve::Options {
            tls,
            budget: Duration::from_millis(role.budget_ms),
            cap: ConnectionCap::new(role.max_conns)
                .map_err(|_| io::Error::other("core connection cap refused"))?,
            public: role.public_listen,
        };
        Ok(Self {
            server: Arc::new(server),
            listener,
            options,
            readiness,
        })
    }

    /// The readiness document: no key, token or path.
    pub fn readiness(&self) -> &Value {
        &self.readiness
    }

    /// Serve until `shutdown` resolves.
    pub async fn run(self, shutdown: impl std::future::Future<Output = ()>) -> io::Result<()> {
        super::serve::serve(self.server, self.listener, self.options, shutdown).await
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
