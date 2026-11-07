//! Generic bounded median/MAD history, migrated from the read-only Go oracle.
//! Measurements carry no serving authority. Runtime owns synchronization/clock.
mod config;
use super::OrderedTime;
pub use super::Unavailable;
pub use config::{Config, Limits};
use std::{collections::BTreeMap, time::Duration};

/// Result against history BEFORE appending this observation.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Absolute robust deviation strictly exceeded configured K.
    pub anomalous: bool,
    /// Zero during warmup or when unfloored MAD is zero.
    pub score: f64,
    /// Historical median; zero during warmup.
    pub median: f64,
    /// Floored historical MAD; zero during warmup.
    pub mad: f64,
    /// Retained historical sample count before this observation.
    pub samples: usize,
    /// Distinguishes insufficient history from a measured normal observation.
    pub warmed_up: bool,
    /// Legacy oldest-key replacement made explicit, never continuous coverage.
    pub evicted_key: Option<String>,
}
struct Series {
    values: Vec<f64>,
    last: Duration,
}

/// No internal I/O, clocks, locks, signals or global state. Configuration bounds
/// retained payload; sorting scratch is at most two additional sample windows.
pub struct Detector {
    config: Config,
    series: BTreeMap<String, Series>,
    clock: OrderedTime,
    reserved_bytes: usize,
}
impl Detector {
    /// All parameters and limits were validated by Config; there are no defaults.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            series: BTreeMap::new(),
            clock: OrderedTime::new(),
            reserved_bytes: 0,
        }
    }

    /// Invalid time/input/budget/arithmetic returns before any state mutation.
    /// Equal caller ticks are permitted. Empty keys retain legacy keyed behavior.
    pub fn observe(
        &mut self,
        key: &str,
        value: f64,
        now: Duration,
    ) -> Result<Measurement, Unavailable> {
        self.clock.check(now)?;
        if !value.is_finite() {
            return Err(Unavailable::InvalidMeasurement);
        }
        if key.len() > self.config.limits.max_key_bytes {
            return Err(Unavailable::Budget);
        }
        let prior = self.series.get(key);
        let mut result = measure(
            prior.map_or(&[], |series| series.values.as_slice()),
            value,
            &self.config,
        )?;
        let mut admitted_bytes = self.reserved_bytes;
        if prior.is_none() {
            let eviction = if self.series.len() >= self.config.limits.max_keys {
                self.series
                    .iter()
                    .min_by(|(ka, a), (kb, b)| a.last.cmp(&b.last).then_with(|| ka.cmp(kb)))
                    .map(|(key, _)| key)
            } else {
                None
            };
            if let Some(old) = eviction {
                admitted_bytes -= self.config.sample_bytes + old.len();
            }
            admitted_bytes = admitted_bytes
                .checked_add(self.config.sample_bytes)
                .and_then(|bytes| bytes.checked_add(key.len()))
                .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
                .ok_or(Unavailable::Budget)?;
            result.evicted_key = eviction.cloned();
        }

        // No fallible policy/measurement operation follows the first mutation.
        if let Some(evicted) = &result.evicted_key {
            self.series.remove(evicted);
        }
        let series = self.series.entry(key.into()).or_insert_with(|| Series {
            values: Vec::with_capacity(self.config.window),
            last: now,
        });
        if series.values.len() == self.config.window {
            series.values.rotate_left(1);
            *series.values.last_mut().expect("nonempty validated window") = value;
        } else {
            series.values.push(value);
        }
        series.last = now;
        self.reserved_bytes = admitted_bytes;
        self.clock.commit(now);
        Ok(result)
    }

    /// Remove keys idle strictly longer than caller TTL. Equality survives.
    pub fn gc(&mut self, now: Duration, ttl: Duration) -> Result<usize, Unavailable> {
        self.clock.check(now)?;
        let before = self.series.len();
        let sample_bytes = self.config.sample_bytes;
        let reserved_bytes = &mut self.reserved_bytes;
        self.series.retain(|key, series| {
            if now - series.last > ttl {
                *reserved_bytes -= sample_bytes + key.len();
                false
            } else {
                true
            }
        });
        self.clock.commit(now);
        Ok(before - self.series.len())
    }

    /// Clear history while retaining ordered-time custody. A new epoch needs a
    /// new Detector. A regressing reset does not erase existing observations.
    pub fn reset(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.series.clear();
        self.reserved_bytes = 0;
        self.clock.commit(now);
        Ok(())
    }
}

fn median(values: &[f64]) -> Result<f64, Unavailable> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let value = if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Unavailable::InvalidMeasurement)
    }
}

fn measure(values: &[f64], value: f64, config: &Config) -> Result<Measurement, Unavailable> {
    let mut result = Measurement {
        anomalous: false,
        score: 0.0,
        median: 0.0,
        mad: 0.0,
        samples: values.len(),
        warmed_up: values.len() >= config.warmup,
        evicted_key: None,
    };
    if !result.warmed_up {
        return Ok(result);
    }
    result.median = median(values)?;
    let deviations: Vec<_> = values
        .iter()
        .map(|sample| (sample - result.median).abs())
        .collect();
    if deviations.iter().any(|value| !value.is_finite()) {
        return Err(Unavailable::InvalidMeasurement);
    }
    result.mad = median(&deviations)?.max(config.mad_floor);
    if result.mad > 0.0 {
        result.score = (value - result.median).abs() / result.mad;
        if !result.score.is_finite() {
            return Err(Unavailable::InvalidMeasurement);
        }
        result.anomalous = result.score > config.k;
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
