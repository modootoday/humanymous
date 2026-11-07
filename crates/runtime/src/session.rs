//! Volatile bounded sessions and request-bound rotating integrity tokens.
use crate::{Rejection, configuration::Configuration, http::Request};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_integrity::request::{self, CanonicalInput, VerifyParams};
use humanymous_signals::report::ClientReport;
use serde::Serialize;
use std::collections::HashMap;
use zeroize::Zeroizing;

/// Consumed request authorization and the actual shared verifier observation.
pub struct Authorization {
    /// Volatile capability identifying the already authenticated session.
    pub session: String,
    /// Server-side shared RIT verification result, never client-supplied.
    pub rit: humanymous_signals::observation::Signal,
    /// Exact signed deadline.
    pub deadline: u64,
    /// Already consumed rotating counter.
    pub counter: u64,
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
                key,
                counter: 0,
                expires: issued.expires,
                report: None,
            },
        );
        Ok(issued)
    }

    /// Validate and consume exactly the next token before downstream work.
    pub fn authorize(
        &mut self,
        config: &Configuration,
        req: &Request,
        now: u64,
    ) -> Result<Authorization, Rejection> {
        self.observe(now)?;
        let id = req
            .header("authorization")
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| s.len() == 43)
            .ok_or(Rejection::new(401, "session"))?;
        let session = self
            .entries
            .get_mut(id)
            .ok_or(Rejection::new(401, "session"))?;
        let deadline = req.number("x-humanymous-deadline")?;
        if now >= deadline || deadline > session.expires || deadline - now > config.timeout_ms {
            return Err(Rejection::new(403, "deadline"));
        }
        let counter = req.number("x-humanymous-counter")?;
        let bucket = req.number("x-humanymous-bucket")?;
        if session.counter == u64::MAX {
            return Err(Rejection::new(403, "counter_exhausted"));
        }
        let canonical = canonical(config, req, id, deadline);
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
        Ok(Authorization {
            session: id.into(),
            rit: signal,
            deadline,
            counter,
        })
    }

    /// Store only a session-bound client report; this does not prove browser collection.
    pub fn collect(&mut self, id: &str, body: &[u8]) -> Result<(), Rejection> {
        let mut report: ClientReport =
            serde_json::from_slice(body).map_err(|_| Rejection::new(400, "client_report"))?;
        if report.session_id != id {
            return Err(Rejection::new(403, "report_session"));
        }
        // Registry metadata and scores are server-owned even when observations
        // are asserted by a client. No network/server signals cross this seam.
        let mut seen = std::collections::HashSet::new();
        for signal in report.signals.iter_mut().flatten() {
            use humanymous_signals::observation::{Layer, Signal, Source, Verdict, lookup};
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
}
