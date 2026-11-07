//! Consumer bookkeeping units, not provider execution or a serving capability.
use super::*;
use std::cell::Cell;

fn tick(value: u64) -> Duration {
    Duration::from_secs(value)
}
fn counted(count: usize, coverage: rate::Coverage) -> rate::Observation {
    rate::Observation::Counted {
        count,
        level: rate::Level::Normal,
        coverage,
    }
}
fn correlation(scope: correlation::Scope) -> correlation::Outcome {
    correlation::Outcome {
        signals: Vec::new(),
        scope,
    }
}

mod categories;
mod continuity;
