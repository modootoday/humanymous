//! Opt-in Apache native module. Build and execute only in the hosted Linux pilot.
#![deny(unsafe_code)]

#[cfg(all(feature = "httpd", not(target_os = "linux")))]
compile_error!("Apache native ABI is qualified only through the Linux hosted pilot");

#[cfg(all(feature = "httpd", target_os = "linux"))]
#[allow(unsafe_code)] // Generated declarations only; pinned upstream ABI.
#[allow(clippy::all)] // Generated upstream layouts; handwritten bridge stays checked.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/apache_bindings.rs"));
}

#[cfg(all(feature = "httpd", target_os = "linux"))]
#[allow(unsafe_code)] // Reviewed host operations documented in ABI-SAFETY.md.
mod host;
