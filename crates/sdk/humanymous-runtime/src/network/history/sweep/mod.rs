//! Decision-probing sweep detector (legacy internal/gate/recon.go, HR-30).
//!
//! Counts distinct sessions per client binding in a true sliding window. One
//! binding fanning out to more than `max_sessions` sessions is a recon pattern.
//! This is a measurement, not a verdict; Runtime decides what to do with it.
//! Unlike Go, bindings and per-binding sessions are bounded and time is ordered.
use super::{OrderedTime, Unavailable};
use std::collections::BTreeMap;
use std::time::Duration;

/// Explicit policy and resource bounds.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    window: Duration,
    max_sessions: usize,
    max_bindings: usize,
}

impl Config {
    /// Positive window, threshold and binding bound.
    pub fn new(
        window: Duration,
        max_sessions: usize,
        max_bindings: usize,
    ) -> Result<Self, Unavailable> {
        if window.is_zero() || max_sessions == 0 || max_bindings == 0 {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            window,
            max_sessions,
            max_bindings,
        })
    }
}

/// Result of one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sweep {
    /// No binding identity was supplied; nothing was counted.
    IgnoredEmptyBinding,
    /// Distinct sessions in the trailing window, and whether it exceeds the bound.
    Counted {
        /// Distinct sessions retained for this binding, including this one.
        sessions: usize,
        /// `sessions > max_sessions`.
        sweeping: bool,
    },
}

struct Binding {
    sessions: BTreeMap<String, Duration>,
    last: Duration,
}

/// Single-owner detector state.
pub struct Detector {
    config: Config,
    bindings: BTreeMap<String, Binding>,
    clock: OrderedTime,
}

impl Detector {
    /// Empty detector.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            bindings: BTreeMap::new(),
            clock: OrderedTime::new(),
        }
    }

    /// Record `(binding, session)` at `now`. Session growth per binding stops at
    /// one past the threshold, which is already a sweep.
    pub fn observe(
        &mut self,
        binding: &str,
        session: &str,
        now: Duration,
    ) -> Result<Sweep, Unavailable> {
        self.clock.check(now)?;
        if binding.is_empty() {
            return Ok(Sweep::IgnoredEmptyBinding);
        }
        if session.is_empty() {
            return Err(Unavailable::InvalidMeasurement);
        }
        if !self.bindings.contains_key(binding) && self.bindings.len() >= self.config.max_bindings {
            return Err(Unavailable::Budget);
        }
        self.clock.commit(now);
        let cut = now.saturating_sub(self.config.window);
        let entry = self
            .bindings
            .entry(binding.into())
            .or_insert_with(|| Binding {
                sessions: BTreeMap::new(),
                last: now,
            });
        entry.sessions.retain(|_, seen| *seen >= cut);
        if entry.sessions.contains_key(session) || entry.sessions.len() <= self.config.max_sessions
        {
            entry.sessions.insert(session.into(), now);
        }
        entry.last = now;
        let sessions = entry.sessions.len();
        Ok(Sweep::Counted {
            sessions,
            sweeping: sessions > self.config.max_sessions,
        })
    }

    /// Evict bindings idle for more than two windows.
    pub fn gc(&mut self, now: Duration) -> Result<usize, Unavailable> {
        self.clock.check(now)?;
        let limit = self.config.window.saturating_mul(2);
        let before = self.bindings.len();
        self.bindings
            .retain(|_, b| now.saturating_sub(b.last) <= limit);
        Ok(before - self.bindings.len())
    }
}

#[cfg(test)]
mod tests;
