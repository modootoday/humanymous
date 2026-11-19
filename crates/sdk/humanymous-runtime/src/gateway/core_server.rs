//! Legacy Core server composition (port of cmd/server `newApp`, `routes`,
//! `withSecurityHeaders`, `withTrafficLog`, `withObservability`, the page,
//! demo, session, collect, report, CSP, PoW, resource, trace, traffic and
//! healthz handlers, the live RIT and the connection-capture wiring of
//! tlsconn.go at baseline 981d25c; shipping plan B05/B07, SoT-00 §5).
//!
//! [`CoreServer`] composes the already-ported kernels (collector store, RIT,
//! network report, traffic consistency, correlation, rate limiter, PoW,
//! resource gate and watermark, frozen scoring engine) behind Go's route table
//! and middleware order. [`serve`] runs it on an explicit loopback listener,
//! one request per connection. Nothing here changes a score, weight,
//! threshold, hard-rule predicate, verdict band or signal identity.
//!
//! Deliberate differences from Go, each pinned by tests:
//! - Connection evidence (ClientHello, wire header order, H2 abuse) is owned by
//!   the accepted [`Connection`] instead of Go's address-keyed `connRegistry`
//!   (owner decision 2026-11-16); an address can no longer read another
//!   connection's evidence.
//! - X-Forwarded-For / X-Real-IP are trusted only from the configured
//!   `trusted_proxies` allowlist (owner decision C1); Go trusted every
//!   loopback, private and link-local peer.
//! - `/api/explain`, `/api/counters` and `/api/mlcorrect` are not mounted on
//!   the Core plane; they live on the separate admin plane (plan B09). The
//!   SoT-30 playground surface is not mounted here either.
//! - The `/pass` and `/api/pass/*` routes answer 404 unless a [`PassRoutes`]
//!   adapter is mounted; mounting `gateway::pass_flow` through it is an open
//!   follow-up.
//! - `/static/` refuses a name with `\`, `:` or NUL, a Windows device name
//!   (on Windows) or any non-`Normal` path component with 404 on every OS; Go
//!   refused these only on Windows (500) and served them as plain names on
//!   Linux.
//! - Every history kernel is bounded by explicit [`Limits`]; a refused
//!   admission, a poisoned state lock or a scoring failure answers 503
//!   instead of growing without bound.
//! - The client report is decoded by `serde_json` (duplicate keys and
//!   invalid UTF-8 are 400, differently cased keys are ignored), as in the
//!   Gate control plane port.
//! - `/static/` and the page handlers send no `Last-Modified`/`Accept-Ranges`,
//!   honour no conditional or range request, list no directory (404) and type
//!   unknown extensions `application/octet-stream` instead of sniffing.
mod collect;
pub mod http;
pub mod rit;
mod routes;
pub mod serve;
pub mod signals;
#[cfg(test)]
mod tests;

pub use http::{Request, Response};

use crate::gateway::external_input;
use crate::gateway::logging::OperationalLogger;
use crate::gateway::observatory::hub::Hub;
use crate::gateway::session::store::CollectorStore;
use crate::network::correlation;
use crate::network::history::{anomaly, rate};
use crate::network::http::WireOrder;
use crate::network::intel::IntelReader;
use crate::network::proxy::Cidr;
use crate::network::tls::ClientHello;
use crate::network::traffic;
use humanymous_core::integrity::resource::MediaTracker;
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::Timestamp;
use humanymous_core::watermark::frame;
use humanymous_core::watermark::issuance::Fallback;
use humanymous_core::watermark::ledger::{Configuration, Ledger};
use humanymous_core::watermark::payload::WatermarkKey;
use signals::FingerprintTracker;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

/// Collector-store and traffic-log retention (Go `30 * time.Minute`).
pub const SESSION_TTL: Duration = Duration::from_secs(30 * 60);
/// Watermark issuance-ledger retention (Go `24 * time.Hour`).
pub const LEDGER_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Cross-session correlation retention (Go `60 * time.Minute`).
pub const CORRELATION_TTL: Duration = Duration::from_secs(60 * 60);
/// Fingerprint-keyed limiter window (Go `abuse.NewLimiter(10s, 40, 80)`).
pub const RATE_WINDOW: Duration = Duration::from_secs(10);
/// Limiter soft cutpoint.
pub const RATE_SOFT: usize = 40;
/// Limiter hard cutpoint.
pub const RATE_HARD: usize = 80;
/// `/api/collect` body bound (Go `MaxBytesReader(.., 1<<18)`).
pub const COLLECT_BODY: usize = 1 << 18;
/// Interval between expiry sweeps (Go's minute GC ticker).
pub const GC_INTERVAL: Duration = Duration::from_secs(60);
/// Watermark reservation lifetime; a reservation lives for one request.
pub const RESERVATION_TTL: Duration = Duration::from_secs(60);
/// Smallest accepted master key.
pub const MIN_MASTER_KEY: usize = 32;

/// Explicit finite bounds for every retained history (no Go counterpart: the
/// Go maps were unbounded).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Traffic-log bounds.
    pub traffic: traffic::Limits,
    /// Correlation-registry bounds.
    pub correlation: correlation::Limits,
    /// Correlation population (MAD) bounds.
    pub correlation_population: anomaly::Limits,
    /// Rate-limiter bounds.
    pub rate: rate::Limits,
    /// Watermark ledger capacity (records plus reservations).
    pub watermark_capacity: usize,
    /// Watermark codec bounds.
    pub watermark_codec: frame::Limits,
    /// Sessions the fingerprint-churn memory retains.
    pub fingerprint_sessions: usize,
}

/// Core server configuration.
pub struct Config {
    /// Web root holding `index.html`, `demo.html` and the static assets.
    pub web_dir: String,
    /// Server master key (RIT, PoW, watermark, IP pseudonyms).
    pub master_key: Zeroizing<Vec<u8>>,
    /// Whether the live RIT is enforced (Go `-rit`).
    pub rit_enabled: bool,
    /// Operator bearer token; empty disables operator-only views (404).
    pub ops_token: String,
    /// Peers whose X-Forwarded-For / X-Real-IP is trusted.
    pub trusted_proxies: Vec<Cidr>,
    /// Build version reported by `/healthz`.
    pub version: String,
    /// Whether an opaque trailer may mark a format without a transform
    /// (Go's `Apply` always fell back to the binary trailer).
    pub watermark_fallback: Fallback,
    /// History bounds.
    pub limits: Limits,
}

/// Why a [`CoreServer`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Master key shorter than [`MIN_MASTER_KEY`] or not a valid watermark key.
    MasterKey,
    /// Empty web root.
    WebDir,
    /// A history configuration was refused by its kernel.
    Limits,
}

/// Wall clock the server reads; injectable for tests.
pub type Clock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

/// Evidence owned by one accepted connection (replaces Go `connRegistry`).
#[derive(Debug, Clone, Default)]
pub struct Connection {
    hello: Option<ClientHello>,
    wire_order: Option<WireOrder>,
    abuse: Option<&'static str>,
}

impl Connection {
    /// A plaintext connection with the wire order its socket peek observed.
    pub fn plaintext(wire_order: Option<WireOrder>) -> Self {
        Self {
            hello: None,
            wire_order,
            abuse: None,
        }
    }

    /// A TLS connection from the crate's capture owner.
    pub(crate) fn tls(
        hello: Option<ClientHello>,
        wire_order: Option<WireOrder>,
        abuse: Option<&'static str>,
    ) -> Self {
        Self {
            hello,
            wire_order,
            abuse,
        }
    }

    /// Captured ClientHello, when TLS was observed.
    pub fn hello(&self) -> Option<&ClientHello> {
        self.hello.as_ref()
    }
}

/// One request handed to the Pass owner.
pub struct PassRequest<'a> {
    /// Request method.
    pub method: &'a str,
    /// Decoded path (`/pass` or `/api/pass/...`).
    pub path: &'a str,
    /// The parsed request.
    pub request: &'a Request,
    /// Session identity from the `hsid` cookie; empty when absent.
    pub session: &'a str,
    /// Legacy client address (trusted-proxy aware).
    pub client_ip: &'a str,
}

/// Mount point for the SoT-36 Pass routes (`/pass`, `/api/pass/new`, `pow`,
/// `attest`, `solve`, `kpi`), owned outside this module.
pub trait PassRoutes: Send + Sync {
    /// Answer one Pass request.
    fn handle(&self, request: &PassRequest<'_>) -> crate::gateway::resource::Response;
}

pub(crate) struct State {
    store: CollectorStore,
    traffic: traffic::Log,
    traffic_limits: traffic::Limits,
    correlation: correlation::Registry,
    limiter: rate::Limiter,
    fingerprints: FingerprintTracker,
    last_tick: Duration,
    last_gc: Duration,
}

/// Request time read under the state lock: monotone ticks for the history
/// kernels plus the wall timestamp for the store.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Now {
    tick: Duration,
    ts: Timestamp,
}

impl Now {
    fn unix(self) -> i64 {
        i64::try_from(self.tick.as_secs()).unwrap_or(i64::MAX)
    }
}

fn timestamp(tick: Duration) -> Timestamp {
    let offset = time::Duration::try_from(tick).unwrap_or(time::Duration::ZERO);
    Timestamp(
        time::OffsetDateTime::UNIX_EPOCH
            .checked_add(offset)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
    )
}

/// The composed legacy Core application.
pub struct CoreServer {
    config: Config,
    engine: Engine,
    state: Mutex<State>,
    media: MediaTracker,
    ledger: Ledger,
    watermark_key: WatermarkKey,
    clock: Clock,
    started: Duration,
    intel: Option<Arc<dyn IntelReader>>,
    hub: Option<Arc<Hub>>,
    receipts: Option<external_input::Writer>,
    logger: Option<Mutex<OperationalLogger>>,
    pass: Option<Arc<dyn PassRoutes>>,
}

fn wall(clock: &Clock) -> Duration {
    clock().duration_since(UNIX_EPOCH).unwrap_or_default()
}

impl CoreServer {
    /// Build the application (Go `newApp`) with the frozen default engine.
    pub fn new(config: Config, clock: Clock) -> Result<Self, Refusal> {
        if config.master_key.len() < MIN_MASTER_KEY {
            return Err(Refusal::MasterKey);
        }
        if config.web_dir.is_empty() {
            return Err(Refusal::WebDir);
        }
        let watermark_key =
            WatermarkKey::new(config.master_key.to_vec()).map_err(|_| Refusal::MasterKey)?;
        let started = wall(&clock);
        let limits = config.limits;
        let traffic = traffic::Log::new(
            traffic::Config::new(SESSION_TTL, limits.traffic).map_err(|_| Refusal::Limits)?,
        );
        let correlation = correlation::Registry::new(
            correlation::Config::new(
                CORRELATION_TTL,
                limits.correlation,
                limits.correlation_population,
            )
            .map_err(|_| Refusal::Limits)?,
        );
        let limiter = rate::Limiter::new(
            rate::Config::new(RATE_WINDOW, RATE_SOFT, RATE_HARD, limits.rate)
                .map_err(|_| Refusal::Limits)?,
        );
        let ledger = Ledger::new(
            Configuration {
                record_ttl: LEDGER_TTL,
                reservation_ttl: RESERVATION_TTL,
                capacity: limits.watermark_capacity,
            },
            started,
        )
        .map_err(|_| Refusal::Limits)?;
        Ok(Self {
            engine: Engine::default(),
            state: Mutex::new(State {
                store: CollectorStore::new(SESSION_TTL),
                traffic,
                traffic_limits: limits.traffic,
                correlation,
                limiter,
                fingerprints: FingerprintTracker::new(limits.fingerprint_sessions),
                last_tick: started,
                last_gc: started,
            }),
            media: MediaTracker::new(),
            ledger,
            watermark_key,
            clock,
            started,
            intel: None,
            hub: None,
            receipts: None,
            logger: None,
            pass: None,
            config,
        })
    }

    /// Classify client addresses with an operator intelligence feed; only
    /// its `datacenter` class feeds scoring (IP reputation SoT).
    pub fn with_intel(mut self, intel: Arc<dyn IntelReader>) -> Self {
        self.intel = Some(intel);
        self
    }

    /// Publish scored sessions and network residuals to a live feed.
    pub fn with_live_feed(mut self, hub: Arc<Hub>) -> Self {
        self.hub = Some(hub);
        self
    }

    /// Write external-input evidence receipts for labelled collects.
    pub fn with_external_input_receipts(mut self, writer: external_input::Writer) -> Self {
        self.receipts = Some(writer);
        self
    }

    /// Record request completion and recover handler panics (Go
    /// `withObservability`).
    pub fn with_operational_logger(mut self, logger: OperationalLogger) -> Self {
        self.logger = Some(Mutex::new(logger));
        self
    }

    /// Mount the Pass routes.
    pub fn with_pass_routes(mut self, pass: Arc<dyn PassRoutes>) -> Self {
        self.pass = Some(pass);
        self
    }

    /// Current wall time as a duration since the Unix epoch.
    fn wall(&self) -> Duration {
        wall(&self.clock)
    }

    /// Lock the shared state and read a monotone request time; sweep expired
    /// entries at most once per [`GC_INTERVAL`].
    pub(crate) fn lock(&self) -> Option<(MutexGuard<'_, State>, Now)> {
        let mut state = self.state.lock().ok()?;
        let tick = self.wall().max(state.last_tick);
        state.last_tick = tick;
        if tick.saturating_sub(state.last_gc) >= GC_INTERVAL {
            state.last_gc = tick;
            let ts = timestamp(tick);
            state.store.gc(ts);
            state.fingerprints.gc(tick, SESSION_TTL);
            let swept = state.traffic.gc(tick).is_ok()
                && state.correlation.gc(tick).is_ok()
                && state.limiter.gc(tick).is_ok()
                && self.ledger.gc(tick).is_ok();
            if !swept {
                return None;
            }
        }
        Some((
            state,
            Now {
                tick,
                ts: timestamp(tick),
            },
        ))
    }
}
