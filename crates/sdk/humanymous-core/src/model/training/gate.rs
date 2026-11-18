//! Offline promotion gate (legacy `cmd/mlcorrect-train/gate.go` `runGate` at
//! baseline 981d25c; SoT model-training-correction, row "both training
//! commands").
//!
//! [`run_gate`] trains a candidate (warm-starting from a base bundle when one
//! is given), evaluates it on a time-ordered (TESSERACT) holdout and decides
//! whether it may be emitted. It performs no file I/O and never exits the
//! process, so the fail-closed invariant is unit-testable: a candidate is
//! returned ONLY IF every gate passed, and a regressed accessibility cohort
//! blocks it. Emitting a candidate is not promotion: staging, signing and the
//! dual-controlled promote step belong to the lab bundle lifecycle, and no
//! production model, threshold or verdict band is touched here.
//!
//! The gate criteria (maximum per-cohort human FPR, bot TPR floor, minimum
//! humans per cohort) are offline promotion parameters supplied by the caller;
//! the legacy CLI defaults are preserved by the `mlcorrect-train` binary.
//!
//! Deliberate fail-closed differences from Go:
//! - Split, standardization, training and evaluation refusals are typed errors
//!   (see [`time_split`] for missing timestamps), never a silent pass.
//! - A test window without any human or without any bot support FAILS the
//!   gate. Go iterated only over observed human cohorts and reported an absent
//!   bot TPR as zero, so an all-bot window could vacuously pass the cohort gate
//!   and a zero floor could pass without bots.
//! - Optionally configured required cohorts must appear in the test window.
//! - Nonfinite or out-of-range gate parameters are a configuration error;
//!   Go compared against NaN, which passes every `>` check.
use super::evaluate::{
    CohortPrediction, CohortStat, EvaluationError, LabeledPrediction, TimedPrediction, aut,
    cohort_fpr, evaluate, time_split,
};
use super::record::Sample;
use super::sgd::{TrainConfig, TrainLimits, TrainingRow, train};
use super::standardize::standardize;
use crate::behavior::features::schema_hash;
use crate::model::inference::{Mlp, MlpBundle, Predictor};
use std::collections::BTreeMap;

/// Legacy minimum number of samples before a candidate can be evaluated.
pub const MIN_SAMPLES: usize = 100;

/// Legacy number of time buckets for the AUT metric.
pub const AUT_BUCKETS: usize = 5;

/// Evaluation budget; matches the shared training and evaluation ceiling.
const MAX_SAMPLES: usize = 100_000;

/// Promotion-gate policy (legacy `gateConfig`).
#[derive(Debug, Clone, Copy)]
pub struct GateConfig<'a> {
    /// Candidate bundle version (at most 128 bytes).
    pub version: &'a str,
    /// Hidden units; zero or negative selects the legacy default 16.
    pub hidden: i64,
    /// Training epochs; negative is zero.
    pub epochs: i64,
    /// Learning rate, narrowed to float32 as in Go; nonpositive selects 0.05.
    pub learning_rate: f64,
    /// Go `math/rand` seed.
    pub seed: i64,
    /// Newest fraction of samples held out for evaluation, in (0, 1).
    pub test_fraction: f64,
    /// BLOCKING: maximum human false-positive rate per cohort, in [0, 1].
    pub max_human_fpr: f64,
    /// BLOCKING: minimum bot true-positive rate (catalog no-regression), in [0, 1].
    pub min_bot_tpr: f64,
    /// Humans a cohort needs before its FPR gate is trusted.
    pub min_cohort_humans: usize,
    /// Cohorts that must be present in the test window (empty: none required).
    pub required_cohorts: &'a [String],
}

/// Gate outcome (legacy `gateResult`). `candidate` is `Some` IFF `failures`
/// is empty.
#[derive(Debug, Clone)]
pub struct GateReport {
    /// Trained candidate, emitted only when every gate passed.
    pub candidate: Option<MlpBundle>,
    /// Human-readable gate failures in stable order.
    pub failures: Vec<String>,
    /// Test-window accuracy; `None` without support.
    pub accuracy: Option<f32>,
    /// Aggregate human FPR (reported, not gated); `None` without humans.
    pub aggregate_human_fpr: Option<f32>,
    /// Bot TPR; `None` without bots.
    pub bot_tpr: Option<f32>,
    /// Area under time over [`AUT_BUCKETS`] buckets; `None` without support.
    pub aut: Option<f32>,
    /// Per-cohort human support and false positives, sorted by cohort.
    pub cohorts: BTreeMap<String, CohortStat>,
    /// Training-window size.
    pub n_train: usize,
    /// Test-window size.
    pub n_test: usize,
}

/// Preconditions under which no candidate can be evaluated at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    /// Nonfinite or out-of-range gate parameter.
    Config,
    /// Fewer than [`MIN_SAMPLES`] samples.
    TooFewSamples(usize),
    /// Time split refused (missing timestamp, bad fraction, empty test window).
    Split(super::evaluate::SplitError),
    /// Standardization statistics could not be computed.
    Standardize(super::standardize::StandardizeError),
    /// Training refused; the category names the stable training failure.
    Training(String),
    /// The candidate's predictions could not be evaluated.
    Evaluation(EvaluationError),
    /// The trained candidate failed validation.
    Candidate,
}

impl std::fmt::Display for GateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config => write!(formatter, "invalid gate configuration"),
            Self::TooFewSamples(got) => {
                write!(formatter, "need >={MIN_SAMPLES} samples, got {got}")
            }
            Self::Split(super::evaluate::SplitError::EmptyWindow) => write!(
                formatter,
                "empty test window — cannot validate a candidate; refusing to emit"
            ),
            Self::Split(error) => write!(formatter, "{error}"),
            Self::Standardize(error) => write!(formatter, "{error}"),
            Self::Training(category) => write!(formatter, "offline training: {category}"),
            Self::Evaluation(error) => write!(formatter, "{error}"),
            Self::Candidate => write!(formatter, "trained candidate failed validation"),
        }
    }
}

impl std::error::Error for GateError {}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// Train a candidate from `samples` and evaluate it on a time-ordered holdout
/// against the promotion gates. Returns an error only for a precondition that
/// cannot be evaluated at all; gate failures are reported in
/// [`GateReport::failures`] with no candidate.
pub fn run_gate(
    samples: &[Sample],
    base: Option<&MlpBundle>,
    config: &GateConfig<'_>,
) -> Result<GateReport, GateError> {
    if !unit_interval(config.max_human_fpr)
        || !unit_interval(config.min_bot_tpr)
        || !config.learning_rate.is_finite()
    {
        return Err(GateError::Config);
    }
    if samples.len() < MIN_SAMPLES {
        return Err(GateError::TooFewSamples(samples.len()));
    }
    let (train_set, test_set) =
        time_split(samples, config.test_fraction).map_err(GateError::Split)?;

    let schema = schema_hash();
    let vectors: Vec<_> = train_set.iter().map(|sample| sample.features).collect();
    let statistics = standardize(&vectors, &schema, MAX_SAMPLES).map_err(GateError::Standardize)?;
    let rows: Vec<_> = train_set
        .iter()
        .map(|sample| TrainingRow {
            features: &sample.features,
            label: sample.record.label,
        })
        .collect();
    let trained = train(
        TrainConfig {
            hidden: config.hidden,
            epochs: config.epochs,
            learning_rate: config.learning_rate as f32,
            seed: config.seed,
            version: config.version,
        },
        &rows,
        &statistics,
        &schema,
        base,
        TrainLimits::default(),
    )
    .map_err(|error| GateError::Training(format!("{error:?}")))?;
    let candidate = trained.candidate;

    // Evaluate through the existing inference boundary; no forked forward pass.
    let model = Mlp::from_bundle(candidate.clone()).map_err(|_| GateError::Candidate)?;
    let pairs: Vec<LabeledPrediction> = test_set
        .iter()
        .map(|sample| LabeledPrediction {
            label: sample.record.label,
            prediction: model.predict(&sample.features),
        })
        .collect();
    let counts = evaluate(&pairs, MAX_SAMPLES).map_err(GateError::Evaluation)?;
    let timed: Vec<_> = test_set
        .iter()
        .zip(&pairs)
        .map(|(sample, pair)| TimedPrediction {
            // time_split already refused samples without a timestamp.
            ts: sample.record.ts.unwrap_or_default(),
            pair: *pair,
        })
        .collect();
    let by_cohort: Vec<_> = test_set
        .iter()
        .zip(&pairs)
        .map(|(sample, pair)| CohortPrediction {
            cohort: sample.record.cohort.as_deref(),
            pair: *pair,
        })
        .collect();
    let mut report = GateReport {
        candidate: None,
        failures: Vec::new(),
        accuracy: counts.accuracy(),
        aggregate_human_fpr: counts.human_fpr(),
        bot_tpr: counts.bot_tpr(),
        aut: aut(&timed, AUT_BUCKETS, MAX_SAMPLES).map_err(GateError::Evaluation)?,
        cohorts: cohort_fpr(&by_cohort, MAX_SAMPLES).map_err(GateError::Evaluation)?,
        n_train: train_set.len(),
        n_test: test_set.len(),
    };

    // Per-cohort human-FP gate (BLOCKING, never aggregate) in stable order.
    if report.cohorts.is_empty() {
        report
            .failures
            .push("no human support in the test window: cannot confirm human-FP≈0".to_owned());
    }
    for required in config.required_cohorts {
        if !report.cohorts.contains_key(required.as_str()) {
            report.failures.push(format!(
                "required cohort {required:?} is absent from the test window: cannot confirm human-FP≈0"
            ));
        }
    }
    for (cohort, stat) in &report.cohorts {
        if stat.humans < config.min_cohort_humans {
            report.failures.push(format!(
                "cohort {cohort:?} has only {} humans (<{}): cannot confirm human-FP≈0",
                stat.humans, config.min_cohort_humans
            ));
        } else if let Some(fpr) = stat.fpr()
            && f64::from(fpr) > config.max_human_fpr
        {
            report.failures.push(format!(
                "cohort {cohort:?} human FPR {fpr:.4} > {:.4}",
                config.max_human_fpr
            ));
        }
    }
    // Catalog no-regression.
    match report.bot_tpr {
        None => report.failures.push(
            "no bot support in the test window: catalog regression cannot be ruled out".to_owned(),
        ),
        Some(tpr) if f64::from(tpr) < config.min_bot_tpr => report.failures.push(format!(
            "bot TPR {tpr:.3} < floor {:.3} (catalog regression)",
            config.min_bot_tpr
        )),
        Some(_) => {}
    }

    if report.failures.is_empty() {
        report.candidate = Some(candidate); // emitted ONLY when every gate passed
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
