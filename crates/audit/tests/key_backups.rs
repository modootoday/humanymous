//! Native local sealed-backup qualification; synthetic keys, not product e2e.
#![cfg(any(unix, windows))]
use ed25519_dalek::{Signer, SigningKey};
use humanymous_audit::{
    control::{
        Action, Administrators, Approval, ControlAuthority, LedgerAnchor, LedgerBootstrap,
        Proposal, Scope, target_commitment,
    },
    keys::{
        KeyMaterial,
        backups::{BackupAnchor, BackupBootstrap, BackupInventory, BackupStatus},
        custody::Owner,
        seal,
    },
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-backup-{}-{}",
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
    fn sub(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn owner(_path: &Path) -> Owner {
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
        let descriptor = wrappers::GetNamedSecurityInfo(
            _path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner,
        )
        .unwrap();
        Owner::WindowsSid(descriptor.owner().unwrap().to_string())
    }
}
fn scope() -> Scope {
    Scope {
        installation: "installation".into(),
        node: "node".into(),
    }
}
fn admins() -> Administrators {
    Administrators::new(
        scope(),
        [
            (
                "one".into(),
                SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes(),
            ),
            (
                "two".into(),
                SigningKey::from_bytes(&[2; 32]).verifying_key().to_bytes(),
            ),
        ]
        .into_iter()
        .collect(),
        60,
    )
    .unwrap()
}
fn proposal(digest: &str, nonce: u8) -> Proposal {
    Proposal {
        scope: scope(),
        action: Action::RetireBackup,
        target: target_commitment(digest.as_bytes()),
        operation_id: hex::encode([nonce; 32]),
        issued_at: 100,
        expires_at: 160,
    }
}
fn approvals(proposal: &Proposal) -> [Approval; 2] {
    [
        Approval {
            administrator: "one".into(),
            signature: hex::encode(
                SigningKey::from_bytes(&[1; 32])
                    .sign(&proposal.signing_bytes().unwrap())
                    .to_bytes(),
            ),
        },
        Approval {
            administrator: "two".into(),
            signature: hex::encode(
                SigningKey::from_bytes(&[2; 32])
                    .sign(&proposal.signing_bytes().unwrap())
                    .to_bytes(),
            ),
        },
    ]
}
fn inventory(path: &Path, bootstrap: BackupBootstrap<'_>) -> BackupInventory {
    BackupInventory::open(path, owner(path), &[21; 32], scope(), 10, bootstrap).unwrap()
}
fn authority(path: &Path, bootstrap: LedgerBootstrap<'_>) -> ControlAuthority {
    ControlAuthority::open(path, owner(path), &[22; 32], admins(), 10, bootstrap).unwrap()
}
fn sealed() -> Vec<u8> {
    seal(
        &KeyMaterial::new([7; 32], vec![8; 32], None, None).unwrap(),
        b"fixture-pass",
    )
    .unwrap()
}
fn write_private(path: &Path, bytes: &[u8]) {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn authenticates_before_create_and_restores_only_hash_verified_active_envelopes() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let mut store = inventory(&path, BackupBootstrap::New);
    let sealed = sealed();
    assert!(store.create(&sealed, b"wrong").is_err());
    assert!(store.orphans().unwrap().is_empty());
    assert!(store.list().unwrap().is_empty());
    let receipt = store.create(&sealed, b"fixture-pass").unwrap();
    let digest = receipt.digest;
    assert_eq!(
        fs::read(path.join(format!("{digest}.sealed"))).unwrap(),
        sealed
    );
    assert_eq!(
        store.read(&digest, b"fixture-pass").unwrap().signing_seed(),
        &[7; 32]
    );
    assert!(store.read(&digest, b"wrong").is_err());
    assert!(store.create(&sealed, b"fixture-pass").is_err());
    let pin = store.anchor().clone();
    assert!(pin.verify(&store.public_key()));
    drop(store);
    assert!(
        BackupInventory::open(
            &path,
            owner(&path),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::New
        )
        .is_err()
    );
    let store = inventory(&path, BackupBootstrap::Pinned(&pin));
    assert_eq!(store.list().unwrap().len(), 1);
    drop(store);
    fs::write(path.join(format!("{digest}.sealed")), b"corrupt").unwrap();
    assert!(
        BackupInventory::open(
            &path,
            owner(&path),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::Pinned(&pin)
        )
        .is_err()
    );
}

#[test]
fn retirement_requires_two_signed_admins_and_keeps_nonrestoreable_tombstone() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let control = temp.sub("control");
    let mut store = inventory(&path, BackupBootstrap::New);
    let mut control = authority(&control, LedgerBootstrap::New);
    let digest = store.create(&sealed(), b"fixture-pass").unwrap().digest;
    let proposal = proposal(&digest, 9);
    let signatures = approvals(&proposal);
    let mut other_action = proposal.clone();
    other_action.action = Action::EraseSubject;
    assert!(
        store
            .retire(&mut control, &other_action, &approvals(&other_action), 110)
            .is_err()
    );
    assert!(
        store
            .retire(&mut control, &proposal, &signatures[..1], 110)
            .is_err()
    );
    assert!(store.reconcile_retirement(&control, &proposal).is_err());
    assert!(path.join(format!("{digest}.sealed")).exists());
    let (receipt, consumed) = store
        .retire(&mut control, &proposal, &signatures, 110)
        .unwrap();
    assert_eq!(consumed.operation_id, proposal.operation_id);
    assert!(!path.join(format!("{digest}.sealed")).exists());
    assert!(store.read(&digest, b"fixture-pass").is_err());
    assert!(
        store
            .retire(&mut control, &proposal, &signatures, 110)
            .is_err()
    );
    assert!(matches!(
        store.list().unwrap()[0].status,
        BackupStatus::Retired { .. }
    ));
    drop(store);
    let reopened = inventory(&path, BackupBootstrap::Pinned(&receipt.anchor));
    assert!(reopened.read(&digest, b"fixture-pass").is_err());
}

#[test]
fn retired_local_copy_reappearance_is_an_explicit_failure() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let control_path = temp.sub("control");
    let mut store = inventory(&path, BackupBootstrap::New);
    let mut control = authority(&control_path, LedgerBootstrap::New);
    let sealed = sealed();
    let digest = store.create(&sealed, b"fixture-pass").unwrap().digest;
    let proposal = proposal(&digest, 37);
    store
        .retire(&mut control, &proposal, &approvals(&proposal), 110)
        .unwrap();
    write_private(&path.join(format!("{digest}.sealed")), &sealed);
    assert!(store.list().is_err());
    assert!(store.read(&digest, b"fixture-pass").is_err());
    let pin = store.anchor().clone();
    drop(store);
    assert!(
        BackupInventory::open(
            &path,
            owner(&path),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::Pinned(&pin)
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn unix_backup_file_aliases_and_wide_modes_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = Temp::new();
    let path = temp.sub("archive");
    let mut store = inventory(&path, BackupBootstrap::New);
    let digest = store.create(&sealed(), b"fixture-pass").unwrap().digest;
    let file = path.join(format!("{digest}.sealed"));
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(store.read(&digest, b"fixture-pass").is_err());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    let other = path.join("extra-copy");
    fs::hard_link(&file, &other).unwrap();
    assert!(store.read(&digest, b"fixture-pass").is_err());
    fs::remove_file(&other).unwrap();
    fs::rename(&file, &other).unwrap();
    symlink(&other, &file).unwrap();
    assert!(store.read(&digest, b"fixture-pass").is_err());
}

#[cfg(windows)]
#[test]
fn windows_backup_archive_reparse_directory_is_refused() {
    let temp = Temp::new();
    let archive = temp.sub("archive");
    let alias = temp.0.join("alias");
    junction::create(&archive, &alias).unwrap();
    assert!(
        BackupInventory::open(
            &alias,
            owner(&archive),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::New
        )
        .is_err()
    );
    junction::delete(&alias).unwrap();
}

#[test]
fn explicit_orphan_adoption_preserves_unknown_copy_residual_and_rejects_wrong_password() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let mut store = inventory(&path, BackupBootstrap::New);
    let sealed = sealed();
    let digest = hex::encode(Sha256::digest(&sealed));
    write_private(&path.join(format!("{digest}.sealed")), &sealed);
    assert_eq!(store.orphans().unwrap(), vec![digest.clone()]);
    assert!(store.read(&digest, b"fixture-pass").is_err());
    assert!(store.adopt_orphan(&digest, b"wrong").is_err());
    store.adopt_orphan(&digest, b"fixture-pass").unwrap();
    assert!(store.orphans().unwrap().is_empty());
    assert!(store.read(&digest, b"fixture-pass").is_ok());
}

#[test]
fn exact_external_anchor_detects_local_inventory_rollback() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let mut store = inventory(&path, BackupBootstrap::New);
    let empty = fs::read(path.join("backups.json")).unwrap();
    let digest = store.create(&sealed(), b"fixture-pass").unwrap().digest;
    let latest = store.anchor().clone();
    drop(store);
    fs::write(path.join("backups.json"), empty).unwrap();
    assert!(
        BackupInventory::open(
            &path,
            owner(&path),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::Pinned(&latest)
        )
        .is_err()
    );
    assert!(path.join(format!("{digest}.sealed")).exists());
}

#[test]
#[ignore = "subprocess entrypoint invoked by crash_reconcile_preserves_consumed_authorization"]
fn child_retire_without_pin_acknowledgement() {
    let root = PathBuf::from(
        std::env::var_os("HUMANYMOUS_BACKUP_CHILD").expect("subprocess fixture path"),
    );
    let path = root.join("archive");
    let control_path = root.join("control");
    let pin: BackupAnchor =
        serde_json::from_slice(&fs::read(root.join("external-backup-pin.json")).unwrap()).unwrap();
    let mut store = inventory(&path, BackupBootstrap::Pinned(&pin));
    let mut control = authority(&control_path, LedgerBootstrap::New);
    let digest = store.list().unwrap()[0].digest.clone();
    let proposal = proposal(&digest, 17);
    store
        .retire(&mut control, &proposal, &approvals(&proposal), 110)
        .unwrap();
    write_private(
        &root.join("external-control-pin.json"),
        &serde_json::to_vec(control.anchor()).unwrap(),
    );
    std::io::Write::write_all(
        &mut std::io::stdout(),
        b"AUDIT_BACKUP_RETIREMENT_PIN_STAGED_OK\n",
    )
    .unwrap();
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    std::process::exit(0);
}

#[test]
fn crash_reconcile_preserves_consumed_authorization() {
    let temp = Temp::new();
    let path = temp.sub("archive");
    let control_path = temp.sub("control");
    let mut store = inventory(&path, BackupBootstrap::New);
    let digest = store.create(&sealed(), b"fixture-pass").unwrap().digest;
    let pin = store.anchor().clone();
    write_private(
        &temp.0.join("external-backup-pin.json"),
        &serde_json::to_vec(&pin).unwrap(),
    );
    drop(store);
    let child = humanymous_test_support::process::capture_with_stderr(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "child_retire_without_pin_acknowledgement",
                "--ignored",
            ])
            .env("HUMANYMOUS_BACKUP_CHILD", &temp.0),
        b"",
        std::time::Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(child.status.success());
    assert!(child.stderr.is_empty());
    assert!(
        child
            .stdout
            .windows(b"AUDIT_BACKUP_RETIREMENT_PIN_STAGED_OK\n".len())
            .any(|w| w == b"AUDIT_BACKUP_RETIREMENT_PIN_STAGED_OK\n")
    );
    let control_pin: LedgerAnchor =
        serde_json::from_slice(&fs::read(temp.0.join("external-control-pin.json")).unwrap())
            .unwrap();
    let control = authority(&control_path, LedgerBootstrap::Pinned(&control_pin));
    assert!(
        BackupInventory::open(
            &path,
            owner(&path),
            &[21; 32],
            scope(),
            10,
            BackupBootstrap::Pinned(&pin)
        )
        .is_err()
    );
    let proposal = proposal(&digest, 17);
    let (store, receipt) = BackupInventory::resume_retirement(
        &path,
        owner(&path),
        &[21; 32],
        scope(),
        10,
        &pin,
        &control,
        &proposal,
    )
    .unwrap();
    assert_eq!(receipt.anchor.revision, pin.revision + 2);
    assert!(matches!(
        store.list().unwrap()[0].status,
        BackupStatus::Retired { .. }
    ));
    assert!(store.read(&digest, b"fixture-pass").is_err());
}

#[cfg(windows)]
#[test]
fn rename_failure_requires_explicit_authenticated_pending_retirement_recovery() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = Temp::new();
    let path = temp.sub("archive");
    let control_path = temp.sub("control");
    let mut store = inventory(&path, BackupBootstrap::New);
    let mut control = authority(&control_path, LedgerBootstrap::New);
    let digest = store.create(&sealed(), b"fixture-pass").unwrap().digest;
    let pin = store.anchor().clone();
    let proposal = proposal(&digest, 29);
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path.join("backups.json"))
        .unwrap();
    assert!(
        store
            .retire(&mut control, &proposal, &approvals(&proposal), 110)
            .is_err()
    );
    assert!(path.join(format!("{digest}.sealed")).exists());
    drop(blocker);
    drop(store);
    let (mut store, _) = BackupInventory::recover_pending(
        &path,
        owner(&path),
        &[21; 32],
        scope(),
        10,
        &pin,
        &control,
    )
    .unwrap();
    store.reconcile_retirement(&control, &proposal).unwrap();
    assert!(!path.join(format!("{digest}.sealed")).exists());
}
