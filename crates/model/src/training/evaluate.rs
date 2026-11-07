//! Pure offline metric reduction; no predictor installation or promotion policy.
use crate::inference::Prediction;

/// A reviewed label paired with an output from the existing inference boundary.
/// Constructing this value does not authenticate the label's provenance.
#[derive(Debug, Clone, Copy)]
pub struct LabeledPrediction {
    /// Zero is human and one is bot.
    pub label: u8,
    /// Existing inference output; abstentions cannot establish evaluation support.
    pub prediction: Prediction,
}

/// Explicit numerators and denominators prevent absent support appearing measured.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Number of valid evaluated pairs.
    pub total: usize,
    /// Correct classifications at the legacy offline cut-point0.5.
    pub correct: usize,
    /// Human-labeled observations.
    pub humans: usize,
    /// Human observations classified as bots.
    pub human_fp: usize,
    /// Bot-labeled observations.
    pub bots: usize,
    /// Bot observations classified as bots.
    pub bot_tp: usize,
}

impl Counts {
    /// Legacy float32 accuracy; absent support is explicitly unavailable.
    pub fn accuracy(self) -> Option<f32> {
        fraction(self.correct, self.total)
    }

    /// Human false-positive rate, unavailable when no humans were evaluated.
    pub fn human_fpr(self) -> Option<f32> {
        fraction(self.human_fp, self.humans)
    }

    /// Bot true-positive rate, unavailable when no bots were evaluated.
    pub fn bot_tpr(self) -> Option<f32> {
        fraction(self.bot_tp, self.bots)
    }
}

fn fraction(numerator: usize, denominator: usize) -> Option<f32> {
    (denominator != 0).then(|| numerator as f32 / denominator as f32)
}

/// Stable failures disclose no label source, feature values or input bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationError {
    /// Budget is zero, exceeds100000 or is smaller than the input count.
    SampleLimit,
    /// Label is not zero or one.
    InvalidLabel,
    /// Abstaining, nonfinite or out-of-range predictions cannot be scored.
    UnusablePrediction,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "training evaluation: {self:?}")
    }
}

impl std::error::Error for EvaluationError {}

/// Reduce existing predictions without duplicating the MLP forward pass.
/// The0.5 cut-point is legacy offline evaluation math, not a production threshold.
/// Any invalid pair rejects the whole result; this function never approves a model.
pub fn evaluate(
    pairs: &[LabeledPrediction],
    max_samples: usize,
) -> Result<Counts, EvaluationError> {
    if max_samples == 0 || max_samples > 100_000 || pairs.len() > max_samples {
        return Err(EvaluationError::SampleLimit);
    }
    let mut counts = Counts::default();
    for pair in pairs {
        if pair.label > 1 {
            return Err(EvaluationError::InvalidLabel);
        }
        let prediction = pair.prediction;
        if prediction.abstain
            || !prediction.p_bot.is_finite()
            || !(0.0..=1.0).contains(&prediction.p_bot)
        {
            return Err(EvaluationError::UnusablePrediction);
        }
        let bot = prediction.p_bot >= 0.5;
        counts.total += 1;
        counts.correct += usize::from(u8::from(bot) == pair.label);
        if pair.label == 0 {
            counts.humans += 1;
            counts.human_fp += usize::from(bot);
        } else {
            counts.bots += 1;
            counts.bot_tp += usize::from(bot);
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(label: u8, p_bot: f32) -> LabeledPrediction {
        LabeledPrediction {
            label,
            prediction: Prediction {
                p_bot,
                abstain: false,
            },
        }
    }

    #[test]
    fn confusion_counts_preserve_inclusive_half_boundary_and_float32_rates() {
        let pairs = [
            pair(0, 0.0),
            pair(0, 0.5),
            pair(0, 0.49),
            pair(1, 0.5),
            pair(1, 1.0),
            pair(1, 0.0),
        ];
        let counts = evaluate(&pairs, 6).unwrap();
        assert_eq!(
            counts,
            Counts {
                total: 6,
                correct: 4,
                humans: 3,
                human_fp: 1,
                bots: 3,
                bot_tp: 2
            }
        );
        assert_eq!(counts.accuracy(), Some(4.0_f32 / 6.0));
        assert_eq!(counts.human_fpr(), Some(1.0_f32 / 3.0));
        assert_eq!(counts.bot_tpr(), Some(2.0_f32 / 3.0));
    }

    #[test]
    fn absent_class_support_is_unavailable_not_a_successful_zero_rate() {
        let empty = evaluate(&[], 1).unwrap();
        assert_eq!(empty.accuracy(), None);
        assert_eq!(empty.human_fpr(), None);
        assert_eq!(empty.bot_tpr(), None);
        let bots = evaluate(&[pair(1, 1.0)], 1).unwrap();
        assert_eq!(bots.human_fpr(), None);
        assert_eq!(bots.bot_tpr(), Some(1.0));
        let humans = evaluate(&[pair(0, 0.0)], 1).unwrap();
        assert_eq!(humans.human_fpr(), Some(0.0));
        assert_eq!(humans.bot_tpr(), None);
    }

    #[test]
    fn invalid_late_pair_rejects_whole_batch_without_mutating_input() {
        for probability in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            let pairs = [pair(0, 0.0), pair(1, probability)];
            assert_eq!(
                evaluate(&pairs, 2),
                Err(EvaluationError::UnusablePrediction)
            );
            assert_eq!(pairs[0].prediction.p_bot, 0.0);
        }
        let mut abstained = pair(0, 0.0);
        abstained.prediction = Prediction::abstained();
        assert_eq!(
            evaluate(&[abstained], 1),
            Err(EvaluationError::UnusablePrediction)
        );
        assert_eq!(
            evaluate(&[pair(2, 0.0)], 1),
            Err(EvaluationError::InvalidLabel)
        );
    }

    #[test]
    fn configured_budget_may_tighten_but_not_exceed_ceiling() {
        for budget in [0, 100_001] {
            assert_eq!(evaluate(&[], budget), Err(EvaluationError::SampleLimit));
        }
        assert!(evaluate(&[pair(0, 0.0)], 1).is_ok());
        assert_eq!(
            evaluate(&[pair(0, 0.0), pair(1, 1.0)], 1),
            Err(EvaluationError::SampleLimit)
        );
    }
}
