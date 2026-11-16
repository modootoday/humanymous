//! Time-ordered holdout and per-cohort and per-time-bucket reductions (legacy
//! `internal/mltrain` `TesseractSplit`, `CohortFPR` and `AUT` at baseline
//! 981d25c; SoT model-training-correction, row "mltrain training").
//!
//! Like [`evaluate`], these reduce predictions the caller already obtained
//! from the existing inference boundary; they never run a forward pass and
//! never approve or promote a model.
//!
//! Deliberate fail-closed differences from Go:
//! - [`time_split`] refuses an out-of-range or NaN fraction, fewer than two
//!   samples, a sample without a timestamp and an empty test window. Go
//!   returned every sample as training data with an empty test window, read a
//!   missing timestamp as zero, and left the refusal to its caller.
//! - [`cohort_fpr`] and [`aut`] validate every pair like [`evaluate`]: one
//!   unusable prediction or label rejects the whole batch.
//! - An empty [`aut`] input is unavailable (`None`), not Go's zero accuracy.
use super::{EvaluationError, LabeledPrediction, evaluate};
use crate::model::training::record::Sample;
use std::collections::BTreeMap;

/// Stable refusals for [`time_split`]; they disclose no sample content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitError {
    /// Test fraction is not strictly between zero and one.
    Fraction,
    /// Fewer than two samples cannot form both windows.
    TooFew,
    /// A sample has no timestamp and cannot be placed in time.
    MissingTimestamp,
    /// The fraction rounds to an empty test window.
    EmptyWindow,
}

impl std::fmt::Display for SplitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "training split: {self:?}")
    }
}

impl std::error::Error for SplitError {}

/// Order samples by timestamp (stable for ties) and hold out the newest
/// `test_fraction` so validation never sees data older than training data.
/// The cut keeps Go's arithmetic: `n - trunc(n * fraction)`, at least one.
pub fn time_split(
    samples: &[Sample],
    test_fraction: f64,
) -> Result<(Vec<Sample>, Vec<Sample>), SplitError> {
    if !(test_fraction > 0.0 && test_fraction < 1.0) {
        return Err(SplitError::Fraction);
    }
    if samples.len() < 2 {
        return Err(SplitError::TooFew);
    }
    let mut ordered = samples
        .iter()
        .map(|sample| {
            sample
                .record
                .ts
                .map(|ts| (ts, sample.clone()))
                .ok_or(SplitError::MissingTimestamp)
        })
        .collect::<Result<Vec<_>, _>>()?;
    ordered.sort_by_key(|(ts, _)| *ts);
    let total = ordered.len();
    let cut = (total - (total as f64 * test_fraction) as usize).max(1);
    if cut >= total {
        return Err(SplitError::EmptyWindow);
    }
    let mut train: Vec<Sample> = ordered.into_iter().map(|(_, sample)| sample).collect();
    let test = train.split_off(cut);
    Ok((train, test))
}

/// A prediction tagged with its accessibility cohort; empty or absent is
/// the `default` cohort, as in Go.
#[derive(Debug, Clone, Copy)]
pub struct CohortPrediction<'a> {
    /// Cohort label from the record, if any.
    pub cohort: Option<&'a str>,
    /// Label and existing inference output.
    pub pair: LabeledPrediction,
}

/// Human support and false positives for one cohort.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CohortStat {
    /// Human-labeled observations in the cohort.
    pub humans: usize,
    /// Human observations classified as bots.
    pub human_fp: usize,
}

impl CohortStat {
    /// Cohort human false-positive rate; absent support is unavailable.
    pub fn fpr(self) -> Option<f32> {
        (self.humans != 0).then(|| self.human_fp as f32 / self.humans as f32)
    }
}

/// Per-cohort human false-positive rates in stable (sorted) cohort order.
/// Bot rows are validated but not tallied, as in Go.
pub fn cohort_fpr(
    rows: &[CohortPrediction<'_>],
    max_samples: usize,
) -> Result<BTreeMap<String, CohortStat>, EvaluationError> {
    let pairs: Vec<LabeledPrediction> = rows.iter().map(|row| row.pair).collect();
    evaluate(&pairs, max_samples)?;
    let mut out = BTreeMap::<String, CohortStat>::new();
    for row in rows.iter().filter(|row| row.pair.label == 0) {
        let cohort = row
            .cohort
            .filter(|value| !value.is_empty())
            .unwrap_or("default");
        let stat = out.entry(cohort.to_owned()).or_default();
        stat.humans += 1;
        stat.human_fp += usize::from(row.pair.prediction.p_bot >= 0.5);
    }
    Ok(out)
}

/// A prediction tagged with its record timestamp.
#[derive(Debug, Clone, Copy)]
pub struct TimedPrediction {
    /// Record timestamp used only for ordering.
    pub ts: i64,
    /// Label and existing inference output.
    pub pair: LabeledPrediction,
}

/// Area under time: mean accuracy over `buckets` equal time-ordered slices,
/// skipping empty slices. Zero buckets is treated as one, as in Go.
pub fn aut(
    rows: &[TimedPrediction],
    buckets: usize,
    max_samples: usize,
) -> Result<Option<f32>, EvaluationError> {
    let mut ordered = rows.to_vec();
    ordered.sort_by_key(|row| row.ts);
    let pairs: Vec<LabeledPrediction> = ordered.iter().map(|row| row.pair).collect();
    evaluate(&pairs, max_samples)?;
    let total = pairs.len();
    let buckets = buckets.max(1);
    let mut sum = 0.0_f32;
    let mut used = 0usize;
    for bucket in 0..buckets {
        let (low, high) = (bucket * total / buckets, (bucket + 1) * total / buckets);
        if let Some(accuracy) = evaluate(&pairs[low..high], max_samples)?.accuracy() {
            sum += accuracy;
            used += 1;
        }
    }
    Ok((used != 0).then(|| sum / used as f32))
}

#[cfg(test)]
mod tests;
