//! Trusted laboratory browser controls; target engine attachment is not ready.
#![forbid(unsafe_code)]

pub mod lifecycle;
pub mod policy;
#[cfg(feature = "native-ui")]
pub mod ui;
pub mod viewport;

#[cfg(feature = "native-ui")]
pub mod icons;
#[cfg(feature = "native-ui")]
pub mod window_chrome;

pub mod frame_geometry;

pub mod internal_urls;
#[cfg(feature = "native-ui")]
pub mod newtab;

#[cfg(feature = "native-ui")]
pub mod ui_navigation;

/// Pure navigation tickets and strict diagnostic-probe accounting.
pub mod navigation;

/// Bounded actual-engine history snapshot validation.
pub mod history;
/// Pure native address composition and confirmation policy.
pub mod input;
/// Pure tab identities, cleanup accounting and browser accelerators.
pub mod tabs;
#[cfg(feature = "native-ui")]
pub mod tabs_ui;
