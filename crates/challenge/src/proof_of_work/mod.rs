//! Session-bound hashcash compatible with the Go proof-of-work protocol.
mod challenge;
mod target;

pub use challenge::{Challenge, WINDOW, issue, parse_solution, verify};
pub use target::{difficulty_for, solve};
