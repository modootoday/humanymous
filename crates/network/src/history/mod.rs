//! Bounded keyed measurement history; no signal or verdict authority.
pub mod anomaly;
pub mod rate;
use std::time::Duration;

/// Missing history/measurement, never a healthy normal observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// Invalid parameters or zero/overflowing resource limits.
    InvalidConfiguration,
    /// Non-finite input or derived floating-point arithmetic.
    InvalidMeasurement,
    /// Caller time regressed behind the last accepted operation.
    TimeRegression,
    /// A configured resource bound prevents admission.
    Budget,
}

// One ordered caller-clock contract for history domains. Owners commit only
// after all fallible admission/measurement work has succeeded.
struct OrderedTime {
    last: Option<Duration>,
}
impl OrderedTime {
    fn new() -> Self {
        Self { last: None }
    }
    fn check(&self, now: Duration) -> Result<(), Unavailable> {
        if self.last.is_some_and(|last| now < last) {
            return Err(Unavailable::TimeRegression);
        }
        Ok(())
    }
    fn commit(&mut self, now: Duration) {
        self.last = Some(now);
    }
}
