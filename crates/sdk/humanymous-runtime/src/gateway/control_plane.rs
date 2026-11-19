//! Reserved gate control plane (SoT-19 §3): session issuance, the collect
//! scoring beacon, the attestation-floor step-up redemption, the collector
//! loader and the liveness/readiness probes.
//!
//! Port of legacy `internal/gate/control.go` (and, in [`collect`],
//! `control_collect.go`) at baseline 981d25c. The handler is a pure kernel:
//! the composition root parses the request into [`Request`], supplies the
//! clocks in [`Now`], writes the returned [`Response`] and appends its audit
//! [`Response::records`] in order. Scoring, session id randomness and subject
//! pseudonyms come from the caller's [`Backend`]. The L5 network observation
//! is built here as Go built it: the gate header projection ([`headers`],
//! Go `headerInfo`/`protoVer`) fed to `network::report::build` with no TLS,
//! HTTP/2 or TCP capture and no datacenter classification. This module never
//! changes a score, weight, threshold or signal ID (detection freeze).
//!
//! Kept from Go, and pinned against the oracle golden
//! `tests/fixtures/gateway/control-plane-go-golden.json`:
//! - Routes `/session`, `/collect`, `/stepup`, `/loader.js`, `/csp-report`
//!   (`204`), `/healthz` and `/readyz` (`503` while draining, with the body
//!   Go sends after its header snapshot: sniffed `text/plain`). Any other path
//!   is `404 page not found`.
//! - `POST only` (`405`) is checked before the flood guard; `/session`,
//!   `/collect` and `/stepup` are metered by [`control_flood::Guard`] before
//!   any session or scoring work; the probes and the loader are not metered.
//! - `hsid` is reused from the cookie or minted as 32 lowercase hex digits
//!   with `Path=/; HttpOnly; Secure; SameSite=Lax`.
//! - Step-up needs a token key (`501`), a session (`400`) and a valid
//!   session-bound receipt (`403` plus one `token.binding_mismatch` record with
//!   the Go reason text); it then mints `hmn_su` for 30 minutes bound to the
//!   header fingerprint and socket subnet and records `token.issued`.
//! - The collect beacon's network report is `network::report::build` over
//!   the gate header projection of the request's own header multimap,
//!   method and protocol version.
//! - JSON bodies are Go `encoding/json` output: sorted keys, HTML escaping and
//!   a trailing newline; `http.Error` bodies end in a newline and carry
//!   `X-Content-Type-Options: nosniff`.
//!
//! Deliberate differences, each pinned by tests:
//! - `/loader.js` serves the collector bundle the composition root supplies
//!   ([`ControlPlane::with_loader`]) with the Go headers. The Go inline
//!   JavaScript reference collector is not carried: authored foreign code is
//!   not allowed in Rust sources (sot:root/rust-migration) and the collector
//!   is the Rust WASM bundle (sot:root/client-collector). Without a bundle the
//!   path is `404`.
//! - A non-canonical path is `404`, not Go `ServeMux`'s `301` redirect to the
//!   cleaned path; path canonicalization belongs to ingress.
//! - A scoring failure, a wall clock before 1970 or an unusable session id
//!   source answers `503` without a sticky verdict or token (fail-closed,
//!   AGENTS.md hard constraint 2); Go could not fail there.
//! - The Go engine overlay hook (`WithEngineConfigurator`, SoT-39) is not
//!   carried; the backend owns its frozen engine.
use crate::audit::record::events::{EVENT_TOKEN_BINDING_MISMATCH, EVENT_TOKEN_ISSUED};
use crate::audit::record::{Actor, Record};
use crate::gateway::control_flood::{self, Decision, Guard};
use crate::gateway::session::store::CollectorStore;
use crate::gateway::token::{self, Epochs, Nonces, Reason, TokenKey};
use crate::gateway::verdict_store::VerdictStore;
use crate::network::history::Unavailable;
use crate::network::history::cohort::{Cohort, Observer};
use humanymous_core::signals::report::{ScoreResult, SessionReport, Timestamp};
use humanymous_core::signals::wire::quote_json_go;
use std::net::IpAddr;
use std::time::Duration;

mod collect;
mod headers;

/// Session id cookie shared with the edge (Go `sessionCookie`).
pub const SESSION_COOKIE: &str = "hsid";
/// Verdict trust-token cookie (Go `verdictCookie`).
pub const VERDICT_COOKIE: &str = "hmn_vt";
/// Step-up proof cookie (Go `stepUpCookie`).
pub const STEP_UP_COOKIE: &str = "hmn_su";
/// Step-up receipt request header (Go `stepUpReceiptHeader`).
pub const STEP_UP_RECEIPT_HEADER: &str = "X-Hmn-Stepup-Receipt";
/// Single-use beacon nonce request header (HR-29).
pub const NONCE_HEADER: &str = "X-Hmn-Nonce";
/// Largest accepted collect body (Go `MaxBytesReader(1<<18)`); the caller
/// must read at most one byte more so an oversize body is detected.
pub const MAX_BEACON: usize = 1 << 18;
/// Beacon nonce retention (Go `NewNonceCache(10 * time.Minute)`).
pub const NONCE_TTL: Duration = Duration::from_secs(10 * 60);
/// Verdict-token and step-up proof lifetime in seconds.
pub const TOKEN_TTL: u64 = 30 * 60;
/// Epoch minted under when no shared epoch source is configured.
pub const FALLBACK_EPOCH: &str = "e1";
/// Audit key id stamped on control-plane records.
pub const KEY_ID: &str = "k1";

/// One parsed control-plane request; the reserved prefix is already removed.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// HTTP method.
    pub method: &'a str,
    /// HTTP major version (Go `Request.ProtoMajor`): `2` for HTTP/2.
    pub proto_major: u8,
    /// Path below the control-plane prefix, for example `/collect`.
    pub path: &'a str,
    /// First `label` query value, when present.
    pub label: Option<&'a str>,
    /// Socket peer address (never a forwarding header).
    pub peer: IpAddr,
    /// First `hsid` cookie value; empty means none.
    pub session: Option<&'a str>,
    /// Handler-visible request header fields in receive order, as Go
    /// `Request.Header` holds them: any name casing, repeated names kept.
    /// Every header the kernel reads comes from here with Go `Header.Get`
    /// semantics, including the L5 projection ([`headers`]).
    pub headers: &'a [(String, String)],
    /// Request body, read up to [`MAX_BEACON`] plus one byte.
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    /// Go `r.Header.Get(name)`: first value under the canonical name; empty
    /// when absent.
    pub fn header(&self, name: &str) -> &'a str {
        headers::first_value(self.headers, name)
    }
}

/// Caller clocks: wall time for the session store and token expiry, a
/// monotonic reading for the flood guard, nonces and sticky verdicts.
#[derive(Debug, Clone, Copy)]
pub struct Now {
    /// Wall-clock time.
    pub wall: Timestamp,
    /// Monotonic time.
    pub mono: Duration,
}

/// One control-plane answer plus the audit records to append in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// Response headers in write order (`Set-Cookie` may repeat).
    pub headers: Vec<(&'static str, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Audit records produced by this request.
    pub records: Vec<Record>,
    /// Cohort shadow measurement (log-only, never a verdict input).
    pub cohort: Option<Cohort>,
}

impl Response {
    /// First value of header `name` (ASCII case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    /// Every `Set-Cookie` value in order.
    pub fn set_cookies(&self) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(key, _)| *key == "Set-Cookie")
            .map(|(_, value)| value.as_str())
            .collect()
    }
    fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type", "text/plain; charset=utf-8".into()),
                ("X-Content-Type-Options", "nosniff".into()),
            ],
            body: format!("{message}\n").into_bytes(),
            ..Self::default()
        }
    }
    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "application/json".into())],
            body: body.into_bytes(),
            ..Self::default()
        }
    }
    /// Go `http.Error` keeps headers set before it, such as `Set-Cookie`.
    fn with_cookies(mut self, cookies: Vec<String>) -> Self {
        let mut headers: Vec<_> = cookies.into_iter().map(|c| ("Set-Cookie", c)).collect();
        headers.append(&mut self.headers);
        self.headers = headers;
        self
    }
}

/// Caller-owned collaborators: randomness, scoring, pseudonyms.
pub trait Backend {
    /// Sixteen fresh random bytes as lowercase hex. An error fails closed.
    fn new_session_id(&mut self) -> Result<String, Unavailable>;
    /// Score a merged report with the frozen engine; `None` fails closed.
    fn score(&mut self, report: &mut SessionReport) -> Option<ScoreResult>;
    /// Keyed subject pseudonym (Go `Vault.Pseudonymize`); empty without a vault.
    fn pseudonymize(&mut self, sid: &str, value: &str) -> String;
    /// Record the handle-to-subject reverse mapping (Go `Vault.Link`).
    fn link(&mut self, handle: &str, sid: &str);
}

/// Single-owner control plane; callers serialize access.
pub struct ControlPlane<B> {
    backend: B,
    store: CollectorStore,
    verdicts: VerdictStore,
    nonces: Nonces,
    guard: Guard,
    token_key: Option<TokenKey>,
    epochs: Option<Epochs>,
    cohort: Option<Observer>,
    loader: Option<Vec<u8>>,
    ready: bool,
}

impl<B: Backend> ControlPlane<B> {
    /// Control plane over shared session and verdict stores with the Go
    /// flood-guard defaults; ready once constructed.
    pub fn new(
        backend: B,
        store: CollectorStore,
        verdicts: VerdictStore,
    ) -> Result<Self, Unavailable> {
        Ok(Self {
            backend,
            store,
            verdicts,
            nonces: Nonces::new(NONCE_TTL, token::MAX_NONCES),
            guard: Guard::new()?,
            token_key: None,
            epochs: None,
            cohort: None,
            loader: None,
            ready: true,
        })
    }
    /// Replace the flood guard (Go `WithControlLimiter`).
    pub fn with_guard(mut self, guard: Guard) -> Self {
        self.guard = guard;
        self
    }
    /// Enable verdict tokens on ALLOW and step-up redemption (Go `WithTokenKey`).
    pub fn with_token_key(mut self, key: TokenKey) -> Self {
        self.token_key = Some(key);
        self
    }
    /// Mint under the shared rotating epochs (Go `WithTokenEpochs`).
    pub fn with_epochs(mut self, epochs: Epochs) -> Self {
        self.epochs = Some(epochs);
        self
    }
    /// Enable the log-only cohort shadow (Go `WithCohortShadow`).
    pub fn with_cohort(mut self, observer: Observer) -> Self {
        self.cohort = Some(observer);
        self
    }
    /// Serve this collector bundle at `/loader.js`.
    pub fn with_loader(mut self, bundle: Vec<u8>) -> Self {
        self.loader = Some(bundle);
        self
    }
    /// Readiness gate (Go `SetReady`); liveness is unaffected.
    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }
    /// Age-evict the cohort shadow (Go `ControlPlane.GC`).
    pub fn gc(&mut self, now: Duration) -> Result<usize, Unavailable> {
        match &mut self.cohort {
            Some(observer) => observer.gc(now),
            None => Ok(0),
        }
    }
    /// Merged session reports.
    pub fn store(&self) -> &CollectorStore {
        &self.store
    }
    /// Sticky verdicts the edge enforces.
    pub fn verdicts(&self) -> &VerdictStore {
        &self.verdicts
    }
    /// Caller collaborators.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Route one request (Go `ControlPlane.Handler`).
    pub fn handle(&mut self, request: &Request<'_>, now: Now) -> Response {
        match request.path {
            "/session" => self.session(request, now),
            "/collect" => self.collect(request, now),
            "/stepup" => self.step_up(request, now),
            "/loader.js" => self.loader(),
            "/csp-report" => Response {
                status: 204,
                ..Response::default()
            },
            "/healthz" => Response::json(200, probe("ok")),
            "/readyz" if self.ready => Response::json(200, probe("ready")),
            "/readyz" => Response {
                status: 503,
                // Go writes the header before setting the JSON content type,
                // so the server sniffs the body.
                headers: vec![("Content-Type", "text/plain; charset=utf-8".into())],
                body: probe("draining").into_bytes(),
                ..Response::default()
            },
            _ => Response::error(404, "404 page not found"),
        }
    }

    fn metered(&mut self, request: &Request<'_>, endpoint: &str, now: Now) -> Option<Response> {
        let Decision::Drop { breach } = self.guard.admit(request.peer, endpoint, now.mono) else {
            return None;
        };
        let mut response = Response::error(control_flood::STATUS, control_flood::BODY.trim_end());
        response
            .headers
            .push(("Retry-After", control_flood::RETRY_AFTER.into()));
        response.records.extend(breach.map(|record| *record));
        Some(response)
    }

    fn session(&mut self, request: &Request<'_>, now: Now) -> Response {
        if let Some(refusal) = self.metered(request, "session", now) {
            return refusal;
        }
        let (sid, cookies) = match self.ensure_session(request) {
            Ok(found) => found,
            Err(refusal) => return refusal,
        };
        self.store.ensure(&sid, now.wall);
        Response::json(200, format!("{{\"sessionId\":{}}}\n", quote_json_go(&sid)))
            .with_cookies(cookies)
    }

    fn step_up(&mut self, request: &Request<'_>, now: Now) -> Response {
        if request.method != "POST" {
            return Response::error(405, "POST only");
        }
        if let Some(refusal) = self.metered(request, "stepup", now) {
            return refusal;
        }
        if self.token_key.is_none() {
            return Response::error(501, "step-up not configured");
        }
        let Some(sid) = session_id(request) else {
            return Response::error(400, "no session");
        };
        let Some(unix) = unix_seconds(now.wall) else {
            return Response::error(503, "clock unavailable");
        };
        let key = self.token_key.as_ref().expect("checked above");
        if let Err(reason) = key.verify_receipt(request.header(STEP_UP_RECEIPT_HEADER), sid, unix) {
            let fail_reason = match reason {
                Reason::BindingMismatch => "step-up receipt sid mismatch — Core Pass likely not co-served in the Gate cookie jar (hsid diverged); real human may loop on the attested route".to_owned(),
                Reason::Expired => "step-up receipt expired (check Core↔Gate clock skew / NTP; the client can re-solve for a fresh receipt)".to_owned(),
                Reason::Malformed | Reason::BadSignature => {
                    "step-up: missing/invalid Pass receipt (bad_sig)".to_owned()
                }
            };
            let mut response = Response::error(403, "invalid or missing step-up receipt");
            response.records.push(Record {
                event_type: EVENT_TOKEN_BINDING_MISMATCH.into(),
                actor: self.subject(sid, sid),
                verdict: "deny".into(),
                enforcement_mode: "enforce".into(),
                fail_reason,
                ..control_record()
            });
            return response;
        }
        let epoch = self.epoch(unix);
        let bind = token_bind(request);
        let Ok(proof) = key.issue_step_up(sid, &bind, &epoch, unix.saturating_add(TOKEN_TTL))
        else {
            return Response::error(503, "step-up unavailable");
        };
        let mut response = Response::json(200, "{\"ok\":true}\n".into())
            .with_cookies(vec![cookie(STEP_UP_COOKIE, &proof)]);
        response.records.push(Record {
            event_type: EVENT_TOKEN_ISSUED.into(),
            actor: self.subject(sid, sid),
            verdict: "allow".into(),
            enforcement_mode: "enforce".into(),
            fail_reason: "step-up proof minted (Pass solved)".into(),
            ..control_record()
        });
        response
    }

    fn loader(&self) -> Response {
        match &self.loader {
            Some(bundle) => Response {
                status: 200,
                headers: vec![
                    (
                        "Content-Type",
                        "application/javascript; charset=utf-8".into(),
                    ),
                    ("Cache-Control", "no-store".into()),
                ],
                body: bundle.clone(),
                ..Response::default()
            },
            None => Response::error(404, "404 page not found"),
        }
    }

    /// Go `ensureSession`: reuse `hsid` or mint one and set its cookie.
    fn ensure_session(&mut self, request: &Request<'_>) -> Result<(String, Vec<String>), Response> {
        if let Some(sid) = session_id(request) {
            return Ok((sid.to_owned(), Vec::new()));
        }
        let sid = self
            .backend
            .new_session_id()
            .ok()
            .filter(|sid| {
                sid.len() == 32 && sid.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            })
            .ok_or_else(|| Response::error(503, "session unavailable"))?;
        let set = cookie(SESSION_COOKIE, &sid);
        Ok((sid, vec![set]))
    }

    /// Go `pseudo`: empty for an empty session.
    fn pseudo(&mut self, sid: &str, value: &str) -> String {
        if sid.is_empty() {
            return String::new();
        }
        self.backend.pseudonymize(sid, value)
    }

    fn subject(&mut self, sid: &str, value: &str) -> Actor {
        Actor {
            kind: "subject".into(),
            id_pseudonym: self.pseudo(sid, value),
        }
    }

    /// Go `tokEpoch`: shared epochs, else the fixed fallback.
    fn epoch(&self, unix: u64) -> String {
        self.epochs
            .as_ref()
            .map_or_else(|| FALLBACK_EPOCH.to_owned(), |epochs| epochs.current(unix))
    }
}

/// Go `sessionID`: the cookie value, where empty means none.
fn session_id<'a>(request: &Request<'a>) -> Option<&'a str> {
    request.session.filter(|sid| !sid.is_empty())
}

/// Go `tokenBind(r)`: header fingerprint plus socket subnet.
fn token_bind(request: &Request<'_>) -> String {
    let key = token::bind_key(
        request.header("User-Agent"),
        request.header("Accept-Language"),
        request.header("Sec-CH-UA"),
    );
    token::token_bind(key.as_deref(), &token::client_subnet(request.peer))
}

fn unix_seconds(wall: Timestamp) -> Option<u64> {
    u64::try_from(wall.0.unix_timestamp()).ok()
}

fn cookie(name: &str, value: &str) -> String {
    format!("{name}={value}; Path=/; HttpOnly; Secure; SameSite=Lax")
}

fn probe(status: &str) -> String {
    format!("{{\"plane\":\"gate\",\"status\":\"{status}\"}}\n")
}

/// Fields every control-plane record shares.
fn control_record() -> Record {
    Record {
        tenant_id: "control".into(),
        route_class: "control".into(),
        key_id: KEY_ID.into(),
        ..Record::default()
    }
}

#[cfg(test)]
mod tests;
