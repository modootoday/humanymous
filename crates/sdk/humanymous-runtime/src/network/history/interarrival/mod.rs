//! Per-fingerprint inter-arrival shadow observer (legacy
//! internal/gate/anomaly_shadow.go at baseline 981d25c, wargame R261-R266).
//!
//! Feeds the whole-millisecond gap between consecutive requests of one
//! fingerprint into the bounded median/MAD history and returns the measurement
//! instead of logging it. It has no access to verdicts, bans or enforcement.
//!
//! Deliberate fail-closed differences, each pinned by tests:
//! - The last-seen map is a hard bound: when it is full after evicting idle
//!   fingerprints, a new fingerprint is refused (Go swept and kept growing).
//! - Time is caller-ordered; a regression is refused before any mutation.
use super::Unavailable;
use super::anomaly::{Config as AnomalyConfig, Detector, Limits, Measurement};
use std::collections::BTreeMap;
use std::time::Duration;

/// Legacy idle eviction for fingerprints and their history.
pub const LEGACY_IDLE: Duration = Duration::from_secs(10 * 60);
/// Legacy bound on the last-seen map.
pub const LEGACY_MAX_FINGERPRINTS: usize = 200_000;

/// The legacy shadow detector parameters (window 64, K 6, warmup 12, MAD
/// floor 2) under caller-owned resource limits.
pub fn legacy_detector(limits: Limits) -> Result<AnomalyConfig, Unavailable> {
    AnomalyConfig::new(64, 6.0, 12, 2.0, limits)
}

/// Result of one observation; never a verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum Arrival {
    /// Empty fingerprint; nothing recorded.
    Ignored,
    /// First request from this fingerprint; no gap yet.
    First,
    /// Gap in whole milliseconds and its measurement against prior history.
    Measured {
        /// Gap since the previous request of this fingerprint.
        gap_ms: u128,
        /// Median/MAD measurement before appending the gap.
        measurement: Measurement,
    },
    /// The last-seen bound refused a new fingerprint.
    Saturated,
}

/// Single-owner observer state.
pub struct Observer {
    detector: Detector,
    last: BTreeMap<String, Duration>,
    idle: Duration,
    max_fingerprints: usize,
    newest: Option<Duration>,
}

impl Observer {
    /// Positive idle period and fingerprint bound.
    pub fn new(
        detector: AnomalyConfig,
        idle: Duration,
        max_fingerprints: usize,
    ) -> Result<Self, Unavailable> {
        if idle.is_zero() || max_fingerprints == 0 {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            detector: Detector::new(detector),
            last: BTreeMap::new(),
            idle,
            max_fingerprints,
            newest: None,
        })
    }

    fn check(&self, now: Duration) -> Result<(), Unavailable> {
        if self.newest.is_some_and(|newest| now < newest) {
            return Err(Unavailable::TimeRegression);
        }
        Ok(())
    }

    /// Record one request from `fingerprint` at `now`.
    pub fn observe(&mut self, fingerprint: &str, now: Duration) -> Result<Arrival, Unavailable> {
        self.check(now)?;
        if fingerprint.is_empty() {
            return Ok(Arrival::Ignored);
        }
        let Some(previous) = self.last.get(fingerprint).copied() else {
            if self.last.len() >= self.max_fingerprints {
                self.evict_idle(now);
                if self.last.len() >= self.max_fingerprints {
                    self.newest = Some(now);
                    return Ok(Arrival::Saturated);
                }
            }
            self.last.insert(fingerprint.into(), now);
            self.newest = Some(now);
            return Ok(Arrival::First);
        };
        let gap_ms = (now - previous).as_millis();
        let measurement = self.detector.observe(fingerprint, gap_ms as f64, now)?;
        self.last.insert(fingerprint.into(), now);
        self.newest = Some(now);
        Ok(Arrival::Measured {
            gap_ms,
            measurement,
        })
    }

    /// Evict fingerprints and history idle strictly longer than the idle period.
    pub fn gc(&mut self, now: Duration) -> Result<usize, Unavailable> {
        self.check(now)?;
        self.detector.gc(now, self.idle)?;
        let removed = self.evict_idle(now);
        self.newest = Some(now);
        Ok(removed)
    }

    /// Number of fingerprints with a retained last-seen time.
    pub fn fingerprints(&self) -> usize {
        self.last.len()
    }

    fn evict_idle(&mut self, now: Duration) -> usize {
        let before = self.last.len();
        let idle = self.idle;
        self.last
            .retain(|_, seen| now.saturating_sub(*seen) <= idle);
        before - self.last.len()
    }
}

#[cfg(test)]
mod tests;
