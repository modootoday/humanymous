//! Integrated lab correction control plane.
//!
//! Fuses calibration, drift, Pass-poisoning, covariate and shadow monitors and
//! the optional canary. It implements only the monitoring seams
//! ([`FeatureObserver`], [`ShadowObserver`]).
//!
//! Deliberate delta from legacy: the legacy controller also implemented the
//! serving threshold-provider seam. This port does NOT implement
//! `ThresholdProvider`; [`Controller::proposed_fire_threshold`] is a lab-only
//! proposal and must not be installed as a production cut-point.
use super::{
    CalibrationStats, CanaryBudget, CanaryGuard, CanaryStats, ConformalThreshold, CorrectionError,
    DriftEvent, DriftMonitor, FeatureMonitor, Label, Outcome, PassGuard, RollbackCallback,
    ShadowComparator, ShadowStats, confirm, lock, probability,
};
use crate::model::inference::{FeatureObserver, Prediction, ShadowObserver};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Aggregate lab observability view; reading it never mutates any count.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerSnapshot {
    /// Proposal, budget, realized human-FP EMA and label counts.
    pub calibration: CalibrationStats,
    /// Current 2-of-3 gate evaluation, not counted as a fired event.
    pub drift: DriftEvent,
    /// Fired drift proposals counted by outcome observations.
    pub drift_events: u64,
    /// Pass solve-rate estimate.
    pub pass_solve_rate: f32,
    /// Pass solve-rate anomaly (suspected oracle poisoning).
    pub pass_anomalous: bool,
    /// Active/candidate divergence.
    pub shadow: ShadowStats,
    /// Probation state; `Off` when no canary is armed.
    pub canary: CanaryStats,
    /// Feature vectors rejected by the covariate observer (Rust addition).
    pub rejected_features: u64,
}

/// Lab correction control plane. Safe for concurrent use.
#[derive(Debug)]
pub struct Controller {
    threshold: ConformalThreshold,
    drift: DriftMonitor,
    guard: PassGuard,
    features: FeatureMonitor,
    shadow: ShadowComparator,
    canary: Mutex<Option<Arc<CanaryGuard>>>,
    rejected_features: AtomicU64,
}

impl Controller {
    /// Build the control plane with the legacy Pass guard (0.4, 0.6) and the
    /// default feature window.
    pub fn new(budget: f32, theta0: f32, gamma: f32) -> Result<Self, CorrectionError> {
        Ok(Self {
            threshold: ConformalThreshold::new(budget, theta0, gamma)?,
            drift: DriftMonitor::new(),
            guard: PassGuard::new(0.4, 0.6)?,
            features: FeatureMonitor::default(),
            shadow: ShadowComparator::default(),
            canary: Mutex::new(None),
            rejected_features: AtomicU64::new(0),
        })
    }

    /// Lab-only calibrated fire-threshold proposal; never a production provider.
    pub fn proposed_fire_threshold(&self) -> f32 {
        self.threshold.threshold()
    }

    /// Feed ONE adjudicated session. `p_bot` is the model score, `engine_bot` the
    /// frozen engine verdict (STUDD teacher), `outcome` the independent oracle.
    /// Ambiguous outcomes update only the label-free mimic stream.
    ///
    /// Stricter than legacy: an invalid probability is rejected before any
    /// monitor changes. Delta from legacy: the canary re-check reads the drift
    /// gate without counting, so one observation counts at most one event.
    pub fn observe_outcome(
        &self,
        outcome: Outcome,
        p_bot: f32,
        engine_bot: bool,
    ) -> Result<DriftEvent, CorrectionError> {
        let p_bot = probability(p_bot)?;
        let teacher: f32 = if engine_bot { 1.0 } else { 0.0 };
        self.drift
            .observe_mimic(f64::from((p_bot - teacher).abs()))?;
        let (label, auto) = confirm(outcome);
        if auto && label != Label::Ambiguous {
            self.threshold.observe(label == Label::Human, p_bot)?;
            let flagged = p_bot >= self.threshold.threshold();
            let is_bot = label == Label::Bot;
            let error = if flagged != is_bot { 1.0 } else { 0.0 };
            self.drift.observe_oracle_error(error)?;
        }
        let event = self.drift.gate();
        self.evaluate_canary()?;
        Ok(event)
    }

    /// Record one Pass attempt for the poisoning guard.
    pub fn observe_pass(&self, solved: bool) {
        self.guard.observe_pass_attempt(solved);
    }

    /// Feed a batch max-feature PSI into the covariate arm.
    pub fn update_covariate(&self, max_feature_psi: f64) -> Result<(), CorrectionError> {
        self.drift.update_psi(max_feature_psi)
    }

    /// Typed covariate tap: when a window closes, push its reading into the gate.
    pub fn observe_feature_vector(&self, features: &[f32]) -> Result<Option<f64>, CorrectionError> {
        let reading = self.features.observe(features)?;
        if let Some(reading) = reading {
            self.drift.update_psi(reading)?;
        }
        Ok(reading)
    }

    /// Put the current model on lab probation, replacing any prior guard. The
    /// callback is a notification only; this controller performs no rollback.
    pub fn arm_canary(
        &self,
        budget: CanaryBudget,
        on_rollback: Option<RollbackCallback>,
    ) -> Result<(), CorrectionError> {
        let guard = Arc::new(CanaryGuard::new(budget, on_rollback)?);
        guard.arm(self.threshold.snapshot().humans_seen);
        *lock(&self.canary) = Some(guard);
        Ok(())
    }

    fn evaluate_canary(&self) -> Result<(), CorrectionError> {
        let Some(guard) = lock(&self.canary).clone() else {
            return Ok(());
        };
        let calibration = self.threshold.snapshot();
        guard.evaluate(
            calibration.humans_seen,
            calibration.human_fp_est,
            self.drift.peek().fired,
            self.guard.solve_rate_anomalous(),
        )
    }

    /// Current control-plane state without mutating drift event counts.
    pub fn snapshot(&self) -> ControllerSnapshot {
        let calibration = self.threshold.snapshot();
        let canary = lock(&self.canary)
            .clone()
            .map(|guard| guard.snapshot(calibration.humans_seen))
            .unwrap_or_default();
        ControllerSnapshot {
            calibration,
            drift: self.drift.peek(),
            drift_events: self.drift.events(),
            pass_solve_rate: self.guard.solve_rate(),
            pass_anomalous: self.guard.solve_rate_anomalous(),
            shadow: self.shadow.snapshot(),
            canary,
            rejected_features: self.rejected_features.load(Ordering::Relaxed),
        }
    }
}

impl FeatureObserver for Controller {
    /// Invalid vectors are counted in `rejected_features` and never sampled.
    fn observe_features(&self, features: &[f32]) {
        if self.observe_feature_vector(features).is_err() {
            self.rejected_features.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl ShadowObserver for Controller {
    /// Compare at the current proposal; invalid pairs count only as rejected.
    fn observe_shadow(&self, active: Prediction, shadow: Prediction) {
        let _rejected = self
            .shadow
            .observe(active, shadow, self.threshold.threshold());
    }
}

#[cfg(test)]
mod tests;
