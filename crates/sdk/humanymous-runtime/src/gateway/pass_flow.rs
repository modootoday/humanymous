//! humanymous Pass HTTP flow (legacy cmd/server pass_handler.go at baseline
//! 981d25c, ported under owner decision B1 with a Go-captured golden).
//!
//! [`PassFlow`] answers the six legacy routes in [`ROUTES`]: the accessible
//! page (`web/pass.html`, keyboard and screen-reader operable), a fresh
//! single-use instance (`/api/pass/new`), the proof-of-work preflight, the
//! rate-limited attestation token, the solve and the wargame KPIs. It is
//! transport-agnostic: the composition root hands it a parsed [`Request`],
//! the caller clock and a [`Backend`] over the session store and the frozen
//! scoring engine, and writes back the [`Response`]. Mount it with [`mount`]
//! and route every path in [`ROUTES`] to [`PassFlow::handle`].
//!
//! Scoring is the frozen engine's: the flow only appends registry signals
//! (`l7.pass.velocity`, `l7.pass.flood`, `l7.pass.solved` and, when enabled,
//! the behavior tells) and re-scores, exactly where Go did. Instance state is
//! single-use: every `/new` advances the instance, resets tries, nonce and
//! the preflight; a solve needs this instance's nonce, the paid PoW, an
//! attestation token once the session is velocity-flagged, a fresh bucket,
//! the server-side alignment and an unseen trace digest. Expiry is reported
//! as expiry, never as a wrong answer.
//!
//! Accessibility: [`MotorEvidence::Off`] (the default) skips the
//! `realEventOK` pre-filter and the behavior tells, which gate or penalise
//! Pass on motor richness and timing. [`MotorEvidence::GoParity`] restores the
//! Go behavior and exists for differential parity; serving it needs an owner
//! decision.
//!
//! Deliberate differences from Go, each pinned by tests:
//! - Bodies decode the first JSON value of the bounded prefix with exact
//!   field names; Go also matched field names case-insensitively and let a
//!   later duplicate key win.
//! - A red-team header outside the KPI label alphabet is refused (400), and
//!   a session id over 128 bytes is refused (400); Go accepted both.
//! - A store, limiter or scoring failure fails closed (503) instead of
//!   proceeding unmetered; a KPI counter overflow drops the count only.
//! - A trace with a sample outside the int64 range is rejected as unusable
//!   (Go hashed an implementation-defined integer).
//! - The step-up receipt is omitted when the session id cannot be sealed.
use crate::gateway::control_plane::Now;
use crate::gateway::demo::{new_session, session_cookie};
use crate::gateway::observatory::hub::Hub;
use crate::gateway::observatory::{Body, Response};
use crate::gateway::pass_proof::{PassProof, real_event_ok, trace_digest};
use crate::gateway::pass_scoring::{
    ATTEST_BUDGET, ATTEST_WINDOW_SECS, VELOCITY_HARD, VELOCITY_SOFT, VELOCITY_WINDOW_SECS,
    behavior_signals, difficulty, label, level_number, pow_difficulty, solved_signal,
    velocity_escalation,
};
use crate::gateway::pass_store::{PASS_MAX_TRIES, PASS_WINDOW_SECS, PassNonce, PassStore, Refusal};
use crate::gateway::session::store::CollectorStore;
use crate::gateway::token::TokenKey;
use crate::network::history::Unavailable;
use crate::network::history::rate::{Config as RateConfig, Level, Limiter, Limits, Observation};
use humanymous_core::challenge::{attestation, pass, proof_of_work};
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::observation::Signal;
use humanymous_core::signals::report::{ScoreResult, SessionReport, Timestamp};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use zeroize::Zeroizing;

/// Accessible Pass page.
pub const PAGE_PATH: &str = "/pass";
/// Fresh challenge instance.
pub const NEW_PATH: &str = "/api/pass/new";
/// Proof-of-work preflight.
pub const POW_PATH: &str = "/api/pass/pow";
/// Attestation token issuance.
pub const ATTEST_PATH: &str = "/api/pass/attest";
/// Solve submission.
pub const SOLVE_PATH: &str = "/api/pass/solve";
/// Wargame KPIs.
pub const KPI_PATH: &str = "/api/pass/kpi";
/// Every path the flow answers.
pub const ROUTES: [&str; 6] = [
    PAGE_PATH,
    NEW_PATH,
    POW_PATH,
    ATTEST_PATH,
    SOLVE_PATH,
    KPI_PATH,
];
/// Session cookie (Go `sessionCookie`).
pub const SESSION_COOKIE: &str = "hsid";
/// Largest solve body read (Go `MaxBytesReader(.., 1<<15)`).
pub const MAX_SOLVE_BODY: usize = 1 << 15;
/// Largest preflight body read (Go `MaxBytesReader(.., 4096)`).
pub const MAX_PREFLIGHT_BODY: usize = 4096;
/// Step-up receipt lifetime in seconds.
pub const RECEIPT_TTL_SECS: u64 = 120;
/// Advertised instance lifetime (two buckets).
pub const EXPIRES_IN_SECS: u64 = PASS_WINDOW_SECS * 2;
/// Most distinct limiter keys retained per limiter.
pub const MAX_LIMITER_KEYS: usize = 65_536;
/// Most hits retained per velocity key inside the window.
pub const MAX_VELOCITY_HITS: usize = 256;
/// Most hits retained per attestation key inside the window.
pub const MAX_ATTEST_HITS: usize = 64;
/// Longest limiter key in bytes.
pub const MAX_LIMITER_KEY_BYTES: usize = 512;
const GC_SCAN: usize = 64;

/// Whether motor-timing evidence may gate or penalise a Pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotorEvidence {
    /// Accessible default: no real-event pre-filter, no behavior tells.
    #[default]
    Off,
    /// Go behavior: `realEventOK` rejects and the behavior tells re-score.
    /// Conflicts with the accessibility rule; needs an owner decision.
    GoParity,
}

/// Session store, frozen engine and telemetry the flow drives.
pub trait Backend {
    /// Create the session report if needed (Go `store.Ensure`).
    fn ensure(&mut self, session: &str, now: Timestamp);
    /// Stored report (Go `store.Get`).
    fn report(&self, session: &str) -> Option<SessionReport>;
    /// Persist a re-scored report (Go `store.StoreScored`).
    fn store_scored(&mut self, session: &str, report: SessionReport, now: Timestamp);
    /// Score with the frozen engine; `None` fails closed.
    fn score(&mut self, report: &mut SessionReport) -> Option<ScoreResult>;
    /// Live telemetry (Go `hub.Publish`); a no-op without a hub.
    fn publish(&mut self, _kind: &str, _payload: &Value) {}
    /// Completed-Pass feed (Go `feedPassOutcome`); a no-op without the
    /// self-correcting control plane.
    fn pass_outcome(&mut self, _report: &SessionReport, _solved: bool) {}
}

/// [`Backend`] over a collector store, the frozen engine and an optional hub.
pub struct StoreBackend<'a> {
    /// Merged session reports.
    pub store: &'a mut CollectorStore,
    /// Frozen scoring engine.
    pub engine: &'a Engine,
    /// Live feed, when the observatory is enabled.
    pub hub: Option<&'a Arc<Hub>>,
}

impl Backend for StoreBackend<'_> {
    fn ensure(&mut self, session: &str, now: Timestamp) {
        self.store.ensure(session, now);
    }
    fn report(&self, session: &str) -> Option<SessionReport> {
        self.store.get(session)
    }
    fn store_scored(&mut self, session: &str, report: SessionReport, now: Timestamp) {
        self.store.store_scored(session, report, now);
    }
    fn score(&mut self, report: &mut SessionReport) -> Option<ScoreResult> {
        self.engine.score(report).ok()
    }
    fn publish(&mut self, kind: &str, payload: &Value) {
        if let Some(hub) = self.hub {
            hub.publish(kind, payload);
        }
    }
}

/// Parsed request the HTTP owner hands over.
#[derive(Debug, Clone, Copy, Default)]
pub struct Request<'a> {
    /// HTTP method.
    pub method: &'a str,
    /// Path without query.
    pub path: &'a str,
    /// `Cookie` header.
    pub cookie: Option<&'a str>,
    /// `X-HM-Redteam` header.
    pub red_team: Option<&'a str>,
    /// Stable fingerprint of the connection: [`super::pass_scoring::fingerprint`]
    /// of the JA4 stable prefix and the trust-gated client subnet.
    pub fingerprint: &'a str,
    /// Request body; at least one byte past the route limit when available.
    pub body: &'a [u8],
}

/// Mount configuration.
pub struct Config {
    /// Server master key for puzzles, proof of work and attestation.
    pub master_key: Vec<u8>,
    /// Directory holding `pass.html`.
    pub web_dir: PathBuf,
    /// Shared Gate token key; enables step-up receipts on a solve.
    pub step_up: Option<TokenKey>,
    /// Motor-evidence policy; [`MotorEvidence::Off`] unless the owner decides.
    pub motor_evidence: MotorEvidence,
}

type NonceSource = Box<dyn FnMut() -> Result<PassNonce, Refusal> + Send>;

/// The mounted Pass flow; callers serialize access.
pub struct PassFlow {
    store: PassStore,
    velocity: Limiter,
    attest: Limiter,
    master_key: Zeroizing<Vec<u8>>,
    web_dir: PathBuf,
    step_up: Option<TokenKey>,
    motor_evidence: MotorEvidence,
    nonces: NonceSource,
}

/// Build the flow with the legacy limiter policy (velocity 30 s soft 4 hard
/// 8; attestation issuance 30 s budget 3) and operating-system nonces.
pub fn mount(config: Config) -> Result<PassFlow, Unavailable> {
    let limiter = |seconds, soft, hard, hits: usize| -> Result<Limiter, Unavailable> {
        let window = Duration::from_secs(seconds);
        let per_key = hits
            .checked_mul(size_of::<Duration>())
            .and_then(|bytes| bytes.checked_add(MAX_LIMITER_KEY_BYTES))
            .ok_or(Unavailable::InvalidConfiguration)?;
        let limits = Limits::new(
            MAX_LIMITER_KEYS,
            hits,
            MAX_LIMITER_KEY_BYTES,
            window,
            MAX_LIMITER_KEYS
                .checked_mul(per_key)
                .ok_or(Unavailable::InvalidConfiguration)?,
            GC_SCAN,
        )?;
        Ok(Limiter::new(RateConfig::new(window, soft, hard, limits)?))
    };
    Ok(PassFlow {
        store: PassStore::new(),
        velocity: limiter(
            VELOCITY_WINDOW_SECS,
            VELOCITY_SOFT,
            VELOCITY_HARD,
            MAX_VELOCITY_HITS,
        )?,
        attest: limiter(
            ATTEST_WINDOW_SECS,
            ATTEST_BUDGET,
            ATTEST_BUDGET,
            MAX_ATTEST_HITS,
        )?,
        master_key: Zeroizing::new(config.master_key),
        web_dir: config.web_dir,
        step_up: config.step_up,
        motor_evidence: config.motor_evidence,
        nonces: Box::new(PassNonce::generate),
    })
}

/// Fail-closed internal failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Limiter,
    Store(Refusal),
    Scoring,
}

fn failure_response(failure: Failure) -> Response {
    match failure {
        Failure::Store(Refusal::SessionId) => text(400, "bad session"),
        Failure::Store(Refusal::Label) => text(400, "bad red-team label"),
        _ => text(503, "pass unavailable"),
    }
}

/// Go `http.Error`.
fn text(status: u16, message: &str) -> Response {
    Response {
        status,
        headers: vec![
            ("Content-Type", "text/plain; charset=utf-8".into()),
            ("X-Content-Type-Options", "nosniff".into()),
        ],
        body: Body::Bytes(format!("{message}\n").into_bytes()),
    }
}

/// Go `writeJSON`: sorted object keys and a trailing newline.
fn json_response(value: &Value) -> Response {
    let mut body = serde_json::to_vec(value).unwrap_or_default();
    body.push(b'\n');
    Response {
        status: 200,
        headers: vec![("Content-Type", "application/json".into())],
        body: Body::Bytes(body),
    }
}

/// First `hsid` cookie with a valid Go cookie value (`r.Cookie`), quotes
/// stripped; `None` when absent or empty.
pub fn session_cookie_value(header: Option<&str>) -> Option<&str> {
    header?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(|(name, _)| name.trim() == SESSION_COOKIE)
        .map(|(_, value)| {
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .unwrap_or(value)
        })
        .find(|value| {
            value
                .bytes()
                .all(|b| (0x20..0x7f).contains(&b) && !b"\";\\".contains(&b))
        })
        .filter(|value| !value.is_empty())
}

/// Decode the first JSON value inside the `limit`-byte prefix into a struct;
/// `null` decodes to the default (Go leaves the target untouched) and any
/// other non-object value is refused, as Go's struct decoding did.
fn decode<T: DeserializeOwned + Default>(body: &[u8], limit: usize) -> Option<T> {
    let prefix = &body[..body.len().min(limit)];
    let first = prefix
        .iter()
        .find(|byte| !matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))?;
    if !matches!(first, b'{' | b'n') {
        return None;
    }
    serde_json::Deserializer::from_slice(prefix)
        .into_iter::<Option<T>>()
        .next()?
        .ok()
        .map(Option::unwrap_or_default)
}

fn unix_seconds(now: &Now) -> u64 {
    u64::try_from(now.wall.0.unix_timestamp()).unwrap_or(0)
}

/// Go `passPoWSession`: the PoW seed binds the session and the instance.
pub fn pow_session(session: &str, nonce: &str) -> String {
    format!("{session}|pass|{nonce}")
}

fn append(report: &mut SessionReport, signals: Vec<Signal>) {
    report
        .network
        .signals
        .get_or_insert_with(Vec::new)
        .extend(signals);
}

fn valid_red_team(header: Option<&str>) -> Result<Option<&str>, Failure> {
    match header.filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value)
            if value.len() <= crate::gateway::pass_store::MAX_LABEL_BYTES
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b)) =>
        {
            Ok(Some(value))
        }
        Some(_) => Err(Failure::Store(Refusal::Label)),
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AttestBody {
    challenge_nonce: String,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct PowBody {
    bucket: u64,
    nonce: String,
    challenge_nonce: String,
}

/// Per-attempt snapshot taken under the store lock.
struct Attempt {
    tries: u32,
    difficulty: u32,
    bucket: u64,
    instance: u64,
    nonce: Option<String>,
    pow_ok: bool,
    attest_required: bool,
}

enum Admission {
    AlreadySolved,
    RetryCap,
    Attempt(Attempt),
}

impl PassFlow {
    /// Replace the nonce source (deterministic tests, other generators).
    pub fn with_nonce_source(
        mut self,
        source: impl FnMut() -> Result<PassNonce, Refusal> + Send + 'static,
    ) -> Self {
        self.nonces = Box::new(source);
        self
    }

    /// Transient Pass state and KPI counters.
    pub fn store(&self) -> &PassStore {
        &self.store
    }

    /// Answer a Pass route; `None` for paths outside [`ROUTES`].
    pub fn handle(
        &mut self,
        backend: &mut dyn Backend,
        request: &Request<'_>,
        now: Now,
    ) -> Option<Response> {
        Some(match request.path {
            PAGE_PATH => self.page(backend, request, now),
            NEW_PATH => self.new_instance(backend, request, now),
            POW_PATH => self.pow(request, now),
            ATTEST_PATH => self.attestation(request, now),
            SOLVE_PATH => self.solve(backend, request, now),
            KPI_PATH => json_response(&serde_json::to_value(self.store.kpi()).unwrap_or_default()),
            _ => return None,
        })
    }

    /// Go `ensureSession`: the cookie session, or a fresh one to set.
    fn ensure_session(request: &Request<'_>) -> Result<(String, Option<String>), Response> {
        match session_cookie_value(request.cookie) {
            Some(session) => Ok((session.to_owned(), None)),
            None => {
                let session = new_session().map_err(|_| text(500, "session unavailable"))?;
                let cookie = session_cookie(&session);
                Ok((session, Some(cookie)))
            }
        }
    }

    fn page(&mut self, backend: &mut dyn Backend, request: &Request<'_>, now: Now) -> Response {
        let (session, cookie) = match Self::ensure_session(request) {
            Ok(issued) => issued,
            Err(response) => return response,
        };
        backend.ensure(&session, now.wall);
        let mut headers = vec![("Content-Type", "text/html; charset=utf-8".to_owned())];
        if let Some(cookie) = cookie {
            headers.push(("Set-Cookie", cookie));
        }
        Response {
            status: 200,
            headers,
            body: Body::File(self.web_dir.join("pass.html")),
        }
    }

    fn observe(limiter: &mut Limiter, key: &str, now: Duration) -> Result<Level, Failure> {
        let level = match limiter.observe(key, now).map_err(|_| Failure::Limiter)? {
            Observation::Counted { level, .. } => level,
            Observation::IgnoredEmptyKey => Level::Normal,
        };
        limiter.gc(now).map_err(|_| Failure::Limiter)?;
        Ok(level)
    }

    /// Legacy `applyPassVelocity`: (combined level, session level).
    fn apply_velocity(
        &mut self,
        backend: &mut dyn Backend,
        session: &str,
        fingerprint: &str,
        now: Now,
    ) -> Result<(u8, u8), Failure> {
        let session_level = level_number(Self::observe(
            &mut self.velocity,
            &format!("s|{session}"),
            now.mono,
        )?);
        let fingerprint_level = level_number(Self::observe(
            &mut self.velocity,
            &format!("f|{fingerprint}"),
            now.mono,
        )?);
        let level = session_level.max(fingerprint_level);
        if level == 0 {
            return Ok((0, session_level));
        }
        let previous = self
            .store
            .with_session(session, now.mono, |state| {
                let previous = state.velocity_level;
                state.velocity_level = previous.max(level);
                previous
            })
            .map_err(Failure::Store)?;
        if let Some(signal) = velocity_escalation(level, previous) {
            let mut report = backend.report(session).unwrap_or_default();
            append(&mut report, vec![signal]);
            backend.score(&mut report).ok_or(Failure::Scoring)?;
            backend.store_scored(session, report, now.wall);
        }
        Ok((level, session_level))
    }

    fn new_instance(
        &mut self,
        backend: &mut dyn Backend,
        request: &Request<'_>,
        now: Now,
    ) -> Response {
        let (session, cookie) = match Self::ensure_session(request) {
            Ok(issued) => issued,
            Err(response) => return response,
        };
        let mut response = self
            .issue(backend, request, &session, now)
            .unwrap_or_else(std::convert::identity);
        if let Some(cookie) = cookie {
            response.headers.push(("Set-Cookie", cookie));
        }
        response
    }

    fn issue(
        &mut self,
        backend: &mut dyn Backend,
        request: &Request<'_>,
        session: &str,
        now: Now,
    ) -> Result<Response, Response> {
        let red_team = valid_red_team(request.red_team).map_err(failure_response)?;
        let issued = (self.nonces)().map_err(|_| text(500, "challenge unavailable"))?;
        let nonce = issued.to_hex();
        let (velocity_level, session_level) = self
            .apply_velocity(backend, session, request.fingerprint, now)
            .map_err(failure_response)?;
        let seconds = unix_seconds(&now);
        let bucket = seconds / PASS_WINDOW_SECS;
        let risk = backend
            .report(session)
            .unwrap_or_default()
            .scoring
            .risk_score;
        let puzzle = difficulty(risk, red_team.is_some());
        let pow_bits = pow_difficulty(puzzle, velocity_level);
        let attest_required = session_level >= 1;
        let instance = self
            .store
            .with_session(session, now.mono, |state| {
                state.instance = state.instance.wrapping_add(1);
                state.bucket = bucket;
                state.issued_at = now.mono;
                state.tries = 0;
                state.solved = false;
                state.nonce = Some(issued.clone());
                state.pow_ok = false;
                state.difficulty = puzzle;
                state.pow_difficulty = pow_bits;
                state.attest_required = attest_required;
                state.instance
            })
            .map_err(|refusal| failure_response(Failure::Store(refusal)))?;
        let challenge = pass::generate(&self.master_key, session, bucket, instance, puzzle as i32);
        let pow = proof_of_work::issue(
            &self.master_key,
            &pow_session(session, &nonce),
            pow_bits as i32,
            seconds / proof_of_work::WINDOW,
        );
        Ok(json_response(&json!({
            "ok": true,
            "bucket": bucket,
            "difficulty": puzzle,
            "challenge": challenge,
            "challengeNonce": nonce,
            "triesLeft": PASS_MAX_TRIES,
            "expiresInS": EXPIRES_IN_SECS,
            "preflight": {"pow": pow, "attestRequired": attest_required},
        })))
    }

    fn preflight_session<'r>(request: &Request<'r>) -> Result<&'r str, Response> {
        if request.method != "POST" {
            return Err(text(405, "POST only"));
        }
        session_cookie_value(request.cookie).ok_or_else(|| text(400, "no session"))
    }

    fn issued_nonce(
        &self,
        session: &str,
        now: Now,
    ) -> Result<(Option<String>, u32, bool), Response> {
        self.store
            .with_session(session, now.mono, |state| {
                (
                    state.nonce.as_ref().map(PassNonce::to_hex),
                    state.pow_difficulty,
                    state.solved,
                )
            })
            .map_err(|refusal| failure_response(Failure::Store(refusal)))
    }

    fn attestation(&mut self, request: &Request<'_>, now: Now) -> Response {
        let session = match Self::preflight_session(request) {
            Ok(session) => session,
            Err(response) => return response,
        };
        let Some(body) = decode::<AttestBody>(request.body, MAX_PREFLIGHT_BODY) else {
            return text(400, "bad body");
        };
        let issued = match self.issued_nonce(session, now) {
            Ok((issued, _, _)) => issued,
            Err(response) => return response,
        };
        let Some(issued) = issued
            .filter(|issued| !body.challenge_nonce.is_empty() && body.challenge_nonce == *issued)
        else {
            return json_response(&json!({"ok": false, "reason": "stale challenge"}));
        };
        let level = match Self::observe(
            &mut self.attest,
            &format!("att|{}", request.fingerprint),
            now.mono,
        ) {
            Ok(level) => level,
            Err(failure) => return failure_response(failure),
        };
        if level == Level::Hard {
            return json_response(&json!({
                "ok": false,
                "reason": "attestation budget exhausted — retry shortly",
            }));
        }
        let window = unix_seconds(&now) / attestation::WINDOW;
        let token = attestation::issue(&self.master_key, session, &issued, window);
        json_response(&json!({"ok": true, "token": token}))
    }

    fn pow(&mut self, request: &Request<'_>, now: Now) -> Response {
        let session = match Self::preflight_session(request) {
            Ok(session) => session,
            Err(response) => return response,
        };
        let Some(body) = decode::<PowBody>(request.body, MAX_PREFLIGHT_BODY) else {
            return text(400, "bad body");
        };
        let (issued, bits, solved) = match self.issued_nonce(session, now) {
            Ok(state) => state,
            Err(response) => return response,
        };
        let current = unix_seconds(&now) / proof_of_work::WINDOW;
        let ok = match issued {
            Some(issued) if !solved && body.challenge_nonce == issued => {
                let verified = proof_of_work::verify(
                    &self.master_key,
                    &pow_session(session, &issued),
                    bits as i32,
                    body.bucket,
                    current,
                    &body.nonce,
                );
                if verified {
                    let marked = self.store.with_session(session, now.mono, |state| {
                        if state.nonce.as_ref().map(PassNonce::to_hex).as_deref() == Some(&issued) {
                            state.pow_ok = true;
                        }
                    });
                    if let Err(refusal) = marked {
                        return failure_response(Failure::Store(refusal));
                    }
                }
                verified
            }
            _ => false,
        };
        json_response(&json!({"ok": ok}))
    }

    fn receipt(&self, session: &str, now: &Now) -> Option<String> {
        let key = self.step_up.as_ref()?;
        key.issue_receipt(session, unix_seconds(now).saturating_add(RECEIPT_TTL_SECS))
            .ok()
    }

    fn admit(&self, session: &str, now: Now) -> Result<Admission, Failure> {
        self.store
            .with_session(session, now.mono, |state| {
                if state.solved {
                    return Admission::AlreadySolved;
                }
                if state.exhausted() {
                    return Admission::RetryCap;
                }
                state.tries += 1;
                Admission::Attempt(Attempt {
                    tries: state.tries,
                    difficulty: state.difficulty,
                    bucket: state.bucket,
                    instance: state.instance,
                    nonce: state.nonce.as_ref().map(PassNonce::to_hex),
                    pow_ok: state.pow_ok,
                    attest_required: state.attest_required,
                })
            })
            .map_err(Failure::Store)
    }

    fn publish(backend: &mut dyn Backend, session: &str, solved: bool, reason: &str) {
        let mut end = session.len().min(8);
        while !session.is_char_boundary(end) {
            end -= 1;
        }
        backend.publish(
            "pass.attempt",
            &json!({"sid": &session[..end], "solved": solved, "reason": reason}),
        );
    }

    fn record(&self, label: &str, difficulty: u32, passed: bool) {
        // KPI overflow or an unrepresentable label drops the count only; the
        // attempt outcome never depends on the scoreboard.
        let _ = self.store.record(label, difficulty, passed);
    }

    fn solve(&mut self, backend: &mut dyn Backend, request: &Request<'_>, now: Now) -> Response {
        let session = match Self::preflight_session(request) {
            Ok(session) => session.to_owned(),
            Err(response) => return response,
        };
        let Some(proof) = decode::<PassProof>(request.body, MAX_SOLVE_BODY) else {
            return text(400, "bad body");
        };
        match self.attempt(backend, request, &session, &proof, now) {
            Ok(response) => response,
            Err(failure) => failure_response(failure),
        }
    }

    fn attempt(
        &mut self,
        backend: &mut dyn Backend,
        request: &Request<'_>,
        session: &str,
        proof: &PassProof,
        now: Now,
    ) -> Result<Response, Failure> {
        let red_team = valid_red_team(request.red_team)?;
        let attempt = match self.admit(session, now)? {
            Admission::AlreadySolved => {
                let mut out = json!({"ok": true, "alreadySolved": true});
                if let Some(receipt) = self.receipt(session, &now) {
                    out["stepUpReceipt"] = Value::String(receipt);
                }
                return Ok(json_response(&out));
            }
            Admission::RetryCap => {
                return Ok(json_response(
                    &json!({"ok": false, "reason": "retry cap reached", "triesLeft": 0}),
                ));
            }
            Admission::Attempt(attempt) => attempt,
        };
        let seconds = unix_seconds(&now);
        let current = seconds / PASS_WINDOW_SECS;
        self.apply_velocity(backend, session, request.fingerprint, now)?;
        let before = backend.report(session).unwrap_or_default();
        let kpi_label = label(before.scoring.risk_score, red_team);
        let tries_left = PASS_MAX_TRIES - attempt.tries;
        let reject = |flow: &Self, backend: &mut dyn Backend, why: &str| {
            flow.record(&kpi_label, attempt.difficulty, false);
            backend.pass_outcome(&before, false);
            Self::publish(backend, session, false, why);
            json_response(&json!({"ok": false, "reason": why, "triesLeft": tries_left}))
        };
        let Some(nonce) = attempt
            .nonce
            .clone()
            .filter(|nonce| proof.challenge_nonce == *nonce)
        else {
            return Ok(reject(self, backend, "stale or missing challenge"));
        };
        if !attempt.pow_ok {
            return Ok(reject(self, backend, "no cryptographic preflight"));
        }
        if attempt.attest_required
            && !attestation::verify(
                &self.master_key,
                session,
                &nonce,
                &proof.attest_token,
                seconds / attestation::WINDOW,
            )
        {
            return Ok(reject(self, backend, "attestation required"));
        }
        if self.motor_evidence == MotorEvidence::GoParity {
            if let Err(why) = real_event_ok(proof) {
                return Ok(reject(self, backend, why));
            }
            let tells = behavior_signals(proof);
            if !tells.is_empty() {
                let mut report = backend.report(session).unwrap_or_default();
                append(&mut report, tells);
                backend.score(&mut report).ok_or(Failure::Scoring)?;
                backend.store_scored(session, report, now.wall);
            }
        }
        if !pass::fresh(attempt.bucket, current) {
            Self::publish(backend, session, false, "expired");
            return Ok(json_response(&json!({
                "ok": false,
                "expired": true,
                "reason": "this check expired — press New puzzle to start a fresh one",
            })));
        }
        if !pass::verify(
            &self.master_key,
            session,
            attempt.bucket,
            current,
            attempt.instance,
            attempt.difficulty as i32,
            &proof.offsets,
        ) {
            self.record(&kpi_label, attempt.difficulty, false);
            Self::publish(backend, session, false, "misaligned");
            return Ok(json_response(&json!({
                "ok": false,
                "reason": "the keys are not all in the slot",
                "triesLeft": tries_left,
            })));
        }
        let Some(digest) = trace_digest(proof) else {
            return Ok(reject(self, backend, "unusable interaction trace"));
        };
        if !self.store.reserve_trace(digest, now.mono) {
            return Ok(reject(self, backend, "duplicate interaction trace"));
        }
        let mut report = backend.report(session).unwrap_or_default();
        append(&mut report, vec![solved_signal()]);
        let scored = backend.score(&mut report).ok_or(Failure::Scoring)?;
        self.record(&kpi_label, attempt.difficulty, true);
        self.store
            .with_session(session, now.mono, |state| state.solved = true)
            .map_err(Failure::Store)?;
        Self::publish(backend, session, true, "");
        backend.store_scored(session, report, now.wall);
        backend.pass_outcome(&before, true);
        let mut out =
            json!({"ok": true, "verdict": scored.verdict, "riskScore": scored.risk_score});
        if let Some(receipt) = self.receipt(session, &now) {
            out["stepUpReceipt"] = Value::String(receipt);
        }
        Ok(json_response(&out))
    }
}

#[cfg(test)]
mod tests;
