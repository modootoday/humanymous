//! Cohort-scoped offline sensor composition. No ThresholdProvider implementation.
use super::{Error, Label, Outcome, reviewed_label, probability};
use super::{features::FeatureMonitor, numeric::{Calibration, Drift, DriftReading, PassGuard, Shadow},
    lifecycle::{Canary, TripReason}};
use crate::model::inference::Prediction;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Controller {
    cohort: String, pub(crate) calibration: Calibration, pub(crate) drift: Drift,
    pub(crate) pass: PassGuard, features: FeatureMonitor, shadow: Shadow,
    pub(crate) canary: Option<Canary>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) cohort: String, pub(crate) calibration_proposal: Calibration,
    pub(crate) drift: DriftReading, pub(crate) pass_rate: f32,
    pub(crate) pass_anomaly: bool, pub(crate) shadow: Shadow,
    pub(crate) feature_reference_ready: bool, pub(crate) canary: Option<Canary>,
    pub(crate) activation: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Proposal {
    pub(crate) drift: DriftReading, pub(crate) rollback: Option<TripReason>,
    pub(crate) activation: bool,
}
impl Controller {
    pub(crate) fn new(cohort: &str, budget: f32, theta: f32, gamma: f32, window: i64) -> Result<Self, Error> {
        if cohort.is_empty() || cohort.len() > 128 { return Err(Error::Config); }
        Ok(Self { cohort: cohort.to_owned(), calibration: Calibration::new(budget, theta, gamma)?,
            drift: Drift::new()?, pass: PassGuard::new(0.4, 0.6)?, features: FeatureMonitor::new(window)?,
            shadow: Shadow::default(), canary: None })
    }
    /// The caller must independently review/authenticate the outcome. This API
    /// cannot turn a model probability, verdict or arbitrary string into truth.
    pub(crate) fn observe_reviewed_outcome(&mut self, outcome: Outcome, p: f32, engine_bot: bool) -> Result<Proposal, Error> {
        probability(p)?;
        let mut calibration = self.calibration.clone();
        let mut drift = self.drift.clone();
        let mut canary = self.canary.clone();
        let teacher = if engine_bot { 1.0_f32 } else { 0.0_f32 };
        drift.mimic.update(f64::from((p - teacher).abs()))?;
        let label = reviewed_label(outcome);
        if label != Label::Ambiguous {
            calibration.observe(label == Label::Human, p)?;
            // Preserve Go's POST-ACI threshold comparison for oracle error.
            let flagged = p >= calibration.theta;
            drift.oracle.update(if flagged != (label == Label::Bot) { 1.0 } else { 0.0 })?;
        }
        let reading = drift.record_event()?;
        let rollback = match canary.as_mut() {
            Some(guard) => guard.evaluate(calibration.humans, calibration.fp_ema, reading.fired, self.pass.anomalous())?,
            None => None,
        };
        self.calibration = calibration; self.drift = drift; self.canary = canary;
        Ok(Proposal { drift: reading, rollback, activation: false })
    }
    pub(crate) fn observe_pass(&mut self, solved: bool) -> Result<(), Error> { self.pass.observe(solved) }
    pub(crate) fn observe_features(&mut self, input: &[f32]) -> Result<Option<f64>, Error> {
        let mut features = self.features.clone();
        let mut drift = self.drift.clone();
        let closed = features.observe(input)?;
        if let Some(value) = closed { drift.update_psi(value)?; }
        self.features = features; self.drift = drift;
        Ok(closed)
    }
    pub(crate) fn observe_shadow(&mut self, active: Prediction, staged: Prediction) -> Result<(), Error> {
        self.shadow.observe(active, staged, self.calibration.theta)
    }
    pub(crate) fn arm_canary(&mut self, mut guard: Canary) -> Result<(), Error> {
        guard.arm(self.calibration.humans)?; self.canary = Some(guard); Ok(())
    }
    /// Read-only: no drift event increment, clock read, callback or active swap.
    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot { cohort: self.cohort.clone(), calibration_proposal: self.calibration.clone(),
            drift: self.drift.snapshot(), pass_rate: self.pass.ema, pass_anomaly: self.pass.anomalous(),
            shadow: self.shadow.clone(), feature_reference_ready: self.features.ready(),
            canary: self.canary.clone(), activation: false }
    }
}
