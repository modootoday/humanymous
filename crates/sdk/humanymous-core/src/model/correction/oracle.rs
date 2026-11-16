//! Independent-oracle label mapping and Pass-poisoning guard.
//!
//! No label derived from the model's own probability ever enters training.
//! Labels come from independent adjudication: a solved Pass is HUMAN; a
//! red-team catalog session or a failed/abandoned CHALLENGE is BOT; anything
//! else is AMBIGUOUS and routed to a human queue. Failed/abandoned CHALLENGE is
//! weak bot evidence preserved for compatibility, not ground truth. The Pass
//! channel is a poisoning surface, so [`PassGuard`] caps its batch fraction and
//! surfaces solve-rate anomalies as drift instead of data.
use super::{CorrectionError, lock};
use std::sync::Mutex;

/// Largest batch label count accepted by [`PassGuard::admit_pass_fraction`].
pub const MAX_BATCH_LABELS: usize = 100_000;

/// Independent engine adjudication of a session, fed after the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Outcome {
    /// No independent adjudication.
    #[default]
    Unknown,
    /// Interactive Pass solved: strong human evidence.
    PassSolved,
    /// CHALLENGE issued and its proof failed: weak bot evidence.
    ChallengeFailed,
    /// CHALLENGE issued and abandoned: weak bot evidence.
    ChallengeAbandoned,
    /// Measured red-team catalog session: gold bot.
    CatalogBot,
}

/// Confirmed training label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Label {
    /// Route to the human analyst queue; never auto-trained.
    #[default]
    Ambiguous,
    /// Confirmed human.
    Human,
    /// Confirmed bot.
    Bot,
}

/// Map an outcome to a label and whether it may be auto-labeled.
pub fn confirm(outcome: Outcome) -> (Label, bool) {
    match outcome {
        Outcome::PassSolved => (Label::Human, true),
        Outcome::CatalogBot | Outcome::ChallengeFailed | Outcome::ChallengeAbandoned => {
            (Label::Bot, true)
        }
        Outcome::Unknown => (Label::Ambiguous, false),
    }
}

/// Pass solve-rate EMA decay.
const SOLVE_EMA_DECAY: f32 = 0.01;
/// Margin above baseline treated as a sustained solve-rate anomaly.
const ANOMALY_MARGIN: f32 = 0.25;

/// Pass-to-human channel guard. Safe for concurrent use.
#[derive(Debug)]
pub struct PassGuard {
    max_pass_fraction: f32,
    solve_baseline: f32,
    solve_ema: Mutex<f32>,
}

impl PassGuard {
    /// Build a guard. Finite out-of-range values take the legacy defaults
    /// (fraction 0.4 unless in `(0, 1]`, baseline 0.6 unless in `(0, 1)`).
    ///
    /// Stricter than legacy: NaN or infinite arguments are rejected.
    pub fn new(max_pass_fraction: f32, solve_baseline: f32) -> Result<Self, CorrectionError> {
        if !max_pass_fraction.is_finite() || !solve_baseline.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        let max_pass_fraction = if max_pass_fraction <= 0.0 || max_pass_fraction > 1.0 {
            0.4
        } else {
            max_pass_fraction
        };
        let solve_baseline = if solve_baseline <= 0.0 || solve_baseline >= 1.0 {
            0.6
        } else {
            solve_baseline
        };
        Ok(Self {
            max_pass_fraction,
            solve_baseline,
            solve_ema: Mutex::new(solve_baseline),
        })
    }

    /// Record one Pass attempt for solve-rate monitoring.
    pub fn observe_pass_attempt(&self, solved: bool) {
        let sample: f32 = if solved { 1.0 } else { 0.0 };
        let mut ema = lock(&self.solve_ema);
        *ema += SOLVE_EMA_DECAY * (sample - *ema);
    }

    /// Whether the solve rate is abnormally far above baseline; callers stop
    /// auto-ingesting Pass-human labels until reviewed.
    pub fn solve_rate_anomalous(&self) -> bool {
        *lock(&self.solve_ema) > self.solve_baseline + ANOMALY_MARGIN
    }

    /// Maximum Pass-human labels admissible next to `other_labels` gold labels so
    /// that `pass / (pass + other) <= fraction`; zero while anomalous.
    ///
    /// Stricter than legacy: counts above [`MAX_BATCH_LABELS`] are rejected,
    /// and the anomaly check and cap read one consistent lock snapshot.
    pub fn admit_pass_fraction(&self, other_labels: usize) -> Result<usize, CorrectionError> {
        if other_labels > MAX_BATCH_LABELS {
            return Err(CorrectionError::Limit);
        }
        let ema = *lock(&self.solve_ema);
        if ema > self.solve_baseline + ANOMALY_MARGIN {
            return Ok(0);
        }
        let fraction = self.max_pass_fraction;
        if fraction >= 1.0 {
            return Ok(other_labels);
        }
        Ok((other_labels as f32 * fraction / (1.0 - fraction)) as usize)
    }

    /// Current Pass solve-rate estimate.
    pub fn solve_rate(&self) -> f32 {
        *lock(&self.solve_ema)
    }
}

#[cfg(test)]
mod tests;
