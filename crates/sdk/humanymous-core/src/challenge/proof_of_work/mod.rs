//! Session-bound hashcash compatible with the Go proof-of-work protocol.
mod challenge;
mod solve_time;
mod target;

pub use challenge::{Challenge, WINDOW, issue, parse_solution, verify};
pub use solve_time::{CEILING_HASHES_PER_SEC, Outcome, SolveTime, plausible_browser_solve};
pub use target::{difficulty_for, solve};
