//! Bounded low-level carrier transforms, distinct from trusted issuance.
//!
//! The composition root must validate source grammar/format and semantics before
//! encoding, then commit the ledger before exposing bytes. CRC alone is not a
//! verified trace. PNG metadata and WASM custom-section modules are L2; PNG LSB
//! stays fail-closed pending alpha/resource qualification.

mod base32;
pub mod binary;
pub mod mime;
pub mod png;
pub mod text;
pub mod wasm;
