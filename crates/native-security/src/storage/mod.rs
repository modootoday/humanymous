//! Native file-handle privacy policies shared by audit, authority and installer.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{InstallationObject, WindowsPrivacy, verify_windows_privacy};
