//! Serialized nonce reservation, trusted evidence, frozen engine and audit order.
use crate::audit::{log::Log, record::Record};
use crate::authority::{
    Rejection,
    configuration::{Configuration, Peer, reserved},
    credentials::Credential,
};
use humanymous_core::integrity::adapter::{self, ValidatedRequest};
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Actual IPC identity obtained from the accepted OS stream, never HTTP fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePeer {
    /// Effective UID from Unix peer credentials.
    Unix(u32),
    /// PID returned by GetNamedPipeClientProcessId on a restricted local pipe.
    Windows(u32),
}
/// Trusted evidence composition seam. Never expose this as adapter wire input.
pub trait Evidence: Send {
    /// Build observations for this exact already-validated request.
    fn report(
        &mut self,
        request: &ValidatedRequest,
        raw_headers: bool,
    ) -> Result<SessionReport, Rejection>;
}
/// The executable's only implemented profile: application HTTP observations.
pub struct ApplicationHttp;
impl Evidence for ApplicationHttp {
    fn report(
        &mut self,
        request: &ValidatedRequest,
        raw: bool,
    ) -> Result<SessionReport, Rejection> {
        use crate::network::{
            http::HeaderInfo,
            report::{Observation, build},
        };
        let mut header = HeaderInfo {
            method: request.request_target().0.into(),
            names: request.headers().iter().map(|h| h[0].clone()).collect(),
            order_reliable: raw,
            casing_reliable: raw,
            ..Default::default()
        };
        // Version, TLS, H2 and original client address are deliberately absent.
        // A User-Agent header is not evidence that a collector ran.
        for [name, value] in request.headers() {
            match name.to_ascii_lowercase().as_str() {
                "user-agent" => header.user_agent.clone_from(value),
                "accept-language" => header.accept_language.clone_from(value),
                "accept-encoding" => header.accept_encoding.clone_from(value),
                "cookie" => header.has_cookie = true,
                "referer" => header.has_referer = true,
                "via" => header.via.clone_from(value),
                "proxy-connection" => header.proxy_connection.clone_from(value),
                "x-cache" => header.x_cache.clone_from(value),
                "x-squid-error" => header.x_squid_error.clone_from(value),
                "x-client-ip" => header.x_client_ip.clone_from(value),
                "x-original-forwarded-for" => header.x_original_forwarded_for.clone_from(value),
                "x-bluecoat-via" => header.x_blue_coat_via.clone_from(value),
                _ => {}
            }
        }
        Ok(SessionReport {
            network: build(&Observation {
                header,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
/// Audit must durably complete before any successful bound decision is returned.
pub trait DurableAudit: Send {
    /// Commit a minimized decision record; an error permanently prevents this response.
    fn append(&mut self, record: Record) -> std::io::Result<()>;
}
impl DurableAudit for Log {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        Log::append(self, record).map(|_| ())
    }
}
impl DurableAudit for crate::audit::log::SealedLog {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        crate::audit::log::SealedLog::append(self, record).map(|_| ())
    }
}
/// Exact seven-field wire response, with no arbitrary response mutation hooks.
#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    /// Experimental protocol identifier.
    pub protocol: &'static str,
    /// Exact request nonce.
    pub id: String,
    /// Exact authenticated tenant.
    pub tenant: String,
    /// Exact authenticated origin.
    pub origin: String,
    /// Exact current configuration revision.
    pub revision: String,
    /// UTC deadline upper bounded by the request's deadline.
    pub expires: u64,
    /// Frozen engine's ALLOW/DENY/CHALLENGE verdict.
    pub verdict: String,
}
/// One serialized authority/credential epoch. A mutex at the composition root
/// ensures nonce check-and-reserve cannot race across concurrent requests.
pub struct Authority {
    config: Configuration,
    credential: Credential,
    nonces: HashMap<String, u64>,
    engine: Engine,
    evidence: Box<dyn Evidence>,
    audit: Box<dyn DurableAudit>,
    poisoned: bool,
    last_now: u64,
}
impl Authority {
    /// Start a new random credential epoch around the existing shared engine.
    pub fn new(
        config: Configuration,
        engine: Engine,
        evidence: Box<dyn Evidence>,
        audit: Box<dyn DurableAudit>,
    ) -> Result<Self, Rejection> {
        config.validate()?;
        Ok(Self {
            config,
            credential: Credential::generate()?,
            nonces: HashMap::new(),
            engine,
            evidence,
            audit,
            poisoned: false,
            last_now: 0,
        })
    }
    /// Fresh token for private supervisor delivery only; never log this value.
    pub fn credential(&self) -> &str {
        self.credential.expose_for_provisioning()
    }
    /// Authenticate an OS-derived peer plus process-lifetime bearer.
    pub fn authenticate(&self, peer: NativePeer, authorization: &str) -> Result<(), Rejection> {
        let matches = match (&self.config.peer, peer) {
            (Peer::Unix { uid }, NativePeer::Unix(actual)) => *uid == actual,
            (Peer::Windows { pid, .. }, NativePeer::Windows(actual)) => *pid == actual,
            _ => false,
        };
        if !matches {
            return Err(Rejection::new(403, "ipc_peer"));
        }
        if !self.credential.authenticates(authorization) {
            return Err(Rejection::new(401, "credentials"));
        }
        Ok(())
    }
    /// Authorize exactly one bound request. Clock is injected to check expiry
    /// again after durable work; errors never undo a consumed nonce.
    pub fn decide(
        &mut self,
        peer: NativePeer,
        authorization: &str,
        bytes: &[u8],
        clock: impl Fn() -> Result<u64, Rejection>,
    ) -> Result<Response, Rejection> {
        self.authenticate(peer, authorization)?;
        if self.poisoned {
            return Err(Rejection::new(503, "authority_poisoned"));
        }
        let now = observe_time(&mut self.last_now, &mut self.poisoned, clock())?;
        let request =
            adapter::parse_with_profile(bytes, now, self.config.adapter).map_err(|code| {
                let status = if code.contains("body") || code.contains("capabilit") {
                    422
                } else {
                    400
                };
                Rejection::new(status, "invalid_envelope")
            })?;
        let scope = &self.config.scope;
        let (origin, revision, authority) = request.origin_binding();
        if request.installation() != scope.installation
            || request.tenant() != scope.tenant
            || origin != scope.origin
            || authority != scope.authority
        {
            return Err(Rejection::new(403, "credential_scope"));
        }
        if revision != scope.revision {
            return Err(Rejection::new(409, "configuration_revision"));
        }
        let (method, target) = request.request_target();
        let path = target.split('?').next().unwrap_or("");
        if reserved(path) {
            return Err(Rejection::new(403, "reserved_route"));
        }
        let route = self
            .config
            .routes
            .iter()
            .find(|r| r.method == method && r.path == path)
            .ok_or(Rejection::new(409, "route_inventory"))?;
        let raw = matches!(request.header_mode(), "node-raw" | "nginx-raw");
        let req = &route.required;
        if req.raw_tls
            || req.original_h2
            || req.collector
            || req.rit
            || (req.raw_header_order && !raw)
        {
            return Err(Rejection::new(422, "required_capability"));
        }
        self.nonces.retain(|_, until| *until >= now);
        if self.nonces.contains_key(request.id()) {
            return Err(Rejection::new(409, "nonce_replay"));
        }
        if self.nonces.len() >= self.config.replay_capacity {
            return Err(Rejection::new(429, "replay_capacity"));
        }
        self.nonces.insert(
            request.id().into(),
            request.deadline().saturating_add(self.config.clock_skew_ms),
        );
        let mut report = self.evidence.report(&request, raw)?;
        let score = self
            .engine
            .score(&mut report)
            .map_err(|_| Rejection::new(503, "scoring_dependency"))?;
        if !["ALLOW", "DENY", "CHALLENGE"].contains(&score.verdict.as_str()) {
            return Err(Rejection::new(503, "invalid_verdict"));
        }
        if observe_time(&mut self.last_now, &mut self.poisoned, clock())? >= request.deadline() {
            return Err(Rejection::new(503, "deadline"));
        }
        let record = Record {
            event_type: "adapter.decision".into(),
            tenant_id: scope.tenant.clone(),
            request_id: request.id().into(),
            correlation_id: hex::encode(Sha256::digest(bytes)),
            config_version: scope.revision.clone(),
            verdict: score.verdict.clone(),
            risk_score: score.risk_score as i64,
            enforcement_action: score.verdict.clone(),
            enforcement_mode: "enforce".into(),
            fail_mode: "fail_closed".into(),
            triggered_rules: (!score.hard_rule_fired.is_empty())
                .then(|| vec![score.hard_rule_fired]),
            ..Default::default()
        };
        if self.audit.append(record).is_err() {
            self.poisoned = true;
            return Err(Rejection::new(503, "durable_audit"));
        }
        if observe_time(&mut self.last_now, &mut self.poisoned, clock())? >= request.deadline() {
            return Err(Rejection::new(503, "deadline"));
        }
        Ok(Response {
            protocol: "humanymous.adapter/0.1",
            id: request.id().into(),
            tenant: scope.tenant.clone(),
            origin: scope.origin.clone(),
            revision: scope.revision.clone(),
            expires: request.deadline(),
            verdict: score.verdict,
        })
    }
}

fn observe_time(
    last: &mut u64,
    poisoned: &mut bool,
    result: Result<u64, Rejection>,
) -> Result<u64, Rejection> {
    let now = result?;
    if now < *last || now > 9_007_199_254_740_991 {
        // Evicted expired nonces cannot become valid again after a wall-clock
        // rollback. Recovery requires a fresh process/credential epoch.
        *poisoned = true;
        return Err(Rejection::new(503, "clock_rollback"));
    }
    *last = now;
    Ok(now)
}
