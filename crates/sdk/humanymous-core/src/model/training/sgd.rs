//! Bounded offline online-SGD; no serving, publication or activation authority.
//!
//! Operation order follows internal/mltrain/train.go, not its mini-batch comment.
//! Cross-language bit parity needs an exact Go toolchain/target oracle: Go may FMA.
use super::rng::{GoRng, InitialWeights, RngError};
use super::standardize::Standardization;
use crate::behavior::features::{FEATURE_DIM, FeatureVector, schema_hash};
use crate::model::inference::MlpBundle;

/// Numeric input only: the caller separately reviews label provenance/admission.
#[derive(Clone, Copy)]
pub(crate) struct TrainingRow<'a> {
    pub(crate) features: &'a FeatureVector,
    pub(crate) label: u8,
}

#[derive(Clone, Copy)]
pub(crate) struct TrainConfig<'a> {
    pub(crate) hidden: i64,
    pub(crate) epochs: i64,
    pub(crate) learning_rate: f32,
    pub(crate) seed: i64,
    pub(crate) version: &'a str,
}

/// All configurable capacities may tighten, never widen, these defaults.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrainLimits {
    pub(crate) samples: usize,
    pub(crate) hidden: usize,
    pub(crate) epochs: u64,
    pub(crate) updates: u64,
    pub(crate) work_units: u64,
    pub(crate) rng_draws: u64,
}

impl Default for TrainLimits {
    fn default() -> Self {
        Self {
            samples: 100_000,
            hidden: 256,
            epochs: 1_000,
            updates: 1_000_000,
            work_units: 100_000_000,
            rng_draws: 1_000_000,
        }
    }
}

/// Stable categories intentionally exclude input values and parser diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainError {
    Config,
    Limit,
    Schema,
    Label,
    Statistics,
    WarmBase,
    Nonfinite,
    RngBudget,
    Allocation,
}

impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "offline training: {self:?}")
    }
}

impl std::error::Error for TrainError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrainingSummary {
    pub(crate) hidden: usize,
    pub(crate) epochs: u64,
    pub(crate) learning_rate: f32,
    pub(crate) updates: u64,
    pub(crate) rng_draws: u64,
    pub(crate) final_order: Vec<usize>,
}

#[derive(Debug)]
pub(crate) struct TrainingResult {
    pub(crate) candidate: MlpBundle,
    pub(crate) summary: TrainingSummary,
}

struct Effective {
    hidden: usize,
    epochs: u64,
    lr: f32,
    updates: u64,
}

fn validate(
    config: TrainConfig<'_>,
    rows: &[TrainingRow<'_>],
    stats: &Standardization,
    expected_schema: &str,
    base: Option<&MlpBundle>,
    limits: TrainLimits,
) -> Result<Effective, TrainError> {
    let cap = TrainLimits::default();
    if limits.samples == 0
        || limits.samples > cap.samples
        || limits.hidden == 0
        || limits.hidden > cap.hidden
        || limits.epochs == 0
        || limits.epochs > cap.epochs
        || limits.updates == 0
        || limits.updates > cap.updates
        || limits.work_units == 0
        || limits.work_units > cap.work_units
        || limits.rng_draws == 0
        || limits.rng_draws > cap.rng_draws
    {
        return Err(TrainError::Limit);
    }
    if expected_schema != schema_hash() {
        return Err(TrainError::Schema);
    }
    if !config.learning_rate.is_finite() || config.version.len() > 128 {
        return Err(TrainError::Config);
    }
    let hidden = if config.hidden <= 0 {
        16
    } else {
        usize::try_from(config.hidden).map_err(|_| TrainError::Limit)?
    };
    let epochs = config.epochs.max(0) as u64;
    let lr = if config.learning_rate <= 0.0 {
        0.05
    } else {
        config.learning_rate
    };
    if hidden > limits.hidden || epochs > limits.epochs || rows.len() > limits.samples {
        return Err(TrainError::Limit);
    }
    let updates = (rows.len() as u64)
        .checked_mul(epochs)
        .ok_or(TrainError::Limit)?;
    let units = updates
        .checked_mul(hidden as u64)
        .and_then(|v| v.checked_mul(FEATURE_DIM as u64))
        .ok_or(TrainError::Limit)?;
    if updates > limits.updates || units > limits.work_units {
        return Err(TrainError::Limit);
    }
    if stats.mean.iter().any(|v| !v.is_finite())
        || stats.std.iter().any(|v| !v.is_finite() || *v <= 0.0)
    {
        return Err(TrainError::Statistics);
    }
    for row in rows {
        if row.label > 1 {
            return Err(TrainError::Label);
        }
        if row.features.iter().any(|v| !v.is_finite()) {
            return Err(TrainError::Nonfinite);
        }
    }
    if let Some(base) = base
        && (base.validate().is_err()
            || base.hidden != hidden as i64
            || base.schema_hash != expected_schema
            || base
                .w1
                .iter()
                .flatten()
                .chain(&base.b1)
                .chain(&base.w2)
                .chain(&base.mean)
                .chain(&base.std)
                .chain([&base.b2, &base.cal_a, &base.cal_b])
                .any(|v| !v.is_finite()))
    {
        return Err(TrainError::WarmBase);
    }
    Ok(Effective {
        hidden,
        epochs,
        lr,
        updates,
    })
}

fn finite(value: f32) -> Result<f32, TrainError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TrainError::Nonfinite)
    }
}

fn copy_buffer<T: Copy>(values: &[T]) -> Result<Vec<T>, TrainError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(values.len())
        .map_err(|_| TrainError::Allocation)?;
    result.extend_from_slice(values);
    Ok(result)
}

fn zero_buffer(size: usize) -> Result<Vec<f32>, TrainError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(size)
        .map_err(|_| TrainError::Allocation)?;
    result.resize(size, 0.0);
    Ok(result)
}

fn rng_error(error: RngError) -> TrainError {
    match error {
        RngError::DrawLimit => TrainError::RngBudget,
        RngError::Nonfinite => TrainError::Nonfinite,
        _ => TrainError::WarmBase,
    }
}

/// One update, with reusable scratch space. Never computes hidden gradients from
/// an already-updated output weight. No fused multiply-add or reordered reduction.
fn update(
    weights: &mut InitialWeights,
    row: TrainingRow<'_>,
    stats: &Standardization,
    lr: f32,
    pre: &mut [f32],
    act: &mut [f32],
) -> Result<(), TrainError> {
    let mut z = [0.0; FEATURE_DIM];
    for (i, value) in z.iter_mut().enumerate() {
        *value = finite(finite(row.features[i] - stats.mean[i])? / stats.std[i])?;
    }
    let mut logit = weights.b2;
    for h in 0..weights.w2.len() {
        let mut acc = weights.b1[h];
        for (i, input) in z.iter().enumerate() {
            acc = finite(acc + finite(weights.w1[h][i] * input)?)?;
        }
        pre[h] = acc;
        act[h] = if acc > 0.0 { acc } else { 0.0 };
        logit = finite(logit + finite(weights.w2[h] * act[h])?)?;
    }
    // exp may be +infinity at a finite negative logit: Go's sigmoid limit is zero.
    let p = finite((1.0_f64 / (1.0 + f64::from(-logit).exp())) as f32)?;
    let d = finite(p - f32::from(row.label))?;
    for h in 0..weights.w2.len() {
        let gw2 = finite(d * act[h])?;
        let dhact = finite(d * weights.w2[h])?;
        let dhpre = if pre[h] > 0.0 { dhact } else { 0.0 };
        let step = finite(lr * dhpre)?;
        for (i, input) in z.iter().enumerate() {
            weights.w1[h][i] = finite(weights.w1[h][i] - finite(step * input)?)?;
        }
        weights.b1[h] = finite(weights.b1[h] - step)?;
        weights.w2[h] = finite(weights.w2[h] - finite(lr * gw2)?)?;
    }
    weights.b2 = finite(weights.b2 - finite(lr * d)?)?;
    Ok(())
}

/// Produce an owned offline candidate atomically with respect to all borrowed
/// inputs. New statistics and reset calibration can change even zero-epoch output.
/// RNG initialize/shuffle have bounded infallible allocations: no global OOM claim.
pub(crate) fn train(
    config: TrainConfig<'_>,
    rows: &[TrainingRow<'_>],
    statistics: &Standardization,
    expected_schema: &str,
    base: Option<&MlpBundle>,
    limits: TrainLimits,
) -> Result<TrainingResult, TrainError> {
    let effective = validate(config, rows, statistics, expected_schema, base, limits)?;
    let mut version = String::new();
    version
        .try_reserve_exact(config.version.len())
        .map_err(|_| TrainError::Allocation)?;
    version.push_str(config.version);
    let mean = copy_buffer(&statistics.mean)?;
    let std = copy_buffer(&statistics.std)?;
    let mut order = Vec::new();
    order
        .try_reserve_exact(rows.len())
        .map_err(|_| TrainError::Allocation)?;
    order.extend(0..rows.len());
    let mut pre = zero_buffer(effective.hidden)?;
    let mut act = zero_buffer(effective.hidden)?;
    let mut rng = GoRng::new(config.seed, limits.rng_draws).map_err(rng_error)?;
    let mut weights = rng.initialize(effective.hidden, base).map_err(rng_error)?;
    let mut updates = 0_u64;
    if !rows.is_empty() {
        for _ in 0..effective.epochs {
            rng.shuffle(&mut order).map_err(rng_error)?;
            for &id in &order {
                update(
                    &mut weights,
                    rows[id],
                    statistics,
                    effective.lr,
                    &mut pre,
                    &mut act,
                )?;
                updates += 1;
            }
        }
    }
    debug_assert_eq!(updates, effective.updates);
    let candidate = MlpBundle {
        version,
        schema_hash: schema_hash(),
        feature_dim: FEATURE_DIM as i64,
        hidden: effective.hidden as i64,
        mean,
        std,
        w1: weights.w1,
        b1: weights.b1,
        w2: weights.w2,
        b2: weights.b2,
        cal_a: 1.0,
        cal_b: 0.0,
    };
    candidate.validate().map_err(|_| TrainError::WarmBase)?;
    Ok(TrainingResult {
        candidate,
        summary: TrainingSummary {
            hidden: effective.hidden,
            epochs: effective.epochs,
            learning_rate: effective.lr,
            updates,
            rng_draws: rng.snapshot().draws,
            final_order: order,
        },
    })
}

#[cfg(test)]
mod pilot;
#[cfg(test)]
mod tests;
