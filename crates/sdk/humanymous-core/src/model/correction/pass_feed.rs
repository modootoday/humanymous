//! Pass-outcome feed into the lab correction controller (legacy
//! cmd/server/mlcorrect_ops.go `feedPassOutcome` and `cohortOf` at baseline
//! 981d25c).
//!
//! A solved Pass is a confirmed human: it yields a labeled, modality-tagged
//! trace for offline retraining and calibrates the controller against the
//! engine's own verdict (the STUDD teacher). A failed Pass only feeds the
//! poisoning guard's solve rate; it is never a bot label. An abstaining model
//! contributes nothing. Nothing here mutates a verdict or installs a threshold.
//!
//! The solved-Pass trace reaches the opt-in JSONL sink through
//! [`append_trace`] (legacy `a.traceSink.Append`); the operator snapshot lives
//! in the runtime's `gateway::operations::mlcorrect`.
use super::{Controller, CorrectionError, DriftEvent, Outcome};
use crate::model::inference::Prediction;
use crate::model::training::record::Record;
use crate::model::training::sink::{SinkError, TraceSink};
use crate::signals::report::BehaviorSummary;

/// Input-modality cohort used to hold human-FP near zero per modality, so
/// keyboard-only and assistive-technology users are not hidden behind the
/// pointer majority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cohort {
    /// Fewer than four pointer samples but some typing.
    Keyboard,
    /// Real pointer motion.
    Pointer,
    /// Neither.
    Default,
}

impl Cohort {
    /// Legacy trace spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Pointer => "pointer",
            Self::Default => "default",
        }
    }
}

/// Classify input modality exactly like Go's `cohortOf`.
pub fn cohort_of(behavior: &BehaviorSummary) -> Cohort {
    if behavior.mouse.samples < 4 {
        if behavior.key.keystrokes > 0 {
            Cohort::Keyboard
        } else {
            Cohort::Default
        }
    } else {
        Cohort::Pointer
    }
}

/// Labeled human trace for offline retraining (legacy `mltrain.Record`).
#[derive(Debug, Clone, PartialEq)]
pub struct HumanTrace {
    /// Always 0 (human).
    pub label: u8,
    /// Collected behavior summary.
    pub behavior: BehaviorSummary,
    /// Report timestamp, Unix seconds.
    pub ts: i64,
    /// Input-modality cohort.
    pub cohort: Cohort,
    /// Always `pass`.
    pub source: &'static str,
}

impl HumanTrace {
    /// The legacy `mltrain.Record` written for this trace.
    pub fn to_record(&self) -> Record {
        Record {
            label: self.label,
            behavior: self.behavior.clone(),
            ts: Some(self.ts),
            cohort: Some(self.cohort.as_str().to_owned()),
            source: Some(self.source.to_owned()),
        }
    }
}

/// Persist a solved Pass's trace to the opt-in sink (legacy
/// `a.traceSink.Append` in `feedPassOutcome`). `None` sink (collection off) and
/// a failed Pass write nothing. Returns whether a line was written. An error is
/// non-fatal by contract: Go logged a warning and continued, so the caller
/// logs it and never changes a decision.
pub fn append_trace(sink: Option<&mut TraceSink>, fed: &Fed) -> Result<bool, SinkError> {
    match (sink, &fed.trace) {
        (Some(sink), Some(trace)) => sink.append(&trace.to_record()).map(|()| true),
        _ => Ok(false),
    }
}

/// What one Pass outcome produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Fed {
    /// Human trace for a solved Pass; `None` for a failure.
    pub trace: Option<HumanTrace>,
    /// Controller drift evaluation when calibration ran.
    pub drift: Option<DriftEvent>,
}

/// Feed one Pass outcome. `controller` is `None` when no bundle is loaded.
/// `engine_denied` is whether the frozen engine's verdict was DENY.
pub fn feed_pass_outcome(
    controller: Option<&Controller>,
    behavior: &BehaviorSummary,
    ts: i64,
    solved: bool,
    prediction: Prediction,
    engine_denied: bool,
) -> Result<Fed, CorrectionError> {
    let trace = solved.then(|| HumanTrace {
        label: 0,
        behavior: behavior.clone(),
        ts,
        cohort: cohort_of(behavior),
        source: "pass",
    });
    let Some(controller) = controller else {
        return Ok(Fed { trace, drift: None });
    };
    controller.observe_pass(solved);
    if !solved || prediction.abstain {
        return Ok(Fed { trace, drift: None });
    }
    let drift = controller.observe_outcome(Outcome::PassSolved, prediction.p_bot, engine_denied)?;
    Ok(Fed {
        trace,
        drift: Some(drift),
    })
}

#[cfg(test)]
mod tests;
