//! Explicit SCM activation grants; never part of ordinary startup or inspection.
use super::windows::{
    InstallationObject, WindowsPrivacy, protected, verify_grants, verify_windows_privacy,
};
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use windows_permissions::{
    LocalBox, SecurityDescriptor, Sid,
    constants::{AccessRights, AceFlags, SeObjectType, SecurityInformation},
    wrappers,
};

/// The service SID never receives owner or generic administrator authority.
#[derive(Clone, Copy)]
pub enum ServiceRole {
    /// Read/execute the immutable installed regular artifact; no write/delete.
    Executable,
    /// Modify this private state directory and its children; no ACL/owner rights.
    StateDirectory,
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "explicit private SCM service role required",
    )
}
fn service_sid(text: &str) -> io::Result<LocalBox<Sid>> {
    let parts: Vec<_> = text
        .strip_prefix("S-1-5-80-")
        .ok_or_else(denied)?
        .split('-')
        .collect();
    if parts.len() != 5
        || parts
            .iter()
            .any(|p| p.parse::<u32>().is_err() || (p.len() > 1 && p.starts_with('0')))
    {
        return Err(denied());
    }
    text.parse()
}
pub(super) fn valid_grant(mask: AccessRights, flags: AceFlags, role: ServiceRole) -> bool {
    let (bits, inherit) = match role {
        ServiceRole::Executable => (0x1200a9, 0),
        ServiceRole::StateDirectory => (0x1301bf, 3),
    };
    mask.bits() == bits && flags.bits() == inherit
}
fn policy(role: ServiceRole) -> WindowsPrivacy {
    WindowsPrivacy::Installation(match role {
        ServiceRole::Executable => InstallationObject::File,
        ServiceRole::StateDirectory => InstallationObject::Parent,
    })
}
/// Resolve only the owned lab service namespace through the native account API.
pub fn resolve_service_sid(name: &str) -> io::Result<String> {
    let tail = name.strip_prefix("humanymous-lab-").ok_or_else(denied)?;
    if tail.is_empty()
        || tail.len() > 48
        || !tail
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(denied());
    }
    let (sid, _, _) = wrappers::LookupAccountName(
        Option::<&std::ffi::OsStr>::None,
        format!(r"NT SERVICE\{name}"),
    )?;
    let text = sid.to_string();
    service_sid(&text)?;
    Ok(text)
}
fn opened(path: &Path) -> io::Result<File> {
    let text = path.to_str().ok_or_else(denied)?;
    let bytes = text.as_bytes();
    if !path.is_absolute()
        || bytes.len() < 3
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
    {
        return Err(denied());
    }
    crate::volume::require_fixed(&format!("{}:\\", bytes[0] as char))?;
    OpenOptions::new()
        .access_mode(0x60000)
        .custom_flags(0x02200000)
        .open(path)
}
fn set_dacl(file: &mut File, sddl: &str) -> io::Result<()> {
    let descriptor: LocalBox<SecurityDescriptor> = sddl.parse()?;
    wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        descriptor.dacl(),
        None,
    )
}
/// Verify the exact opened object, protected descriptor, owner and single service
/// grant. Ordinary installer/audit/custody profiles still reject this descriptor.
pub fn verify_service_access(
    file: &File,
    owner_sid: &str,
    service: &str,
    role: ServiceRole,
) -> io::Result<()> {
    let service = service_sid(service)?;
    let owner: LocalBox<Sid> = owner_sid.parse()?;
    if owner == service {
        return Err(denied());
    }
    let metadata = file.metadata()?;
    if metadata.file_attributes() & 0x400 != 0
        || match role {
            ServiceRole::Executable => !metadata.is_file(),
            ServiceRole::StateDirectory => !metadata.is_dir(),
        }
    {
        return Err(denied());
    }
    let descriptor = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    protected(&descriptor)?;
    verify_grants(&descriptor, &owner, policy(role), Some((&service, role)))
}
/// Opt-in activation only: require an already private installer-owned object,
/// then set the DACL through that same non-reparse handle. No ownership repair.
pub fn grant_service_access(path: &Path, service: &str, role: ServiceRole) -> io::Result<()> {
    let identity = crate::process::current_identity()?;
    service_sid(service)?;
    let mut file = opened(path)?;
    if verify_service_access(&file, identity.sid(), service, role).is_ok() {
        return Ok(());
    }
    verify_windows_privacy(&file, identity.sid(), policy(role))?;
    let owner = identity.sid();
    let sddl = match role {
        ServiceRole::Executable => format!("D:P(A;;FA;;;{owner})(A;;0x1200a9;;;{service})"),
        ServiceRole::StateDirectory => {
            format!("D:P(A;OICI;FA;;;{owner})(A;OICI;0x1301bf;;;{service})")
        }
    };
    set_dacl(&mut file, &sddl)?;
    verify_service_access(&file, owner, service, role)
}
/// Remove only an exact previously verified service-role grant after the caller
/// has stopped the service. Unexpected grants/owners are never repaired.
pub fn revoke_service_access(path: &Path, service: &str, role: ServiceRole) -> io::Result<()> {
    let identity = crate::process::current_identity()?;
    let mut file = opened(path)?;
    verify_service_access(&file, identity.sid(), service, role)?;
    let inheritance = match role {
        ServiceRole::Executable => "",
        ServiceRole::StateDirectory => "OICI",
    };
    set_dacl(
        &mut file,
        &format!("D:P(A;{inheritance};FA;;;{})", identity.sid()),
    )?;
    verify_windows_privacy(&file, identity.sid(), policy(role))
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: &str = "S-1-5-21-1-2-3-1000";
    const SERVICE: &str = "S-1-5-80-1-2-3-4-5";
    #[test]
    fn service_sid_is_not_an_arbitrary_user_or_all_services_group() {
        assert!(service_sid(SERVICE).is_ok());
        for bad in [
            OWNER,
            "S-1-5-80-0",
            "S-1-5-80-01-2-3-4-5",
            "S-1-5-80-1-2-3-4-4294967296",
        ] {
            assert!(service_sid(bad).is_err());
        }
    }
    #[test]
    fn exact_service_role_never_weakens_ordinary_profiles() {
        let owner: LocalBox<Sid> = OWNER.parse().unwrap();
        let service = service_sid(SERVICE).unwrap();
        let descriptor: LocalBox<SecurityDescriptor> =
            format!("O:{OWNER}D:P(A;;FA;;;{OWNER})(A;;0x1200a9;;;{SERVICE})")
                .parse()
                .unwrap();
        assert!(
            verify_grants(
                &descriptor,
                &owner,
                policy(ServiceRole::Executable),
                Some((&service, ServiceRole::Executable))
            )
            .is_ok()
        );
        assert!(verify_grants(&descriptor, &owner, policy(ServiceRole::Executable), None).is_err());
        for rights in ["FA", "FR", "0x1201bf"] {
            let bad: LocalBox<SecurityDescriptor> =
                format!("O:{OWNER}D:P(A;;FA;;;{OWNER})(A;;{rights};;;{SERVICE})")
                    .parse()
                    .unwrap();
            assert!(
                verify_grants(
                    &bad,
                    &owner,
                    policy(ServiceRole::Executable),
                    Some((&service, ServiceRole::Executable))
                )
                .is_err()
            );
        }
    }
    #[test]
    fn actual_owned_file_grant_and_revoke_preserve_strict_profile_boundaries() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "humanymous-service-acl-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        drop(file);
        let owner = crate::process::current_identity().unwrap();
        let mut file = opened(&path).unwrap();
        set_dacl(&mut file, &format!("D:P(A;;FA;;;{})", owner.sid())).unwrap();
        drop(file);
        grant_service_access(&path, SERVICE, ServiceRole::Executable).unwrap();
        let file = opened(&path).unwrap();
        assert!(
            verify_service_access(&file, owner.sid(), SERVICE, ServiceRole::Executable).is_ok()
        );
        assert!(verify_windows_privacy(&file, owner.sid(), WindowsPrivacy::File).is_err());
        assert!(
            revoke_service_access(&path, "S-1-5-80-5-4-3-2-1", ServiceRole::Executable).is_err()
        );
        drop(file);
        revoke_service_access(&path, SERVICE, ServiceRole::Executable).unwrap();
        assert!(
            verify_windows_privacy(&opened(&path).unwrap(), owner.sid(), WindowsPrivacy::File)
                .is_ok()
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn actual_state_directory_role_is_separate_from_custody() {
        let path = std::env::temp_dir().join(format!(
            "humanymous-service-state-acl-{}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        let identity = crate::process::current_identity().unwrap();
        let mut file = opened(&path).unwrap();
        set_dacl(&mut file, &format!("D:P(A;OICI;FA;;;{})", identity.sid())).unwrap();
        drop(file);
        grant_service_access(&path, SERVICE, ServiceRole::StateDirectory).unwrap();
        grant_service_access(&path, SERVICE, ServiceRole::StateDirectory).unwrap();
        let file = opened(&path).unwrap();
        assert!(
            verify_service_access(&file, identity.sid(), SERVICE, ServiceRole::StateDirectory)
                .is_ok()
        );
        assert!(
            verify_windows_privacy(&file, identity.sid(), WindowsPrivacy::InheritableDirectory)
                .is_err()
        );
        assert!(
            verify_service_access(&file, identity.sid(), SERVICE, ServiceRole::Executable).is_err()
        );
        drop(file);
        revoke_service_access(&path, SERVICE, ServiceRole::StateDirectory).unwrap();
        assert!(
            verify_windows_privacy(
                &opened(&path).unwrap(),
                identity.sid(),
                WindowsPrivacy::InheritableDirectory
            )
            .is_ok()
        );
        std::fs::remove_dir(path).unwrap();
    }
}
