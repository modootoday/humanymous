//! Rust Rack adapter. Default compilation does not qualify native behavior.
//! Native compilation and hosted gates explicitly enable the `native` feature.
#[cfg(feature = "native")]
mod native;
