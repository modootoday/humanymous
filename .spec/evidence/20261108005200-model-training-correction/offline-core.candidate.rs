//! REVIEW CANDIDATE: intended for Core model/training/offline.rs, not integrated.
//! Unsigned offline data only. No runtime install, score, promotion or signing API.
use super::{evaluate::{self, Counts, LabeledPrediction}, record::{Admission, Limits as RecordLimits, Sample}, sgd, standardize};
use crate::behavior::features::schema_hash;
use crate::model::inference::{Mlp, MlpBundle, Predictor};
use serde::Serialize;
use std::collections::BTreeMap;

/// Caller-reviewed aggregate samples; this operation does not authenticate labels.
pub struct OfflineInput<'a> {
    pub train: &'a [Sample],
    pub holdout: &'a [Sample],
    pub base: Option<&'a MlpBundle>,
}

/// Legacy SGD configuration. New statistics and CalA=1/CalB=0 apply even at epoch0.
pub struct OfflineConfig<'a> {
    pub schema: &'a str,
    pub version: &'a str,
    pub hidden: i64,
    pub epochs: i64,
    pub learning_rate: f32,
    pub seed: i64,
}

/// Explicit offline support/capacity policy; no production verdict thresholds.
#[derive(Clone, Copy)]
pub struct OfflineLimits {
    pub total_samples: usize,
    pub cohorts: usize,
    pub min_holdout_humans_per_cohort: usize,
    pub hidden: usize,
    pub epochs: u64,
    pub updates: u64,
    pub work_units: u64,
    pub rng_draws: u64,
}

impl Default for OfflineLimits {
    fn default() -> Self {
        Self { total_samples: 100_000, cohorts: 64, min_holdout_humans_per_cohort: 20,
            hidden: 256, epochs: 1000, updates: 1_000_000,
            work_units: 100_000_000, rng_draws: 1_000_000 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineError {
    Limits, Schema, Input, FeatureMismatch, Admission, Time, Cohort, Support,
    Config, Statistics, WarmBase, Nonfinite, RngBudget, Allocation, Evaluation,
}

impl std::fmt::Display for OfflineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "offline training: {self:?}")
    }
}
impl std::error::Error for OfflineError {}

/// Exact numerators/denominators and legacy inclusive0.5 offline metrics.
#[derive(Debug, Serialize)]
pub struct OfflineMetrics {
    pub total: usize, pub correct: usize, pub humans: usize, pub human_fp: usize,
    pub bots: usize, pub bot_tp: usize,
    pub accuracy: Option<f32>, pub human_fpr: Option<f32>, pub bot_tpr: Option<f32>,
}
impl From<Counts> for OfflineMetrics {
    fn from(c: Counts) -> Self {
        Self { total: c.total, correct: c.correct, humans: c.humans, human_fp: c.human_fp,
            bots: c.bots, bot_tp: c.bot_tp, accuracy: c.accuracy(),
            human_fpr: c.human_fpr(), bot_tpr: c.bot_tpr() }
    }
}

#[derive(Debug, Serialize)]
pub struct OfflineCohortMetrics {
    pub cohort: String, pub humans: usize, pub false_positives: usize, pub human_fpr: f32,
}

#[derive(Debug, Serialize)]
pub struct OfflineSummary {
    pub train_samples: usize, pub holdout_samples: usize,
    pub hidden: usize, pub epochs: u64, pub learning_rate: f32,
    pub updates: u64, pub rng_draws: u64,
}

/// Data, not admission authority. Model mean/std contain train-only statistics.
#[derive(Debug, Serialize)]
pub struct OfflineReport {
    pub model: MlpBundle,
    pub metrics: OfflineMetrics,
    pub cohorts: Vec<OfflineCohortMetrics>,
    pub summary: OfflineSummary,
    pub activation: bool,
    pub signed: bool,
}

fn validate_limits(l: OfflineLimits) -> Result<(), OfflineError> {
    let c = OfflineLimits::default();
    if l.total_samples == 0 || l.total_samples > c.total_samples
        || l.cohorts == 0 || l.cohorts > c.cohorts
        || l.min_holdout_humans_per_cohort == 0
        || l.min_holdout_humans_per_cohort > l.total_samples
        || l.hidden == 0 || l.hidden > c.hidden || l.epochs == 0 || l.epochs > c.epochs
        || l.updates == 0 || l.updates > c.updates
        || l.work_units == 0 || l.work_units > c.work_units
        || l.rng_draws == 0 || l.rng_draws > c.rng_draws
    { return Err(OfflineError::Limits); }
    Ok(())
}

fn cohort(s: &Sample) -> Result<&str, OfflineError> {
    let name = s.record.cohort.as_deref().ok_or(OfflineError::Cohort)?;
    if name.len() > 128 { return Err(OfflineError::Cohort); }
    Ok(if name.is_empty() { "default" } else { name })
}

fn admitted(s: &Sample, schema: &str) -> Result<(), OfflineError> {
    // Public Sample fields can be forged; revalidate the original aggregate and
    // compare supplied features, instead of trusting its admission enum alone.
    if s.record.source.as_ref().is_some_and(|v| v.len() > 128)
        || s.record.cohort.as_ref().is_some_and(|v| v.len() > 128)
    { return Err(OfflineError::Input); }
    let fresh = s.record.clone().sample(schema, RecordLimits::default())
        .map_err(|_| OfflineError::Input)?;
    if fresh.admission != Admission::ReviewRequired || s.admission != fresh.admission {
        return Err(OfflineError::Admission);
    }
    if s.features.iter().zip(fresh.features).any(|(a, b)| a.to_bits() != b.to_bits()) {
        return Err(OfflineError::FeatureMismatch);
    }
    Ok(())
}

fn training_error(error: sgd::TrainError) -> OfflineError {
    match error {
        sgd::TrainError::Config => OfflineError::Config,
        sgd::TrainError::Limit => OfflineError::Limits,
        sgd::TrainError::Schema => OfflineError::Schema,
        sgd::TrainError::Label => OfflineError::Input,
        sgd::TrainError::Statistics => OfflineError::Statistics,
        sgd::TrainError::WarmBase => OfflineError::WarmBase,
        sgd::TrainError::Nonfinite => OfflineError::Nonfinite,
        sgd::TrainError::RngBudget => OfflineError::RngBudget,
        sgd::TrainError::Allocation => OfflineError::Allocation,
    }
}

/// Real domain composition over one private SGD implementation. Strict time and
/// cohort support checks are offline preconditions, NOT full promotion policy.
/// Empty/weak datasets refuse; calibration is reset per legacy Train semantics.
pub fn fit_offline(input: OfflineInput<'_>, config: OfflineConfig<'_>, limits: OfflineLimits)
    -> Result<OfflineReport, OfflineError>
{
    validate_limits(limits)?;
    if config.schema != schema_hash() { return Err(OfflineError::Schema); }
    let total = input.train.len().checked_add(input.holdout.len()).ok_or(OfflineError::Limits)?;
    if total > limits.total_samples { return Err(OfflineError::Limits); }
    if input.train.is_empty() || input.holdout.is_empty() { return Err(OfflineError::Support); }
    let mut required: BTreeMap<&str, usize> = BTreeMap::new();
    let mut all_cohorts = std::collections::BTreeSet::new();
    let mut max_train = i64::MIN;
    let mut min_holdout = i64::MAX;
    let mut train_labels = [false; 2];
    let mut holdout_labels = [false; 2];
    for (samples, is_train) in [(input.train, true), (input.holdout, false)] {
        for s in samples {
            admitted(s, config.schema)?;
            let name = cohort(s)?;
            all_cohorts.insert(name);
            if all_cohorts.len() > limits.cohorts { return Err(OfflineError::Cohort); }
            let ts = s.record.ts.ok_or(OfflineError::Time)?;
            if is_train {
                max_train = max_train.max(ts);
                train_labels[usize::from(s.record.label)] = true;
            } else {
                min_holdout = min_holdout.min(ts);
                holdout_labels[usize::from(s.record.label)] = true;
            }
            if s.record.label == 0 {
                let count = required.entry(name).or_default();
                if !is_train { *count += 1; }
            }
        }
    }
    if max_train >= min_holdout { return Err(OfflineError::Time); }
    if train_labels.contains(&false) || holdout_labels.contains(&false)
        || required.values().any(|n| *n < limits.min_holdout_humans_per_cohort)
    { return Err(OfflineError::Support); }
    let features: Vec<_> = input.train.iter().map(|s| s.features).collect();
    let statistics = standardize::standardize(&features, config.schema, limits.total_samples)
        .map_err(|_| OfflineError::Statistics)?;
    let rows: Vec<_> = input.train.iter().map(|s| sgd::TrainingRow {
        features: &s.features, label: s.record.label,
    }).collect();
    let trained = sgd::train(sgd::TrainConfig { hidden: config.hidden, epochs: config.epochs,
        learning_rate: config.learning_rate, seed: config.seed, version: config.version },
        &rows, &statistics, config.schema, input.base,
        sgd::TrainLimits { samples: limits.total_samples, hidden: limits.hidden, epochs: limits.epochs,
            updates: limits.updates, work_units: limits.work_units, rng_draws: limits.rng_draws })
        .map_err(training_error)?;
    let predictor = Mlp::from_bundle(trained.candidate.clone()).map_err(|_| OfflineError::Evaluation)?;
    let pairs: Vec<_> = input.holdout.iter().map(|s| LabeledPrediction {
        label: s.record.label, prediction: predictor.predict(&s.features),
    }).collect();
    let counts = evaluate::evaluate(&pairs, limits.total_samples).map_err(|_| OfflineError::Evaluation)?;
    let mut cohort_counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (s, p) in input.holdout.iter().zip(&pairs) {
        if s.record.label == 0 {
            let count = cohort_counts.entry(cohort(s)?).or_default();
            count.0 += 1;
            count.1 += usize::from(p.prediction.p_bot >= 0.5);
        }
    }
    let cohorts = cohort_counts.into_iter().map(|(name, (humans, fp))| OfflineCohortMetrics {
        cohort: name.to_owned(), humans, false_positives: fp, human_fpr: fp as f32 / humans as f32,
    }).collect();
    Ok(OfflineReport { model: trained.candidate, metrics: counts.into(), cohorts,
        summary: OfflineSummary { train_samples: input.train.len(), holdout_samples: input.holdout.len(),
            hidden: trained.summary.hidden, epochs: trained.summary.epochs,
            learning_rate: trained.summary.learning_rate, updates: trained.summary.updates,
            rng_draws: trained.summary.rng_draws }, activation: false, signed: false })
}

#[cfg(test)]
#[path = "offline/tests.rs"]
mod tests;
