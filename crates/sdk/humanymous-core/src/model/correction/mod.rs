//! Lab-only self-correction kernels ported from the legacy correction package.
//!
//! These monitors SENSE (drift, calibration, shadow divergence) and PROPOSE
//! (retrain candidates, threshold proposals, lab rollback notifications). They
//! never mutate a verdict, never install a production `ThresholdProvider` and
//! never activate a model on an inference `Runtime`. Production weights,
//! thresholds, predicates and verdict bands remain frozen; any production
//! activation needs a separately accepted design and explicit authorization.
//!
//! Invalid inputs fail closed with [`CorrectionError`] before any state changes.
//! Where the legacy code silently accepted or defaulted such inputs, the owning
//! item documents the stricter Rust behavior.
use std::sync::{Mutex, MutexGuard};

mod bundle;
mod calibration;
mod canary;
mod controller;
mod drift;
mod features;
mod oracle;
mod shadow;
#[cfg(test)]
mod test_support;

pub use bundle::{BundleIdentity, BundleManager, MAX_BUNDLE_BYTES, digest};
pub use calibration::{CalibrationStats, ConformalThreshold};
pub use canary::{
    CanaryBudget, CanaryGuard, CanaryState, CanaryStats, MIN_CANARY_OBSERVE, RollbackCallback,
};
pub use controller::{Controller, ControllerSnapshot};
pub use drift::{DriftEvent, DriftMonitor, PageHinkley, psi};
pub use features::{DEFAULT_WINDOW, FeatureMonitor, MAX_WINDOW, PSI_BUCKETS};
pub use oracle::{Label, MAX_BATCH_LABELS, Outcome, PassGuard, confirm};
pub use shadow::{ShadowComparator, ShadowStats};

/// Bounded failures; diagnostics never carry features, scores or bundle bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionError {
    /// A numeric input or configuration value is NaN or infinite.
    Nonfinite,
    /// A probability, fraction, indicator or threshold is outside `[0, 1]`.
    Probability,
    /// A feature vector or distribution has the wrong or an empty shape.
    Shape,
    /// A window, batch or artifact size is zero or exceeds its budget.
    Limit,
    /// A cumulative count moved backwards relative to its arm point.
    CountRegressed,
    /// Promotion was requested without a verified staged candidate.
    NoStaged,
    /// Rollback was requested without an active or previous bundle.
    NoPrevious,
    /// The bundle bytes do not match the pinned SHA-256 digest.
    Digest,
    /// Signature verification is not available in this build.
    SignatureUnsupported,
    /// The bundle schema differs from the running feature schema.
    StaleSchema,
    /// The bundle bytes do not decode into a shape-valid model.
    InvalidBundle,
}

impl std::fmt::Display for CorrectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "model correction: {self:?}")
    }
}

impl std::error::Error for CorrectionError {}

/// Recover a poisoned monitor lock; every guarded update is applied atomically.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// Validate a finite `f32` probability-like value in `[0, 1]`.
fn probability(value: f32) -> Result<f32, CorrectionError> {
    if !value.is_finite() {
        return Err(CorrectionError::Nonfinite);
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(CorrectionError::Probability);
    }
    Ok(value)
}

/// Validate a finite `f64` probability-like value in `[0, 1]`.
fn probability64(value: f64) -> Result<f64, CorrectionError> {
    if !value.is_finite() {
        return Err(CorrectionError::Nonfinite);
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(CorrectionError::Probability);
    }
    Ok(value)
}
