//! Sticky per-session verdicts with a coherence TTL (legacy internal/gate
//! enforce.go `VerdictStore` at baseline 981d25c, SoT-21 §2).
//!
//! This is the in-process store only. It grants nothing: an absent or stale
//! entry reads as `Unknown`, and what an enforcement owner does with `Unknown`
//! is not decided here. The legacy verdict-to-action mapping let `Unknown` pass
//! on safe public routes; that fail-open rule is deliberately not ported.
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// Hard cap on tracked sessions (Go `maxVerdicts`).
pub const MAX_VERDICTS: usize = 1 << 20;

/// Edge verdict, with the legacy wire names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verdict {
    /// `allow`.
    Allow,
    /// `challenge`.
    Challenge,
    /// `deny`.
    Deny,
    /// `none`: no evidence yet.
    #[default]
    Unknown,
}

impl Verdict {
    /// Map an engine verdict (`ALLOW`/`CHALLENGE`/`DENY`); anything else is
    /// `Unknown`.
    pub fn from_engine(engine: &str) -> Self {
        match engine {
            "DENY" => Self::Deny,
            "CHALLENGE" => Self::Challenge,
            "ALLOW" => Self::Allow,
            _ => Self::Unknown,
        }
    }
    /// Legacy wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Challenge => "challenge",
            Self::Deny => "deny",
            Self::Unknown => "none",
        }
    }
}

/// A cached scoring outcome for one session.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sticky {
    /// Edge verdict.
    pub verdict: Verdict,
    /// Risk score that produced it.
    pub risk: f64,
    /// Hard rule that fired, if any.
    pub rule: String,
    /// Top contributing signal ids.
    pub top: Vec<String>,
    /// When it was recorded (monotonic time supplied by the caller).
    pub updated: Duration,
}

/// Bounded, TTL-coherent sticky-verdict store.
pub struct VerdictStore {
    entries: Mutex<HashMap<String, Sticky>>,
    ttl: Duration,
    capacity: usize,
}

impl VerdictStore {
    /// Store with a coherence `ttl`; capacity is `MAX_VERDICTS`.
    pub fn new(ttl: Duration) -> Self {
        Self::with_capacity(ttl, MAX_VERDICTS)
    }
    /// Store with an explicit capacity, clamped to `1..=MAX_VERDICTS`.
    pub fn with_capacity(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            capacity: capacity.clamp(1, MAX_VERDICTS),
        }
    }
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Sticky>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Record or update a session's verdict. At capacity a new session is not
    /// tracked (it reads as `Unknown` and is re-scored); existing ones update.
    /// Returns whether the verdict is now stored.
    pub fn set(&self, sid: &str, sticky: Sticky) -> bool {
        let mut entries = self.lock();
        if !entries.contains_key(sid) && entries.len() >= self.capacity {
            return false;
        }
        entries.insert(sid.to_owned(), sticky);
        true
    }
    /// The session's verdict, or an `Unknown` default when absent or older
    /// than the TTL at `now`.
    pub fn get(&self, sid: &str, now: Duration) -> Sticky {
        self.lock()
            .get(sid)
            .filter(|s| now.saturating_sub(s.updated) <= self.ttl)
            .cloned()
            .unwrap_or_default()
    }
    /// Evict every entry older than the TTL at `now`.
    pub fn gc(&self, now: Duration) {
        let ttl = self.ttl;
        self.lock()
            .retain(|_, s| now.saturating_sub(s.updated) <= ttl);
    }
    /// Number of tracked sessions.
    pub fn len(&self) -> usize {
        self.lock().len()
    }
    /// Whether no session is tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests;
