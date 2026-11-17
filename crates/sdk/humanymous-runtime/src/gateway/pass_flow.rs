//! humanymous Pass HTTP flow (legacy cmd/server/pass_handler.go and
//! pass_scoring.go at baseline 981d25c, SoT-36; roadmap G3).
//!
//! The interactive challenge served on a CHALLENGE verdict: `/pass` serves the
//! page, `/api/pass/new` issues a fresh instance, `/api/pass/pow` and
//! `/api/pass/attest` carry the crypto axis, `/api/pass/solve` verifies the
//! placement server-side and grants the trust upgrade, and `/api/pass/kpi`
//! reports the wargame scoreboard. All scoring is server-side; the client only
//! renders the public scene and collects interaction.
//!
//! This module composes existing kernels and owns no transport: the HTTP
//! owner hands it a parsed [`Request`] plus a [`PassContext`] and writes the
//! [`Response`]. Engine, report store, clock, outcome feed, live-feed hub and
//! step-up key are injected.
//!
//! Intentional differences from Go, each pinned by tests:
//! - `realEventOK` (pointer/keyboard timing pre-filter) is not ported: it gated
//!   a Pass on motor richness and speed, which the accessibility rule forbids
//!   (owner decision 2026-11-17). The soft behavioral tells in [`scoring`] are
//!   ported because they only raise risk and never reject.
//! - Unavailable rate history (key budget, clock regression) or a failed
//!   engine score refuses the request (503/500) instead of proceeding
//!   unmetered; a malformed red-team label is refused before it consumes a try.
//! - A submission whose trace cannot be digested (out-of-range sample) is
//!   rejected as `invalid interaction trace` instead of hashing an
//!   implementation-defined integer.
//! - Store refusals from [`PassStore`] keep its documented fail-closed
//!   behaviour (bounded ids, refusal at saturation).
//! - Request bodies must be exactly one JSON value within the Go size limits.
//!
//! Mounting: [`crate::gateway::legacy`] dispatches these paths from the live
//! gateway listener when its explicit `legacy.pass` configuration is set,
//! supplying the accepted peer address and the captured stable JA4 (empty
//! when no ClientHello was captured) through [`PassContext`].
pub mod scoring;

use crate::gateway::demo::{new_session, session_cookie, session_from_cookie};
use crate::gateway::observatory::{Body, Request, Response, hub::Hub};
use crate::gateway::pass_proof::{PassProof, trace_digest};
use crate::gateway::pass_store::{
    MetricKey, Outcome, PASS_MAX_TRIES, PASS_WINDOW_SECS, PassNonce, PassSession, PassStore,
    Refusal,
};
use crate::gateway::token::TokenKey;
use crate::network::history::rate::{Config, Level, Limiter, Limits, Observation, Unavailable};
use humanymous_core::challenge::{attestation, pass, proof_of_work};
use humanymous_core::model::correction::{Controller, HumanTrace, feed_pass_outcome};
use humanymous_core::model::inference::Prediction;
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::{BehaviorSummary, SessionReport};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

/// Page route.
pub const PAGE_PATH: &str = "/pass";
/// Fresh instance route.
pub const NEW_PATH: &str = "/api/pass/new";
/// Proof-of-work route.
pub const POW_PATH: &str = "/api/pass/pow";
/// Attestation issuance route.
pub const ATTEST_PATH: &str = "/api/pass/attest";
/// Solve route.
pub const SOLVE_PATH: &str = "/api/pass/solve";
/// KPI route.
pub const KPI_PATH: &str = "/api/pass/kpi";

/// Velocity governor window (Go `passVel`).
pub const VELOCITY_WINDOW: Duration = Duration::from_secs(30);
/// Velocity soft cutpoint.
pub const VELOCITY_SOFT: usize = 4;
/// Velocity hard cutpoint.
pub const VELOCITY_HARD: usize = 8;
/// Attestation issuance window per fingerprint (Go `attestLim`).
pub const ATTEST_WINDOW: Duration = Duration::from_secs(30);
/// Attestation soft cutpoint.
pub const ATTEST_SOFT: usize = 3;
/// Attestation hard cutpoint; reaching it denies issuance.
pub const ATTEST_HARD: usize = 3;
/// Step-up receipt lifetime.
pub const RECEIPT_TTL: Duration = Duration::from_secs(120);
/// Largest PoW or attestation body.
pub const MAX_SMALL_BODY: usize = 4096;
/// Largest solve body.
pub const MAX_SOLVE_BODY: usize = 1 << 15;
/// Retained rate-history keys per meter.
pub const METER_MAX_KEYS: usize = 32_768;
/// Retained hits per rate-history key.
pub const METER_MAX_HITS: usize = 64;
/// Largest rate-history key in bytes.
pub const METER_MAX_KEY_BYTES: usize = 512;

/// Live-feed event kind for an attempt.
pub const EVENT_ATTEMPT: &str = "pass.attempt";

/// Instance TTL reported to clients: two buckets.
const EXPIRES_IN_SECS: u64 = PASS_WINDOW_SECS * 2;

/// Time source. Wall time derives challenge, proof-of-work and attestation
/// buckets and receipt expiry; monotonic time drives retention windows.
pub trait Clock: Send + Sync {
    /// Time since the Unix epoch.
    fn wall(&self) -> Duration;
    /// Time since an arbitrary process-local origin; never decreases.
    fn monotonic(&self) -> Duration;
}

/// Operating-system clock.
pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn wall(&self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }
    fn monotonic(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Session report store (Go `a.store`).
pub trait Reports: Send + Sync {
    /// Register a session so later collection attaches to it.
    fn ensure(&self, session: &str);
    /// Stored report, or the empty report when unknown (Go zero value).
    fn get(&self, session: &str) -> SessionReport;
    /// Persist a freshly scored report.
    fn store_scored(&self, session: &str, report: SessionReport);
}

/// Receives each completed Pass outcome with the report as it stood before
/// the attempt (Go `feedPassOutcome`). It must never fail the request.
pub type OutcomeFeed = Arc<dyn Fn(&SessionReport, bool) + Send + Sync>;

/// No outcome consumer.
pub fn no_feed() -> OutcomeFeed {
    Arc::new(|_, _| {})
}

/// Outcome feed into the correction controller through the shared
/// [`feed_pass_outcome`] kernel. `predict` scores the stored behavior summary
/// (Go `mlserve.Score(behavior.Extract(..))`), `traces` receives labeled
/// human traces, and the engine verdict DENY is the STUDD teacher. Controller
/// errors are monitor-only and never reach the request.
pub fn correction_feed(
    controller: Option<Arc<Controller>>,
    predict: Arc<dyn Fn(&BehaviorSummary) -> Prediction + Send + Sync>,
    traces: Arc<dyn Fn(HumanTrace) + Send + Sync>,
) -> OutcomeFeed {
    Arc::new(move |report: &SessionReport, solved: bool| {
        let behavior = &report.client.behavior;
        let ts = report.timestamp.0.unix_timestamp();
        let engine_denied = report.scoring.verdict == "DENY";
        // The labeled trace does not depend on the controller (Go appended
        // it before consulting the control plane).
        if let Ok(fed) = feed_pass_outcome(
            None,
            behavior,
            ts,
            solved,
            Prediction::abstained(),
            engine_denied,
        ) && let Some(trace) = fed.trace
        {
            traces(trace);
        }
        if let Some(controller) = controller.as_deref() {
            let prediction = if solved {
                predict(behavior)
            } else {
                Prediction::abstained()
            };
            let _ = feed_pass_outcome(
                Some(controller),
                behavior,
                ts,
                solved,
                prediction,
                engine_denied,
            );
        }
    })
}

/// Injected collaborators.
pub struct Dependencies {
    /// Frozen scoring engine.
    pub engine: Arc<Engine>,
    /// Session report store.
    pub reports: Arc<dyn Reports>,
    /// Time source.
    pub clock: Arc<dyn Clock>,
    /// Outcome consumer.
    pub feed: OutcomeFeed,
    /// Live telemetry hub, when the observatory is enabled.
    pub hub: Option<Arc<Hub>>,
    /// Shared Gate key for step-up receipts, when behind a Gate.
    pub step_up: Option<Arc<TokenKey>>,
}

/// Per-request facts the HTTP owner derives from the connection.
#[derive(Debug, Clone, Copy, Default)]
pub struct PassContext<'a> {
    /// Stable fingerprint key; see [`scoring::fingerprint`].
    pub fingerprint: &'a str,
    /// [`scoring::REDTEAM_HEADER`] value, if present.
    pub redteam: Option<&'a str>,
}

/// Failure that refuses a request instead of degrading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowError {
    /// Rate history could not count the request.
    Meter(Unavailable),
    /// Pass store refusal.
    Store(Refusal),
    /// The engine refused to score.
    Scoring,
}

impl FlowError {
    fn response(self) -> Response {
        match self {
            Self::Store(Refusal::SessionId) => Response::text(400, "bad session"),
            Self::Store(Refusal::Label) => Response::text(400, "bad red-team label"),
            Self::Scoring => Response::text(500, "scoring unavailable"),
            Self::Meter(_) | Self::Store(_) => Response::text(503, "challenge unavailable"),
        }
    }
}

/// Bounded sliding-window meter over the shared rate history. Time is
/// clamped to the meter's high-water mark so concurrent requests that read
/// the clock out of lock order never present a regression.
struct Meter {
    inner: Mutex<(Limiter, Duration)>,
}

impl Meter {
    fn new(window: Duration, soft: usize, hard: usize) -> Self {
        let limits = Limits::new(
            METER_MAX_KEYS,
            METER_MAX_HITS,
            METER_MAX_KEY_BYTES,
            window,
            METER_MAX_KEYS * (METER_MAX_HITS * size_of::<Duration>() + METER_MAX_KEY_BYTES),
            64,
        )
        .and_then(|limits| Config::new(window, soft, hard, limits))
        .expect("static meter configuration is valid");
        Self {
            inner: Mutex::new((Limiter::new(limits), Duration::ZERO)),
        }
    }

    fn observe(&self, key: &str, now: Duration) -> Result<Level, FlowError> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let (limiter, last) = &mut *guard;
        let now = now.max(*last);
        limiter.gc(now).map_err(FlowError::Meter)?;
        let observation = limiter.observe(key, now).map_err(FlowError::Meter)?;
        *last = now;
        Ok(match observation {
            Observation::IgnoredEmptyKey => Level::Normal,
            Observation::Counted { level, .. } => level,
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AttestBody {
    challenge_nonce: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PowBody {
    bucket: u64,
    nonce: String,
    challenge_nonce: String,
}

/// The Pass surface.
pub struct PassFlow {
    master_key: Zeroizing<Vec<u8>>,
    web_dir: PathBuf,
    deps: Dependencies,
    store: PassStore,
    velocity: Meter,
    attest: Meter,
}

/// Binds the proof of work to the session and this instance's nonce, so a
/// solved PoW cannot be reused across instances (Go `passPoWSession`).
pub fn pow_session(session: &str, nonce: &str) -> String {
    format!("{session}|pass|{nonce}")
}

fn bucket(wall: Duration, window: u64) -> u64 {
    wall.as_secs() / window
}

fn ok_json(value: Value) -> Response {
    Response::json(&value)
}

fn decode<T: for<'de> Deserialize<'de>>(body: &[u8], limit: usize) -> Option<T> {
    if body.len() > limit {
        return None;
    }
    serde_json::from_slice(body).ok()
}

fn short_session(session: &str) -> &str {
    let end = (0..=session.len().min(8))
        .rev()
        .find(|index| session.is_char_boundary(*index))
        .unwrap_or(0);
    &session[..end]
}

impl PassFlow {
    /// Build the surface; `web_dir` holds `pass.html`.
    pub fn new(master_key: Vec<u8>, web_dir: PathBuf, deps: Dependencies) -> Self {
        Self {
            master_key: Zeroizing::new(master_key),
            web_dir,
            deps,
            store: PassStore::new(),
            velocity: Meter::new(VELOCITY_WINDOW, VELOCITY_SOFT, VELOCITY_HARD),
            attest: Meter::new(ATTEST_WINDOW, ATTEST_SOFT, ATTEST_HARD),
        }
    }

    /// Transient Pass state.
    pub fn store(&self) -> &PassStore {
        &self.store
    }

    /// Answer a Pass request; `None` for paths outside the surface.
    pub fn handle(&self, request: &Request<'_>, context: &PassContext<'_>) -> Option<Response> {
        let result = match request.path {
            PAGE_PATH => self.page(request),
            NEW_PATH => self.new_instance(request, context),
            ATTEST_PATH => self.attest(request, context),
            POW_PATH => self.pow(request),
            SOLVE_PATH => self.solve(request, context),
            KPI_PATH => Ok(self.kpi()),
            _ => return None,
        };
        Some(result.unwrap_or_else(FlowError::response))
    }

    fn ensure_session(request: &Request<'_>) -> Option<(String, Option<String>)> {
        match session_from_cookie(request.cookie) {
            Some(session) => Some((session.to_owned(), None)),
            None => {
                let session = new_session().ok()?;
                let cookie = session_cookie(&session);
                Some((session, Some(cookie)))
            }
        }
    }

    fn page(&self, request: &Request<'_>) -> Result<Response, FlowError> {
        let Some((session, cookie)) = Self::ensure_session(request) else {
            return Ok(Response::text(500, "session unavailable"));
        };
        self.deps.reports.ensure(&session);
        Ok(Response {
            status: 200,
            headers: cookie.map(|c| ("Set-Cookie", c)).into_iter().collect(),
            body: Body::File(self.web_dir.join("pass.html")),
        })
    }

    /// Axis 3 (Go `applyPassVelocity`): meter session and fingerprint
    /// cadence and, on a newly reached level, fold the velocity signal into
    /// the session score. Returns (combined level, session-only level).
    fn apply_velocity(
        &self,
        session: &str,
        fingerprint: &str,
        monotonic: Duration,
    ) -> Result<(Level, Level), FlowError> {
        let session_level = self.velocity.observe(&format!("s|{session}"), monotonic)?;
        let fingerprint_level = self
            .velocity
            .observe(&format!("f|{fingerprint}"), monotonic)?;
        let level = if scoring::rank(fingerprint_level) > scoring::rank(session_level) {
            fingerprint_level
        } else {
            session_level
        };
        let Some(signal) = scoring::velocity_signal(level) else {
            return Ok((level, session_level));
        };
        let raised = self
            .store
            .with_session(session, monotonic, |state| {
                let previous = state.velocity_level;
                let rank = scoring::rank(level);
                if rank > previous {
                    state.velocity_level = rank;
                }
                rank > previous
            })
            .map_err(FlowError::Store)?;
        if raised {
            self.rescore(session, vec![signal])?;
        }
        Ok((level, session_level))
    }

    fn rescore(
        &self,
        session: &str,
        signals: Vec<humanymous_core::signals::observation::Signal>,
    ) -> Result<humanymous_core::signals::report::ScoreResult, FlowError> {
        let mut report = self.deps.reports.get(session);
        report
            .network
            .signals
            .get_or_insert_with(Vec::new)
            .extend(signals);
        let result = self
            .deps
            .engine
            .score(&mut report)
            .map_err(|_| FlowError::Scoring)?;
        self.deps.reports.store_scored(session, report);
        Ok(result)
    }

    fn new_instance(
        &self,
        request: &Request<'_>,
        context: &PassContext<'_>,
    ) -> Result<Response, FlowError> {
        let Some((session, cookie)) = Self::ensure_session(request) else {
            return Ok(Response::text(500, "challenge unavailable"));
        };
        let Ok(nonce) = PassNonce::generate() else {
            return Ok(Response::text(500, "challenge unavailable"));
        };
        let (wall, monotonic) = (self.deps.clock.wall(), self.deps.clock.monotonic());
        let (velocity, session_velocity) =
            self.apply_velocity(&session, context.fingerprint, monotonic)?;
        let difficulty = scoring::difficulty(
            context.redteam,
            self.deps.reports.get(&session).scoring.risk_score,
        );
        let challenge_bucket = bucket(wall, PASS_WINDOW_SECS);
        let nonce_hex = nonce.to_hex();
        let (instance, pow_difficulty, attest_required) = self
            .store
            .with_session(&session, monotonic, |state| {
                state.instance = state.instance.wrapping_add(1);
                state.bucket = challenge_bucket;
                state.issued_at = monotonic;
                state.tries = 0;
                state.solved = false;
                state.nonce = Some(nonce);
                state.pow_ok = false;
                state.difficulty = difficulty;
                state.pow_difficulty = scoring::pow_difficulty(difficulty, velocity);
                state.attest_required = session_velocity != Level::Normal;
                (state.instance, state.pow_difficulty, state.attest_required)
            })
            .map_err(FlowError::Store)?;
        let challenge = pass::generate(
            &self.master_key,
            &session,
            challenge_bucket,
            instance,
            difficulty as i32,
        );
        let pow = proof_of_work::issue(
            &self.master_key,
            &pow_session(&session, &nonce_hex),
            pow_difficulty as i32,
            bucket(wall, proof_of_work::WINDOW),
        );
        let mut response = ok_json(json!({
            "ok": true,
            "bucket": challenge_bucket,
            "difficulty": difficulty,
            "challenge": challenge,
            "challengeNonce": nonce_hex,
            "triesLeft": PASS_MAX_TRIES,
            "expiresInS": EXPIRES_IN_SECS,
            "preflight": {
                "pow": pow,
                "attestRequired": attest_required,
            },
        }));
        if let Some(cookie) = cookie {
            response.headers.push(("Set-Cookie", cookie));
        }
        Ok(response)
    }

    fn issued_nonce(&self, session: &str, monotonic: Duration) -> Result<String, FlowError> {
        self.store
            .with_session(session, monotonic, |state| {
                state
                    .nonce
                    .as_ref()
                    .map(PassNonce::to_hex)
                    .unwrap_or_default()
            })
            .map_err(FlowError::Store)
    }

    fn attest(
        &self,
        request: &Request<'_>,
        context: &PassContext<'_>,
    ) -> Result<Response, FlowError> {
        if request.method != "POST" {
            return Ok(Response::text(405, "POST only"));
        }
        let Some(session) = session_from_cookie(request.cookie) else {
            return Ok(Response::text(400, "no session"));
        };
        let Some(body) = decode::<AttestBody>(request.body, MAX_SMALL_BODY) else {
            return Ok(Response::text(400, "bad body"));
        };
        let (wall, monotonic) = (self.deps.clock.wall(), self.deps.clock.monotonic());
        let issued = self.issued_nonce(session, monotonic)?;
        if body.challenge_nonce.is_empty() || body.challenge_nonce != issued {
            return Ok(ok_json(json!({"ok": false, "reason": "stale challenge"})));
        }
        // The budget is keyed on the fingerprint a cookie-rotating flood
        // shares; the token binds to the session id.
        let level = self
            .attest
            .observe(&format!("att|{}", context.fingerprint), monotonic)?;
        if level == Level::Hard {
            return Ok(ok_json(json!({
                "ok": false,
                "reason": "attestation budget exhausted — retry shortly",
            })));
        }
        let token = attestation::issue(
            &self.master_key,
            session,
            &issued,
            bucket(wall, attestation::WINDOW),
        );
        Ok(ok_json(json!({"ok": true, "token": token})))
    }

    fn pow(&self, request: &Request<'_>) -> Result<Response, FlowError> {
        if request.method != "POST" {
            return Ok(Response::text(405, "POST only"));
        }
        let Some(session) = session_from_cookie(request.cookie) else {
            return Ok(Response::text(400, "no session"));
        };
        let Some(body) = decode::<PowBody>(request.body, MAX_SMALL_BODY) else {
            return Ok(Response::text(400, "bad body"));
        };
        let (wall, monotonic) = (self.deps.clock.wall(), self.deps.clock.monotonic());
        let (issued, pow_difficulty, solved) = self
            .store
            .with_session(session, monotonic, |state| {
                (
                    state
                        .nonce
                        .as_ref()
                        .map(PassNonce::to_hex)
                        .unwrap_or_default(),
                    state.pow_difficulty,
                    state.solved,
                )
            })
            .map_err(FlowError::Store)?;
        let ok = !solved
            && !issued.is_empty()
            && body.challenge_nonce == issued
            && proof_of_work::verify(
                &self.master_key,
                &pow_session(session, &issued),
                pow_difficulty as i32,
                body.bucket,
                bucket(wall, proof_of_work::WINDOW),
                &body.nonce,
            );
        if ok {
            self.store
                .with_session(session, monotonic, |state| {
                    if state.nonce.as_ref().map(PassNonce::to_hex).as_deref() == Some(&issued) {
                        state.pow_ok = true;
                    }
                })
                .map_err(FlowError::Store)?;
        }
        Ok(ok_json(json!({"ok": ok})))
    }

    fn receipt(&self, session: &str, wall: Duration) -> Option<String> {
        let key = self.deps.step_up.as_ref()?;
        key.issue_receipt(session, (wall + RECEIPT_TTL).as_secs())
            .ok()
    }

    fn publish(&self, session: &str, solved: bool, reason: &str) {
        if let Some(hub) = &self.deps.hub {
            hub.publish(
                EVENT_ATTEMPT,
                &json!({"sid": short_session(session), "solved": solved, "reason": reason}),
            );
        }
    }

    fn solve(
        &self,
        request: &Request<'_>,
        context: &PassContext<'_>,
    ) -> Result<Response, FlowError> {
        if request.method != "POST" {
            return Ok(Response::text(405, "POST only"));
        }
        let Some(session) = session_from_cookie(request.cookie) else {
            return Ok(Response::text(400, "no session"));
        };
        let Some(proof) = decode::<PassProof>(request.body, MAX_SOLVE_BODY) else {
            return Ok(Response::text(400, "bad body"));
        };
        if let Some(strategy) = context.redteam.filter(|value| !value.is_empty())
            && !valid_label(strategy)
        {
            return Err(FlowError::Store(Refusal::Label));
        }
        let (wall, monotonic) = (self.deps.clock.wall(), self.deps.clock.monotonic());

        enum Gate {
            AlreadySolved,
            Exhausted,
            Attempt(PassSession),
        }
        let gate = self
            .store
            .with_session(session, monotonic, |state| {
                if state.solved {
                    return Gate::AlreadySolved;
                }
                if state.exhausted() {
                    return Gate::Exhausted;
                }
                state.tries += 1;
                Gate::Attempt(state.clone())
            })
            .map_err(FlowError::Store)?;
        let attempt = match gate {
            Gate::AlreadySolved => {
                // Re-issue a fresh receipt so a benign receipt expiry recovers
                // without a full re-solve.
                let mut out = Map::new();
                out.insert("ok".into(), Value::Bool(true));
                out.insert("alreadySolved".into(), Value::Bool(true));
                if let Some(receipt) = self.receipt(session, wall) {
                    out.insert("stepUpReceipt".into(), Value::String(receipt));
                }
                return Ok(ok_json(Value::Object(out)));
            }
            Gate::Exhausted => {
                return Ok(ok_json(json!({
                    "ok": false,
                    "reason": "retry cap reached",
                    "triesLeft": 0,
                })));
            }
            Gate::Attempt(attempt) => attempt,
        };
        let PassSession {
            tries,
            difficulty,
            bucket: issued_bucket,
            instance,
            pow_ok,
            attest_required,
            ..
        } = attempt;
        let issued = attempt
            .nonce
            .as_ref()
            .map(PassNonce::to_hex)
            .unwrap_or_default();
        let tries_left = PASS_MAX_TRIES - tries;
        let current = bucket(wall, PASS_WINDOW_SECS);
        self.apply_velocity(session, context.fingerprint, monotonic)?;
        let before = self.deps.reports.get(session);
        let label = scoring::label(before.scoring.risk_score, context.redteam).to_owned();

        let reject = |why: &str| {
            self.record(&label, difficulty, false);
            (self.deps.feed)(&before, false);
            self.publish(session, false, why);
            ok_json(json!({"ok": false, "reason": why, "triesLeft": tries_left}))
        };

        // 1. Instance binding.
        if issued.is_empty() || proof.challenge_nonce != issued {
            return Ok(reject("stale or missing challenge"));
        }
        // 2. Crypto axis: the non-interactive proof must be paid. This
        // carries the keyboard lane where motor microstructure is inadmissible.
        if !pow_ok {
            return Ok(reject("no cryptographic preflight"));
        }
        // 2b. Identity gate when the session is flagged for velocity.
        if attest_required
            && !attestation::verify(
                &self.master_key,
                session,
                &issued,
                &proof.attest_token,
                bucket(wall, attestation::WINDOW),
            )
        {
            return Ok(reject("attestation required"));
        }
        // 3. The legacy real-event pre-filter is intentionally absent; see the
        // module documentation. 3b. Behavioral tells are fused soft.
        let behavior = scoring::behavior_signals(&proof);
        if !behavior.is_empty() {
            self.rescore(session, behavior)?;
        }
        // 5a. Expiry is not a wrong answer (ACC-1).
        if !pass::fresh(issued_bucket, current) {
            self.publish(session, false, "expired");
            return Ok(ok_json(json!({
                "ok": false,
                "expired": true,
                "reason": "this check expired — press New puzzle to start a fresh one",
            })));
        }
        // 5b. Server-side re-simulation of the placement.
        if !pass::verify(
            &self.master_key,
            session,
            issued_bucket,
            current,
            instance,
            difficulty as i32,
            &proof.offsets,
        ) {
            self.record(&label, difficulty, false);
            self.publish(session, false, "misaligned");
            return Ok(ok_json(json!({
                "ok": false,
                "reason": "the keys are not all in the slot",
                "triesLeft": tries_left,
            })));
        }
        // 5c. Anti-replay, reserved only after the solve verified.
        let Some(digest) = trace_digest(&proof) else {
            return Ok(reject("invalid interaction trace"));
        };
        if !self.store.reserve_trace(digest, monotonic) {
            return Ok(reject("duplicate interaction trace"));
        }
        self.record(&label, difficulty, true);

        // 6. Cleared: grant the trust upgrade and re-score.
        self.store
            .with_session(session, monotonic, |state| state.solved = true)
            .map_err(FlowError::Store)?;
        self.publish(session, true, "");
        let result = self.rescore(session, vec![scoring::solved_signal()])?;
        (self.deps.feed)(&before, true);
        let mut out = Map::new();
        out.insert("ok".into(), Value::Bool(true));
        out.insert("verdict".into(), Value::String(result.verdict));
        out.insert("riskScore".into(), json!(result.risk_score));
        if let Some(receipt) = self.receipt(session, wall) {
            out.insert("stepUpReceipt".into(), Value::String(receipt));
        }
        Ok(ok_json(Value::Object(out)))
    }

    /// KPI counting is telemetry; a full key space drops the count rather
    /// than failing an attempt whose enforcement already completed.
    fn record(&self, label: &str, difficulty: u32, passed: bool) {
        let _ = self.store.record(label, difficulty, passed);
    }

    fn kpi(&self) -> Response {
        ok_json(kpi(&self.store.metrics()))
    }
}

/// Label alphabet accepted by [`PassStore::record`].
fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= crate::gateway::pass_store::MAX_LABEL_BYTES
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
}

/// Wargame scoreboard from KPI counters (Go `handlePassKPI`).
pub fn kpi(metrics: &BTreeMap<MetricKey, u64>) -> Value {
    #[derive(Default, Clone, Copy)]
    struct Tally {
        passed: u64,
        attempts: u64,
    }
    fn rate(tally: Option<&Tally>) -> f64 {
        match tally {
            Some(tally) if tally.attempts > 0 => tally.passed as f64 / tally.attempts as f64,
            _ => 0.0,
        }
    }
    let mut by_label: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut by_difficulty: BTreeMap<String, Tally> = BTreeMap::new();
    let mut raw = Map::new();
    for (key, count) in metrics {
        raw.insert(key.to_string(), json!(count));
        let passed = if key.outcome == Outcome::Pass {
            *count
        } else {
            0
        };
        for tally in [
            by_label.entry(key.label.as_str()).or_default(),
            by_difficulty
                .entry(format!("d{}", key.difficulty))
                .or_default(),
        ] {
            tally.attempts += count;
            tally.passed += passed;
        }
    }
    let (mut bot_attempts, mut bot_passed) = (0_u64, 0_u64);
    let mut per_strategy = Map::new();
    for (label, tally) in &by_label {
        if !scoring::is_bot_label(label) {
            continue;
        }
        bot_attempts += tally.attempts;
        bot_passed += tally.passed;
        per_strategy.insert((*label).to_owned(), json!(rate(Some(tally))));
    }
    let bypass = if bot_attempts > 0 {
        bot_passed as f64 / bot_attempts as f64
    } else {
        0.0
    };
    let per_difficulty: Map<String, Value> = by_difficulty
        .iter()
        .map(|(difficulty, tally)| (difficulty.clone(), json!(rate(Some(tally)))))
        .collect();
    json!({
        "bypassRate": bypass,
        "botAttempts": bot_attempts,
        "perStrategy": per_strategy,
        "humanPassRate": rate(by_label.get("human")),
        "humanAttempts": by_label.get("human").map_or(0, |tally| tally.attempts),
        "unknownPassRate": rate(by_label.get("unknown")),
        "perDifficulty": per_difficulty,
        "raw": raw,
    })
}

#[cfg(test)]
mod tests;
