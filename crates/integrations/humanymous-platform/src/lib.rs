//! Explicit platform roles; features do not establish trusted provider authority.
#![deny(unsafe_code)]

/// Opt-in contract assertions and owned qualification-process helpers.
#[cfg(feature = "testing")]
pub mod testing;

/// Windows object queries and descriptor-at-create bridge with private observations.
#[cfg(all(windows, feature = "windows-object"))]
pub mod windows_object;
