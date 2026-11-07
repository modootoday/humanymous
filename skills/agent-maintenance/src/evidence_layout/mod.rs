//! Supplied-assertion planning only. No candidate/archive I/O or retirement authority.
mod model;
mod validate;
use crate::selected_inputs::{
    Clock, ContentRead, Limits, MAX_REQUEST_BYTES, RequestError, SystemClock, capture_request_file,
};
pub use model::Plan;
use serde::Serialize;
use std::path::Path;

/// Strict finite JSON; diagnostics never include submitted field values.
pub fn decode_and_plan(bytes: &[u8]) -> Result<Plan, RequestError> {
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err(RequestError::TooLarge);
    }
    let request: model::Request =
        serde_json::from_slice(bytes).map_err(|error| RequestError::InvalidJson {
            line: error.line(),
            column: error.column(),
        })?;
    Ok(validate::plan(&request))
}

#[derive(Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub request_read: ContentRead,
    pub plan: Plan,
    pub observation_elapsed_micros: u128,
    pub command_budget_complete: bool,
}

/// Only the bounded request file is opened. Every candidate fact remains unverified.
pub fn inspect_request_file(path: &Path) -> Result<Receipt, RequestError> {
    let mut clock = SystemClock::default();
    let (bytes, request_read) = capture_request_file(path, &mut clock)?;
    let mut plan = decode_and_plan(&bytes)?;
    let elapsed = clock.elapsed();
    let command_budget_complete = elapsed < Limits::default().max_elapsed;
    if !command_budget_complete {
        plan.errors.push("cooperative_deadline");
        plan.structurally_valid = false;
        plan.state = "refused";
    }
    Ok(Receipt {
        schema: "humanymous.evidence-layout-cli.v1",
        request_read,
        plan,
        observation_elapsed_micros: elapsed.as_micros(),
        command_budget_complete,
    })
}

#[cfg(test)]
mod tests;
