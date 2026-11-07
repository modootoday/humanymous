//! Bounded keyed request history. Runtime owns identities, clock and activation.
mod config;
use super::OrderedTime;
pub use super::Unavailable;
pub use config::{Config, Limits};
use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
    time::Duration,
};

/// Classification against configured count cutpoints, never a serving verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Count is below both configured cutpoints.
    Normal,
    /// Soft cutpoint matched, without matching hard.
    Soft,
    /// Hard cutpoint matched; takes precedence over soft.
    Hard,
}

/// This describes expansion gaps only, never general health or complete capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Actual retained observations, without a currently known expansion gap.
    Retained,
    /// A larger window cannot recover already discarded history.
    WindowExpanded {
        /// Gap applies strictly before this point, not at equality.
        complete_after: Duration,
    },
}

/// Empty required identity is distinct from a measured normal count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    /// No identity was counted and the clock did not advance.
    IgnoredEmptyKey,
    /// A request was appended to this key's retained history.
    Counted {
        /// Rolling count including this request.
        count: usize,
        /// Classification using current configured cutpoints.
        level: Level,
        /// Expansion-gap metadata only, not general capture health.
        coverage: Coverage,
    },
}

/// Policy update preserves hits and makes retrospective loss explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigurationEffect {
    /// Candidate window exceeded the previous configured window.
    pub window_expanded: bool,
    /// A retrospective expansion gap remains at the update time.
    pub history_incomplete: bool,
}

/// A bounded pass does not assert that every stale key was reclaimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GcResult {
    /// Keys actually visited in this bounded pass.
    pub scanned: usize,
    /// Visited keys expired strictly beyond TTL.
    pub removed: usize,
}

/// Pure history with exclusive mutable custody. Never evicts a recent key to
/// admit another, saturates a count, or clears state on admission failure.
pub struct Limiter {
    config: Config,
    hits: BTreeMap<String, Vec<Duration>>,
    clock: OrderedTime,
    reserved_bytes: usize,
    gc_cursor: Option<String>,
    incomplete_until: Option<Duration>,
}
impl Limiter {
    /// Create empty counters with validated explicit policy and resource bounds.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            hits: BTreeMap::new(),
            clock: OrderedTime::new(),
            reserved_bytes: 0,
            gc_cursor: None,
            incomplete_until: None,
        }
    }

    /// Hard wins if both inclusive cutpoints match, including equal cutpoints.
    pub fn level(&self, count: usize) -> Level {
        if count >= self.config.hard {
            Level::Hard
        } else if count >= self.config.soft {
            Level::Soft
        } else {
            Level::Normal
        }
    }

    /// Count the new request after expiring hits exactly at the window boundary.
    /// Unavailable means the caller must retain its missing-coverage state; later
    /// successful calls cannot prove that failed admissions were recorded.
    pub fn observe(&mut self, key: &str, now: Duration) -> Result<Observation, Unavailable> {
        self.clock.check(now)?;
        if key.is_empty() {
            return Ok(Observation::IgnoredEmptyKey);
        }
        if key.len() > self.config.limits.max_key_bytes {
            return Err(Unavailable::Budget);
        }
        let prior = self.hits.get(key);
        let retained = prior.map_or(0, |hits| {
            hits.iter()
                .filter(|time| now - **time < self.config.window)
                .count()
        });
        let count = retained
            .checked_add(1)
            .filter(|count| *count <= self.config.limits.max_hits_per_key)
            .ok_or(Unavailable::Budget)?;
        let mut reserved_bytes = self.reserved_bytes;
        if prior.is_none() {
            if self.hits.len() >= self.config.limits.max_keys {
                return Err(Unavailable::Budget);
            }
            reserved_bytes = reserved_bytes
                .checked_add(self.config.sample_bytes)
                .and_then(|bytes| bytes.checked_add(key.len()))
                .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
                .ok_or(Unavailable::Budget)?;
        }
        let coverage = match self.incomplete_until {
            Some(until) if now < until => Coverage::WindowExpanded {
                complete_after: until,
            },
            _ => Coverage::Retained,
        };
        let result = Observation::Counted {
            count,
            level: self.level(count),
            coverage,
        };
        let window = self.config.window;
        let capacity = self.config.limits.max_hits_per_key;
        let hits = self
            .hits
            .entry(key.into())
            .or_insert_with(|| Vec::with_capacity(capacity));
        hits.retain(|time| now - *time < window);
        hits.push(now);
        self.reserved_bytes = reserved_bytes;
        self.clock.commit(now);
        Ok(result)
    }

    /// No list is cleared or pruned. Resource limits cannot change here.
    pub fn configure(
        &mut self,
        window: Duration,
        soft: usize,
        hard: usize,
        now: Duration,
    ) -> Result<ConfigurationEffect, Unavailable> {
        self.clock.check(now)?;
        let candidate = Config::new(window, soft, hard, self.config.limits)?;
        let expanded = window > self.config.window;
        let mut incomplete_until = self.incomplete_until;
        if expanded {
            let until = now
                .checked_add(window)
                .ok_or(Unavailable::InvalidConfiguration)?;
            incomplete_until = Some(incomplete_until.map_or(until, |old| old.max(until)));
        }
        let effect = ConfigurationEffect {
            window_expanded: expanded,
            history_incomplete: incomplete_until.is_some_and(|until| now < until),
        };
        self.config = candidate;
        self.incomplete_until = incomplete_until;
        self.clock.commit(now);
        Ok(effect)
    }

    /// Deterministic cursor replaces the legacy random subset. Stable key sets
    /// progress across passes; no key is visited twice within a single pass.
    pub fn gc(&mut self, now: Duration) -> Result<GcResult, Unavailable> {
        self.clock.check(now)?;
        let budget = self.config.limits.gc_scan_budget.min(self.hits.len());
        let mut result = GcResult {
            scanned: 0,
            removed: 0,
        };
        for _ in 0..budget {
            let next = self
                .gc_cursor
                .as_ref()
                .and_then(|cursor| {
                    self.hits
                        .range::<str, _>((Excluded(cursor.as_str()), Unbounded))
                        .next()
                })
                .or_else(|| self.hits.first_key_value())
                .map(|(key, hits)| (key.clone(), hits.last().copied()));
            let Some((key, last)) = next else {
                break;
            };
            if last.is_none_or(|last| now - last > self.config.ttl) {
                self.hits.remove(&key);
                self.reserved_bytes -= self.config.sample_bytes + key.len();
                result.removed += 1;
            }
            self.gc_cursor = Some(key);
            result.scanned += 1;
        }
        self.clock.commit(now);
        Ok(result)
    }

    /// Authorized run isolation belongs to runtime. Reset retains policy,
    /// immutable limits, clock high-water and any pending expansion gap.
    pub fn reset(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.hits.clear();
        self.gc_cursor = None;
        self.reserved_bytes = 0;
        self.clock.commit(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
