//! Explicit new service state initialization, never inspection-time ACL repair.
use super::{ServiceRole, WindowsPrivacy, verify_service_access, verify_windows_privacy};
use std::{
    fs::{self, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use windows_permissions::{
    LocalBox, Sid,
    constants::{SeObjectType, SecurityInformation},
    wrappers,
};

fn child_component(child: &str) -> io::Result<()> {
    if child.is_empty()
        || child.len() > 64
        || !child
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid private service child component",
        ));
    }
    Ok(())
}

/// Create one new private child beneath a previously delegated state parent.
/// The signed caller must supply the original installer SID and trusted ancestors.
/// Existing children are refused. This is not protection against that parent owner
/// replacing paths later. No secret is written by this initializer.
pub fn create_service_private_directory(
    parent: &Path,
    child: &str,
    installer_owner_sid: &str,
    service_name: &str,
) -> io::Result<PathBuf> {
    child_component(child)?;
    let service = actual_service_sid(service_name)?;
    let parent_handle = inspect_parent(parent)?;
    verify_service_access(
        &parent_handle,
        installer_owner_sid,
        &service,
        ServiceRole::StateDirectory,
    )?;
    create_child(parent, child, &service)
}

/// Create a new protected child beneath an already private service-owned parent.
/// This has a different trust boundary from an installer-owned delegated parent.
pub fn create_service_private_subdirectory(
    parent: &Path,
    child: &str,
    service_name: &str,
) -> io::Result<PathBuf> {
    child_component(child)?;
    let service = actual_service_sid(service_name)?;
    create_under_private_parent(parent, child, &service)
}

/// Create a new private child for an actual ordinary user beneath an already
/// protected, inheritable current-owner parent. No ambient TEMP or ACL repair.
pub fn create_user_private_directory(parent: &Path, child: &str) -> io::Result<PathBuf> {
    child_component(child)?;
    let identity = crate::native_security::process::current_identity()?;
    if !identity.installer_allowed() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ordinary private directory actor required",
        ));
    }
    create_under_private_parent(parent, child, identity.sid())
}

fn create_under_private_parent(parent: &Path, child: &str, service: &str) -> io::Result<PathBuf> {
    let parent_handle = inspect_parent(parent)?;
    verify_windows_privacy(&parent_handle, service, WindowsPrivacy::ProtectedDirectory)?;
    verify_windows_privacy(
        &parent_handle,
        service,
        WindowsPrivacy::InheritableDirectory,
    )?;
    create_child(parent, child, service)
}

fn actual_service_sid(service_name: &str) -> io::Result<String> {
    let identity = crate::native_security::process::current_identity()?;
    let service = super::resolve_service_sid(service_name)?;
    if identity.sid() != service {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "actual service principal required",
        ));
    }
    Ok(service)
}

fn inspect_parent(parent: &Path) -> io::Result<std::fs::File> {
    super::service::require_local_path(parent)?;
    // Service Modify deliberately excludes WRITE_DAC on the installer parent.
    OpenOptions::new()
        .access_mode(0x20000)
        .share_mode(1 | 2)
        .custom_flags(0x02200000)
        .open(parent)
}

fn create_child(parent: &Path, child: &str, owner_sid: &str) -> io::Result<PathBuf> {
    let path = parent.join(child);
    fs::create_dir(&path)?;
    let mut handle = super::service::opened(&path)?;
    let metadata = handle.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "new non-reparse service directory required",
        ));
    }
    let descriptor = wrappers::GetSecurityInfo(
        &handle,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    let owner: LocalBox<Sid> = owner_sid.parse()?;
    if descriptor.owner() != Some(&owner) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "new service directory owner mismatch",
        ));
    }
    super::service::set_dacl(&mut handle, &format!("D:P(A;OICI;FA;;;{owner_sid})"))?;
    verify_windows_privacy(&handle, owner_sid, WindowsPrivacy::ProtectedDirectory)?;
    verify_windows_privacy(&handle, owner_sid, WindowsPrivacy::InheritableDirectory)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_one_bounded_canonical_child_component_is_allowed() {
        assert!(child_component("audit-state-1").is_ok());
        for child in [
            "", ".", "..", "a/b", "a\\b", "a:b", "a ", "A", "a\0b", "a.txt",
        ] {
            assert!(child_component(child).is_err());
        }
        assert!(child_component(&"a".repeat(65)).is_err());
    }
}
