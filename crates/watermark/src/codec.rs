//! Bounded low-level carrier transforms, distinct from trusted issuance.
//!
//! The composition root must validate source grammar/format and semantics before
//! encoding, then commit the ledger before exposing bytes. CRC alone is not a
//! verified trace. PNG and WASM dispatch entries are pending codec implementations.

mod base32;
pub mod binary;
pub mod mime;
pub mod text;
