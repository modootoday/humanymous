//! Native file-handle privacy policies shared by audit, authority and installer.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{
    InstallationObject, WindowsPrivacy, verify_windows_pipe_privacy, verify_windows_privacy,
};
#[cfg(windows)]
mod service;
#[cfg(windows)]
pub use service::{
    ServiceRole, grant_service_access, resolve_service_sid, revoke_service_access,
    verify_service_access,
};
