//! Cohort behavioral shadow observer (legacy internal/gate/cohort_shadow.go at
//! baseline 981d25c, wargame R251-R260).
//!
//! Many distinct sessions behind one server-observed cohort key (fingerprint
//! plus subnet) that present the same coarse motor signature suggest a shared
//! synthetic behavior generator. This is a measurement only: it returns a
//! `Flagged` outcome instead of logging, and has no access to verdicts, bans or
//! enforcement. Promotion to any weight is a separate scoring decision.
//!
//! Deliberate fail-closed differences, each pinned by tests:
//! - Time is caller-ordered; a regression is refused before any mutation.
//! - Distinct signatures per cohort are bounded (Go grew them without limit).
//! - A non-finite moment yields no signature instead of a saturated bucket.
use super::{OrderedTime, Unavailable};
use humanymous_core::signals::report::BehaviorSummary;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Legacy distinct-session threshold for one cohort and signature.
pub const LEGACY_MIN_COHORT: usize = 6;
/// Legacy idle eviction for cohorts.
pub const LEGACY_IDLE: Duration = Duration::from_secs(10 * 60);
/// Legacy bound on retained cohorts.
pub const LEGACY_MAX_COHORTS: usize = 100_000;

/// Coarse bucketed motor signature; two cohorts collide only on equal buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signature([u16; 6]);

/// Quantize the motor moments exactly like Go: round each moment by its step
/// and keep the low 16 bits. `None` when there is no interaction at all or a
/// moment is not finite.
pub fn signature(b: &BehaviorSummary) -> Option<Signature> {
    if b.mouse.samples == 0 && b.key.keystrokes == 0 && b.events.total_events == 0 {
        return None;
    }
    let moments = [
        (b.mouse.mean_curvature, 0.05),
        (b.mouse.velocity_std_dev, 0.5),
        (b.mouse.coalesced_ratio, 0.5),
        (b.mouse.straight_line_frac, 0.1),
        (b.key.mean_flight_ms, 10.0),
        (b.key.flight_std_dev, 5.0),
    ];
    let mut parts = [0u16; 6];
    for (part, (value, step)) in parts.iter_mut().zip(moments) {
        let bucket = (value / step).round();
        if !bucket.is_finite() {
            return None;
        }
        // Go keeps byte(p), byte(p>>8) of the int bucket: the low 16 bits.
        *part = (bucket as i64) as u16;
    }
    Some(Signature(parts))
}

/// Explicit policy and resource bounds.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    min_cohort: usize,
    idle: Duration,
    max_cohorts: usize,
    max_signatures: usize,
}

impl Config {
    /// All values must be positive.
    pub fn new(
        min_cohort: usize,
        idle: Duration,
        max_cohorts: usize,
        max_signatures: usize,
    ) -> Result<Self, Unavailable> {
        if min_cohort == 0 || idle.is_zero() || max_cohorts == 0 || max_signatures == 0 {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            min_cohort,
            idle,
            max_cohorts,
            max_signatures,
        })
    }
}

/// Result of one observation; never a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cohort {
    /// Empty cohort key, `|`, empty session or no behavioral content.
    Ignored,
    /// Distinct sessions now sharing this signature, below the threshold.
    Counted(usize),
    /// Threshold reached for the first time; the session set is released.
    Flagged(usize),
    /// This cohort and signature were already flagged; nothing is retained.
    AlreadyFlagged,
    /// A resource bound refused the new cohort or signature.
    Saturated,
}

struct Entry {
    signatures: BTreeMap<Signature, BTreeSet<String>>,
    flagged: BTreeSet<Signature>,
    updated: Duration,
}

/// Single-owner observer state.
pub struct Observer {
    config: Config,
    cohorts: BTreeMap<String, Entry>,
    clock: OrderedTime,
}

impl Observer {
    /// Empty observer.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cohorts: BTreeMap::new(),
            clock: OrderedTime::new(),
        }
    }

    /// Record one session's coarse signature under its cohort.
    pub fn observe(
        &mut self,
        cohort: &str,
        behavior: &BehaviorSummary,
        session: &str,
        now: Duration,
    ) -> Result<Cohort, Unavailable> {
        self.clock.check(now)?;
        if cohort.is_empty() || cohort == "|" || session.is_empty() {
            return Ok(Cohort::Ignored);
        }
        let Some(signature) = signature(behavior) else {
            return Ok(Cohort::Ignored);
        };
        if !self.cohorts.contains_key(cohort) && self.cohorts.len() >= self.config.max_cohorts {
            self.evict_idle(now);
            if self.cohorts.len() >= self.config.max_cohorts {
                self.clock.commit(now);
                return Ok(Cohort::Saturated);
            }
        }
        self.clock.commit(now);
        let entry = self.cohorts.entry(cohort.into()).or_insert_with(|| Entry {
            signatures: BTreeMap::new(),
            flagged: BTreeSet::new(),
            updated: now,
        });
        entry.updated = now;
        if entry.flagged.contains(&signature) {
            return Ok(Cohort::AlreadyFlagged);
        }
        if !entry.signatures.contains_key(&signature)
            && entry.signatures.len() + entry.flagged.len() >= self.config.max_signatures
        {
            return Ok(Cohort::Saturated);
        }
        let sessions = entry.signatures.entry(signature).or_default();
        sessions.insert(session.into());
        let count = sessions.len();
        if count >= self.config.min_cohort {
            entry.signatures.remove(&signature);
            entry.flagged.insert(signature);
            return Ok(Cohort::Flagged(count));
        }
        Ok(Cohort::Counted(count))
    }

    /// Evict cohorts idle strictly longer than the configured idle period.
    pub fn gc(&mut self, now: Duration) -> Result<usize, Unavailable> {
        self.clock.check(now)?;
        let removed = self.evict_idle(now);
        self.clock.commit(now);
        Ok(removed)
    }

    /// Number of retained cohorts.
    pub fn cohorts(&self) -> usize {
        self.cohorts.len()
    }

    /// Distinct sessions retained for one cohort and signature (zero once flagged).
    pub fn retained_sessions(&self, cohort: &str, signature: Signature) -> usize {
        self.cohorts
            .get(cohort)
            .and_then(|entry| entry.signatures.get(&signature))
            .map_or(0, BTreeSet::len)
    }

    /// Whether this cohort and signature have been flagged.
    pub fn is_flagged(&self, cohort: &str, signature: Signature) -> bool {
        self.cohorts
            .get(cohort)
            .is_some_and(|entry| entry.flagged.contains(&signature))
    }

    fn evict_idle(&mut self, now: Duration) -> usize {
        let before = self.cohorts.len();
        let idle = self.config.idle;
        self.cohorts
            .retain(|_, entry| now.saturating_sub(entry.updated) <= idle);
        before - self.cohorts.len()
    }
}

#[cfg(test)]
mod tests;
