//! Resource watermark issuance primitives and bounded volatile custody.
//!
//! Legacy-v1 carriers identify an issuance, not a human or a policy verdict.
//! Low-level carrier transforms do not validate source grammar or authorize HTTP
//! issuance. PNG and standards-validated WASM codecs remain unimplemented.

pub mod codec;
mod error;
pub mod frame;
pub mod ledger;
pub mod payload;

pub use error::Error;
