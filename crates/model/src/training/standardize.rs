//! Offline float32 population statistics, preserving the legacy operation order.
use humanymous_behavior::features::{FEATURE_DIM, FeatureVector, schema_hash};

/// Standardization statistics; this value never installs or activates a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Standardization {
    /// Per-feature float32 mean in the shared feature order.
    pub mean: FeatureVector,
    /// Population deviation plus the legacy epsilon; empty input uses one.
    pub std: FeatureVector,
}

/// Deterministic input failures without training data in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardizeError {
    /// Dataset feature identity does not match the shared schema.
    SchemaMismatch,
    /// Budget is zero, exceeds100000, or is smaller than the input count.
    SampleLimit,
    /// A feature or intermediate float32 statistic is not finite.
    Nonfinite,
}

impl std::fmt::Display for StandardizeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "training standardization: {self:?}")
    }
}

impl std::error::Error for StandardizeError {}

/// Compute legacy population statistics without sorting, clamping or modifying input.
/// Admission, provenance review and train/holdout selection belong to the caller.
/// Finite negative features remain signed. No RNG or training iteration is used.
pub fn standardize(
    vectors: &[FeatureVector],
    expected_schema: &str,
    max_samples: usize,
) -> Result<Standardization, StandardizeError> {
    if expected_schema != schema_hash() {
        return Err(StandardizeError::SchemaMismatch);
    }
    if max_samples == 0 || max_samples > 100_000 || vectors.len() > max_samples {
        return Err(StandardizeError::SampleLimit);
    }
    let mut result = Standardization {
        mean: [0.0; FEATURE_DIM],
        std: [1.0; FEATURE_DIM],
    };
    if vectors.is_empty() {
        return Ok(result);
    }
    for vector in vectors {
        for (mean, value) in result.mean.iter_mut().zip(vector) {
            if !value.is_finite() {
                return Err(StandardizeError::Nonfinite);
            }
            *mean += value;
            if !mean.is_finite() {
                return Err(StandardizeError::Nonfinite);
            }
        }
    }
    let count = vectors.len() as f32;
    for mean in &mut result.mean {
        *mean /= count;
    }
    result.std.fill(0.0);
    for vector in vectors {
        for ((variance, value), mean) in result.std.iter_mut().zip(vector).zip(&result.mean) {
            let delta = value - mean;
            *variance += delta * delta;
            if !variance.is_finite() {
                return Err(StandardizeError::Nonfinite);
            }
        }
    }
    for deviation in &mut result.std {
        *deviation = f64::from(*deviation / count).sqrt() as f32 + 1e-6_f32;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_constant_inputs_keep_distinct_legacy_defaults() {
        let empty = standardize(&[], &schema_hash(), 1).unwrap();
        assert_eq!(empty.mean, [0.0; FEATURE_DIM]);
        assert_eq!(empty.std, [1.0; FEATURE_DIM]);
        let constant = standardize(&[[-3.0; FEATURE_DIM]], &schema_hash(), 1).unwrap();
        assert_eq!(constant.mean, [-3.0; FEATURE_DIM]);
        assert_eq!(constant.std, [1e-6_f32; FEATURE_DIM]);
    }

    #[test]
    fn population_variance_feature_order_and_input_bytes_are_preserved() {
        let mut vectors = [[1.0; FEATURE_DIM], [3.0; FEATURE_DIM]];
        vectors[0][1] = -4.0;
        vectors[1][1] = 4.0;
        let before = vectors;
        let stats = standardize(&vectors, &schema_hash(), 2).unwrap();
        assert_eq!(stats.mean[0].to_bits(), 2.0_f32.to_bits());
        assert_eq!(stats.std[0].to_bits(), (1.0_f32 + 1e-6).to_bits());
        assert_eq!(stats.mean[1], 0.0);
        assert_eq!(stats.std[1].to_bits(), (4.0_f32 + 1e-6).to_bits());
        assert_eq!(vectors, before);
    }

    #[test]
    fn mean_retains_input_order_float32_rounding_instead_of_float64_sum() {
        let vectors = [
            [16_777_216.0; FEATURE_DIM],
            [1.0; FEATURE_DIM],
            [-16_777_216.0; FEATURE_DIM],
        ];
        let stats = standardize(&vectors, &schema_hash(), 3).unwrap();
        assert_eq!(stats.mean, [0.0; FEATURE_DIM]);
        let reordered = [vectors[0], vectors[2], vectors[1]];
        let stats = standardize(&reordered, &schema_hash(), 3).unwrap();
        assert_eq!(stats.mean, [1.0_f32 / 3.0; FEATURE_DIM]);
    }

    #[test]
    fn rejects_schema_budget_nonfinite_and_finite_input_overflow() {
        assert_eq!(
            standardize(&[], "stale", 1),
            Err(StandardizeError::SchemaMismatch)
        );
        for budget in [0, 100_001] {
            assert_eq!(
                standardize(&[], &schema_hash(), budget),
                Err(StandardizeError::SampleLimit)
            );
        }
        assert_eq!(
            standardize(&[[0.0; FEATURE_DIM]; 2], &schema_hash(), 1),
            Err(StandardizeError::SampleLimit)
        );
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                standardize(&[[value; FEATURE_DIM]], &schema_hash(), 1),
                Err(StandardizeError::Nonfinite)
            );
        }
        for vectors in [
            [[f32::MAX; FEATURE_DIM], [f32::MAX; FEATURE_DIM]],
            [[f32::MAX; FEATURE_DIM], [-f32::MAX; FEATURE_DIM]],
        ] {
            assert_eq!(
                standardize(&vectors, &schema_hash(), 2),
                Err(StandardizeError::Nonfinite)
            );
        }
    }
}
