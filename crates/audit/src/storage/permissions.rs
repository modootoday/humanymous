//! Shared safe Windows handle-based owner/DACL verification; never provisions ACLs.
use std::{fs::File, io};
use windows_permissions::{
    LocalBox, SecurityDescriptor, Sid,
    constants::{AccessRights, AceFlags, AceType, SeObjectType, SecurityInformation},
    wrappers,
};

/// Distinct caller requirements; accepting a file never qualifies a directory.
#[derive(Clone, Copy)]
pub enum WindowsPrivacy {
    /// Owner-only regular file, including files inheriting an owner-only ACL.
    File,
    /// Owner-only directory propagating full owner access to files/directories.
    InheritableDirectory,
    /// Owner-only directory with inheritance explicitly blocked.
    ProtectedDirectory,
}

fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "private owner-only Windows DACL required",
    )
}

/// Inspect the actual opened non-reparse object without shell execution or ACL
/// repair. Trusted ancestor and same-owner replacement premises still apply.
pub fn windows_owner_only(file: &File, sid: &str, policy: WindowsPrivacy) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    let metadata = file.metadata()?;
    let expected_dir = !matches!(policy, WindowsPrivacy::File);
    if metadata.file_attributes() & 0x400 != 0
        || metadata.is_dir() != expected_dir
        || (!expected_dir && !metadata.is_file())
    {
        return Err(denied());
    }
    let expected: LocalBox<Sid> = sid.parse()?;
    let descriptor = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    verify(&descriptor, &expected, policy)
}

fn verify(sd: &SecurityDescriptor, expected: &Sid, policy: WindowsPrivacy) -> io::Result<()> {
    if sd.owner() != Some(expected) {
        return Err(denied());
    }
    let acl = sd.dacl().ok_or_else(denied)?;
    if acl.len() == 0 {
        return Err(denied());
    }
    let mut inherited_files = false;
    let mut inherited_dirs = false;
    for i in 0..acl.len() {
        let ace = acl.get_ace(i).ok_or_else(denied)?;
        if ace.ace_type() != AceType::ACCESS_ALLOWED_ACE_TYPE || ace.sid() != Some(expected) {
            return Err(denied());
        }
        if ace.mask().contains(AccessRights::FileAllAccess)
            && !ace.flags().contains(AceFlags::InheritOnly)
        {
            inherited_files |= ace.flags().contains(AceFlags::ObjectInherit);
            inherited_dirs |= ace.flags().contains(AceFlags::ContainerInherit);
        }
    }
    match policy {
        WindowsPrivacy::InheritableDirectory if !(inherited_files && inherited_dirs) => {
            Err(denied())
        }
        WindowsPrivacy::ProtectedDirectory => {
            let sddl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
                sd,
                SecurityInformation::Dacl,
            )?;
            let text = sddl.to_str().ok_or_else(denied)?;
            // Native DACL-only SDDL flags precede the first ACE; P is the SDK's
            // SE_DACL_PROTECTED token. Never inspect SID/ACE text for this bit.
            let flags = text
                .strip_prefix("D:")
                .and_then(|s| s.split_once('(').map(|(flags, _)| flags))
                .ok_or_else(denied)?;
            if flags.contains('P') {
                Ok(())
            } else {
                Err(denied())
            }
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: &str = "S-1-5-21-1-2-3-1000";
    fn descriptor(dacl: &str) -> LocalBox<SecurityDescriptor> {
        format!("O:{OWNER}{dacl}").parse().unwrap()
    }
    fn check(dacl: &str, policy: WindowsPrivacy) -> io::Result<()> {
        verify(
            &descriptor(dacl),
            &OWNER.parse::<LocalBox<Sid>>().unwrap(),
            policy,
        )
    }
    #[test]
    fn protection_and_child_inheritance_are_separate_requirements() {
        let all = format!("D:P(A;OICI;FA;;;{OWNER})");
        assert!(check(&all, WindowsPrivacy::ProtectedDirectory).is_ok());
        assert!(check(&all, WindowsPrivacy::InheritableDirectory).is_ok());
        assert!(
            check(
                &format!("D:(A;OICI;FA;;;{OWNER})"),
                WindowsPrivacy::ProtectedDirectory
            )
            .is_err()
        );
        assert!(
            check(
                &format!("D:P(A;;FA;;;{OWNER})"),
                WindowsPrivacy::InheritableDirectory
            )
            .is_err()
        );
        assert!(check(&format!("D:P(A;;FA;;;{OWNER})"), WindowsPrivacy::File).is_ok());
    }
    #[test]
    fn foreign_null_empty_and_denied_descriptors_are_refused() {
        for dacl in [
            "".to_owned(),
            "D:P".into(),
            "D:P(A;;FA;;;WD)".into(),
            format!("D:P(D;;FA;;;{OWNER})"),
        ] {
            assert!(check(&dacl, WindowsPrivacy::ProtectedDirectory).is_err());
        }
        let sd = descriptor(&format!("D:P(A;;FA;;;{OWNER})"));
        let other: LocalBox<Sid> = "S-1-5-21-1-2-3-1001".parse().unwrap();
        assert!(verify(&sd, &other, WindowsPrivacy::ProtectedDirectory).is_err());
    }
}
