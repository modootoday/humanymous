//! Bounded observations of a caller-frozen selection, never a security snapshot.
mod files;
mod model;
mod reader;

pub use files::observe;
pub use files::{FileObservation, PreparationReceipt, Stability};
pub use model::{FrozenSelection, Input, Limits, Root, SelectionError};
pub use reader::{Clock, ContentRead, ReadBudget, ReadStatus, SystemClock};

#[cfg(test)]
mod tests;
