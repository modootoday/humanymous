//! Lab observation replay through the sole detection authority; no HTTP enforcement.
use humanymous_scoring::{
    engine::{Engine, ScoringError},
    trace::ScoreTrace,
};
use humanymous_signals::report::{ScoreResult, SessionReport};
use serde::Serialize;

/// Real engine result with the engine's explanation and derived report.
#[derive(Debug, Serialize)]
pub struct Evaluation {
    /// Report after the engine derives consistency observations.
    pub report: SessionReport,
    /// Actual verdict, including CHALLENGE as distinct from ALLOW.
    pub result: ScoreResult,
    /// Same-path scoring explanation, without harness verdict logic.
    pub trace: ScoreTrace,
}

/// Replay with frozen defaults. Errors propagate without a manufactured verdict.
pub fn evaluate(mut report: SessionReport) -> Result<Evaluation, ScoringError> {
    let (result, trace) = Engine::default().score_with_trace(&mut report)?;
    Ok(Evaluation {
        report,
        result,
        trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_client_retains_real_engine_challenge() {
        let actual = evaluate(SessionReport::default()).unwrap();
        assert_eq!(actual.result.verdict, "CHALLENGE");
        assert_eq!(actual.result.hard_rule_fired, "HR-10");
        assert_eq!(actual.result.verdict, actual.trace.verdict);
    }
    #[test]
    fn invalid_arithmetic_is_an_error_without_verdict() {
        use humanymous_signals::observation::Signal;
        let mut report = SessionReport::default();
        report.client.signals = Some(vec![Signal {
            score: f64::NAN,
            ..Default::default()
        }]);
        assert!(evaluate(report).is_err());
    }
}
