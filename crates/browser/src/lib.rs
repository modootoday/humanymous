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
