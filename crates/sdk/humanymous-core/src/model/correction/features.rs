//! Covariate-shift arm of the drift gate.
//!
//! The first `window` feature vectors become a frozen reference: per feature,
//! quantile bucket edges and reference fractions are fixed. Every following
//! `window` observations form a current window compared by PSI; the maximum
//! over features is the label-free covariate reading. Per-feature buckets are
//! required because extracted features have heterogeneous ranges.
use super::{CorrectionError, lock, psi};
use crate::behavior::features::{FEATURE_DIM, FeatureVector};
use std::sync::Mutex;

/// Quantile buckets per feature.
pub const PSI_BUCKETS: usize = 8;
/// Legacy default reference and current window size.
pub const DEFAULT_WINDOW: usize = 500;
/// Largest accepted window, bounding the reference buffer.
pub const MAX_WINDOW: usize = 4096;

#[derive(Debug)]
struct Frozen {
    edges: [[f64; PSI_BUCKETS + 1]; FEATURE_DIM],
    reference: [[f64; PSI_BUCKETS]; FEATURE_DIM],
    counts: [[usize; PSI_BUCKETS]; FEATURE_DIM],
    current: usize,
}

#[derive(Debug)]
struct State {
    reference: Vec<FeatureVector>,
    frozen: Option<Box<Frozen>>,
    last_max: f64,
}

/// Reference-then-rolling feature distribution monitor. Safe for concurrent use.
#[derive(Debug)]
pub struct FeatureMonitor {
    window: usize,
    state: Mutex<State>,
}

impl Default for FeatureMonitor {
    fn default() -> Self {
        Self {
            window: DEFAULT_WINDOW,
            state: Mutex::new(State {
                reference: Vec::new(),
                frozen: None,
                last_max: 0.0,
            }),
        }
    }
}

impl FeatureMonitor {
    /// Build a monitor with an explicit window.
    ///
    /// Stricter than legacy: zero is a `Limit` error instead of selecting the
    /// default (use [`FeatureMonitor::default`]), and windows above
    /// [`MAX_WINDOW`] are rejected to bound the reference buffer.
    pub fn new(window: usize) -> Result<Self, CorrectionError> {
        if window == 0 || window > MAX_WINDOW {
            return Err(CorrectionError::Limit);
        }
        Ok(Self {
            window,
            ..Self::default()
        })
    }

    /// Configured window size.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Feed one feature vector. Returns `Some(max_psi)` exactly when a current
    /// window closes, otherwise `None`.
    ///
    /// Stricter than legacy: a wrong width is a `Shape` error instead of a silent
    /// no-op, and nonfinite features are rejected before any state changes.
    pub fn observe(&self, features: &[f32]) -> Result<Option<f64>, CorrectionError> {
        let vector: FeatureVector = features.try_into().map_err(|_| CorrectionError::Shape)?;
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(CorrectionError::Nonfinite);
        }
        let mut state = lock(&self.state);
        let Some(frozen) = state.frozen.as_mut() else {
            state.reference.push(vector);
            if state.reference.len() >= self.window {
                let reference = std::mem::take(&mut state.reference);
                state.frozen = Some(Box::new(freeze(&reference)));
            }
            return Ok(None);
        };
        for (index, value) in vector.iter().enumerate() {
            let bucket = bucket_of(&frozen.edges[index], f64::from(*value));
            frozen.counts[index][bucket] += 1;
        }
        frozen.current += 1;
        if frozen.current < self.window {
            return Ok(None);
        }
        let mut max = 0.0;
        for index in 0..FEATURE_DIM {
            let current = fractions(&frozen.counts[index], frozen.current);
            let reading = psi(&frozen.reference[index], &current)?;
            if reading > max {
                max = reading;
            }
            frozen.counts[index] = [0; PSI_BUCKETS];
        }
        frozen.current = 0;
        state.last_max = max;
        Ok(Some(max))
    }

    /// Most recent closed-window reading; zero before the first window closes.
    pub fn last_max_psi(&self) -> f64 {
        lock(&self.state).last_max
    }

    /// Whether the reference window has been captured and frozen.
    pub fn ready(&self) -> bool {
        lock(&self.state).frozen.is_some()
    }
}

fn freeze(reference: &[FeatureVector]) -> Frozen {
    let mut frozen = Frozen {
        edges: [[0.0; PSI_BUCKETS + 1]; FEATURE_DIM],
        reference: [[0.0; PSI_BUCKETS]; FEATURE_DIM],
        counts: [[0; PSI_BUCKETS]; FEATURE_DIM],
        current: 0,
    };
    for index in 0..FEATURE_DIM {
        let mut values: Vec<f64> = reference
            .iter()
            .map(|vector| f64::from(vector[index]))
            .collect();
        values.sort_by(f64::total_cmp);
        frozen.edges[index] = quantile_edges(&values);
        let mut counts = [0; PSI_BUCKETS];
        for value in &values {
            counts[bucket_of(&frozen.edges[index], *value)] += 1;
        }
        frozen.reference[index] = fractions(&counts, values.len());
    }
    frozen
}

/// Edges at even quantiles of sorted values; degenerate features collapse.
fn quantile_edges(sorted: &[f64]) -> [f64; PSI_BUCKETS + 1] {
    let mut edges = [0.0; PSI_BUCKETS + 1];
    let count = sorted.len();
    if count == 0 {
        return edges;
    }
    for (bucket, edge) in edges.iter_mut().enumerate() {
        *edge = sorted[bucket * (count - 1) / PSI_BUCKETS];
    }
    edges
}

/// Highest bucket whose lower edge is at or below the value; clamped to zero.
fn bucket_of(edges: &[f64; PSI_BUCKETS + 1], value: f64) -> usize {
    (0..PSI_BUCKETS)
        .rev()
        .find(|bucket| value >= edges[*bucket])
        .unwrap_or(0)
}

fn fractions(counts: &[usize; PSI_BUCKETS], total: usize) -> [f64; PSI_BUCKETS] {
    let mut out = [0.0; PSI_BUCKETS];
    if total == 0 {
        return out;
    }
    for (fraction, count) in out.iter_mut().zip(counts) {
        *fraction = *count as f64 / total as f64;
    }
    out
}

#[cfg(test)]
mod tests;
