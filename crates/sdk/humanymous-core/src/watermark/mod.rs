//! Resource watermark issuance primitives and bounded volatile custody.
//!
//! Legacy-v1 carriers identify an issuance, not a human or a policy verdict.
//! Low-level carrier transforms do not validate source grammar or authorize HTTP
//! issuance. `issuance::apply` composes carrier, codec selection and typed
//! results; `recovery::recover` and `Ledger::trace` recover and verify leaked
//! bytes. PNG pixel LSB follows the legacy premultiplied model; real-resource
//! pilots, runtime wiring and browser export execution remain separately gated.

pub mod codec;
mod error;
pub mod frame;
pub mod issuance;
pub mod ledger;
pub mod payload;
pub mod recovery;

#[cfg(test)]
mod tests;

pub use error::Error;
