//! Native OS security primitives; no detection policy or implicit provisioning.
pub mod process;
pub mod secret;
pub mod storage;
#[cfg(windows)]
pub mod volume;
