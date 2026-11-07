//! Bounded cold/warm weight initialization on the caller's existing RNG stream.
use super::{GoRng, MAX_HIDDEN, RngError, normal};
use crate::inference::MlpBundle;
use humanymous_behavior::features::{FEATURE_DIM, schema_hash};

/// Offline initial weights, without a trained/calibrated/activated model claim.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialWeights {
    /// Hidden weights in shared feature order.
    pub w1: Vec<Vec<f32>>,
    /// Hidden biases.
    pub b1: Vec<f32>,
    /// Output weights.
    pub w2: Vec<f32>,
    /// Output bias.
    pub b2: f32,
}

impl GoRng {
    /// Cold: one stream draws27W1 normals then1W2 normal for each hidden row.
    /// Warm: validate and copy weights without ANY RNG draw. Unlike the legacy
    /// fallback, malformed requested warm state is rejected before sampling.
    /// All failures leave the stream unchanged; no SGD loop or activation occurs.
    pub fn initialize(
        &mut self,
        hidden: usize,
        base: Option<&MlpBundle>,
    ) -> Result<InitialWeights, RngError> {
        if hidden == 0 || hidden > MAX_HIDDEN {
            return Err(RngError::Shape);
        }
        if let Some(base) = base {
            if base.hidden != hidden as i64 || base.validate().is_err() {
                return Err(RngError::Shape);
            }
            if base.schema_hash != schema_hash() {
                return Err(RngError::Schema);
            }
            if base.w1.iter().flatten().chain(&base.b1).chain(&base.w2)
                .chain(&base.mean).chain(&base.std)
                .chain([&base.b2, &base.cal_a, &base.cal_b])
                .any(|value| !value.is_finite())
            {
                return Err(RngError::Nonfinite);
            }
            return Ok(InitialWeights {
                w1: base.w1.clone(), b1: base.b1.clone(), w2: base.w2.clone(), b2: base.b2,
            });
        }
        let mut candidate = self.clone();
        let scale1 = (2.0_f64 / FEATURE_DIM as f64).sqrt() as f32;
        let scale2 = (2.0_f64 / hidden as f64).sqrt() as f32;
        let mut result = InitialWeights {
            w1: Vec::with_capacity(hidden), b1: vec![0.0; hidden],
            w2: Vec::with_capacity(hidden), b2: 0.0,
        };
        for _ in 0..hidden {
            let mut row = Vec::with_capacity(FEATURE_DIM);
            for _ in 0..FEATURE_DIM {
                row.push(normal::sample(&mut candidate)? as f32 * scale1);
            }
            result.w1.push(row);
            result.w2.push(normal::sample(&mut candidate)? as f32 * scale2);
        }
        if result.w1.iter().flatten().chain(&result.w2).any(|value| !value.is_finite()) {
            return Err(RngError::Nonfinite);
        }
        *self = candidate;
        Ok(result)
    }
}
