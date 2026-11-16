//! Active-versus-candidate divergence, observed without serving the candidate.
//!
//! Quantifies how far the calibrated probability moved and, at a fire
//! threshold, how often the candidate would fire differently, split by
//! direction. Everything is aggregate; no identifiers are retained.
use super::{CorrectionError, lock, probability};
use crate::model::inference::Prediction;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct State {
    n: u64,
    sum_abs_delta: f64,
    both_scored: u64,
    shadow_abstain: u64,
    active_abstain: u64,
    fire_agree: u64,
    shadow_fires_more: u64,
    active_fires_more: u64,
    rejected: u64,
}

/// Aggregate active/candidate agreement statistics. Safe for concurrent use.
#[derive(Debug, Default)]
pub struct ShadowComparator {
    state: Mutex<State>,
}

/// Observability view of an in-progress shadow evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowStats {
    /// Accepted pairs seen with a candidate installed.
    pub n: u64,
    /// Pairs both models scored.
    pub both_scored: u64,
    /// Mean absolute probability delta over comparable pairs.
    pub mean_abs_delta: f64,
    /// Fraction of comparable pairs firing the same way.
    pub fire_agreement: f64,
    /// Candidate would fire where active does not.
    pub shadow_fires_more: u64,
    /// Active fires where the candidate would not.
    pub active_fires_more: u64,
    /// Candidate abstained where active scored.
    pub shadow_abstained: u64,
    /// Active abstained where the candidate scored (tracked but unexposed in legacy).
    pub active_abstained: u64,
    /// Pairs rejected as invalid; not part of any other count (Rust addition).
    pub rejected: u64,
}

impl ShadowComparator {
    /// Record one pair at fire threshold `theta`. Abstaining probabilities are
    /// ignored; scored probabilities must be valid.
    ///
    /// Stricter than legacy: a nonfinite or out-of-range threshold or scored
    /// probability rejects the whole pair, counted only in `rejected`.
    pub fn observe(
        &self,
        active: Prediction,
        shadow: Prediction,
        theta: f32,
    ) -> Result<(), CorrectionError> {
        let checked = probability(theta).and_then(|_| {
            if !active.abstain {
                probability(active.p_bot)?;
            }
            if !shadow.abstain {
                probability(shadow.p_bot)?;
            }
            Ok(())
        });
        let mut state = lock(&self.state);
        if let Err(error) = checked {
            state.rejected = state.rejected.saturating_add(1);
            return Err(error);
        }
        state.n = state.n.saturating_add(1);
        if active.abstain && !shadow.abstain {
            state.active_abstain = state.active_abstain.saturating_add(1);
        }
        if shadow.abstain && !active.abstain {
            state.shadow_abstain = state.shadow_abstain.saturating_add(1);
        }
        if active.abstain || shadow.abstain {
            return Ok(());
        }
        state.both_scored = state.both_scored.saturating_add(1);
        state.sum_abs_delta += f64::from((active.p_bot - shadow.p_bot).abs());
        let active_fires = active.p_bot >= theta;
        let shadow_fires = shadow.p_bot >= theta;
        let counter = match (active_fires, shadow_fires) {
            (left, right) if left == right => &mut state.fire_agree,
            (false, true) => &mut state.shadow_fires_more,
            _ => &mut state.active_fires_more,
        };
        *counter = counter.saturating_add(1);
        Ok(())
    }

    /// Current comparison; safe to expose on a lab operations surface.
    pub fn snapshot(&self) -> ShadowStats {
        let state = lock(&self.state);
        let mut stats = ShadowStats {
            n: state.n,
            both_scored: state.both_scored,
            mean_abs_delta: 0.0,
            fire_agreement: 0.0,
            shadow_fires_more: state.shadow_fires_more,
            active_fires_more: state.active_fires_more,
            shadow_abstained: state.shadow_abstain,
            active_abstained: state.active_abstain,
            rejected: state.rejected,
        };
        if state.both_scored > 0 {
            stats.mean_abs_delta = state.sum_abs_delta / state.both_scored as f64;
            stats.fire_agreement = state.fire_agree as f64 / state.both_scored as f64;
        }
        stats
    }
}

#[cfg(test)]
mod tests;
