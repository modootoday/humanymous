//! Native filesystem qualification; synthetic secrets only, no product e2e claim.
#![cfg(any(unix, windows))]
use humanymous_audit::keys::{
    KeyMaterial,
    custody::{Custody, Owner},
    vault::{Mode, Vault},
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-key-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        #[cfg(windows)]
        {
            use windows_permissions::{
                LocalBox, SecurityDescriptor,
                constants::{SeObjectType, SecurityInformation},
                wrappers,
            };
            let Owner::WindowsSid(sid) = owner(&path);
            let descriptor: LocalBox<SecurityDescriptor> =
                format!("D:P(A;OICI;FA;;;{sid})").parse().unwrap();
            wrappers::SetNamedSecurityInfo(
                path.as_os_str(),
                SeObjectType::SE_FILE_OBJECT,
                SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
                None,
                None,
                descriptor.dacl(),
                None,
            )
            .unwrap();
        }
        Self(fs::canonicalize(path).unwrap())
    }
    fn acquire(&self) -> Custody {
        Custody::acquire(&self.0, owner(&self.0)).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn owner(_path: &std::path::Path) -> Owner {
    #[cfg(unix)]
    {
        Owner::CurrentUser
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            constants::{SeObjectType, SecurityInformation},
            wrappers,
        };
        let sd = wrappers::GetNamedSecurityInfo(
            _path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner,
        )
        .unwrap();
        Owner::WindowsSid(sd.owner().unwrap().to_string())
    }
}
fn material() -> Result<KeyMaterial, humanymous_audit::keys::KeyError> {
    KeyMaterial::new([7; 32], vec![8; 32], None, None)
}
fn no_create() -> Result<KeyMaterial, humanymous_audit::keys::KeyError> {
    panic!("must not generate replacement identity")
}
#[test]
fn restart_rotation_and_encrypted_snapshot() {
    let temp = Temp::new();
    let mut store = temp.acquire();
    let (key, durability) = store.load_or_create(b"old", material).unwrap();
    assert!(durability.is_some());
    assert_eq!(key.signing_seed(), &[7; 32]);
    let vault = Vault::new(Mode::Hmac, 20).unwrap();
    let pseudonym = vault
        .pseudonymize("synthetic-subject", "synthetic-value")
        .unwrap();
    vault.link("synthetic-handle", "synthetic-subject").unwrap();
    store.persist_vault(&vault, b"old").unwrap();
    store.rotate(b"old", b"new").unwrap();
    drop(store);
    let mut store = temp.acquire();
    assert!(store.load_or_create(b"old", no_create).is_err());
    let (key, durability) = store.load_or_create(b"new", no_create).unwrap();
    assert!(durability.is_none());
    let restored = Vault::load(key.vault().unwrap(), Mode::Hmac, 20).unwrap();
    assert_eq!(
        restored
            .pseudonymize("synthetic-subject", "synthetic-value")
            .unwrap(),
        pseudonym
    );
    assert_eq!(
        restored.resolve("synthetic-handle").unwrap().as_deref(),
        Some("synthetic-subject")
    );
    assert!(
        store
            .persist_vault(&Vault::new(Mode::Hmac, 20).unwrap(), b"new")
            .is_err()
    );
    let bytes = fs::read(temp.0.join("keys.sealed")).unwrap();
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains("synthetic-subject")
    );
}
#[test]
fn partial_corruption_and_pending_never_create() {
    let temp = Temp::new();
    let mut store = temp.acquire();
    store.load_or_create(b"pass", material).unwrap();
    let committed = fs::read(temp.0.join("keys.sealed")).unwrap();
    fs::write(temp.0.join("keys.sealed"), b"partial").unwrap();
    assert!(store.load_or_create(b"pass", no_create).is_err());
    assert_eq!(fs::read(temp.0.join("keys.sealed")).unwrap(), b"partial");
    fs::write(temp.0.join("keys.sealed"), &committed).unwrap();
    // Test fixture staging uses the same inherited private ACL/mode policy.
    fs::copy(temp.0.join("keys.sealed"), temp.0.join("keys.pending")).unwrap();
    assert!(store.load_or_create(b"pass", no_create).is_err());
    assert!(store.recover_pending(b"pass", b"wrong").is_err());
    store.recover_pending(b"pass", b"pass").unwrap();
    assert!(!temp.0.join("keys.pending").exists());
    assert!(store.load_or_create(b"pass", no_create).is_ok());
    fs::copy(temp.0.join("keys.sealed"), temp.0.join("keys.pending")).unwrap();
    fs::write(temp.0.join("keys.pending"), b"{").unwrap();
    drop(store);
    assert!(temp.acquire().load_or_create(b"pass", no_create).is_err());
}
#[test]
#[ignore = "subprocess helper exercised by competing_process_cannot_load_or_create"]
fn child_lock_probe() {
    if let Some(path) = std::env::var_os("HUMANYMOUS_KEY_LOCK_PROBE") {
        let path = PathBuf::from(path);
        assert!(
            matches!(Custody::acquire(&path, owner(&path)), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(b"\nhumanymous-key-lock-refused\n")
            .unwrap();
        stdout.flush().unwrap();
    }
}
#[test]
fn competing_process_cannot_load_or_create() {
    let temp = Temp::new();
    let guard = temp.acquire();
    let captured = humanymous_platform::testing::process::capture_with_stderr(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_lock_probe", "--ignored"])
            .env("HUMANYMOUS_KEY_LOCK_PROBE", &temp.0),
        b"",
        std::time::Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(captured.status.success());
    assert!(captured.stderr.is_empty());
    assert!(
        String::from_utf8(captured.stdout)
            .unwrap()
            .lines()
            .any(|line| line == "humanymous-key-lock-refused")
    );
    assert!(!temp.0.join("keys.sealed").exists());
    drop(guard);
    let (key, created) = temp.acquire().load_or_create(b"pass", material).unwrap();
    assert!(created.is_some());
    assert_eq!(key.signing_seed(), &[7; 32]);
    assert_eq!(key.hmac_key(), &[8; 32]);
}
#[test]
fn wide_directory_permissions_are_refused() {
    let temp = Temp::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp.0, fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            wrappers,
        };
        let Owner::WindowsSid(sid) = owner(&temp.0);
        let sd: LocalBox<SecurityDescriptor> = format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FR;;;WD)")
            .parse()
            .unwrap();
        wrappers::SetNamedSecurityInfo(
            temp.0.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
            None,
            None,
            sd.dacl(),
            None,
        )
        .unwrap();
    }
    assert!(Custody::acquire(&temp.0, owner(&temp.0)).is_err());
}

#[test]
fn persisted_tombstones_survive_rotation_and_cannot_be_lost() {
    let temp = Temp::new();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/keys.json")).unwrap();
    let snapshot = serde_json::to_vec(&fixture["shredded_snapshot"]).unwrap();
    let vault = Vault::load(&snapshot, Mode::Hmac, 100).unwrap();
    let mut store = temp.acquire();
    store
        .load_or_create(b"old", || {
            KeyMaterial::new([7; 32], vec![8; 32], Some(snapshot), None)
        })
        .unwrap();
    store.persist_vault(&vault, b"old").unwrap();
    assert!(
        store
            .persist_vault(&Vault::new(Mode::Hmac, 100).unwrap(), b"old")
            .is_err()
    );
    store.rotate(b"old", b"new").unwrap();
    drop(store);
    let (key, _) = temp.acquire().load_or_create(b"new", no_create).unwrap();
    let restored = Vault::load(key.vault().unwrap(), Mode::Hmac, 100).unwrap();
    assert_eq!(
        restored.pseudonymize("subject-A", "any").unwrap(),
        fixture["tombstone"].as_str().unwrap()
    );
}

#[test]
fn recovery_cannot_change_identity_or_discard_committed_tombstones() {
    let temp = Temp::new();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/keys.json")).unwrap();
    let snapshot = serde_json::to_vec(&fixture["shredded_snapshot"]).unwrap();
    let mut store = temp.acquire();
    store
        .load_or_create(b"pass", || {
            KeyMaterial::new([7; 32], vec![8; 32], Some(snapshot), None)
        })
        .unwrap();
    let pending = temp.0.join("keys.pending");
    fs::copy(temp.0.join("keys.sealed"), &pending).unwrap();
    let changed = KeyMaterial::new([9; 32], vec![8; 32], None, None).unwrap();
    fs::write(
        &pending,
        humanymous_audit::keys::seal(&changed, b"pass").unwrap(),
    )
    .unwrap();
    assert!(store.recover_pending(b"pass", b"pass").is_err());
    // Same writer identity still cannot lose a committed tombstone.
    fs::write(
        &pending,
        humanymous_audit::keys::seal(&material().unwrap(), b"pass").unwrap(),
    )
    .unwrap();
    assert!(store.recover_pending(b"pass", b"pass").is_err());
    assert!(pending.exists());
    assert!(store.load_or_create(b"pass", no_create).is_err());
}

#[test]
#[ignore = "subprocess helper exercised by process_exit_releases_lock_and_rotation_pending_requires_recovery"]
fn child_interrupted_writer() {
    if let Some(path) = std::env::var_os("HUMANYMOUS_KEY_INTERRUPTED_WRITER") {
        let path = PathBuf::from(path);
        let mut guard = Custody::acquire(&path, owner(&path)).unwrap();
        let (material, _) = guard.load_or_create(b"old", material).unwrap();
        let sealed = humanymous_audit::keys::seal(&material, b"new").unwrap();
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path.join("keys.pending")).unwrap();
        use std::io::Write;
        file.write_all(&sealed).unwrap();
        file.sync_all().unwrap();
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(b"\nhumanymous-key-pending-synced\n")
            .unwrap();
        stdout.flush().unwrap();
        // Deliberately bypass destructors, releasing OS locks by process exit.
        std::process::exit(0);
    }
}

#[test]
fn process_exit_releases_lock_and_rotation_pending_requires_recovery() {
    let temp = Temp::new();
    let captured = humanymous_platform::testing::process::capture_with_stderr(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_interrupted_writer", "--ignored"])
            .env("HUMANYMOUS_KEY_INTERRUPTED_WRITER", &temp.0),
        b"",
        std::time::Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(captured.status.success());
    assert!(captured.stderr.is_empty());
    assert!(
        String::from_utf8(captured.stdout)
            .unwrap()
            .lines()
            .any(|line| line == "humanymous-key-pending-synced")
    );
    assert!(temp.0.join("keys.pending").exists());
    let mut guard = temp.acquire();
    assert!(guard.load_or_create(b"old", no_create).is_err());
    assert!(guard.recover_pending(b"wrong", b"new").is_err());
    guard.recover_pending(b"old", b"new").unwrap();
    assert!(guard.load_or_create(b"old", no_create).is_err());
    assert!(!temp.0.join("keys.pending").exists());
    let (key, created) = guard.load_or_create(b"new", no_create).unwrap();
    assert!(created.is_none());
    assert_eq!(key.signing_seed(), &[7; 32]);
    assert_eq!(key.hmac_key(), &[8; 32]);
}

#[cfg(windows)]
#[test]
fn windows_rename_sharing_failure_poisons_until_authenticated_recovery() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = Temp::new();
    let mut guard = temp.acquire();
    guard.load_or_create(b"old", material).unwrap();
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(temp.0.join("keys.sealed"))
        .unwrap();
    assert!(guard.rotate(b"old", b"new").is_err());
    assert!(temp.0.join("keys.pending").exists());
    drop(blocker);
    assert!(guard.load_or_create(b"old", no_create).is_err());
    assert!(guard.recover_pending(b"old", b"new").is_err());
    drop(guard);
    let mut guard = temp.acquire();
    guard.recover_pending(b"old", b"new").unwrap();
    assert!(guard.load_or_create(b"new", no_create).is_ok());
}

#[cfg(windows)]
#[test]
fn windows_foreign_owner_and_wide_file_dacl_are_refused() {
    use windows_permissions::{
        LocalBox, SecurityDescriptor,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let temp = Temp::new();
    assert!(Custody::acquire(&temp.0, Owner::WindowsSid("S-1-1-0".into())).is_err());
    temp.acquire().load_or_create(b"pass", material).unwrap();
    let Owner::WindowsSid(sid) = owner(&temp.0);
    let sd: LocalBox<SecurityDescriptor> =
        format!("D:P(A;;FA;;;{sid})(A;;FR;;;WD)").parse().unwrap();
    wrappers::SetNamedSecurityInfo(
        temp.0.join("keys.sealed").as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        sd.dacl(),
        None,
    )
    .unwrap();
    assert!(temp.acquire().load_or_create(b"pass", no_create).is_err());
}

#[cfg(windows)]
#[test]
fn windows_reparse_directory_and_ancestor_are_refused() {
    let temp = Temp::new();
    let alias = temp.0.join("alias");
    junction::create(&temp.0, &alias).unwrap();
    assert!(Custody::acquire(&alias, owner(&temp.0)).is_err());
    let child = temp.0.join("child");
    fs::create_dir(&child).unwrap();
    assert!(Custody::acquire(&alias.join("child"), owner(&temp.0)).is_err());
    junction::delete(&alias).unwrap();
}

#[cfg(unix)]
#[test]
fn unix_symlinks_hardlinks_and_wide_files_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = Temp::new();
    temp.acquire().load_or_create(b"pass", material).unwrap();
    let alias = temp.0.with_extension("alias");
    symlink(&temp.0, &alias).unwrap();
    assert!(Custody::acquire(&alias, Owner::CurrentUser).is_err());
    fs::remove_file(alias).unwrap();
    let key = temp.0.join("keys.sealed");
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(temp.acquire().load_or_create(b"pass", no_create).is_err());
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(&key, temp.0.join("extra-link")).unwrap();
    assert!(temp.acquire().load_or_create(b"pass", no_create).is_err());
    fs::remove_file(temp.0.join("extra-link")).unwrap();
    fs::rename(&key, temp.0.join("original")).unwrap();
    symlink(temp.0.join("original"), &key).unwrap();
    assert!(temp.acquire().load_or_create(b"pass", no_create).is_err());
}
