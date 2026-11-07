//! One composition owner: session, observed network, frozen engine and audit.
use crate::{
    Rejection,
    browser::{self, Ack},
    configuration::{Configuration, RitProfile},
    http::{self, Request},
    session::Sessions,
};
use humanymous_audit::record::Record;
use humanymous_authority::{decision::DurableAudit, transport::Admission};
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use humanymous_network::{
    http::HeaderInfo,
    report::{Observation, build},
};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Instant, timeout_at},
};
mod settings;
mod tls;
// L1 consumer adapter only; no serving observation point is accepted yet.
#[cfg(test)]
mod temporal;

/// Internal result: ALLOW is a one-use origin dispatch, never a reusable ticket.
pub enum Decision {
    /// Bounded local response; CHALLENGE has no recovery in this profile.
    Local(u16, Vec<u8>),
    /// Audited request allowed for this exact request until its deadline.
    Origin {
        /// UTC deadline in milliseconds.
        deadline: u64,
    },
}

/// Serialized bounded-context owner; a single accept loop owns this value.
pub struct Application {
    config: Configuration,
    sessions: Sessions,
    engine: Engine,
    audit: Box<dyn DurableAudit>,
    managed_settings: Option<settings::ManagedSettings>,
}
impl Application {
    /// Compose existing services without changing policy or deriving new tells.
    pub fn new(
        config: Configuration,
        engine: Engine,
        audit: Box<dyn DurableAudit>,
    ) -> Result<Self, Rejection> {
        config.validate()?;
        Ok(Self {
            config,
            sessions: Sessions::new(),
            engine,
            audit,
            managed_settings: None,
        })
    }

    /// Exact readiness manifest; no session secrets or raw client observations.
    pub fn readiness(&self) -> serde_json::Value {
        self.config.readiness()
    }
    /// Consume one durably committed resource change in this serialized owner.
    /// Frozen scoring is untouched and a poisoned epoch cannot be revived.
    pub fn apply_committed_settings(
        &mut self,
        settings: crate::settings::CommittedSettings,
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
        tls: Option<&humanymous_capture::tls::Connection>,
    ) -> Result<Decision, Rejection> {
        let now = clock()?;
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
        let collect = path == "/_humanymous/collect" || path == "/_humanymous/browser/collect";
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
            if humanymous_authority::configuration::reserved(path) {
                return Err(Rejection::new(403, "reserved_route"));
            }
            let route = self
                .config
                .routes
                .iter()
                .find(|r| r.method == req.method && r.path == path)
                .ok_or(Rejection::new(409, "route_inventory"))?;
            if route.rit_profile != profile {
                return Err(Rejection::new(409, "route_integrity_profile"));
            }
            if route.required.raw_tls && tls.and_then(|owner| owner.client_hello()).is_none()
                || route.required.original_h2
                || route.required.collector
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
        let (verdict, risk, rule) = if collect {
            match authorized.profile {
                RitProfile::NativeFull => self.sessions.collect(id, &req.body)?,
                RitProfile::BrowserAuthOnly => self.sessions.collect_browser(id, &req.body)?,
            }
            ("COLLECTED".to_owned(), 0.0, String::new())
        } else {
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
            let mut report = SessionReport {
                client: self.sessions.report(id),
                network: build(&Observation {
                    header,
                    hello: tls.and_then(|owner| owner.client_hello()).cloned(),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Some(tls) = tls {
                // The shared report's offered ALPN default is not negotiation.
                report.network.negotiated_alpn = match tls.negotiated_alpn() {
                    Some(b"http/1.1") => "http/1.1".into(),
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
        // Commitment binds all observed body/header bytes, including unsigned fields,
        // without writing those values or a usable capability to the WAL.
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
        let request_id = hex::encode(Sha256::digest(
            format!("{id}:{}", authorized.counter).as_bytes(),
        ));
        let record = Record {
            event_type: if collect {
                "runtime.collect"
            } else {
                "runtime.decision"
            }
            .into(),
            tenant_id: self.config.scope.tenant.clone(),
            request_id,
            correlation_id: hex::encode(hash.finalize()),
            config_version: self.config.scope.revision.clone(),
            verdict: verdict.clone(),
            risk_score: risk as i64,
            enforcement_action: if collect { "collect" } else { &verdict }.into(),
            enforcement_mode: "enforce".into(),
            fail_mode: "fail_closed".into(),
            triggered_rules: (!rule.is_empty()).then(|| vec![rule]),
            ..Default::default()
        };
        if self.audit.append(record).is_err() {
            self.sessions.poison();
            return Err(Rejection::new(503, "durable_audit"));
        }
        self.check_deadline(deadline, clock())?;
        if verdict == "ALLOW" {
            Ok(Decision::Origin { deadline })
        } else {
            Ok(Decision::Local(
                if collect { 200 } else { 403 },
                serde_json::to_vec(&serde_json::json!({"verdict":verdict}))
                    .map_err(|_| Rejection::new(503, "serialization"))?,
            ))
        }
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
        mut shutdown: tokio::sync::watch::Receiver<Option<crate::service::ShutdownRequest>>,
    ) -> std::io::Result<()> {
        if !listener.local_addr()?.ip().is_loopback() {
            return Err(std::io::Error::other("loopback only"));
        }
        let mut admission = Admission::new(self.config.rate_per_second);
        let mut commands = self.take_settings_commands();
        let mut admission_rate = self.config.rate_per_second;
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
        shutdown: &tokio::sync::watch::Receiver<Option<crate::service::ShutdownRequest>>,
        ack: &mut Option<Ack>,
    ) -> Result<(), Rejection> {
        let request = http::read(stream).await?;
        connected(stream)?;
        let decision = match self.decide_request(&request, humanymous_authority::now_ms, ack) {
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
            Decision::Origin { deadline } => {
                if shutdown.borrow().is_some() {
                    return Err(Rejection::new(503, "service_stopping"));
                }
                connected(stream)?;
                self.check_deadline(deadline, humanymous_authority::now_ms())?;
                let remaining = deadline
                    .checked_sub(humanymous_authority::now_ms()?)
                    .filter(|v| *v > 0)
                    .ok_or(Rejection::new(503, "deadline"))?;
                let until = connection_until.min(Instant::now() + Duration::from_millis(remaining));
                let origin = self.config.origin;
                let streaming = self.config.routes.iter().any(|route| {
                    route.method == request.method
                        && route.path == request.target.split('?').next().unwrap_or("")
                        && route.response_profile
                            == crate::configuration::ResponseProfile::NativeStream
                });
                let bytes = http::origin_request(&request, &self.config.scope.authority);
                let response = timeout_at(until, async {
                    let mut upstream = TcpStream::connect(origin)
                        .await
                        .map_err(|_| Rejection::new(503, "origin_connect"))?;
                    // Cancellation/expiry before the first origin byte has no effect.
                    if shutdown.borrow().is_some() {
                        return Err(Rejection::new(503, "service_stopping"));
                    }
                    connected(stream)?;
                    self.check_deadline(deadline, humanymous_authority::now_ms())?;
                    if Instant::now() >= connection_until {
                        return Err(Rejection::new(503, "connection_deadline"));
                    }
                    upstream
                        .write_all(&bytes)
                        .await
                        .map_err(|_| Rejection::new(503, "origin_write"))?;
                    if streaming {
                        http::stream::forward(
                            &mut upstream,
                            stream,
                            request.method == "HEAD",
                            response_started,
                        )
                        .await?;
                        return Ok(None);
                    }
                    let mut response = Vec::new();
                    upstream
                        .take((http::MAX_ORIGIN_FRAME + 1) as u64)
                        .read_to_end(&mut response)
                        .await
                        .map_err(|_| Rejection::new(503, "origin_read"))?;
                    http::Response::parse(&response, request.method == "HEAD").map(Some)
                })
                .await
                .map_err(|_| Rejection::new(503, "origin_timeout"))??;
                let Some(response) = response else {
                    return Ok(());
                };
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

/// A local fallback can stop immediately. A consumed ACK does not undo an
/// already dispatched origin effect. Partial response cancellation closes it.
async fn write_or_stop(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
    extra: &[(String, String)],
    until: Instant,
    shutdown: &mut tokio::sync::watch::Receiver<Option<crate::service::ShutdownRequest>>,
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
