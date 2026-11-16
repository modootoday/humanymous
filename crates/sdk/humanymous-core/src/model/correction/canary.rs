//! Probationary lab rollback watchdog.
//!
//! When armed, every oracle confirmation re-checks the guard budgets and the
//! FIRST breach invokes the caller's rollback callback exactly once, outside
//! the guard lock. A breach-free probation graduates and disarms. Rollback is
//! the conservative direction; this guard never promotes or activates a model,
//! and the callback is only a lab notification.
use super::{CorrectionError, lock, probability};
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// Confirmed humans required after arming before the FP budget is judged.
pub const MIN_CANARY_OBSERVE: u64 = 20;

/// Lab rollback notification, invoked outside every lock with the breach reason.
pub type RollbackCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Probation policy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CanaryBudget {
    /// Realized human-FP EMA ceiling during probation.
    pub max_human_fp: f32,
    /// Graduate after this many confirmed humans without a breach.
    pub probation_humans: u64,
    /// Trip when the 2-of-3 drift gate fires.
    pub rollback_on_drift: bool,
    /// Trip when the Pass solve rate looks poisoned.
    pub rollback_on_pass_anomaly: bool,
}

/// Probation lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CanaryState {
    /// No probation armed.
    #[default]
    Off,
    /// Probation in progress.
    Armed,
    /// A breach fired the rollback notification.
    Tripped,
    /// Probation completed without a breach.
    Graduated,
}

impl CanaryState {
    /// Legacy lowercase state name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Armed => "armed",
            Self::Tripped => "tripped",
            Self::Graduated => "graduated",
        }
    }
}

/// Probation observability view.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanaryStats {
    /// Current lifecycle state.
    pub state: CanaryState,
    /// Breach reason when tripped.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// Confirmed humans since arming.
    pub humans_into_probation: u64,
    /// Humans required to graduate.
    pub probation_humans: u64,
}

struct State {
    budget: CanaryBudget,
    state: CanaryState,
    armed_at_humans: u64,
    reason: String,
    on_rollback: Option<RollbackCallback>,
}

/// One-shot probation guard. Safe for concurrent use.
pub struct CanaryGuard {
    state: Mutex<State>,
}

impl std::fmt::Debug for CanaryGuard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = lock(&self.state);
        formatter
            .debug_struct("CanaryGuard")
            .field("budget", &state.budget)
            .field("state", &state.state)
            .field("armed_at_humans", &state.armed_at_humans)
            .finish_non_exhaustive()
    }
}

impl CanaryGuard {
    /// Build a guard. Finite out-of-range ceilings use the legacy default 0.02
    /// and a zero probation uses 200 humans.
    ///
    /// Stricter than legacy: a nonfinite FP ceiling is rejected.
    pub fn new(
        mut budget: CanaryBudget,
        on_rollback: Option<RollbackCallback>,
    ) -> Result<Self, CorrectionError> {
        if !budget.max_human_fp.is_finite() {
            return Err(CorrectionError::Nonfinite);
        }
        if budget.max_human_fp <= 0.0 || budget.max_human_fp >= 1.0 {
            budget.max_human_fp = 0.02;
        }
        if budget.probation_humans == 0 {
            budget.probation_humans = 200;
        }
        Ok(Self {
            state: Mutex::new(State {
                budget,
                state: CanaryState::Off,
                armed_at_humans: 0,
                reason: String::new(),
                on_rollback,
            }),
        })
    }

    /// Start probation from the current confirmed-human count.
    pub fn arm(&self, humans_seen: u64) {
        let mut state = lock(&self.state);
        state.state = CanaryState::Armed;
        state.armed_at_humans = humans_seen;
        state.reason.clear();
    }

    /// Re-check one oracle confirmation: trip on the first breach, or graduate
    /// after a breach-free probation. A no-op unless armed.
    ///
    /// Stricter than legacy: an invalid FP estimate is rejected, and a human
    /// count below the arm point is `CountRegressed` instead of an unsigned
    /// wrap that would immediately trip or graduate.
    pub(super) fn evaluate(
        &self,
        humans_seen: u64,
        human_fp: f32,
        drift_fired: bool,
        pass_anomalous: bool,
    ) -> Result<(), CorrectionError> {
        let human_fp = probability(human_fp)?;
        let mut state = lock(&self.state);
        if state.state != CanaryState::Armed {
            return Ok(());
        }
        let since_arm = humans_seen
            .checked_sub(state.armed_at_humans)
            .ok_or(CorrectionError::CountRegressed)?;
        let budget = state.budget;
        let trip = if since_arm >= MIN_CANARY_OBSERVE && human_fp > budget.max_human_fp {
            Some(format!(
                "human-FP {human_fp:.4} over canary budget {:.4}",
                budget.max_human_fp
            ))
        } else if budget.rollback_on_drift && drift_fired {
            Some("drift gate fired during probation".to_owned())
        } else if budget.rollback_on_pass_anomaly && pass_anomalous {
            Some("Pass solve-rate anomaly (suspected poisoning) during probation".to_owned())
        } else {
            None
        };
        if let Some(reason) = trip {
            state.state = CanaryState::Tripped;
            state.reason.clone_from(&reason);
            let callback = state.on_rollback.clone();
            drop(state);
            if let Some(callback) = callback {
                callback(&reason);
            }
            return Ok(());
        }
        if since_arm >= budget.probation_humans {
            state.state = CanaryState::Graduated;
        }
        Ok(())
    }

    /// Guard state relative to the current confirmed-human count.
    pub fn snapshot(&self, humans_seen: u64) -> CanaryStats {
        let state = lock(&self.state);
        let humans_into_probation = if state.state != CanaryState::Off {
            humans_seen.saturating_sub(state.armed_at_humans)
        } else {
            0
        };
        CanaryStats {
            state: state.state,
            reason: state.reason.clone(),
            humans_into_probation,
            probation_humans: state.budget.probation_humans,
        }
    }
}

#[cfg(test)]
mod tests;
