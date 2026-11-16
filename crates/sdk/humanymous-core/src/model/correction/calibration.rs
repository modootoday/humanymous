//! Human-only adaptive conformal threshold proposals (Gibbs and Candes ACI).
//!
//! The controller proposes the probability cut-point at which the behavioral
//! residual annotation would fire, driving the realized false-positive rate on
//! oracle-confirmed HUMAN sessions toward a budget. Bots and ambiguous outcomes
//! never move it. The proposal is lab-only and is never a production
//! `ThresholdProvider`.
use super::{CorrectionError, lock, probability};
use std::sync::Mutex;

/// The proposal never drops below this floor, so it cannot fire on everyone.
const MIN_THETA: f32 = 0.5;
/// Decay of the realized human false-positive EMA.
const FP_EMA_DECAY: f32 = 0.02;

#[derive(Debug, Clone, Copy)]
struct State {
    theta: f32,
    budget: f32,
    gamma: f32,
    fp_ema: f32,
    humans: u64,
    bots: u64,
}

/// Adaptive conformal controller for one cohort. Safe for concurrent use.
#[derive(Debug)]
pub struct ConformalThreshold {
    state: Mutex<State>,
}

/// Aggregate calibration snapshot without identifiers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationStats {
    /// Current proposed firing threshold.
    pub theta: f32,
    /// Target human false-positive rate.
    pub budget: f32,
    /// EMA of the realized human false-positive rate.
    pub human_fp_est: f32,
    /// Confirmed humans observed.
    pub humans_seen: u64,
    /// Confirmed bots observed.
    pub bots_seen: u64,
}

impl ConformalThreshold {
    /// Build a controller. Finite out-of-range values take the legacy defaults
    /// (budget 0.005 unless in `(0, 1)`, theta0 0.5 unless in `(0, 1)`, gamma
    /// 0.01 unless in `(0, 0.5]`).
    ///
    /// Stricter than legacy: NaN or infinite arguments are rejected instead of
    /// being retained, because NaN passed every legacy range comparison.
    pub fn new(budget: f32, theta0: f32, gamma: f32) -> Result<Self, CorrectionError> {
        if !budget.is_finite() || !theta0.is_finite() || !gamma.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        let budget = if budget <= 0.0 || budget >= 1.0 {
            0.005
        } else {
            budget
        };
        let theta = if theta0 <= 0.0 || theta0 >= 1.0 {
            0.5
        } else {
            theta0
        };
        let gamma = if gamma <= 0.0 || gamma > 0.5 {
            0.01
        } else {
            gamma
        };
        Ok(Self {
            state: Mutex::new(State {
                theta,
                budget,
                gamma,
                fp_ema: budget,
                humans: 0,
                bots: 0,
            }),
        })
    }

    /// Current proposed firing threshold.
    pub fn threshold(&self) -> f32 {
        lock(&self.state).theta
    }

    /// Whether a probability is at or above the current proposal.
    ///
    /// Stricter than legacy: nonfinite or out-of-range probabilities are rejected.
    pub fn fires(&self, p_bot: f32) -> Result<bool, CorrectionError> {
        let p_bot = probability(p_bot)?;
        Ok(p_bot >= self.threshold())
    }

    /// Update from ONE oracle-confirmed session. `is_human` must come from an
    /// independent adjudicator, never from the model's own probability. Bot
    /// observations are counted but never move the proposal.
    ///
    /// Stricter than legacy: an invalid probability is rejected before any count
    /// or threshold changes.
    pub fn observe(&self, is_human: bool, p_bot: f32) -> Result<(), CorrectionError> {
        let p_bot = probability(p_bot)?;
        let mut state = lock(&self.state);
        if !is_human {
            state.bots = state.bots.saturating_add(1);
            return Ok(());
        }
        state.humans = state.humans.saturating_add(1);
        let flagged: f32 = if p_bot >= state.theta { 1.0 } else { 0.0 };
        // All operands are validated finite, so clamping cannot observe NaN.
        state.theta = (state.theta + state.gamma * (flagged - state.budget)).clamp(MIN_THETA, 1.0);
        state.fp_ema += FP_EMA_DECAY * (flagged - state.fp_ema);
        Ok(())
    }

    /// Current controller state for lab observability and the canary guard.
    pub fn snapshot(&self) -> CalibrationStats {
        let state = *lock(&self.state);
        CalibrationStats {
            theta: state.theta,
            budget: state.budget,
            human_fp_est: state.fp_ema,
            humans_seen: state.humans,
            bots_seen: state.bots,
        }
    }
}

#[cfg(test)]
mod tests;
