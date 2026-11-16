//! Label-free drift monitoring fused by a 2-of-3 gate.
//!
//! Three independent families each catch a different failure: STUDD
//! student/teacher disagreement against the frozen engine verdict
//! (Page-Hinkley), covariate shift by per-feature PSI against a frozen
//! reference, and an oracle-error stream on independently confirmed labels
//! (Page-Hinkley). The monitor only emits a retrain-candidate proposal; it never
//! retrains or mutates a verdict.
use super::{CorrectionError, lock, probability64};
use std::sync::Mutex;

/// One-sided Page-Hinkley detector for a rising stream mean (Page 1954).
#[derive(Debug, Clone, PartialEq)]
pub struct PageHinkley {
    delta: f64,
    lambda: f64,
    n: u64,
    mean: f64,
    cum_sum: f64,
    min_sum: f64,
    alarm: bool,
}

impl PageHinkley {
    /// Build a detector; nonpositive finite arguments take the legacy defaults
    /// (delta 0.005, lambda 5).
    ///
    /// Stricter than legacy: NaN or infinite arguments are rejected.
    pub fn new(delta: f64, lambda: f64) -> Result<Self, CorrectionError> {
        if !delta.is_finite() || !lambda.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        Ok(Self {
            delta: if delta <= 0.0 { 0.005 } else { delta },
            lambda: if lambda <= 0.0 { 5.0 } else { lambda },
            n: 0,
            mean: 0.0,
            cum_sum: 0.0,
            min_sum: 0.0,
            alarm: false,
        })
    }

    /// Tolerated change magnitude before a value contributes.
    pub fn delta(&self) -> f64 {
        self.delta
    }

    /// Alarm threshold on the cumulative positive deviation.
    pub fn lambda(&self) -> f64 {
        self.lambda
    }

    /// Alarm state after the latest update.
    pub fn alarm(&self) -> bool {
        self.alarm
    }

    /// Feed one value and report whether a rising change is detected this step.
    ///
    /// Stricter than legacy: a nonfinite value is rejected without updating state,
    /// because one NaN would otherwise poison the running mean permanently.
    pub fn update(&mut self, value: f64) -> Result<bool, CorrectionError> {
        if !value.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        self.n = self.n.saturating_add(1);
        self.mean += (value - self.mean) / self.n as f64;
        self.cum_sum += value - self.mean - self.delta;
        if self.cum_sum < self.min_sum {
            self.min_sum = self.cum_sum;
        }
        self.alarm = (self.cum_sum - self.min_sum) > self.lambda;
        Ok(self.alarm)
    }

    /// Clear accumulated state after a confirmed change, keeping configuration.
    pub fn reset(&mut self) {
        *self = Self {
            delta: self.delta,
            lambda: self.lambda,
            n: 0,
            mean: 0.0,
            cum_sum: 0.0,
            min_sum: 0.0,
            alarm: false,
        };
    }
}

/// Population Stability Index over the same buckets:
/// `sum((cur - ref) * ln(cur / ref))`, with fractions floored at `1e-6`.
///
/// Stricter than legacy: mismatched or empty inputs return `Shape` instead of
/// a silent zero, and nonfinite or out-of-range fractions are rejected.
pub fn psi(reference: &[f64], current: &[f64]) -> Result<f64, CorrectionError> {
    if reference.len() != current.len() || reference.is_empty() {
        return Err(CorrectionError::Shape);
    }
    const EPS: f64 = 1e-6;
    let mut total = 0.0;
    for (reference, current) in reference.iter().zip(current) {
        let reference = probability64(*reference)?.max(EPS);
        let current = probability64(*current)?.max(EPS);
        total += (current - reference) * (current / reference).ln();
    }
    Ok(total)
}

/// Retrain-candidate proposal; never an action.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriftEvent {
    /// At least two of the three arms agree.
    pub fired: bool,
    /// STUDD mimic-loss detector alarm.
    pub mimic_alarm: bool,
    /// Oracle-error detector alarm.
    pub oracle_alarm: bool,
    /// Latest max-feature PSI exceeds the shift threshold.
    pub covariate_shift: bool,
    /// Latest max-feature PSI.
    pub max_psi: f64,
}

#[derive(Debug)]
struct State {
    mimic: PageHinkley,
    oracle: PageHinkley,
    psi_threshold: f64,
    last_psi: f64,
    events: u64,
}

/// Fused 2-of-3 drift monitor. Safe for concurrent use.
#[derive(Debug)]
pub struct DriftMonitor {
    state: Mutex<State>,
}

impl Default for DriftMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl DriftMonitor {
    /// Legacy defaults: both detectors delta 0.01 / lambda 6, PSI threshold 0.25.
    pub fn new() -> Self {
        let detector = PageHinkley {
            delta: 0.01,
            lambda: 6.0,
            n: 0,
            mean: 0.0,
            cum_sum: 0.0,
            min_sum: 0.0,
            alarm: false,
        };
        Self {
            state: Mutex::new(State {
                mimic: detector.clone(),
                oracle: detector,
                psi_threshold: 0.25,
                last_psi: 0.0,
                events: 0,
            }),
        }
    }

    /// Feed one STUDD residual: absolute model/frozen-engine disagreement.
    ///
    /// Stricter than legacy: values outside `[0, 1]` or nonfinite are rejected.
    pub fn observe_mimic(&self, disagreement: f64) -> Result<bool, CorrectionError> {
        let disagreement = probability64(disagreement)?;
        lock(&self.state).mimic.update(disagreement)
    }

    /// Feed one error indicator against an oracle-confirmed label.
    ///
    /// Stricter than legacy: values outside `[0, 1]` or nonfinite are rejected.
    pub fn observe_oracle_error(&self, indicator: f64) -> Result<bool, CorrectionError> {
        let indicator = probability64(indicator)?;
        lock(&self.state).oracle.update(indicator)
    }

    /// Record the latest max-feature PSI against the frozen reference.
    ///
    /// Stricter than legacy: negative or nonfinite readings are rejected.
    pub fn update_psi(&self, max_feature_psi: f64) -> Result<(), CorrectionError> {
        if !max_feature_psi.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        if max_feature_psi < 0.0 {
            return Err(CorrectionError::Probability);
        }
        lock(&self.state).last_psi = max_feature_psi;
        Ok(())
    }

    /// Evaluate the 2-of-3 rule and count a fired proposal.
    pub fn gate(&self) -> DriftEvent {
        let mut state = lock(&self.state);
        let event = evaluate(&state);
        if event.fired {
            state.events = state.events.saturating_add(1);
        }
        event
    }

    /// Evaluate the 2-of-3 rule without counting a proposal.
    ///
    /// Delta from legacy: snapshots use this so observability reads never
    /// mutate the fired-event count.
    pub fn peek(&self) -> DriftEvent {
        evaluate(&lock(&self.state))
    }

    /// Number of fired proposals counted by [`DriftMonitor::gate`].
    pub fn events(&self) -> u64 {
        lock(&self.state).events
    }
}

fn evaluate(state: &State) -> DriftEvent {
    let mut event = DriftEvent {
        fired: false,
        mimic_alarm: state.mimic.alarm,
        oracle_alarm: state.oracle.alarm,
        covariate_shift: state.last_psi > state.psi_threshold,
        max_psi: state.last_psi,
    };
    let agreeing = [event.mimic_alarm, event.oracle_alarm, event.covariate_shift]
        .into_iter()
        .filter(|alarm| *alarm)
        .count();
    event.fired = agreeing >= 2;
    event
}

#[cfg(test)]
mod tests;
