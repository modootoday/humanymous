//! Bounded Go1.26.3 math/rand compatibility for offline training only.
//!
//! This predictable generator is not suitable for secrets, session IDs or security
//! decisions. No module installs a predictor or changes production thresholds.
//! Go output goldens and Rust-derived state comparisons are separate test classes.
mod initialize;
mod normal;
mod shuffle;
mod source;
mod tables;

pub use initialize::InitialWeights;
pub use source::{GoRng, Snapshot};

/// Primitive-draw ceiling for a single bounded offline stream, not a training budget.
pub const MAX_DRAWS: u64 = 1_000_000;
/// Largest supported shuffle; the Go huge-index branch is deliberately excluded.
pub const MAX_SAMPLES: usize = 100_000;
/// Largest supported initialization shape.
pub const MAX_HIDDEN: usize = 256;

/// Bounded failures contain no input features, weights or generator state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RngError {
    /// Primitive draw budget must be positive and no larger than MAX_DRAWS.
    InvalidBudget,
    /// No primitive draws remain; the public compound operation is rolled back.
    DrawLimit,
    /// The requested shuffle exceeds MAX_SAMPLES.
    SampleLimit,
    /// Hidden width or warm-start tensor dimensions are invalid.
    Shape,
    /// Warm-start feature identity differs from the shared schema.
    Schema,
    /// Warm-start or sampled weights contain nonfinite values.
    Nonfinite,
}

impl std::fmt::Display for RngError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "offline Go RNG: {self:?}")
    }
}

impl std::error::Error for RngError {}

/// Internal primitive seam supports finite tapes and a separate Rust test model.
trait DrawSource {
    fn draw_u64(&mut self) -> Result<u64, RngError>;
}

impl GoRng {
    /// One Go NormFloat64 call. Rejection/tail draws use this same stream.
    /// Failure preserves the state from before the compound call.
    pub fn norm_float64(&mut self) -> Result<f64, RngError> {
        let mut candidate = self.clone();
        let value = normal::sample(&mut candidate)?;
        *self = candidate;
        Ok(value)
    }

    /// Shuffle an existing sample-ID order in place, without resetting the RNG or
    /// restoring the original order between epochs. Failure leaves both unchanged.
    pub fn shuffle(&mut self, order: &mut [usize]) -> Result<(), RngError> {
        if order.len() > MAX_SAMPLES {
            return Err(RngError::SampleLimit);
        }
        let mut candidate = self.clone();
        let mut staged = order.to_vec();
        shuffle::apply(&mut candidate, &mut staged)?;
        order.copy_from_slice(&staged);
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
