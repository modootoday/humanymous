use super::{Config, FleetInput, Scope, Unavailable, velocity::Population};
use crate::history::OrderedTime;
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

#[derive(Clone, Default)]
struct Entry {
    subnets: BTreeSet<String>,
    sessions: BTreeSet<String>,
    arrivals: Vec<Duration>,
    updated: Duration,
}
impl Entry {
    fn bytes(&self) -> usize {
        self.subnets
            .iter()
            .chain(self.sessions.iter())
            .map(String::len)
            .sum::<usize>()
            + self.arrivals.len() * size_of::<Duration>()
    }
}
#[derive(Clone, Default)]
struct Fanout {
    fingerprints: BTreeSet<String>,
    subnets: BTreeSet<String>,
    updated: Duration,
}
impl Fanout {
    fn bytes(&self) -> usize {
        self.fingerprints
            .iter()
            .chain(self.subnets.iter())
            .map(String::len)
            .sum()
    }
}
/// Existing signals and count origin, never healthy admission coverage.
pub struct Outcome {
    /// Frozen local signals, including optional weight-zero velocity.
    pub signals: Vec<Signal>,
    /// Origin/degradation of the observed counts, not a fleet attestation.
    pub scope: Scope,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Local counts without synthesizing zero for absent history.
pub enum Stats {
    /// No retained entry exists for this key.
    Absent,
    /// Counts from the retained local entry; GC is explicit.
    Observed {
        /// Distinct nonempty subnet keys.
        subnets: usize,
        /// Distinct supplied session identifiers.
        sessions: usize,
    },
}
/// Bounded local correlation history with one generic MAD population.
pub struct Registry {
    config: Config,
    entries: BTreeMap<String, Entry>,
    fanout: BTreeMap<String, Fanout>,
    population: Population,
    clock: OrderedTime,
    bytes: usize,
    fallbacks: u64,
}
impl Registry {
    /// Construct empty history; runtime owns synchronization and activation.
    pub fn new(config: Config) -> Self {
        let population = Population::new(config.population);
        Self {
            config,
            entries: BTreeMap::new(),
            fanout: BTreeMap::new(),
            population,
            clock: OrderedTime::new(),
            bytes: 0,
            fallbacks: 0,
        }
    }
    /// Local load-bearing counts never consume reported coordinator totals.
    pub fn observe(
        &mut self,
        key: &str,
        subnet: &str,
        session: &str,
        now: Duration,
        fleet: FleetInput,
    ) -> Result<Outcome, Unavailable> {
        self.clock.check(now)?;
        self.config.limits.fields(&[key, subnet, session])?;
        if key.is_empty() || key == "|" {
            return Ok(Outcome {
                signals: Vec::new(),
                scope: Scope::MissingKey,
            });
        }
        let previous = self.entries.get(key);
        if previous.is_none() && self.entries.len() >= self.config.limits.max_keys {
            return Err(Unavailable::Budget);
        }
        let mut candidate = previous.cloned().unwrap_or_default();
        let new_subnet = !subnet.is_empty() && candidate.subnets.insert(subnet.into());
        if new_subnet && candidate.arrivals.len() < 512 {
            candidate.arrivals.push(now);
        }
        candidate.sessions.insert(session.into());
        candidate.updated = now;
        if candidate.subnets.len() > self.config.limits.max_distinct
            || candidate.sessions.len() > self.config.limits.max_distinct
        {
            return Err(Unavailable::Budget);
        }
        // Preserve legacy append-before-prune and strict local cutoff behavior.
        let first = candidate
            .arrivals
            .iter()
            .position(|time| now - *time <= Duration::from_secs(120))
            .unwrap_or(candidate.arrivals.len());
        candidate.arrivals.drain(..first);
        let old = previous.map_or(0, |entry| key.len() + entry.bytes());
        let bytes = (self.bytes - old)
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(candidate.bytes()))
            .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
            .ok_or(Unavailable::Budget)?;
        let (new, velocity, scope) = if subnet.is_empty() {
            (new_subnet, candidate.arrivals.len(), Scope::LocalOnly)
        } else {
            fleet.counts(new_subnet, candidate.arrivals.len())
        };
        // MAD can fail before local registry mutation; accepted time is shared.
        let measurement = if new {
            Some(self.population.observe(velocity, now)?)
        } else {
            None
        };
        let mut signals = Vec::new();
        if candidate.subnets.len() >= 3 {
            signals.push(signal(
                "l5.correlation.proxy_rotation",
                json!(candidate.subnets.len()),
                Verdict::Bot,
                1.0,
                "one fingerprint across many subnets (residential-proxy rotation)",
            ));
        } else if candidate.sessions.len() >= 5 {
            signals.push(signal(
                "l5.correlation.shared_fingerprint",
                json!(candidate.sessions.len()),
                Verdict::Suspicious,
                1.0,
                "one fingerprint shared by many sessions (coordinated botnet)",
            ));
        }
        if velocity >= 8 && measurement.as_ref().is_some_and(|result| result.anomalous) {
            signals.push(signal(
                "l5.correlation.ip_velocity",
                json!({"newSubnetsInWindow":velocity,"madScore":measurement.unwrap().score}),
                Verdict::Suspicious,
                0.6,
                "one fingerprint acquiring /24 subnets abnormally fast (fleet-velocity)",
            ));
        }
        self.entries.insert(key.into(), candidate);
        self.bytes = bytes;
        if scope == Scope::LocalDegraded {
            self.fallbacks = self.fallbacks.saturating_add(1);
        }
        self.clock.commit(now);
        Ok(Outcome { signals, scope })
    }
    /// Existing producer API only; do not newly activate this in serving.
    pub fn observe_ja4_fanout(
        &mut self,
        ja4: &str,
        fingerprint: &str,
        subnet: &str,
        now: Duration,
    ) -> Result<Outcome, Unavailable> {
        self.clock.check(now)?;
        self.config.limits.fields(&[ja4, fingerprint, subnet])?;
        if ja4.is_empty() || fingerprint.is_empty() {
            return Ok(Outcome {
                signals: Vec::new(),
                scope: Scope::MissingKey,
            });
        }
        let previous = self.fanout.get(ja4);
        if previous.is_none() && self.fanout.len() >= self.config.limits.max_fanout_keys {
            return Err(Unavailable::Budget);
        }
        let mut candidate = previous.cloned().unwrap_or_default();
        candidate.fingerprints.insert(fingerprint.into());
        if !subnet.is_empty() {
            candidate.subnets.insert(subnet.into());
        }
        candidate.updated = now;
        if candidate.fingerprints.len() > self.config.limits.max_distinct
            || candidate.subnets.len() > self.config.limits.max_distinct
        {
            return Err(Unavailable::Budget);
        }
        let old = previous.map_or(0, |entry| ja4.len() + entry.bytes());
        let bytes = (self.bytes - old)
            .checked_add(ja4.len())
            .and_then(|bytes| bytes.checked_add(candidate.bytes()))
            .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
            .ok_or(Unavailable::Budget)?;
        let mut signals = Vec::new();
        if candidate.fingerprints.len() >= 3 && candidate.subnets.len() >= 3 {
            signals.push(signal("l5.correlation.fp_churn_proxy", json!({"fingerprints":candidate.fingerprints.len(),"subnets":candidate.subnets.len()}),
                Verdict::Bot, 1.0, "many fingerprints + many subnets under one JA4 (proxy fp-churn evasion)"));
        }
        self.fanout.insert(ja4.into(), candidate);
        self.bytes = bytes;
        self.clock.commit(now);
        Ok(Outcome {
            signals,
            scope: Scope::LocalOnly,
        })
    }
    /// Read local retained counts without expiry, mutation or fleet promotion.
    pub fn stats(&self, key: &str) -> Stats {
        self.entries
            .get(key)
            .map_or(Stats::Absent, |entry| Stats::Observed {
                subnets: entry.subnets.len(),
                sessions: entry.sessions.len(),
            })
    }
    /// Lifetime degraded-observation count, retained across reset.
    pub fn fleet_fallbacks(&self) -> u64 {
        self.fallbacks
    }
    /// Expire local/population histories strictly older than TTL; commit time.
    pub fn gc(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.population.gc(now, self.config.ttl)?;
        self.entries.retain(|key, entry| {
            if now - entry.updated > self.config.ttl {
                self.bytes -= key.len() + entry.bytes();
                false
            } else {
                true
            }
        });
        self.fanout.retain(|key, entry| {
            if now - entry.updated > self.config.ttl {
                self.bytes -= key.len() + entry.bytes();
                false
            } else {
                true
            }
        });
        self.clock.commit(now);
        Ok(())
    }
    /// Clear histories while retaining clock custody and fallback telemetry.
    pub fn reset(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.population.reset(now)?;
        self.entries.clear();
        self.fanout.clear();
        self.bytes = 0;
        self.clock.commit(now);
        Ok(())
    }
}
fn signal(
    id: &str,
    value: serde_json::Value,
    verdict: Verdict,
    confidence: f64,
    notes: &str,
) -> Signal {
    Signal::new(id, value, verdict, confidence, Source::Server, notes)
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use crate::{correlation::Limits, history::anomaly};
    fn registry(distinct: usize, bytes: usize) -> Registry {
        Registry::new(
            Config::new(
                Duration::from_secs(3600),
                Limits::new(2, 2, distinct, 64, bytes).unwrap(),
                anomaly::Limits::new(16, 256, 64, 10000).unwrap(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn full_arrival_ring_appends_before_prune_and_keeps_cutoff_equality() {
        let mut r = registry(1024, 100000);
        for i in 0..512 {
            r.observe(
                "key",
                &format!("sub{i}"),
                "s",
                Duration::ZERO,
                FleetInput::LocalOnly,
            )
            .unwrap();
        }
        r.observe(
            "key",
            "at-cut",
            "s",
            Duration::from_secs(120),
            FleetInput::LocalOnly,
        )
        .unwrap();
        assert_eq!(r.entries["key"].arrivals.len(), 512);
        r.observe(
            "key",
            "past-cut",
            "s",
            Duration::from_secs(121),
            FleetInput::LocalOnly,
        )
        .unwrap();
        assert!(r.entries["key"].arrivals.is_empty());
        r.observe(
            "key",
            "next",
            "s",
            Duration::from_secs(121),
            FleetInput::LocalOnly,
        )
        .unwrap();
        assert_eq!(r.entries["key"].arrivals.len(), 1);
    }
    #[test]
    fn fanout_and_payload_exhaustion_preserve_prior_state() {
        let mut r = registry(1, 10000);
        r.observe_ja4_fanout("ja4", "fp1", "sub", Duration::ZERO)
            .unwrap();
        assert!(
            r.observe_ja4_fanout("ja4", "fp2", "sub", Duration::ZERO)
                .is_err()
        );
        assert_eq!(r.fanout["ja4"].fingerprints.len(), 1);
        let mut tiny = registry(1, 1);
        assert!(
            tiny.observe("key", "sub", "s", Duration::ZERO, FleetInput::LocalOnly)
                .is_err()
        );
        assert!(tiny.entries.is_empty());
        assert_eq!(tiny.bytes, 0);
    }
    #[test]
    fn repeated_subnet_is_not_a_second_arrival_and_reset_clears_fanout() {
        let mut r = registry(10, 10000);
        for _ in 0..10 {
            r.observe("key", "sub", "s", Duration::ZERO, FleetInput::LocalOnly)
                .unwrap();
        }
        assert_eq!(r.entries["key"].arrivals.len(), 1);
        r.observe_ja4_fanout("ja4", "fp", "sub", Duration::ZERO)
            .unwrap();
        r.reset(Duration::ZERO).unwrap();
        assert!(r.entries.is_empty() && r.fanout.is_empty());
        assert_eq!(r.bytes, 0);
    }
}
