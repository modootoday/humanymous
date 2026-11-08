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
use windows_permissions::{LocalBox, Sid, constants::{AceType, SeObjectType, SecurityInformation}, wrappers};

fn locator() -> PathBuf {
    // Source/run, actual ordinary token, held private parent and original outer
    // watchdog must be supplied independently by Root's hosted fixture actor.
    let parent = PathBuf::from(std::env::var_os("HUMANYMOUS_B1_FIXTURE_PARENT").expect("explicit fixture parent locator required"));
    let metadata = fs::symlink_metadata(&parent).expect("existing fixture parent required");
    assert!(metadata.is_dir() && metadata.file_attributes() & 0x400 == 0);
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
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
    let sd = wrappers::GetSecurityInfo(created.file(), SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl).unwrap();
    assert!(sd.owner() == Some(sid));
    let acl = sd.dacl().unwrap();
    assert_eq!(acl.len(), 1);
    let ace = acl.get_ace(0).unwrap();
    assert!(ace.sid() == Some(sid));
    assert_eq!(ace.ace_type(), AceType::ACCESS_ALLOWED_ACE_TYPE);
    assert_eq!(ace.flags().bits(), 0);
    assert_eq!(ace.mask().bits(), 0x001f_01ff);
    let dacl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(&sd, SecurityInformation::Dacl).unwrap();
    let flags = dacl.to_str().unwrap().strip_prefix("D:").unwrap().split_once('(').unwrap().0;
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
    let error = OpenOptions::new().read(true).open(&path).err().expect("second open must fail");
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
    let mut original = OpenOptions::new().write(true).read(true).create_new(true).open(&path).unwrap();
    original.write_all(b"owned synthetic collision sentinel").unwrap();
    let before = wrappers::GetSecurityInfo(&original, SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl).unwrap();
    let before = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(&before,
        SecurityInformation::Owner | SecurityInformation::Dacl).unwrap();
    drop(original);
    assert!(matches!(create_new_file_with_descriptor(&path, &sd), Err(BridgeError::OsFailure)));
    let mut after = OpenOptions::new().read(true).open(&path).unwrap();
    let mut bytes = Vec::new();
    after.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"owned synthetic collision sentinel");
    let current = wrappers::GetSecurityInfo(&after, SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl).unwrap();
    let current = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(&current,
        SecurityInformation::Owner | SecurityInformation::Dacl).unwrap();
    assert_eq!(before, current);
    drop(after);
    fs::remove_file(&path).unwrap();
}

struct RetainedValidationFailure { created: CreatedFile }
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
    assert_eq!(OpenOptions::new().read(true).open(&path).err().unwrap().raw_os_error(), Some(32));
    drop(retained);
    assert!(path.is_file()); // Closing the owner did not claim deletion/erasure.
    fs::remove_file(&path).unwrap();
}
