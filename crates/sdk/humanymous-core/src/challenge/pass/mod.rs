//! Deterministic public alignment puzzles, without issuance or verdict authority.
//!
//! Answers are intentionally derivable for accessibility. Alignment alone proves
//! neither humanity nor replay protection; callers separately authenticate the
//! instance and enforce issuance, single use, event proof and attestation policy.
mod generation;
mod verification;

use serde::{Deserialize, Serialize};

pub use generation::generate;
pub use verification::{fresh, solution_offsets, verify};

/// Number of independently aligned public rows.
pub const ROWS: usize = 3;
/// Inclusive freshness age in caller-supplied, historically 30-second buckets.
pub const MAX_BUCKET_AGE: u64 = 20;

/// A cyclic row with a public marked position; characters need not be unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// Public character cells in their original cyclic order.
    pub chars: Vec<String>,
    /// Public marked cell, including a cell already at the center.
    #[serde(rename = "keyIndex")]
    pub key_index: usize,
}

/// Public puzzle data. Deserializing it grants no verification authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// Cells per row: 9, 11, 13 or 15.
    pub n: usize,
    /// Zero-based target column.
    pub center: usize,
    /// Three public rows.
    pub rows: Vec<Row>,
}

#[cfg(test)]
mod tests;
