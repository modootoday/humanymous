//! Native integration planning; generated metadata does not qualify any SDK/runtime.
#![forbid(unsafe_code)]

pub mod arguments;
pub mod manifest;
pub mod storage;

#[cfg(all(test, windows))]
mod test_support;
#[cfg(test)]
mod tests;
