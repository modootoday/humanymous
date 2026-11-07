//! Safe ownership foundation for a future versioned C ABI.
//!
//! This library exports Rust APIs only. No FFI symbols, foreign pointers,
//! security verdicts, callbacks, workers or OS operations are implemented.
#![forbid(unsafe_code)]

mod boundary;
mod command;
mod registry;
mod wire;

pub use boundary::Boundary;
pub use command::{Command, Response};
pub use registry::{MAX_BUFFER_BYTES, MAX_BUFFERS, MAX_CONTEXTS, MAX_TOTAL_BYTES, Registry};
pub use wire::{Reply, Status, Version};
