//! Actual private filesystem operations with synthetic administrator identities.
#![cfg(any(unix, windows))]
use ed25519_dalek::{Signer, SigningKey};
use humanymous_audit::{
    control::{
        Action, Administrators, Approval, ApprovalRequest, ControlAuthority, LedgerBootstrap,
        Proposal, Scope, target_commitment,
    },
    keys::{
        KeyMaterial,
        custody::{Custody, Owner},
        vault::{Mode, Vault},
    },
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-control-{}-{}",
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
    fn authority(&self, bootstrap: LedgerBootstrap<'_>) -> std::io::Result<ControlAuthority> {
        ControlAuthority::open(&self.0, owner(&self.0), &[13; 32], policy(), 20, bootstrap)
    }
    fn custody(&self) -> Custody {
        Custody::acquire(&self.0, owner(&self.0)).unwrap()
    }
}
impl Drop for Dir {
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
fn policy() -> Administrators {
    Administrators::new(
        scope(),
        BTreeMap::from([
            (
                "one".into(),
                SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes(),
            ),
            (
                "two".into(),
                SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes(),
            ),
        ]),
        100,
    )
    .unwrap()
}
fn approvals(proposal: &Proposal) -> Vec<Approval> {
    [("one", 11), ("two", 12)]
        .iter()
        .map(|(id, seed)| Approval {
            administrator: (*id).into(),
            signature: hex::encode(
                SigningKey::from_bytes(&[*seed; 32])
                    .sign(&proposal.signing_bytes().unwrap())
                    .to_bytes(),
            ),
        })
        .collect()
}
fn request<'a>(proposal: &'a Proposal, approvals: &'a [Approval]) -> ApprovalRequest<'a> {
    ApprovalRequest {
        proposal,
        approvals,
        now: 20,
    }
}
fn material() -> Result<KeyMaterial, humanymous_audit::keys::KeyError> {
    KeyMaterial::new([7; 32], vec![8; 32], None, None)
}

#[test]
fn durable_consumption_rejects_replay_and_whole_file_rollback() {
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let first = authority.anchor().clone();
    let original = fs::read(directory.0.join("control.json")).unwrap();
    let proposal = authority
        .propose(Action::KillSwitch, target_commitment(b"switch"), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    assert!(authority.consume(&proposal, &signatures[..1], 20).is_err());
    let permit = authority
        .authorize_action(request(&proposal, &signatures))
        .unwrap();
    let latest = permit.receipt().anchor.clone();
    assert!(authority.consume(&proposal, &signatures, 20).is_err());
    permit
        .execute(&scope(), Action::KillSwitch, &proposal.target, 20, |_| {
            Ok(())
        })
        .unwrap();
    let committed = fs::read(directory.0.join("control.json")).unwrap();
    drop(authority);
    assert!(
        directory
            .authority(LedgerBootstrap::Pinned(&first))
            .is_err()
    );
    let restored = directory
        .authority(LedgerBootstrap::Pinned(&latest))
        .unwrap();
    restored.verify_consumed(&proposal).unwrap();
    drop(restored);
    fs::write(directory.0.join("control.json"), &original).unwrap();
    assert!(
        directory
            .authority(LedgerBootstrap::Pinned(&latest))
            .is_err()
    );
    fs::write(directory.0.join("control.json"), committed).unwrap();
}

#[test]
fn exact_subject_scope_erasure_is_durable_and_cannot_authorize_another_subject() {
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let mut custody = directory.custody();
    custody.load_or_create(b"pass", material).unwrap();
    let vault = Vault::new_scoped(Mode::Hmac, 20, scope()).unwrap();
    vault.pseudonymize("subject-a", "synthetic").unwrap();
    vault.pseudonymize("subject-b", "synthetic").unwrap();
    custody.persist_vault(&vault, b"pass").unwrap();
    let proposal = authority
        .propose(
            Action::EraseSubject,
            target_commitment(b"subject-a"),
            10,
            100,
        )
        .unwrap();
    let signatures = approvals(&proposal);
    assert!(
        custody
            .erase_approved(
                &vault,
                b"wrong",
                &mut authority,
                request(&proposal, &signatures),
                "subject-a"
            )
            .is_err()
    );
    let (permit, receipt) = authority
        .issue_erasure(&proposal, &signatures, 20, "subject-a")
        .unwrap();
    assert!(vault.shred("subject-b", &permit).is_err());
    let other = Vault::new_scoped(
        Mode::Hmac,
        20,
        Scope {
            installation: "foreign".into(),
            node: "node".into(),
        },
    )
    .unwrap();
    assert!(other.shred("subject-a", &permit).is_err());
    vault.shred("subject-a", &permit).unwrap();
    custody.persist_erasure(&vault, b"pass", &permit).unwrap();
    drop(custody);
    drop(authority);
    let mut authority = directory
        .authority(LedgerBootstrap::Pinned(&receipt.anchor))
        .unwrap();
    assert!(
        authority
            .issue_erasure(&proposal, &signatures, 20, "subject-a")
            .is_err()
    );
    let resumed = authority
        .resume_erasure(&proposal, &signatures, 20, "subject-a")
        .unwrap();
    let mut custody = directory.custody();
    let key = custody
        .load_or_create(b"pass", || panic!("never remint identity"))
        .unwrap()
        .0;
    let restored = Vault::load_scoped(key.vault().unwrap(), Mode::Hmac, 20, scope()).unwrap();
    assert!(
        restored
            .pseudonymize("subject-a", "value")
            .unwrap()
            .starts_with("shredded:")
    );
    assert!(
        !restored
            .pseudonymize("subject-b", "value")
            .unwrap()
            .starts_with("shredded:")
    );
    restored.shred("subject-a", &resumed).unwrap();
    custody
        .persist_erasure(&restored, b"pass", &resumed)
        .unwrap();
}

#[test]
fn approved_rotation_binds_exact_ciphertext_and_preserves_identity() {
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let mut custody = directory.custody();
    custody.load_or_create(b"old", material).unwrap();
    let proposal = authority
        .propose(
            Action::RotatePassphrase,
            target_commitment(custody.committed_digest().unwrap().as_bytes()),
            10,
            100,
        )
        .unwrap();
    let signatures = approvals(&proposal);
    assert!(
        custody
            .rotate_approved(
                &mut authority,
                request(&proposal, &signatures),
                b"wrong",
                b"new"
            )
            .is_err()
    );
    custody
        .rotate_approved(
            &mut authority,
            request(&proposal, &signatures),
            b"old",
            b"new",
        )
        .unwrap();
    assert!(
        custody
            .load_or_create(b"old", || panic!("never create"))
            .is_err()
    );
    assert_eq!(
        custody
            .load_or_create(b"new", || panic!("never create"))
            .unwrap()
            .0
            .signing_seed(),
        &[7; 32]
    );
    assert!(
        custody
            .rotate_approved(
                &mut authority,
                request(&proposal, &signatures),
                b"new",
                b"next"
            )
            .is_err()
    );
}

#[test]
fn observed_clock_rollback_and_action_substitution_fail_closed() {
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let proposal = authority
        .propose(Action::PermanentBan, target_commitment(b"subject"), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let permit = authority
        .authorize_action(request(&proposal, &signatures))
        .unwrap();
    let mut acted = false;
    assert!(
        permit
            .execute(&scope(), Action::KillSwitch, &proposal.target, 20, |_| {
                acted = true;
                Ok(())
            })
            .is_err()
    );
    assert!(!acted);
    let unused = authority
        .propose(Action::KillSwitch, target_commitment(b"switch"), 10, 100)
        .unwrap();
    let signatures = approvals(&unused);
    assert!(authority.consume(&unused, &signatures, 150).is_err());
    assert!(authority.consume(&unused, &signatures, 30).is_err());
}

#[cfg(windows)]
#[test]
fn interrupted_consumption_requires_pinned_recovery_before_resume() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let pin = authority.anchor().clone();
    let proposal = authority
        .propose(Action::EraseSubject, target_commitment(b"subject"), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(directory.0.join("control.json"))
        .unwrap();
    assert!(
        authority
            .issue_erasure(&proposal, &signatures, 20, "subject")
            .is_err()
    );
    drop(blocker);
    assert!(authority.consume(&proposal, &signatures, 20).is_err());
    drop(authority);
    assert!(directory.authority(LedgerBootstrap::Pinned(&pin)).is_err());
    let (mut recovered, receipt) = ControlAuthority::recover_pending(
        &directory.0,
        owner(&directory.0),
        &[13; 32],
        policy(),
        20,
        &pin,
    )
    .unwrap();
    assert_eq!(receipt.operation_id, proposal.operation_id);
    assert!(
        recovered
            .issue_erasure(&proposal, &signatures, 20, "subject")
            .is_err()
    );
    recovered
        .resume_erasure(&proposal, &signatures, 20, "subject")
        .unwrap();
}

#[cfg(windows)]
#[test]
fn interrupted_tombstone_requires_its_exact_subject_permit() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let mut custody = directory.custody();
    custody.load_or_create(b"pass", material).unwrap();
    let vault = Vault::new_scoped(Mode::Hmac, 20, scope()).unwrap();
    vault.pseudonymize("subject-a", "synthetic").unwrap();
    vault.pseudonymize("subject-b", "synthetic").unwrap();
    custody.persist_vault(&vault, b"pass").unwrap();
    let a = authority
        .propose(
            Action::EraseSubject,
            target_commitment(b"subject-a"),
            10,
            100,
        )
        .unwrap();
    let (permit_a, _) = authority
        .issue_erasure(&a, &approvals(&a), 20, "subject-a")
        .unwrap();
    vault.shred("subject-a", &permit_a).unwrap();
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(directory.0.join("keys.sealed"))
        .unwrap();
    assert!(custody.persist_erasure(&vault, b"pass", &permit_a).is_err());
    drop(blocker);
    drop(custody);
    let mut custody = directory.custody();
    assert!(custody.recover_pending(b"pass", b"pass").is_err());
    let b = authority
        .propose(
            Action::EraseSubject,
            target_commitment(b"subject-b"),
            10,
            100,
        )
        .unwrap();
    let (permit_b, _) = authority
        .issue_erasure(&b, &approvals(&b), 20, "subject-b")
        .unwrap();
    assert!(
        custody
            .recover_erasure_pending(b"pass", b"pass", &permit_b)
            .is_err()
    );
    custody
        .recover_erasure_pending(b"pass", b"pass", &permit_a)
        .unwrap();
    let key = custody
        .load_or_create(b"pass", || panic!("never remint"))
        .unwrap()
        .0;
    let restored = Vault::load_scoped(key.vault().unwrap(), Mode::Hmac, 20, scope()).unwrap();
    assert!(
        restored
            .pseudonymize("subject-a", "value")
            .unwrap()
            .starts_with("shredded:")
    );
    assert!(
        !restored
            .pseudonymize("subject-b", "value")
            .unwrap()
            .starts_with("shredded:")
    );
}

#[test]
fn independent_witness_log_and_verified_sink_restart_reject_tamper() {
    use humanymous_audit::{
        checkpoint::{DurableWitness, WitnessBootstrap},
        log::{Config, Log, WitnessDurability},
        record::Record,
        sink::{DurableSink, SinkBootstrap, SinkPolicy},
        storage::{Wal, WalOptions},
    };
    let logdir = Dir::new();
    let witnessdir = Dir::new();
    let sinkdir = Dir::new();
    let writer = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
    let witness = DurableWitness::open(
        &witnessdir.0,
        owner(&witnessdir.0),
        &[22; 32],
        writer,
        "node".into(),
        WitnessBootstrap::CreateNew,
    )
    .unwrap();
    let public = witness.public_key();
    let config = Config {
        node_id: "node".into(),
        hmac_key: vec![8; 32],
        signing_seed: [7; 32],
        checkpoint_every: 1,
        witness: None,
    };
    let mut log = Log::open_with_external_witness(
        Wal::open(&logdir.0, WalOptions::default()).unwrap(),
        config,
        None,
        true,
        Box::new(witness),
        WitnessDurability::FileSynced,
    )
    .unwrap();
    log.append(Record {
        event_type: "test.synthetic".into(),
        ..Default::default()
    })
    .unwrap();
    log.flush().unwrap();
    let records = log.records().unwrap();
    let checkpoints = log.checkpoints().unwrap();
    let policy = || SinkPolicy {
        node: "node".into(),
        hmac_key: vec![8; 32],
        writer,
        witness: Some(public),
    };
    let mut sink =
        DurableSink::open(&sinkdir.0, owner(&sinkdir.0), policy(), SinkBootstrap::New).unwrap();
    let receipt = sink.publish(records.clone(), checkpoints.clone()).unwrap();
    let mut forged = records.clone();
    forged[0].event_type = "tampered".into();
    assert!(sink.publish(forged, checkpoints.clone()).is_err());
    drop(sink);
    let restored = DurableSink::open(
        &sinkdir.0,
        owner(&sinkdir.0),
        policy(),
        SinkBootstrap::Pinned(&receipt.checkpoint),
    )
    .unwrap();
    assert_eq!(restored.head(), Some(&receipt.checkpoint));
    drop(restored);
    let bytes = fs::read(sinkdir.0.join("sink.json")).unwrap();
    fs::write(sinkdir.0.join("sink.json"), b"partial").unwrap();
    assert!(
        DurableSink::open(
            &sinkdir.0,
            owner(&sinkdir.0),
            policy(),
            SinkBootstrap::Pinned(&receipt.checkpoint)
        )
        .is_err()
    );
    fs::write(sinkdir.0.join("sink.json"), bytes).unwrap();
}
