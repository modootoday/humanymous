//! Body-bound nginx ingress. Decisions, IPC authentication and audit stay shared.
pub mod package;
pub mod report;
pub mod request;

#[cfg(all(feature = "native", target_os = "linux"))]
#[allow(unsafe_code)] // Reviewed nginx ABI only; request/IPC logic stays safe.
mod native;

#[cfg(all(feature = "native", not(target_os = "linux")))]
compile_error!("the native nginx module is qualified only on Linux, not this target");
