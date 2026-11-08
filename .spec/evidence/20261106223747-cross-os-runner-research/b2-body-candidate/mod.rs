//! UNAPPROVED B2 body candidate. Low-level owned containment, never NativeProof.
#![deny(unsafe_code)]
#![cfg(windows)]

mod facade;
mod launch;
mod native;
mod state;

pub use facade::{OwnedTree, RetainedTree, TerminalTree, TreeObservation};
pub use launch::QualificationHelper;
pub use state::{BudgetLimits, FirstFailure, Issue, Phase};

#[cfg(test)]
mod native_tests;
