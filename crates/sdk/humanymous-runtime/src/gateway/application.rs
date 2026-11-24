//! One composition owner: session, observed network, frozen engine and audit.
use crate::audit::record::Record;
use crate::authority::{decision::DurableAudit, transport::Admission};
use crate::gateway::{
    Rejection,
    browser::{self, Ack},
    configuration::{ClientAddressSource, Configuration, RitProfile},
    http::{self, Request},
    session::Sessions,
};
use crate::network::{
    http::HeaderInfo,
    intel::IntelReader,
    proxy,
    report::{Observation, build},
};
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use sha2::{Digest, Sha256};
use std::{net::IpAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Instant, timeout_at},
};
mod edge;
mod gates;
mod inject;
mod maintenance;
pub mod pass;
mod settings;
mod sticky;
mod tls;
// L1 consumer adapter only; no serving observation point is accepted yet.
#[cfg(test)]
mod temporal;
pub use edge::{GateResponse, HEALTHZ, READYZ, ReadySwitch};
pub use gates::AGENT_NONCE_TTL;

/// Internal result: ALLOW is a one-use origin dispatch, never a reusable ticket.
pub enum Decision {
    /// Bounded local response; CHALLENGE has no recovery in this profile.
    Local(u16, Vec<u8>),
    /// Edge gate refusal (ban or forged trust header); origin never contacted.
    Gate(GateResponse),
    /// Liveness or readiness probe: status and fixed JSON body.
    Probe(u16, &'static str),
    /// Audited request allowed for this exact request until its deadline.
    Origin {
        /// UTC deadline in milliseconds.
        deadline: u64,
        /// Linkage for a later `upstream.error` record; no secrets or values.
        upstream: Upstream,
    },
}

/// Minimized linkage from the ALLOW decision record to an origin failure
/// record (legacy PLAN-07 R15 correlation): the same opaque request id and
/// commitment, the route class and the decision start time.
pub struct Upstream {
    request_id: String,
    correlation_id: String,
    route_class: String,
    started_ms: u64,
}

/// Origin-side refusal plus the transport error class when one was observed.
pub(in crate::gateway) struct OriginError {
    rejection: Rejection,
    class: Option<&'static str>,
}
impl From<Rejection> for OriginError {
    fn from(rejection: Rejection) -> Self {
        Self {
            rejection,
            class: None,
        }
    }
}
impl OriginError {
    pub(in crate::gateway) fn io(code: &'static str, error: &std::io::Error) -> Self {
        Self {
            rejection: Rejection::new(503, code),
            class: Some(crate::gateway::request::upstream_error_class(error)),
        }
    }
}

/// Serialized bounded-context owner; a single accept loop owns this value.
pub struct Application {
    config: Configuration,
    sessions: Sessions,
    engine: Engine,
    audit: Box<dyn DurableAudit>,
    managed_settings: Option<settings::ManagedSettings>,
    intel: Option<Arc<dyn IntelReader>>,
    /// Original client of the connection being served, recovered from the
    /// trusted edge's PROXY v2 header. Connection-scoped: the accept loop is
    /// serialized and resets it for every accepted connection.
    client: Option<IpAddr>,
    /// Shared ban store the edge ban gate consults, once installed.
    bans: Option<edge::Bans>,
    /// Origin-cloak key from the private configuration, when enabled.
    origin_key: Option<crate::gateway::guard::OriginKey>,
    /// Credential, upgrade and sweep gate capabilities, once installed.
    installed: gates::Installed,
    /// Readiness gate of the `/_humanymous/readyz` probe.
    ready: ReadySwitch,
    /// Period of the composite in-memory sweep (legacy maintenance ticker).
    sweep_interval: Duration,
    /// Signed effective-policy version source (owner decision #8), once
    /// installed; records carry `scope.revision` until then.
    signed_policy: Option<crate::gateway::settings::configsign::SignedPolicy>,
    /// Active settings overlay whose explicit gate modes override the boot
    /// modes on the request path (owner decision #3), once installed.
    gate_overlay:
        Option<Arc<std::sync::RwLock<Option<crate::gateway::settings::overlay::Overlay>>>>,
    /// Gateway Pass API for the step-up proof (owner decision #4), when
    /// configured.
    pass: Option<pass::Mounted>,
}
impl Application {
    /// Compose existing services without changing policy or deriving new tells.
    pub fn new(
        config: Configuration,
        engine: Engine,
        audit: Box<dyn DurableAudit>,
    ) -> Result<Self, Rejection> {
        config.validate()?;
        let origin_key = config.edge.origin_key()?;
        // Owner decision #4: the gateway Pass API, only when configured.
        let pass = config
            .pass
            .as_ref()
            .map(|pass| pass::Mounted::new(Duration::from_millis(pass.step_up_ttl_ms)))
            .transpose()?;
        Ok(Self {
            config,
            sessions: Sessions::new(),
            engine,
            audit,
            managed_settings: None,
            intel: None,
            client: None,
            bans: None,
            origin_key,
            installed: gates::Installed::default(),
            ready: ReadySwitch::default(),
            sweep_interval: maintenance::INTERVAL,
            signed_policy: None,
            gate_overlay: None,
            pass,
        })
    }

    /// Bind the signed effective-policy version (legacy
    /// `Server.configVersion()`, internal/gate/configsign.go) in place of
    /// `scope.revision` as the `config_version` of every decision, collect
    /// and upstream-error record (owner decision #8). The version is read at
    /// record time, so a kill-switch flip or an overlay change in the shared
    /// handle shows in the next record. Policy-neutral: it changes no score,
    /// verdict or enforcement.
    pub fn install_signed_policy(
        &mut self,
        policy: crate::gateway::settings::configsign::SignedPolicy,
    ) {
        self.signed_policy = Some(policy);
    }

    /// Read gate modes from the active settings overlay (owner decision #3,
    /// Go `gateModuleMode`): a mode the overlay names for a gate replaces that
    /// gate's boot mode from the configuration; a gate the overlay leaves out
    /// keeps its boot mode. Read on every request, so the overlay the settings
    /// engine publishes (applied, rolled back, or cleared on expiry by the
    /// admin sweep) governs the next request. Gates read here: `gate.ban` and
    /// `gate.spoof_header`; the other gate kernels are not yet composed.
    pub fn install_gate_overlay(
        &mut self,
        slot: Arc<std::sync::RwLock<Option<crate::gateway::settings::overlay::Overlay>>>,
    ) {
        self.gate_overlay = Some(slot);
    }

    /// The mode of gate `id` now: the active overlay's explicit mode, else
    /// `boot`. A mode spelling the validator would refuse never reaches an
    /// active overlay; if one did it enforces (Go's default).
    pub(in crate::gateway) fn gate_mode(
        &self,
        id: &str,
        boot: crate::gateway::ban::gate::Mode,
    ) -> crate::gateway::ban::gate::Mode {
        use crate::gateway::ban::gate::Mode as Gate;
        use crate::gateway::settings::overlay::Mode as Overlay;
        let Some(slot) = &self.gate_overlay else {
            return boot;
        };
        let active = slot
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match active.as_ref().and_then(|overlay| overlay.gates.get(id)) {
            None => boot,
            Some(Overlay::Off) => Gate::Off,
            Some(Overlay::Monitor) => Gate::Monitor,
            Some(Overlay::Shadow) => Gate::Shadow,
            Some(Overlay::Enforce | Overlay::Other(_)) => Gate::Enforce,
        }
    }

    /// `config_version` of a record written now.
    pub(in crate::gateway) fn config_version(&self) -> String {
        self.signed_policy.as_ref().map_or_else(
            || self.config.scope.revision.clone(),
            crate::gateway::settings::configsign::SignedPolicy::version,
        )
    }

    /// Install the operator IP reputation reader (`sot:root/ip-reputation`).
    /// Only its datacenter bit reaches the frozen `is_datacenter_ip` input.
    pub fn install_intel(&mut self, reader: Arc<dyn IntelReader>) {
        self.intel = Some(reader);
    }

    /// Datacenter bit for the current connection's recovered client address.
    fn datacenter_client(&self, now_ms: u64) -> bool {
        self.intel
            .as_ref()
            .is_some_and(|reader| reader.classify(self.client, now_ms / 1000).datacenter)
    }

    /// Trusted client-address source of this configuration and its header
    /// deadline, if any.
    fn edge_policy(&self) -> Option<(ClientAddressSource, Duration)> {
        self.config.ip_reputation.as_ref().map(|policy| {
            (
                policy.client_address,
                Duration::from_millis(policy.proxy_header_timeout_ms),
            )
        })
    }

    /// Whether served connections carry an attributable client address: the
    /// loopback edge's PROXY v2 contract, or the public listener's socket peer
    /// or trusted-balancer source (plan B26). Gates keyed by client address
    /// (bans, credential fan-out) need one.
    fn attributes_clients(&self) -> bool {
        self.edge_policy().is_some() || self.config.public_listen
    }

    /// Exact readiness manifest; no session secrets or raw client observations.
    pub fn readiness(&self) -> serde_json::Value {
        self.config.readiness()
    }
    /// Live fail-closed latch of this serving epoch; see [`EpochHealth`].
    ///
    /// [`EpochHealth`]: crate::gateway::session::EpochHealth
    pub fn epoch_health(&self) -> crate::gateway::session::EpochHealth {
        self.sessions.health()
    }
    /// Consume one durably committed resource change in this serialized owner.
    /// Frozen scoring is untouched and a poisoned epoch cannot be revived.
    pub fn apply_committed_settings(
        &mut self,
        settings: crate::gateway::settings::CommittedSettings,
    ) -> Result<(), Rejection> {
        settings.snapshot().validate_binding(&self.config)?;
        self.sessions.invalidate_all()?;
        settings.apply(&mut self.config)
    }

    /// Decide synchronously. A returned Origin may be consumed only by the same
    /// application request, after rechecking deadline and connection liveness.
    pub fn decide(
        &mut self,
        req: &Request,
        clock: impl Fn() -> Result<u64, Rejection>,
    ) -> Result<Decision, Rejection> {
        let mut ack = None;
        self.decide_request(req, clock, &mut ack)
    }

    fn decide_request(
        &mut self,
        req: &Request,
        clock: impl Fn() -> Result<u64, Rejection>,
        ack: &mut Option<Ack>,
    ) -> Result<Decision, Rejection> {
        self.decide_observed(req, clock, ack, None)
    }

    fn decide_observed(
        &mut self,
        req: &Request,
        clock: impl Fn() -> Result<u64, Rejection>,
        ack: &mut Option<Ack>,
        tls: Option<&crate::capture::tls::Connection>,
    ) -> Result<Decision, Rejection> {
        let now = clock()?;
        if let Some(probe) = self.probe(req) {
            return probe;
        }
        self.sessions.observe(now)?;
        if req.header("host") != Some(self.config.scope.authority.as_str()) {
            return Err(Rejection::new(403, "authority"));
        }
        let path = req.target.split('?').next().unwrap_or("");
        if path == "/_humanymous/browser/session" {
            if req.method != "POST"
                || req.target != path
                || req.header("content-type") != Some("application/json")
                || req.header("authorization").is_some()
                || req.header("x-humanymous-session").is_some()
                || req.header("x-humanymous-protocol").is_some_and(|value| {
                    value != humanymous_core::integrity::request::browser::PROTOCOL
                })
                || !serde_json::from_slice::<serde_json::Value>(&req.body)
                    .ok()
                    .and_then(|value| value.as_object().map(serde_json::Map::is_empty))
                    .unwrap_or(false)
            {
                return Err(Rejection::new(400, "browser_bootstrap"));
            }
            browser::validate(&self.config, req)?;
            let issued = self.sessions.issue_browser(&self.config, now)?;
            return Ok(Decision::Local(
                200,
                serde_json::to_vec(&issued).map_err(|_| Rejection::new(503, "serialization"))?,
            ));
        }
        let profile = browser::profile(req)?;
        if path == "/_humanymous/session" {
            if req.method != "POST"
                || req.target != path
                || !req.body.is_empty()
                || req.header("authorization").is_some()
                || profile != RitProfile::NativeFull
            {
                return Err(Rejection::new(400, "session_request"));
            }
            let issued = self.sessions.issue(&self.config, now)?;
            return Ok(Decision::Local(
                200,
                serde_json::to_vec(&issued).map_err(|_| Rejection::new(503, "serialization"))?,
            ));
        }
        // Owner decision #4: the gateway Pass API, authenticated by request
        // integrity; the RIT session is the Pass session.
        if let Some(flow_path) = pass::flow_path(path) {
            if !self.pass_mounted() {
                return Err(Rejection::new(403, "reserved_route"));
            }
            let mut authorized = self.sessions.authorize(&self.config, req, now)?;
            *ack = authorized.ack.take();
            if !self.sessions.admit_authorized(&authorized.session) {
                return Err(Rejection::new(429, "session_admission"));
            }
            let sid = authorized.session.clone();
            return self.serve_pass(req, flow_path, &sid, now);
        }
        let collect = path == "/_humanymous/collect" || path == "/_humanymous/browser/collect";
        // Fields after a non-enforcing spoof gate stripped forged trust headers.
        let mut stripped = None;
        // Owner decision #4: the matched route's preset (collect routes enforce).
        let (mut enforcing, mut preset) = (true, "balanced");
        // Ceiling-guard #1: the matched route carries the attestation floor.
        let mut attested = false;
        if collect {
            if req.method != "POST"
                || req.target != path
                || req.header("content-type") != Some("application/json")
                || (path == "/_humanymous/browser/collect")
                    != (profile == RitProfile::BrowserAuthOnly)
            {
                return Err(Rejection::new(400, "collect_request"));
            }
        } else {
            if crate::authority::configuration::reserved(path) {
                return Err(Rejection::new(403, "reserved_route"));
            }
            match self.front_gates(req)? {
                edge::Front::Refuse(refusal) => return Ok(refusal),
                edge::Front::Continue(headers) => stripped = headers,
            }
            let route = self
                .config
                .routes
                .iter()
                .find(|r| r.method == req.method && r.path == path)
                .ok_or(Rejection::new(409, "route_inventory"))?;
            enforcing = route.policy().enforce;
            preset = route.policy().name;
            attested = route.policy().attest_floor;
            if route.rit_profile != profile {
                return Err(Rejection::new(409, "route_integrity_profile"));
            }
            if route.required.raw_tls && tls.and_then(|owner| owner.client_hello()).is_none()
                || route.required.original_h2
                    && !tls.is_some_and(|owner| original_h2_observation(owner, req))
                || route.required.collector && !self.sessions.fresh_collector_provenance(req)
            {
                return Err(Rejection::new(422, "required_capability"));
            }
        }
        let mut authorized = self.sessions.authorize(&self.config, req, now)?;
        *ack = authorized.ack.take();
        let id = &authorized.session;
        if !self.sessions.admit_authorized(id) {
            return Err(Rejection::new(429, "session_admission"));
        }
        let deadline = authorized.deadline;
        let request_id = hex::encode(Sha256::digest(
            format!("{id}:{}", authorized.counter).as_bytes(),
        ));
        let correlation_id = commitment(req);
        if !collect {
            // Legacy ServeHTTP gates between route resolution and the verdict.
            match self.run_gates(
                req,
                id,
                now,
                (&request_id, &correlation_id),
                gates::Matched {
                    enforcing,
                    attest_floor: attested,
                    name: preset,
                },
            )? {
                gates::Gated::Continue => {}
                gates::Gated::Answer(decision) => return Ok(decision),
                gates::Gated::Forward => {
                    self.check_deadline(deadline, clock())?;
                    let route_class = crate::gateway::request::route_class(&req.headers);
                    return Ok(Decision::Origin {
                        deadline,
                        upstream: Upstream {
                            request_id,
                            correlation_id,
                            route_class: route_class.as_str().into(),
                            started_ms: now,
                        },
                    });
                }
            }
        }
        let (verdict, risk, rule) = if collect {
            match authorized.profile {
                RitProfile::NativeFull => {
                    self.sessions
                        .collect(id, &req.body, authorized.counter, now)?
                }
                RitProfile::BrowserAuthOnly => {
                    self.sessions
                        .collect_browser(id, &req.body, authorized.counter, now)?
                }
            }
            ("COLLECTED".to_owned(), 0.0, String::new())
        } else {
            // Legacy spoofHeaderGate strips forged trust headers from the
            // request before scoring (stripInbound); integrity verification
            // above covered the bytes the client sent.
            let scored;
            let req = match stripped {
                Some(headers) => {
                    scored = Request {
                        method: req.method.clone(),
                        target: req.target.clone(),
                        headers,
                        body: Vec::new(),
                    };
                    &scored
                }
                None => req,
            };
            let mut header = HeaderInfo {
                method: req.method.clone(),
                version: "11".into(),
                names: req.headers.iter().map(|(n, _)| n.clone()).collect(),
                order_reliable: true,
                casing_reliable: true,
                user_agent: req.header("user-agent").unwrap_or("").into(),
                accept_language: req.header("accept-language").unwrap_or("").into(),
                accept_encoding: req.header("accept-encoding").unwrap_or("").into(),
                has_cookie: req.header("cookie").is_some(),
                has_referer: req.header("referer").is_some(),
                ..Default::default()
            };
            // Forwarding claims remain observed header claims, not transport identity.
            header.via = req.header("via").unwrap_or("").into();
            header.x_cache = req.header("x-cache").unwrap_or("").into();
            header.x_squid_error = req.header("x-squid-error").unwrap_or("").into();
            header.x_blue_coat_via = req.header("x-bluecoat-via").unwrap_or("").into();
            header.x_forwarded_for = req.header("x-forwarded-for").unwrap_or("").into();
            header.forwarded = req.header("forwarded").unwrap_or("").into();
            header.cf_connecting_ip = req.header("cf-connecting-ip").unwrap_or("").into();
            header.true_client_ip = req.header("true-client-ip").unwrap_or("").into();
            header.x_client_ip = req.header("x-client-ip").unwrap_or("").into();
            header.x_original_forwarded_for =
                req.header("x-original-forwarded-for").unwrap_or("").into();
            header.cookie_names = req
                .header("cookie")
                .unwrap_or("")
                .split(';')
                .filter_map(|pair| pair.trim().split_once('=').map(|(name, _)| name.to_owned()))
                .collect();
            let is_datacenter_ip = self.datacenter_client(now);
            let mut report = SessionReport {
                client: self.sessions.report(id),
                network: build(&Observation {
                    header,
                    hello: tls.and_then(|owner| owner.client_hello()).cloned(),
                    is_datacenter_ip,
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Some(tls) = tls {
                // The shared report's offered ALPN default is not negotiation.
                report.network.negotiated_alpn = match tls.negotiated_alpn() {
                    Some(b"http/1.1") => "http/1.1".into(),
                    Some(b"h2") => "h2".into(),
                    None => String::new(),
                    _ => return Err(Rejection::new(422, "tls_application_protocol")),
                };
            }
            report
                .network
                .signals
                .get_or_insert_default()
                .push(authorized.rit);
            let score = self
                .engine
                .score(&mut report)
                .map_err(|_| Rejection::new(503, "scoring"))?;
            if !matches!(score.verdict.as_str(), "ALLOW" | "DENY" | "CHALLENGE") {
                return Err(Rejection::new(503, "verdict"));
            }
            (score.verdict, score.risk_score, score.hard_rule_fired)
        };
        self.check_deadline(deadline, clock())?;
        let route_class: String = crate::gateway::request::route_class(&req.headers)
            .as_str()
            .into();
        let upstream = Upstream {
            request_id: request_id.clone(),
            correlation_id: correlation_id.clone(),
            route_class: route_class.clone(),
            started_ms: now,
        };
        // Ceiling-guard #1 (legacy applyVerdict): on an enforcing attested
        // route a scored ALLOW without a step-up proof is held for step-up
        // (enforce) or recorded as `would_challenge` (monitor, shadow). The
        // verdict, score and rules are recorded unchanged.
        let held = verdict == "ALLOW"
            && !collect
            && enforcing
            && attested
            && self.floor_holds(req, id, &route_class)?;
        let record = Record {
            event_type: if collect {
                "runtime.collect"
            } else {
                "runtime.decision"
            }
            .into(),
            tenant_id: self.config.scope.tenant.clone(),
            request_id,
            correlation_id,
            route_class,
            config_version: self.config_version(),
            verdict: verdict.clone(),
            risk_score: risk as i64,
            enforcement_action: if collect {
                "collect"
            } else if held {
                "step_up_challenge"
            } else {
                &verdict
            }
            .into(),
            // Legacy applyVerdict: a non-enforcing preset (monitor, off)
            // records mode `monitor` and passes through.
            enforcement_mode: if enforcing { "enforce" } else { "monitor" }.into(),
            fail_mode: "fail_closed".into(),
            triggered_rules: (!rule.is_empty()).then(|| vec![rule]),
            ..Default::default()
        };
        if self.audit.append(record).is_err() {
            self.sessions.poison();
            return Err(Rejection::new(503, "durable_audit"));
        }
        self.check_deadline(deadline, clock())?;
        // Owner decision #4 (option 1): a scored ALLOW on an enforcing route
        // is kept on the server for the verdict fast path, when installed.
        if verdict == "ALLOW" && !collect && enforcing {
            self.remember_allow(req, id);
        }
        if held {
            // Go served the step-up interstitial with 401; the client runs
            // the gateway Pass (`/_humanymous/pass/*`) and retries.
            Ok(Decision::Local(
                401,
                serde_json::to_vec(&serde_json::json!({"verdict":verdict,"step_up":"required"}))
                    .map_err(|_| Rejection::new(503, "serialization"))?,
            ))
        } else if verdict == "ALLOW" || !collect && !enforcing {
            Ok(Decision::Origin { deadline, upstream })
        } else {
            Ok(Decision::Local(
                if collect { 200 } else { 403 },
                serde_json::to_vec(&serde_json::json!({"verdict":verdict}))
                    .map_err(|_| Rejection::new(503, "serialization"))?,
            ))
        }
    }

    /// Legacy `handleUpstreamError` (PLAN-07 R16): an origin refusal before
    /// any client byte appends one `upstream.error` record linked to the ALLOW
    /// record, then the caller answers with the unchanged refusal. Only static
    /// classes and codes are recorded, never transport error text. A failed
    /// append poisons the epoch and answers `durable_audit` (fail closed).
    pub(in crate::gateway) fn upstream_failure(
        &mut self,
        upstream: &Upstream,
        error: OriginError,
        response_started: bool,
    ) -> Rejection {
        let OriginError { rejection, class } = error;
        if response_started || !rejection.code.starts_with("origin_") {
            return rejection;
        }
        let class = class.unwrap_or(crate::gateway::request::origin_error_class(rejection.code));
        let latency_us = crate::authority::now_ms().map_or(0, |now| {
            crate::gateway::request::latency_us(
                Some(Duration::from_millis(upstream.started_ms)),
                Duration::from_millis(now),
            )
        });
        let record = Record {
            event_type: "upstream.error".into(),
            actor: crate::audit::record::Actor {
                kind: "system".into(),
                ..Default::default()
            },
            tenant_id: self.config.scope.tenant.clone(),
            request_id: upstream.request_id.clone(),
            correlation_id: upstream.correlation_id.clone(),
            host: self.config.scope.authority.clone(),
            route_class: upstream.route_class.clone(),
            latency_us,
            config_version: self.config_version(),
            enforcement_mode: "enforce".into(),
            fail_mode: "degraded".into(),
            fail_reason: format!("upstream {class}: {}", rejection.code),
            upstream: Some(crate::audit::record::Upstream {
                status: i64::from(rejection.status),
                error_class: class.into(),
            }),
            ..Default::default()
        };
        if self.audit.append(record).is_err() {
            self.sessions.poison();
            return Rejection::new(503, "durable_audit");
        }
        rejection
    }

    fn check_deadline(
        &mut self,
        deadline: u64,
        time: Result<u64, Rejection>,
    ) -> Result<(), Rejection> {
        let now = time?;
        self.sessions.observe(now)?;
        if now >= deadline {
            return Err(Rejection::new(503, "deadline"));
        }
        Ok(())
    }

    /// Serve one request per connection, serialized to bound allocations and
    /// session/audit races. Slow clients can cause denial of service; this is an
    /// experimental loopback pilot, not a production throughput claim.
    pub async fn serve(mut self, listener: TcpListener) -> std::io::Result<()> {
        let (_sender, receiver) = tokio::sync::watch::channel(None);
        self.serve_until(listener, receiver).await
    }

    /// Stop accepting on an observed request, then bound the active request drain.
    pub async fn serve_until(
        &mut self,
        listener: TcpListener,
        mut shutdown: tokio::sync::watch::Receiver<
            Option<crate::gateway::service::ShutdownRequest>,
        >,
    ) -> std::io::Result<()> {
        if !listener.local_addr()?.ip().is_loopback() {
            return Err(std::io::Error::other("loopback only"));
        }
        let mut admission = Admission::new(self.config.rate_per_second);
        let mut commands = self.take_settings_commands();
        let mut admission_rate = self.config.rate_per_second;
        let mut sweep = maintenance::ticker(self.sweep_interval);
        loop {
            if shutdown.borrow().is_some() {
                return Ok(());
            }
            let (mut stream, peer) = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| std::io::Error::other("shutdown controller closed"))?;
                    continue;
                }
                command = settings::next_command(&mut commands) => {
                    self.apply_settings_command(command, &shutdown)?;
                    continue;
                }
                _ = sweep.tick() => {
                    self.maintenance_tick();
                    continue;
                }
                accepted = listener.accept() => accepted?,
            };
            if !peer.ip().is_loopback() {
                continue;
            }
            let until = Instant::now() + Duration::from_millis(self.config.timeout_ms);
            if admission_rate != self.config.rate_per_second {
                admission_rate = self.config.rate_per_second;
                admission = Admission::new(admission_rate);
            }
            if !admission.admit() {
                if write_or_stop(
                    &mut stream,
                    429,
                    b"{\"error\":\"admission\"}",
                    &[],
                    until,
                    &mut shutdown,
                )
                .await?
                {
                    return Ok(());
                }
                continue;
            }
            self.client = None;
            match edge_client(self.edge_policy(), &mut stream, until, &mut shutdown).await? {
                Edge::Client(client) => self.client = client,
                // The trusted edge contract was violated; drop this connection only.
                Edge::Drop => continue,
                // No request was read yet, so there is nothing to drain.
                Edge::Stop => return Ok(()),
            }
            let mut response_started = false;
            let mut ack = None;
            let (result, stopping) = {
                let connection_shutdown = shutdown.clone();
                let pending = timeout_at(
                    until,
                    self.connection(
                        &mut stream,
                        until,
                        &mut response_started,
                        &connection_shutdown,
                        &mut ack,
                    ),
                );
                tokio::pin!(pending);
                tokio::select! {
                    biased;
                    changed = shutdown.changed() => {
                        changed.map_err(|_| std::io::Error::other("shutdown controller closed"))?;
                        let request = *shutdown.borrow();
                        if let Some(request) = request {
                            let result = tokio::time::timeout(request.drain_timeout(), &mut pending)
                                .await.map_err(|_| std::io::Error::other("shutdown drain incomplete"))?;
                            (result, true)
                        } else { (pending.await, false) }
                    }
                    result = &mut pending => (result, false),
                }
            };
            if stopping {
                return match result {
                    Ok(_) => Ok(()),
                    Err(_) => Err(std::io::Error::other(
                        "shutdown request deadline incomplete",
                    )),
                };
            }
            if let Ok(Err(error)) = result {
                if response_started {
                    continue;
                }
                let body = serde_json::to_vec(&serde_json::json!({"error":error.code}))
                    .unwrap_or_default();
                let Ok(extra) = ack
                    .as_ref()
                    .map(|ack| ack.headers(error.status))
                    .transpose()
                else {
                    continue;
                };
                let extra = extra.unwrap_or_default();
                if write_or_stop(
                    &mut stream,
                    error.status,
                    &body,
                    &extra,
                    until,
                    &mut shutdown,
                )
                .await?
                {
                    return Ok(());
                }
            }
        }
    }

    async fn connection(
        &mut self,
        stream: &mut TcpStream,
        connection_until: Instant,
        response_started: &mut bool,
        shutdown: &tokio::sync::watch::Receiver<Option<crate::gateway::service::ShutdownRequest>>,
        ack: &mut Option<Ack>,
    ) -> Result<(), Rejection> {
        let request = http::read(stream).await?;
        connected(stream)?;
        let decision = match self.decide_request(&request, crate::authority::now_ms, ack) {
            Ok(decision) => decision,
            Err(error) => {
                let extra = ack
                    .as_ref()
                    .map(|ack| ack.headers(error.status))
                    .transpose()?
                    .unwrap_or_default();
                let body = serde_json::to_vec(&serde_json::json!({"error":error.code}))
                    .map_err(|_| Rejection::new(503, "serialization"))?;
                *response_started = true;
                return http::write_headers(stream, error.status, &body, &extra)
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"));
            }
        };
        if Instant::now() >= connection_until {
            return Err(Rejection::new(503, "connection_deadline"));
        }
        match decision {
            Decision::Local(status, body) => {
                let body = zeroize::Zeroizing::new(body);
                let extra = ack
                    .as_ref()
                    .map(|ack| ack.headers(status))
                    .transpose()?
                    .unwrap_or_default();
                *response_started = true;
                http::write_headers(stream, status, &body, &extra)
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))
            }
            Decision::Gate(gate) => {
                let fields = gate.with_ack(ack.as_ref())?;
                *response_started = true;
                let head = request.method == "HEAD";
                http::write_text(stream, gate.status, gate.body, &fields, head)
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))
            }
            Decision::Probe(status, body) => {
                *response_started = true;
                let frame = http::probe_frame(status, body, request.method == "HEAD");
                stream
                    .write_all(&frame)
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))?;
                stream
                    .shutdown()
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))
            }
            Decision::Origin { deadline, upstream } => {
                if shutdown.borrow().is_some() {
                    return Err(Rejection::new(503, "service_stopping"));
                }
                connected(stream)?;
                self.check_deadline(deadline, crate::authority::now_ms())?;
                let remaining = deadline
                    .checked_sub(crate::authority::now_ms()?)
                    .filter(|v| *v > 0)
                    .ok_or(Rejection::new(503, "deadline"))?;
                let until = connection_until.min(Instant::now() + Duration::from_millis(remaining));
                let origin = self.config.origin;
                let streaming = self.config.routes.iter().any(|route| {
                    route.method == request.method
                        && route.path == request.target.split('?').next().unwrap_or("")
                        && route.response_profile
                            == crate::gateway::configuration::ResponseProfile::NativeStream
                });
                let bytes = self.origin_bytes(&request, self.origin_auth()?.as_deref());
                let outcome = timeout_at(until, async {
                    let mut origin_stream = TcpStream::connect(origin)
                        .await
                        .map_err(|error| OriginError::io("origin_connect", &error))?;
                    // Cancellation/expiry before the first origin byte has no effect.
                    if shutdown.borrow().is_some() {
                        return Err(Rejection::new(503, "service_stopping").into());
                    }
                    connected(stream)?;
                    self.check_deadline(deadline, crate::authority::now_ms())?;
                    if Instant::now() >= connection_until {
                        return Err(Rejection::new(503, "connection_deadline").into());
                    }
                    origin_stream
                        .write_all(&bytes)
                        .await
                        .map_err(|error| OriginError::io("origin_write", &error))?;
                    if streaming {
                        http::stream::forward(
                            &mut origin_stream,
                            stream,
                            request.method == "HEAD",
                            response_started,
                        )
                        .await?;
                        return Ok(None);
                    }
                    let mut response = Vec::new();
                    origin_stream
                        .take((http::MAX_ORIGIN_FRAME + 1) as u64)
                        .read_to_end(&mut response)
                        .await
                        .map_err(|error| OriginError::io("origin_read", &error))?;
                    Ok::<_, OriginError>(
                        http::Response::parse(&response, request.method == "HEAD").map(Some)?,
                    )
                })
                .await
                .unwrap_or_else(|_| Err(Rejection::new(503, "origin_timeout").into()));
                let response = match outcome {
                    Ok(response) => response,
                    Err(error) => {
                        return Err(self.upstream_failure(&upstream, error, *response_started));
                    }
                };
                let Some(mut response) = response else {
                    return Ok(());
                };
                // Owner decision #7: opt-in loader injection, before the
                // add-only security headers (Go `modifyResponse` order aside,
                // both are add-only and independent).
                self.inject_html(&request, &mut response)?;
                // Plaintext listener: HSTS is never added here (owner decision C10).
                response.secure(false);
                let extra = ack
                    .as_ref()
                    .map(|ack| ack.headers(response.status))
                    .transpose()?
                    .unwrap_or_default();
                let response = response.encode(&extra)?;
                *response_started = true;
                stream
                    .write_all(&response)
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))?;
                stream
                    .shutdown()
                    .await
                    .map_err(|_| Rejection::new(503, "response_write"))
            }
        }
    }
}

/// Outcome of waiting for the trusted edge's PROXY v2 header.
#[derive(Debug, PartialEq, Eq)]
enum Edge {
    /// Header read: the original client, or `None` for LOCAL/UNSPEC (and
    /// when no client-address source is configured).
    Client(Option<IpAddr>),
    /// Edge contract violated (no, malformed, truncated or late header):
    /// drop this connection only.
    Drop,
    /// Shutdown was requested while waiting; no request has been read.
    Stop,
}

/// Wait for the edge header without holding up shutdown. The wait ends at the
/// earlier of the request deadline and the configured header deadline.
async fn edge_client(
    policy: Option<(ClientAddressSource, Duration)>,
    stream: &mut TcpStream,
    until: Instant,
    shutdown: &mut tokio::sync::watch::Receiver<Option<crate::gateway::service::ShutdownRequest>>,
) -> std::io::Result<Edge> {
    let Some((source, header_timeout)) = policy else {
        return Ok(Edge::Client(None));
    };
    let deadline = until.min(Instant::now() + header_timeout);
    let header = transport_client(source, stream, deadline);
    tokio::pin!(header);
    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                changed.map_err(|_| std::io::Error::other("shutdown controller closed"))?;
                if shutdown.borrow().is_some() {
                    return Ok(Edge::Stop);
                }
            }
            result = &mut header => return Ok(result.map_or(Edge::Drop, Edge::Client)),
        }
    }
}

/// Recover the original client from the trusted loopback edge. `Err` means
/// the connection must be dropped: the edge contract was violated.
/// LOCAL/UNSPEC gives `None` (unattributed).
async fn transport_client(
    source: ClientAddressSource,
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<Option<IpAddr>, Rejection> {
    match source {
        ClientAddressSource::ProxyV2 => {
            let header = timeout_at(deadline, read_proxy_v2(stream))
                .await
                .map_err(|_| Rejection::new(408, "proxy_protocol"))??;
            Ok(match header {
                proxy::Header::Source(source) => Some(source.ip()),
                proxy::Header::Local | proxy::Header::Unspecified => None,
            })
        }
    }
}

/// Read exactly one PROXY v2 header (and nothing after it) and decode it with
/// the network context's single parser.
async fn read_proxy_v2(stream: &mut TcpStream) -> Result<proxy::Header, Rejection> {
    let refused = |_| Rejection::new(400, "proxy_protocol");
    let mut header = vec![0u8; 16];
    stream.read_exact(&mut header).await.map_err(refused)?;
    if header[..12] != proxy::SIGNATURE {
        return Err(Rejection::new(400, "proxy_protocol"));
    }
    let length = usize::from(u16::from_be_bytes([header[14], header[15]]));
    header.resize(16 + length, 0);
    stream
        .read_exact(&mut header[16..])
        .await
        .map_err(refused)?;
    proxy::read_header(&mut header.as_slice()).map_err(|_| Rejection::new(400, "proxy_protocol"))
}
/// A local fallback can stop immediately. A consumed ACK does not undo an
/// already dispatched origin effect. Partial response cancellation closes it.
async fn write_or_stop(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
    extra: &[(String, String)],
    until: Instant,
    shutdown: &mut tokio::sync::watch::Receiver<Option<crate::gateway::service::ShutdownRequest>>,
) -> std::io::Result<bool> {
    if shutdown.borrow().is_some() {
        return Ok(true);
    }
    let pending = timeout_at(until, http::write_headers(stream, status, body, extra));
    tokio::pin!(pending);
    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                changed.map_err(|_| std::io::Error::other("shutdown controller closed"))?;
                if shutdown.borrow().is_some() { return Ok(true); }
            }
            _ = &mut pending => return Ok(false),
        }
    }
}

/// Commitment over all observed body and header bytes, including unsigned
/// fields, without writing those values or a usable capability to the WAL.
fn commitment(req: &Request) -> String {
    let mut hash = Sha256::new();
    for value in [&req.method, &req.target] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    for (name, value) in &req.headers {
        for field in [name, value] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
    }
    hash.update(&req.body);
    hex::encode(hash.finalize())
}

/// R14 same-owner observation for `required.original_h2` (fail-closed).
fn original_h2_observation(tls: &crate::capture::tls::Connection, req: &Request) -> bool {
    let Some(authority) = req.header("host") else {
        return false;
    };
    let claim = crate::network::ingress::H2Claim {
        stream: 1,
        method: &req.method,
        authority,
        target: &req.target,
        body_bytes: req.body.len() as u64,
    };
    tls.observe_h2(&claim).is_ok()
}

fn connected(stream: &TcpStream) -> Result<(), Rejection> {
    // Bypass readiness caching: a synchronous durable append may have blocked
    // the task before Tokio could poll a newly arrived FIN/RST. The upstream
    // safe socket API performs MSG_PEEK on the actual nonblocking socket.
    let mut byte = [std::mem::MaybeUninit::uninit(); 1];
    match socket2::SockRef::from(stream).peek(&mut byte) {
        Ok(0) => Err(Rejection::new(400, "client_closed")),
        Ok(_) => Err(Rejection::new(400, "trailing_frame")),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(_) => Err(Rejection::new(400, "client_read")),
    }
}
