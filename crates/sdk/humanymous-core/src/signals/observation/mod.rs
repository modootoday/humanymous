//! Atomic observations, their wire vocabulary and their canonical definitions.
mod registry;
mod signal;
mod verdict;
mod vocabulary;

pub use registry::{Definition, all, group_of, lookup};
pub use signal::Signal;
pub use verdict::{compute_score, severity};
pub use vocabulary::{Layer, Source, Verdict};
