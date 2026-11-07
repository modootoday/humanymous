//! Volatile bounded sessions and request-bound rotating integrity tokens.
use crate::gateway::{
    Rejection, browser,
    configuration::{Configuration, RitProfile},
    http::Request,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::integrity::request::{self, CanonicalInput, VerifyParams};
use humanymous_core::signals::report::ClientReport;
use serde::Serialize;
use std::collections::HashMap;
use zeroize::Zeroizing;
mod admission;
use admission::Window;

/// Consumed request authorization and the actual shared verifier observation.
pub struct Authorization {
    /// Volatile capability identifying the already authenticated session.
    pub session: String,
    /// Server-side shared RIT verification result, never client-supplied.
    pub rit: humanymous_core::signals::observation::Signal,
    /// Exact signed deadline.
    pub deadline: u64,
    /// Already consumed rotating counter.
    pub counter: u64,
    /// Exact consumed integrity contract, separate from transport provenance.
    pub profile: RitProfile,
    pub(in crate::gateway) ack: Option<browser::Ack>,
}

/// Private provisioning reply; never audit or log its serialized bytes.
#[derive(Serialize)]
pub struct Issued {
    /// Random capability, also the report sessionId.
    pub session: String,
    /// Base64url integrity key for this volatile epoch.
    pub key: String,
    /// Last accepted counter, initially zero.
    pub counter: u64,
    /// Session expiry in UTC milliseconds.
    pub expires: u64,
}

struct Session {
    key: Zeroizing<[u8; 32]>,
    counter: u64,
    expires: u64,
    report: Option<ClientReport>,
    profile: RitProfile,
    created: std::time::Instant,
    admission: Window,
}

/// Application binding layered over the unchanged shared canonical protocol.
pub fn canonical(config: &Configuration, request: &Request, id: &str, deadline: u64) -> String {
    let mut text = String::from("humanymous.runtime/0.1\n");
    for value in [
        &config.scope.installation,
        &config.scope.tenant,
        &config.scope.origin,
        &config.scope.revision,
        &config.scope.authority,
        &request.method,
        &request.target,
        &deadline.to_string(),
    ] {
        text.push_str(&format!("{}:{value}\n", value.len()));
    }
    // Bind every observed header including order/casing and application cookies.
    // Only the signature itself is excluded to avoid a circular definition.
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("x-humanymous-token") {
            continue;
        }
        for field in [name, value] {
            text.push_str(&format!("{}:{field}\n", field.len()));
        }
    }
    text.push('\n');
    text.push_str(&request::canonical(CanonicalInput {
        headers: &request
            .headers
            .iter()
            .map(|(n, v)| (n.to_ascii_lowercase(), v.clone()))
            .collect(),
        cookie_id: id,
        body: &request.body,
    }));
    text
}

/// One serialized application epoch. Callers never evict live sessions.
pub struct Sessions {
    entries: HashMap<String, Session>,
    last_now: u64,
    poisoned: bool,
}

impl Default for Sessions {
    fn default() -> Self {
        Self::new()
    }
}

impl Sessions {
    /// Empty fresh epoch; restart invalidates old capabilities.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            last_now: 0,
            poisoned: false,
        }
    }

    /// Reject rollback without reviving expired/consumed work.
    pub fn observe(&mut self, now: u64) -> Result<(), Rejection> {
        if self.poisoned || now < self.last_now || now > 9_007_199_254_740_991 {
            self.poisoned = true;
            return Err(Rejection::new(503, "session_epoch"));
        }
        self.last_now = now;
        self.entries.retain(|_, s| s.expires > now);
        Ok(())
    }

    /// Random capability plus independent secret; no caller-chosen session IDs.
    pub fn issue(&mut self, config: &Configuration, now: u64) -> Result<Issued, Rejection> {
        self.issue_profile(config, now, RitProfile::NativeFull)
    }

    fn issue_profile(
        &mut self,
        config: &Configuration,
        now: u64,
        profile: RitProfile,
    ) -> Result<Issued, Rejection> {
        self.observe(now)?;
        if self.entries.len() >= config.session_capacity {
            return Err(Rejection::new(429, "session_capacity"));
        }
        let mut id = [0; 32];
        let mut key = Zeroizing::new([0; 32]);
        getrandom::fill(&mut id).map_err(|_| Rejection::new(503, "entropy"))?;
        getrandom::fill(key.as_mut()).map_err(|_| Rejection::new(503, "entropy"))?;
        let id = URL_SAFE_NO_PAD.encode(id);
        if self.entries.contains_key(&id) {
            return Err(Rejection::new(503, "entropy_collision"));
        }
        let issued = Issued {
            session: id.clone(),
            key: URL_SAFE_NO_PAD.encode(key.as_ref()),
            counter: 0,
            expires: now
                .checked_add(config.session_ttl_ms)
                .ok_or(Rejection::new(503, "clock"))?,
        };
        self.entries.insert(
            id,
            Session {
                created: std::time::Instant::now(),
                admission: Window::new(config.session_rate_per_second),
                key,
                counter: 0,
                expires: issued.expires,
                report: None,
                profile,
            },
        );
        Ok(issued)
    }

    /// Provision only the explicitly configured browser contract. Caller first
    /// validates the actual request's Origin, metadata and cookie-free boundary.
    pub(in crate::gateway) fn issue_browser(
        &mut self,
        config: &Configuration,
        now: u64,
    ) -> Result<browser::Bootstrap, Rejection> {
        if config.browser.is_none() {
            return Err(Rejection::new(422, "browser_profile_unavailable"));
        }
        if now
            .checked_add(config.session_ttl_ms)
            .is_none_or(|value| value > request::browser::MAX_SAFE_INTEGER)
        {
            return Err(Rejection::new(503, "browser_expiry"));
        }
        let issued = self.issue_profile(config, now, RitProfile::BrowserAuthOnly)?;
        Ok(browser::Bootstrap {
            protocol: request::browser::PROTOCOL,
            session: Zeroizing::new(issued.session),
            key: Zeroizing::new(issued.key),
            counter: issued.counter,
            expires: issued.expires,
            installation: config.scope.installation.clone(),
            tenant: config.scope.tenant.clone(),
            origin: config.scope.origin.clone(),
            revision: config.scope.revision.clone(),
            authority: config.scope.authority.clone(),
        })
    }

    /// Validate and consume exactly the next token before downstream work.
    pub fn authorize(
        &mut self,
        config: &Configuration,
        req: &Request,
        now: u64,
    ) -> Result<Authorization, Rejection> {
        self.observe(now)?;
        let profile = browser::profile(req)?;
        if profile == RitProfile::BrowserAuthOnly {
            browser::validate(config, req)?;
            if req
                .header("authorization")
                .and_then(|v| v.strip_prefix("Bearer "))
                .and_then(|id| self.entries.get(id))
                .is_some_and(|session| session.profile == RitProfile::NativeFull)
            {
                return Err(Rejection::new(409, "mixed_integrity_profile"));
            }
        }
        let id = match profile {
            RitProfile::NativeFull => req
                .header("authorization")
                .and_then(|s| s.strip_prefix("Bearer ")),
            RitProfile::BrowserAuthOnly => req.header("x-humanymous-session"),
        }
        .filter(|s| s.len() == 43)
        .ok_or(Rejection::new(401, "session"))?;
        let session = self
            .entries
            .get_mut(id)
            .ok_or(Rejection::new(401, "session"))?;
        if session.profile != profile {
            return Err(Rejection::new(409, "integrity_profile"));
        }
        let deadline = req.number("x-humanymous-deadline")?;
        if now >= deadline || deadline > session.expires || deadline - now > config.timeout_ms {
            return Err(Rejection::new(403, "deadline"));
        }
        let counter = req.number("x-humanymous-counter")?;
        let bucket = req.number("x-humanymous-bucket")?;
        if session.counter == u64::MAX {
            return Err(Rejection::new(403, "counter_exhausted"));
        }
        if profile == RitProfile::BrowserAuthOnly && counter != session.counter + 1 {
            return Err(Rejection::new(409, "counter_replay_or_gap"));
        }
        let canonical = match profile {
            RitProfile::NativeFull => canonical(config, req, id, deadline),
            RitProfile::BrowserAuthOnly => {
                request::browser::canonical(&request::browser::Binding {
                    installation: &config.scope.installation,
                    tenant: &config.scope.tenant,
                    origin_identity: &config.scope.origin,
                    revision: &config.scope.revision,
                    authority: &config.scope.authority,
                    session: id,
                    method: &req.method,
                    target: &req.target,
                    counter,
                    bucket,
                    deadline,
                    content_type: req.header("content-type").unwrap_or(""),
                    application_authorization: req.header("authorization").unwrap_or(""),
                    body: &req.body,
                })
                .map_err(|_| Rejection::new(400, "browser_binding"))?
            }
        };
        let signal = request::verify(VerifyParams {
            session_key: session.key.as_ref(),
            last_n: session.counter,
            presented_n: counter,
            presented_tb: bucket,
            current_tb: now / 10_000,
            token: req.header("x-humanymous-token").unwrap_or(""),
            canonical: &canonical,
            first_req: false,
        });
        if signal.id != "l5.rit.ok" {
            return Err(Rejection::new(403, "request_integrity"));
        }
        session.counter = counter;
        let ack = (profile == RitProfile::BrowserAuthOnly)
            .then(|| browser::Ack::new(&session.key, req, counter, bucket));
        Ok(Authorization {
            session: id.into(),
            rit: signal,
            deadline,
            counter,
            profile,
            ack,
        })
    }

    /// Store only a session-bound client report; this does not prove browser collection.
    pub fn collect(&mut self, id: &str, body: &[u8]) -> Result<(), Rejection> {
        let report: ClientReport =
            serde_json::from_slice(body).map_err(|_| Rejection::new(400, "client_report"))?;
        if report.session_id != id {
            return Err(Rejection::new(403, "report_session"));
        }
        self.store_report(id, report)
    }

    /// Only call after successful next-token consumption. A429 is still a
    /// consumed request, so the caller must retain any browser ACK signer.
    pub(in crate::gateway) fn admit_authorized(&mut self, id: &str) -> bool {
        self.entries
            .get_mut(id)
            .is_some_and(|session| session.admission.admit(session.created.elapsed()))
    }

    /// Browser report projection is bound by the authenticated transport session,
    /// never by a caller-selected subject. Native matching remains unchanged.
    pub(in crate::gateway) fn collect_browser(
        &mut self,
        id: &str,
        body: &[u8],
    ) -> Result<(), Rejection> {
        let mut report: ClientReport =
            serde_json::from_slice(body).map_err(|_| Rejection::new(400, "client_report"))?;
        if !report.session_id.is_empty() && report.session_id != id {
            return Err(Rejection::new(403, "report_session"));
        }
        report.session_id = id.to_owned();
        self.store_report(id, report)
    }

    fn store_report(&mut self, id: &str, mut report: ClientReport) -> Result<(), Rejection> {
        // Registry metadata and scores are server-owned even when observations
        // are asserted by a client. No network/server signals cross this seam.
        let mut seen = std::collections::HashSet::new();
        for signal in report.signals.iter_mut().flatten() {
            use humanymous_core::signals::observation::{Layer, Signal, Source, Verdict, lookup};
            let definition = lookup(&signal.id).ok_or(Rejection::new(400, "client_signal"))?;
            if !matches!(
                definition.layer,
                Layer::Static | Layer::Fingerprint | Layer::Integrity | Layer::Behavioral
            ) || !matches!(signal.collected, Source::Js | Source::Wasm)
                || !matches!(
                    signal.verdict,
                    Verdict::Ok | Verdict::Suspicious | Verdict::Bot | Verdict::Unknown
                )
                || !signal.confidence.is_finite()
                || !(0.0..=1.0).contains(&signal.confidence)
                || !seen.insert(signal.id.clone())
            {
                return Err(Rejection::new(400, "client_signal"));
            }
            *signal = Signal::new(
                &signal.id,
                signal.value.clone(),
                signal.verdict.clone(),
                signal.confidence,
                signal.collected.clone(),
                "",
            );
        }
        self.entries
            .get_mut(id)
            .ok_or(Rejection::new(401, "session"))?
            .report = Some(report);
        Ok(())
    }

    /// A client assertion remains separate from actual network/server observations.
    pub fn report(&self, id: &str) -> ClientReport {
        self.entries
            .get(id)
            .and_then(|s| s.report.clone())
            .unwrap_or_default()
    }

    /// Fail closed for the lifetime of this process after an audit failure.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }
    pub(in crate::gateway) fn invalidate_all(&mut self) -> Result<(), Rejection> {
        if self.poisoned {
            return Err(Rejection::new(503, "session_epoch"));
        }
        self.entries.clear();
        Ok(())
    }
}
