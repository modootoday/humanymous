//! Actual native fixture SOURCE ONLY, gated on explicit allocated Windows venue.
//! The configured parent is a locator, not an admitted parent/provider capability.
#![forbid(unsafe_code)]

use super::*;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::windows::fs::MetadataExt,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_permissions::{
    LocalBox, Sid,
    constants::{AceType, SeObjectType, SecurityInformation},
    wrappers,
};

fn locator() -> PathBuf {
    // Source/run, actual ordinary token, held private parent and original outer
    // watchdog must be supplied independently by Root's hosted fixture actor.
    let parent = PathBuf::from(
        std::env::var_os("HUMANYMOUS_B1_FIXTURE_PARENT")
            .expect("explicit fixture parent locator required"),
    );
    let metadata = fs::symlink_metadata(&parent).expect("existing fixture parent required");
    assert!(metadata.is_dir() && metadata.file_attributes() & 0x400 == 0);
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Uniqueness locator only, not CSPRNG, native identity or execution authority.
    let path = parent.join(format!("b1-{}-{sequence}-{time}.bin", std::process::id()));
    wide_locator(&path).expect("supported explicit fixture locator required");
    path
}

fn descriptor() -> (LocalBox<Sid>, LocalBox<SecurityDescriptor>) {
    // Safe upstream actual process SID query, not unattended original-I proof.
    let sid = windows_permissions::utilities::current_process_sid().unwrap();
    let sd = format!("O:{sid}D:P(A;;FA;;;{sid})").parse().unwrap();
    (sid, sd)
}

fn assert_actual_descriptor(created: &CreatedFile, sid: &Sid) {
    let sd = wrappers::GetSecurityInfo(
        created.file(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    assert!(sd.owner() == Some(sid));
    let acl = sd.dacl().unwrap();
    assert_eq!(acl.len(), 1);
    let ace = acl.get_ace(0).unwrap();
    assert!(ace.sid() == Some(sid));
    assert_eq!(ace.ace_type(), AceType::ACCESS_ALLOWED_ACE_TYPE);
    assert_eq!(ace.flags().bits(), 0);
    assert_eq!(ace.mask().bits(), 0x001f_01ff);
    let dacl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &sd,
        SecurityInformation::Dacl,
    )
    .unwrap();
    let flags = dacl
        .to_str()
        .unwrap()
        .strip_prefix("D:")
        .unwrap()
        .split_once('(')
        .unwrap()
        .0;
    assert!(flags.contains('P'));
}

#[test]
#[ignore = "required actual B1 Windows fixture: Root must allocate ordinary actor, private parent and original outer budget"]
fn actual_descriptor_is_present_on_first_owned_file_before_any_repair() {
    let path = locator();
    let (sid, sd) = descriptor();
    let created = create_new_file_with_descriptor(&path, &sd).unwrap();
    assert_eq!(created.path(), path);
    assert_eq!(created.file().metadata().unwrap().len(), 0);
    assert_actual_descriptor(&created, &sid);
    // Same-opened provider/ID observation is separate from its namespace policy.
    let observed = super::super::inspect_opened(created.file()).unwrap();
    assert!(!observed.is_directory());
    drop(created);
    fs::remove_file(&path).unwrap(); // Explicit fixture-only cleanup, never Drop.
}

#[test]
#[ignore = "required actual B1 Windows fixture: share denial and DACL readback must be separate facts"]
fn actual_share0_denial_does_not_substitute_for_descriptor_readback() {
    let path = locator();
    let (sid, sd) = descriptor();
    let created = create_new_file_with_descriptor(&path, &sd).unwrap();
    assert_actual_descriptor(&created, &sid);
    let error = OpenOptions::new()
        .read(true)
        .open(&path)
        .expect_err("second open must fail");
    assert_eq!(error.raw_os_error(), Some(32)); // ERROR_SHARING_VIOLATION, not ACL proof.
    drop(created);
    let reopened = OpenOptions::new().read(true).open(&path).unwrap();
    assert_eq!(reopened.metadata().unwrap().len(), 0);
    drop(reopened);
    fs::remove_file(&path).unwrap();
}

#[test]
#[ignore = "required actual B1 Windows fixture: collision must preserve the already owned synthetic file"]
fn actual_create_new_collision_preserves_existing_bytes_and_descriptor() {
    let path = locator();
    let (_, sd) = descriptor();
    let mut original = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    original
        .write_all(b"owned synthetic collision sentinel")
        .unwrap();
    let before = wrappers::GetSecurityInfo(
        &original,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    let before = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &before,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    drop(original);
    assert!(matches!(
        create_new_file_with_descriptor(&path, &sd),
        Err(BridgeError::OsFailure)
    ));
    let mut after = OpenOptions::new().read(true).open(&path).unwrap();
    let mut bytes = Vec::new();
    after.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"owned synthetic collision sentinel");
    let current = wrappers::GetSecurityInfo(
        &after,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    let current = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &current,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    assert_eq!(before, current);
    drop(after);
    fs::remove_file(&path).unwrap();
}

struct RetainedValidationFailure {
    created: CreatedFile,
}
fn reject_after_creation(created: CreatedFile) -> Result<(), RetainedValidationFailure> {
    Err(RetainedValidationFailure { created })
}

#[test]
#[ignore = "required actual B1 Windows fixture: post-validation failure retains File and creation obligation"]
fn actual_rejected_validation_keeps_owner_and_drop_never_deletes() {
    let path = locator();
    let (_, sd) = descriptor();
    let created = create_new_file_with_descriptor(&path, &sd).unwrap();
    let retained = reject_after_creation(created).err().unwrap();
    assert_eq!(retained.created.path(), path);
    assert_eq!(retained.created.file().metadata().unwrap().len(), 0);
    assert_eq!(
        OpenOptions::new()
            .read(true)
            .open(&path)
            .err()
            .unwrap()
            .raw_os_error(),
        Some(32)
    );
    drop(retained);
    assert!(path.is_file()); // Closing the owner did not claim deletion/erasure.
    fs::remove_file(&path).unwrap();
}

fn owner_and_dacl(file: &File) -> (Option<String>, u32, bool) {
    let sd = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .unwrap();
    let owner = sd.owner().map(|sid| sid.to_string());
    let entries = sd.dacl().map_or(0, |acl| acl.len());
    let dacl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &sd,
        SecurityInformation::Dacl,
    )
    .unwrap();
    let flags = dacl
        .to_str()
        .unwrap()
        .strip_prefix("D:")
        .unwrap()
        .split_once('(')
        .unwrap()
        .0;
    (owner, entries, flags.contains('P'))
}

/// D5-N4 option (a) (issue #75 comment 6999924070): an elevated, admin-enabled
/// creator sets owner BUILTIN\Administrators in the creating call itself, for
/// the carrier directory and a carrier file, and a foreign owner is refused
/// with nothing created. No owner of an existing object is ever changed.
#[test]
#[ignore = "requires an elevated admin token; run alone by delivery component dotnet-host on windows-2025"]
fn actual_elevated_administrators_owner_at_creation_and_foreign_owner_refused() {
    use std::os::windows::fs::OpenOptionsExt;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hm-b1-owner-{}-{time}", std::process::id()));
    let sd: LocalBox<SecurityDescriptor> =
        "O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)".parse().unwrap();
    let created = super::super::create_directory_with_descriptor(&root, &sd).unwrap();
    assert_eq!(created.path(), root);
    let directory = OpenOptions::new()
        .access_mode(0x0002_0000)
        .custom_flags(0x0220_0000)
        .open(&root)
        .unwrap();
    assert_eq!(
        owner_and_dacl(&directory),
        (Some("S-1-5-32-544".into()), 2, true)
    );
    drop(directory);

    let path = root.join("context.json");
    let sd: LocalBox<SecurityDescriptor> = "O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)".parse().unwrap();
    let file = create_new_file_with_descriptor(&path, &sd).unwrap();
    assert_eq!(
        owner_and_dacl(file.file()),
        (Some("S-1-5-32-544".into()), 2, true)
    );
    let mut writer = file.file();
    writer.write_all(b"carrier").unwrap();
    // CREATE_NEW refuses the existing object; it is never reopened or repaired.
    assert!(matches!(
        create_new_file_with_descriptor(&path, &sd),
        Err(BridgeError::OsFailure)
    ));
    drop(file);
    assert_eq!(fs::metadata(&path).unwrap().len(), 7);

    // Everyone is a token group without the owner attribute: refused, nothing created.
    let foreign: LocalBox<SecurityDescriptor> = "O:WDD:P(A;;FA;;;BA)".parse().unwrap();
    let refused_file = root.join("foreign.json");
    assert!(matches!(
        create_new_file_with_descriptor(&refused_file, &foreign),
        Err(BridgeError::OsFailure)
    ));
    assert!(!refused_file.exists());
    let refused_dir = root.join("foreign");
    assert!(super::super::create_directory_with_descriptor(&refused_dir, &foreign).is_err());
    assert!(!refused_dir.exists());

    fs::remove_file(&path).unwrap();
    fs::remove_dir(&root).unwrap();
}
