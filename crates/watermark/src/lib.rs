//! Resource watermark issuance primitives and bounded volatile custody.
//!
//! Legacy-v1 carriers identify an issuance, not a human or a policy verdict.
//! Format codecs and HTTP integration are not implemented by this initial slice.

mod error;
pub mod ledger;
pub mod payload;

pub use error::Error;
