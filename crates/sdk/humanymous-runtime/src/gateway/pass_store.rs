//! Transient Pass challenge state, the trace anti-replay registry, the
//! per-instance challenge nonce, the wargame KPI counters and their
//! aggregation (shipping row B08; legacy cmd/server/pass_store.go and the
//! `handlePassKPI` aggregation of pass_handler.go at baseline 981d25c).
//!
//! The store only stores. It performs no scoring, no timing or motor analysis
//! and never gates on motor richness or speed: the trace registry receives an
//! opaque digest computed elsewhere and answers only "seen before inside the
//! retention window". Every collection is bounded and the caller supplies a
//! monotonic time (`Duration` since an arbitrary process-local origin).
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - At the session cap Go served an unstored, ephemeral session (so the
//!   retry cap was not enforced for it); here the request is refused with
//!   [`Refusal::Saturated`].
//! - Session ids are bounded (1..=128 bytes); Go accepted any string.
//! - KPI labels come from a request header in Go and grew the metric map
//!   without bound. Labels are validated (1..=64 bytes of `[A-Za-z0-9-_.:/]`)
//!   and the number of distinct metric keys is capped; overflow is refused.
//! - Session state is mutated inside [`PassStore::with_session`] under the
//!   store lock; Go handed out a shared pointer mutated outside it.
//! - A time earlier than a stored stamp never expires that entry.
//!
//! Kept from Go: the trace sweep is amortized to once per `TRACE_TTL / 4`
//! (or eagerly at saturation), so an expired digest stays a replay until the
//! next sweep; the replay window errs long, never short.
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;

/// Seconds per challenge bucket (challenge TTL granularity).
pub const PASS_WINDOW_SECS: u64 = 30;
/// Hard retry cap per challenge instance; failure requires a new instance.
pub const PASS_MAX_TRIES: u32 = 4;
/// Anti-replay retention: a trace digest is admissible once inside it.
pub const TRACE_TTL: Duration = Duration::from_secs(10 * 60);
/// Hard cap on retained trace digests.
pub const MAX_TRACES: usize = 1 << 19;
/// Hard cap on retained Pass sessions.
pub const MAX_PASS_SESSIONS: usize = 50_000;
/// Sessions older than this are swept when the cap is under pressure.
pub const SESSION_STALE: Duration = Duration::from_secs(30 * 60);
/// Hard cap on distinct `label|outcome|difficulty` KPI keys.
pub const MAX_METRIC_KEYS: usize = 4096;
/// Upper bound on a session id in bytes.
pub const MAX_SESSION_ID_BYTES: usize = 128;
/// Upper bound on a KPI label in bytes.
pub const MAX_LABEL_BYTES: usize = 64;
/// Entropy bytes in a challenge nonce (hex encoded to 36 characters).
pub const NONCE_BYTES: usize = 18;

/// Static store refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Session id empty or longer than [`MAX_SESSION_ID_BYTES`].
    SessionId,
    /// Session store is full of fresh sessions.
    Saturated,
    /// KPI label empty, too long or outside the allowed alphabet.
    Label,
    /// KPI key space is exhausted.
    MetricsFull,
    /// Operating-system entropy unavailable.
    Entropy,
}

impl Refusal {
    /// Stable code.
    pub fn code(self) -> &'static str {
        match self {
            Self::SessionId => "pass_session_id",
            Self::Saturated => "pass_store_saturated",
            Self::Label => "pass_kpi_label",
            Self::MetricsFull => "pass_kpi_full",
            Self::Entropy => "entropy",
        }
    }
}

/// Per-instance challenge nonce; binds the crypto proof and the solve.
#[derive(Clone, PartialEq, Eq)]
pub struct PassNonce([u8; NONCE_BYTES]);

impl PassNonce {
    /// Nonce from caller-supplied entropy (deterministic tests, other RNGs).
    pub fn from_entropy(bytes: [u8; NONCE_BYTES]) -> Self {
        Self(bytes)
    }
    /// Fresh nonce from operating-system entropy.
    pub fn generate() -> Result<Self, Refusal> {
        let mut bytes = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| Refusal::Entropy)?;
        Ok(Self(bytes))
    }
    /// Lowercase hex, as Go `hex.EncodeToString`.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for PassNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PassNonce(..)")
    }
}

/// Opaque digest of an accepted trace; computed by the proof layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceDigest(pub [u8; 32]);

/// Per-session Pass state. Fields are stored verbatim for the handler layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassSession {
    /// Challenge bucket (`now_secs / PASS_WINDOW_SECS`) of the instance.
    pub bucket: u64,
    /// Increments per new instance so every puzzle is fresh.
    pub instance: u64,
    /// Puzzle difficulty of the instance.
    pub difficulty: u32,
    /// Creation (or last re-issue) time; used only for stale sweeping.
    pub issued_at: Duration,
    /// Attempts against the current instance.
    pub tries: u32,
    /// Whether the instance was solved.
    pub solved: bool,
    /// Current instance nonce, if issued.
    pub nonce: Option<PassNonce>,
    /// Proof-of-work leading-zero-bit target for the instance.
    pub pow_difficulty: u32,
    /// Proof-of-work axis satisfied for the instance.
    pub pow_ok: bool,
    /// Highest attempt-rate level flagged (0 ok, 1 elevated, 2 flood).
    pub velocity_level: u8,
    /// Instance additionally requires a rate-limited attestation token.
    pub attest_required: bool,
}

impl PassSession {
    fn fresh(now: Duration) -> Self {
        Self {
            bucket: 0,
            instance: 0,
            difficulty: 0,
            issued_at: now,
            tries: 0,
            solved: false,
            nonce: None,
            pow_difficulty: 0,
            pow_ok: false,
            velocity_level: 0,
            attest_required: false,
        }
    }
    /// Whether the retry cap for this instance is reached.
    pub fn exhausted(&self) -> bool {
        self.tries >= PASS_MAX_TRIES
    }
}

/// Attempt outcome recorded in the KPI counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// Attempt failed.
    Fail,
    /// Attempt passed.
    Pass,
}

/// One KPI counter key; renders as Go's `label|outcome|dN`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetricKey {
    /// Validated label.
    pub label: String,
    /// Outcome.
    pub outcome: Outcome,
    /// Difficulty.
    pub difficulty: u32,
}

impl fmt::Display for MetricKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let outcome = match self.outcome {
            Outcome::Fail => "fail",
            Outcome::Pass => "pass",
        };
        write!(f, "{}|{outcome}|d{}", self.label, self.difficulty)
    }
}

struct Inner {
    sessions: HashMap<String, PassSession>,
    metrics: BTreeMap<MetricKey, u64>,
    traces: HashMap<TraceDigest, Duration>,
    last_sweep: Option<Duration>,
}

/// Mutex-guarded transient Pass state.
pub struct PassStore {
    inner: Mutex<Inner>,
}

impl Default for PassStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PassStore {
    /// Empty store.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                sessions: HashMap::new(),
                metrics: BTreeMap::new(),
                traces: HashMap::new(),
                last_sweep: None,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panicked holder cannot leave a half-applied invariant here beyond
        // a counter or flag; continue with the recovered state.
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Register a trace digest; `true` when fresh inside [`TRACE_TTL`]. A
    /// replay collides and returns `false`. At [`MAX_TRACES`] the registry
    /// fails closed (treats the digest as a replay) until expiry drains it.
    pub fn reserve_trace(&self, digest: TraceDigest, now: Duration) -> bool {
        let mut inner = self.lock();
        let due = inner
            .last_sweep
            .is_none_or(|last| now.saturating_sub(last) > TRACE_TTL / 4);
        if due || inner.traces.len() >= MAX_TRACES {
            inner
                .traces
                .retain(|_, seen| now.saturating_sub(*seen) <= TRACE_TTL);
            inner.last_sweep = Some(now);
        }
        if inner.traces.contains_key(&digest) || inner.traces.len() >= MAX_TRACES {
            return false;
        }
        inner.traces.insert(digest, now);
        true
    }

    /// Run `apply` on the session's state under the store lock, creating it
    /// stamped `now` on first use. Under pressure stale sessions are swept;
    /// a store full of fresh sessions refuses rather than growing.
    pub fn with_session<R>(
        &self,
        session_id: &str,
        now: Duration,
        apply: impl FnOnce(&mut PassSession) -> R,
    ) -> Result<R, Refusal> {
        if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_BYTES {
            return Err(Refusal::SessionId);
        }
        let mut inner = self.lock();
        if !inner.sessions.contains_key(session_id) && inner.sessions.len() >= MAX_PASS_SESSIONS {
            inner
                .sessions
                .retain(|_, s| now.saturating_sub(s.issued_at) <= SESSION_STALE);
            if inner.sessions.len() >= MAX_PASS_SESSIONS {
                return Err(Refusal::Saturated);
            }
        }
        let session = inner
            .sessions
            .entry(session_id.to_owned())
            .or_insert_with(|| PassSession::fresh(now));
        Ok(apply(session))
    }

    /// Number of retained sessions.
    pub fn session_count(&self) -> usize {
        self.lock().sessions.len()
    }

    /// Count one attempt into the KPI counters.
    pub fn record(&self, label: &str, difficulty: u32, passed: bool) -> Result<(), Refusal> {
        if label.is_empty()
            || label.len() > MAX_LABEL_BYTES
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
        {
            return Err(Refusal::Label);
        }
        let key = MetricKey {
            label: label.to_owned(),
            outcome: if passed { Outcome::Pass } else { Outcome::Fail },
            difficulty,
        };
        let mut inner = self.lock();
        if !inner.metrics.contains_key(&key) && inner.metrics.len() >= MAX_METRIC_KEYS {
            return Err(Refusal::MetricsFull);
        }
        let count = inner.metrics.entry(key).or_insert(0);
        *count = count.saturating_add(1);
        Ok(())
    }

    /// Consistent snapshot of the KPI counters, ordered by key.
    pub fn metrics(&self) -> BTreeMap<MetricKey, u64> {
        self.lock().metrics.clone()
    }

    /// Wargame scoreboard aggregated from one snapshot (legacy
    /// `handlePassKPI`): per-label and per-difficulty pass rates, the bot
    /// bypass rate over every label but `human` and `unknown`, and the raw
    /// counters keyed `label|outcome|dN`.
    pub fn kpi(&self) -> Kpi {
        Kpi::from_metrics(&self.metrics())
    }
}

/// Pass wargame KPIs with the legacy JSON field names.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Kpi {
    /// Share of bot-labelled attempts that passed; drive it down.
    pub bypass_rate: f64,
    /// Bot-labelled attempts.
    pub bot_attempts: u64,
    /// Pass rate per bot label (red-team strategy or `bot`).
    pub per_strategy: BTreeMap<String, f64>,
    /// Human pass-rate floor.
    pub human_pass_rate: f64,
    /// Human-labelled attempts.
    pub human_attempts: u64,
    /// Pass rate of the ambiguous band.
    pub unknown_pass_rate: f64,
    /// Pass rate per `dN` difficulty.
    pub per_difficulty: BTreeMap<String, f64>,
    /// Raw counters.
    pub raw: BTreeMap<String, u64>,
}

#[derive(Default, Clone, Copy)]
struct Tally {
    passed: u64,
    attempts: u64,
}

impl Tally {
    fn add(&mut self, count: u64, passed: bool) {
        self.attempts = self.attempts.saturating_add(count);
        if passed {
            self.passed = self.passed.saturating_add(count);
        }
    }
    fn rate(self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.passed as f64 / self.attempts as f64
        }
    }
}

impl Kpi {
    /// Aggregate a counter snapshot exactly as Go did.
    pub fn from_metrics(metrics: &BTreeMap<MetricKey, u64>) -> Self {
        let mut by_label: BTreeMap<&str, Tally> = BTreeMap::new();
        let mut by_difficulty: BTreeMap<String, Tally> = BTreeMap::new();
        let mut raw = BTreeMap::new();
        for (key, count) in metrics {
            let passed = key.outcome == Outcome::Pass;
            by_label
                .entry(key.label.as_str())
                .or_default()
                .add(*count, passed);
            by_difficulty
                .entry(format!("d{}", key.difficulty))
                .or_default()
                .add(*count, passed);
            raw.insert(key.to_string(), *count);
        }
        let mut bots = Tally::default();
        let mut per_strategy = BTreeMap::new();
        for (label, tally) in &by_label {
            if !crate::gateway::pass_scoring::is_bot_label(label) {
                continue;
            }
            bots.attempts = bots.attempts.saturating_add(tally.attempts);
            bots.passed = bots.passed.saturating_add(tally.passed);
            per_strategy.insert((*label).to_owned(), tally.rate());
        }
        let human = by_label.get("human").copied().unwrap_or_default();
        Self {
            bypass_rate: bots.rate(),
            bot_attempts: bots.attempts,
            per_strategy,
            human_pass_rate: human.rate(),
            human_attempts: human.attempts,
            unknown_pass_rate: by_label.get("unknown").copied().unwrap_or_default().rate(),
            per_difficulty: by_difficulty
                .into_iter()
                .map(|(difficulty, tally)| (difficulty, tally.rate()))
                .collect(),
            raw,
        }
    }
}

#[cfg(test)]
mod tests;
