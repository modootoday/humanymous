//! Actual private filesystem operations with synthetic administrator identities.
#![cfg(any(unix, windows))]
use ed25519_dalek::{Signer, SigningKey};
use humanymous_runtime::audit::{
    control::{
        Action, Administrators, Approval, ApprovalRequest, ControlAnchorRetainer, ControlAuthority,
        LedgerAnchor, LedgerBootstrap, NativeAnchorRetainer, Proposal, RetainedControlAnchor,
        RetentionRequirement, Scope, target_commitment,
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
fn material() -> Result<KeyMaterial, humanymous_runtime::audit::keys::KeyError> {
    KeyMaterial::new([7; 32], vec![8; 32], None, None)
}

#[test]
fn retained_erasure_rejects_scope_and_recovers_only_with_fresh_exact_approval() {
    let local = Dir::new();
    let independent = Dir::new();
    let keys = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let vault = Vault::new_scoped(Mode::Hmac, 20, scope()).unwrap();
    let before = vault.pseudonymize("subject-a", "synthetic-value").unwrap();
    vault.link("handle-a", "subject-a").unwrap();
    let snapshot = vault.snapshot().unwrap();
    let mut custody = keys.custody();
    custody
        .load_or_create(b"synthetic-passphrase", || {
            KeyMaterial::new([7; 32], vec![8; 32], Some(snapshot.to_vec()), Some([9; 32]))
        })
        .unwrap();
    let committed = fs::read(keys.0.join("keys.sealed")).unwrap();
    for wrong_scope in [true, false] {
        let proposal = authority
            .propose(
                Action::EraseSubject,
                target_commitment(b"subject-a"),
                10,
                100,
            )
            .unwrap();
        let signatures = approvals(&proposal);
        let grant = authority
            .authorize_retained_action(
                request(&proposal, &signatures),
                &mut retainer,
                RetentionRequirement::FileSynced,
            )
            .unwrap();
        let mut configured = scope();
        if wrong_scope {
            configured.node.push_str("-other");
        }
        let result = custody.erase_retained(
            &vault,
            b"synthetic-passphrase",
            grant,
            &configured,
            20,
            "subject-a",
        );
        if wrong_scope {
            assert!(result.is_err());
            assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), committed);
            assert_eq!(
                vault.pseudonymize("subject-a", "synthetic-value").unwrap(),
                before
            );
        } else {
            assert_eq!(result.unwrap().1.operation_id, proposal.operation_id);
        }
        assert!(
            authority
                .authorize_retained_action(
                    request(&proposal, &signatures),
                    &mut retainer,
                    RetentionRequirement::FileSynced,
                )
                .is_err()
        );
    }
    assert!(vault.resolve("handle-a").unwrap().is_none());
    let erased = fs::read(keys.0.join("keys.sealed")).unwrap();
    drop(custody);
    // Stage the real authenticated tombstone as a crash-before-promotion fixture.
    fs::copy(keys.0.join("keys.sealed"), keys.0.join("keys.pending")).unwrap();
    fs::write(keys.0.join("keys.sealed"), &committed).unwrap();
    let mut custody = keys.custody();
    for subject in ["subject-other", "subject-a"] {
        let proposal = authority
            .propose(
                Action::EraseSubject,
                target_commitment(subject.as_bytes()),
                10,
                100,
            )
            .unwrap();
        let signatures = approvals(&proposal);
        let grant = authority
            .authorize_retained_action(
                request(&proposal, &signatures),
                &mut retainer,
                RetentionRequirement::FileSynced,
            )
            .unwrap();
        let result = custody.recover_retained_erasure(
            b"synthetic-passphrase",
            b"synthetic-passphrase",
            grant,
            &scope(),
            20,
            subject,
        );
        if subject == "subject-other" {
            assert!(result.is_err());
            assert_eq!(fs::read(keys.0.join("keys.pending")).unwrap(), erased);
            assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), committed);
        } else {
            assert_eq!(result.unwrap().1.operation_id, proposal.operation_id);
        }
    }
    assert!(!keys.0.join("keys.pending").exists());
    let (restored, created) = custody
        .load_or_create(b"synthetic-passphrase", || panic!("no remint"))
        .unwrap();
    assert!(created.is_none());
    assert_eq!(restored.signing_seed(), &[7; 32]);
    assert_eq!(restored.hmac_key(), &[8; 32]);
    assert_eq!(restored.witness_seed(), Some(&[9; 32]));
    let restored_vault =
        Vault::load_scoped(restored.vault().unwrap(), Mode::Hmac, 20, scope()).unwrap();
    assert!(
        restored_vault
            .pseudonymize("subject-a", "synthetic-value")
            .unwrap()
            .starts_with("shredded:")
    );
    assert!(restored_vault.resolve("handle-a").unwrap().is_none());
}

#[test]
fn retained_state_authenticates_absence_and_preserves_unapplied_expired_kill_intent() {
    let directory = Dir::new();
    let independent = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let target = target_commitment(b"canonical-one-way-kill-target");
    let initial = authority
        .retained_action_state(&mut retainer, Action::KillSwitch, &target)
        .unwrap();
    assert!(initial.consumed().is_none());
    assert!(initial.other_consumed_targets().is_empty());
    assert_eq!(initial.scope(), &scope());
    assert_eq!(initial.action(), Action::KillSwitch);
    assert_eq!(initial.target(), target);
    assert_eq!(initial.authority_public(), authority.public_key());
    assert_eq!(initial.anchor(), authority.anchor());
    let proposal = authority
        .propose(Action::KillSwitch, target.clone(), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    drop(grant); // Intent retained; no runtime effect or audit commit was applied.
    let later = authority
        .propose(
            Action::KillSwitch,
            target_commitment(b"different-target-protocol"),
            200,
            250,
        )
        .unwrap();
    let signatures = approvals(&later);
    let grant = authority
        .authorize_retained_action(
            ApprovalRequest {
                proposal: &later,
                approvals: &signatures,
                now: 210,
            },
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    drop(grant);
    let latest = authority.anchor().clone();
    drop(authority);
    let authority = directory
        .authority(LedgerBootstrap::Pinned(&latest))
        .unwrap();
    let observed = authority
        .retained_action_state(&mut retainer, Action::KillSwitch, &target)
        .unwrap();
    assert_eq!(observed.consumed(), Some(&proposal));
    assert_eq!(observed.anchor(), &latest);
    assert!(observed.consumed().unwrap().expires_at < 210);
    assert_eq!(
        observed.other_consumed_targets(),
        std::slice::from_ref(&later.target)
    );
    let changed_protocol = authority
        .retained_action_state(
            &mut retainer,
            Action::KillSwitch,
            &target_commitment(b"third-target-protocol"),
        )
        .unwrap();
    assert!(changed_protocol.consumed().is_none());
    assert_eq!(changed_protocol.other_consumed_targets().len(), 2);
    assert!(
        authority
            .retained_action_state(&mut retainer, Action::KillSwitch, "invalid-target")
            .is_err()
    );
    let committed = directory.0.join("control.json");
    let saved = directory.0.join("control.saved-for-loss-probe");
    fs::rename(&committed, &saved).unwrap();
    assert!(
        authority
            .retained_action_state(&mut retainer, Action::KillSwitch, &target)
            .is_err()
    );
    fs::rename(&saved, &committed).unwrap();
    assert_eq!(
        authority
            .retained_action_state(&mut retainer, Action::KillSwitch, &target)
            .unwrap()
            .consumed(),
        Some(&proposal)
    );
}

#[test]
fn pending_erasure_rejects_same_subject_permit_from_foreign_scope() {
    let directory = Dir::new();
    let foreign_directory = Dir::new();
    let mut authority = directory.authority(LedgerBootstrap::New).unwrap();
    let foreign_scope = Scope {
        installation: "foreign-installation".into(),
        node: "node".into(),
    };
    let foreign_policy = Administrators::new(
        foreign_scope.clone(),
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
    .unwrap();
    let mut foreign_authority = ControlAuthority::open(
        &foreign_directory.0,
        owner(&foreign_directory.0),
        &[14; 32],
        foreign_policy,
        20,
        LedgerBootstrap::New,
    )
    .unwrap();
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
    let (permit, _) = authority
        .issue_erasure(&proposal, &approvals(&proposal), 20, "subject-a")
        .unwrap();
    let foreign_proposal = foreign_authority
        .propose(
            Action::EraseSubject,
            target_commitment(b"subject-a"),
            10,
            100,
        )
        .unwrap();
    let (foreign_permit, _) = foreign_authority
        .issue_erasure(
            &foreign_proposal,
            &approvals(&foreign_proposal),
            20,
            "subject-a",
        )
        .unwrap();
    vault.shred("subject-a", &permit).unwrap();
    let candidate = KeyMaterial::new(
        [7; 32],
        vec![8; 32],
        Some(vault.snapshot().unwrap().to_vec()),
        None,
    )
    .unwrap();
    let sealed = humanymous_runtime::audit::keys::seal(&candidate, b"pass").unwrap();
    let original = fs::read(directory.0.join("keys.sealed")).unwrap();
    let pending = directory.0.join("keys.pending");
    fs::copy(directory.0.join("keys.sealed"), &pending).unwrap();
    fs::write(&pending, &sealed).unwrap();
    drop(custody);
    let mut custody = directory.custody();
    assert!(
        custody
            .recover_erasure_pending(b"pass", b"pass", &foreign_permit, &scope())
            .is_err()
    );
    assert!(
        custody
            .recover_erasure_pending(b"pass", b"pass", &permit, &foreign_scope)
            .is_err()
    );
    assert_eq!(fs::read(directory.0.join("keys.sealed")).unwrap(), original);
    assert_eq!(fs::read(&pending).unwrap(), sealed);
    custody
        .recover_erasure_pending(b"pass", b"pass", &permit, &scope())
        .unwrap();
    assert!(!pending.exists());
    let key = custody
        .load_or_create(b"pass", || panic!("never regenerate erased linkage"))
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
fn interrupted_rotation_requires_exact_authenticated_candidate_and_fresh_retained_grant() {
    let local = Dir::new();
    let independent = Dir::new();
    let keys = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let mut custody = keys.custody();
    custody
        .load_or_create(b"old-synthetic-passphrase", material)
        .unwrap();
    let original = fs::read(keys.0.join("keys.sealed")).unwrap();
    let pending = keys.0.join("keys.pending");
    fs::copy(keys.0.join("keys.sealed"), &pending).unwrap();
    let sealed =
        humanymous_runtime::audit::keys::seal(&material().unwrap(), b"new-synthetic-passphrase")
            .unwrap();
    fs::write(&pending, &sealed).unwrap();
    drop(custody); // Explicit reopen models operator recovery of staged state.
    let mut custody = keys.custody();
    assert!(
        custody
            .inspect_pending_rotation(b"wrong", b"new-synthetic-passphrase")
            .is_err()
    );
    assert!(
        custody
            .inspect_pending_rotation(b"old-synthetic-passphrase", b"wrong")
            .is_err()
    );
    let recovery = custody
        .inspect_pending_rotation(b"old-synthetic-passphrase", b"new-synthetic-passphrase")
        .unwrap();
    let proposal = authority
        .propose(Action::RotatePassphrase, recovery.target().into(), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    custody
        .recover_prepared_rotation(recovery, grant, &scope(), 20)
        .unwrap();
    assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), sealed);
    assert_ne!(sealed, original);
    assert!(!pending.exists());
    assert!(
        authority
            .authorize_action(request(&proposal, &signatures))
            .is_err()
    );
}

#[test]
fn interrupted_rotation_rejects_vault_mutation_and_changed_pending_bytes() {
    let local = Dir::new();
    let independent = Dir::new();
    let keys = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let mut custody = keys.custody();
    custody
        .load_or_create(b"old-synthetic-passphrase", material)
        .unwrap();
    let original = fs::read(keys.0.join("keys.sealed")).unwrap();
    let pending = keys.0.join("keys.pending");
    fs::copy(keys.0.join("keys.sealed"), &pending).unwrap();
    let vault = Vault::new_scoped(Mode::Hmac, 20, scope()).unwrap();
    let changed = KeyMaterial::new(
        [7; 32],
        vec![8; 32],
        Some(vault.snapshot().unwrap().to_vec()),
        None,
    )
    .unwrap();
    fs::write(
        &pending,
        humanymous_runtime::audit::keys::seal(&changed, b"new-synthetic-passphrase").unwrap(),
    )
    .unwrap();
    assert!(
        custody
            .inspect_pending_rotation(b"old-synthetic-passphrase", b"new-synthetic-passphrase")
            .is_err()
    );
    fs::write(
        &pending,
        humanymous_runtime::audit::keys::seal(&material().unwrap(), b"new-synthetic-passphrase")
            .unwrap(),
    )
    .unwrap();
    let recovery = custody
        .inspect_pending_rotation(b"old-synthetic-passphrase", b"new-synthetic-passphrase")
        .unwrap();
    let proposal = authority
        .propose(Action::RotatePassphrase, recovery.target().into(), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    let substituted =
        humanymous_runtime::audit::keys::seal(&material().unwrap(), b"new-synthetic-passphrase")
            .unwrap();
    fs::write(&pending, &substituted).unwrap();
    assert!(
        custody
            .recover_prepared_rotation(recovery, grant, &scope(), 20)
            .is_err()
    );
    assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), original);
    assert_eq!(fs::read(&pending).unwrap(), substituted);
    authority.verify_consumed(&proposal).unwrap();
}

#[test]
fn trusted_snapshot_enforces_bound_lock_and_exact_pending_recovery() {
    use humanymous_runtime::audit::storage::TrustedSnapshot;
    let directory = Dir::new();
    assert!(TrustedSnapshot::open(&directory.0, owner(&directory.0), "runtime-state", 0).is_err());
    assert!(
        TrustedSnapshot::open(
            &directory.0,
            owner(&directory.0),
            "runtime-state",
            (4 << 20) + 1
        )
        .is_err()
    );
    let mut snapshot =
        TrustedSnapshot::open(&directory.0, owner(&directory.0), "runtime-state", 1024).unwrap();
    assert!(snapshot.read().unwrap().is_none());
    assert!(
        TrustedSnapshot::open(&directory.0, owner(&directory.0), "runtime-state", 1024).is_err()
    );
    snapshot.replace(b"caller-authenticated-committed").unwrap();
    assert!(snapshot.replace(&vec![0; 1025]).is_err());
    assert_eq!(
        snapshot.read().unwrap().as_deref(),
        Some(b"caller-authenticated-committed".as_slice())
    );
    drop(snapshot);
    let committed = directory.0.join("runtime-state.json");
    let pending = directory.0.join("runtime-state.pending");
    fs::copy(&committed, &pending).unwrap();
    fs::write(&pending, b"caller-authenticated-pending").unwrap();
    let mut snapshot =
        TrustedSnapshot::open(&directory.0, owner(&directory.0), "runtime-state", 1024).unwrap();
    assert!(snapshot.read().is_err());
    let inspected = snapshot.inspect().unwrap();
    assert_eq!(
        inspected.pending.as_deref(),
        Some(b"caller-authenticated-pending".as_slice())
    );
    assert!(
        snapshot
            .promote_pending(Some(b"wrong-committed"), b"caller-authenticated-pending")
            .is_err()
    );
    assert!(
        snapshot
            .promote_pending(Some(b"caller-authenticated-committed"), b"wrong-pending")
            .is_err()
    );
    snapshot
        .promote_pending(
            Some(b"caller-authenticated-committed"),
            b"caller-authenticated-pending",
        )
        .unwrap();
    assert_eq!(
        snapshot.read().unwrap().as_deref(),
        Some(b"caller-authenticated-pending".as_slice())
    );
    assert!(!pending.exists());
}

struct SealedFixture {
    keys: Dir,
    wal: Dir,
    witness: Dir,
}
impl SealedFixture {
    fn new() -> Self {
        let fixture = Self {
            keys: Dir::new(),
            wal: Dir::new(),
            witness: Dir::new(),
        };
        let mut custody = fixture.keys.custody();
        custody
            .load_or_create(b"synthetic-sealed-passphrase", material)
            .unwrap();
        fixture
    }
    fn options(
        &self,
        head: Option<humanymous_runtime::audit::checkpoint::Checkpoint>,
    ) -> humanymous_runtime::audit::log::SealedLogOptions {
        humanymous_runtime::audit::log::SealedLogOptions {
            key_directory: self.keys.0.clone(),
            wal_directory: self.wal.0.clone(),
            owner: owner(&self.keys.0),
            node_id: "node".into(),
            checkpoint_every: 2,
            allow_new_history: head.is_none(),
            trusted_head: head,
            require_external_witness: true,
        }
    }
    fn client(
        &self,
        head: Option<&humanymous_runtime::audit::checkpoint::Checkpoint>,
    ) -> humanymous_runtime::audit::log::PinnedWitness {
        use humanymous_runtime::audit::{
            checkpoint::{DurableWitness, WitnessBootstrap},
            log::{PinnedWitness, WitnessDurability},
        };
        let writer = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
        let bootstrap = match head {
            Some(minimum) => WitnessBootstrap::Pinned {
                minimum,
                consistency_proof: &[],
            },
            None => WitnessBootstrap::CreateNew,
        };
        let witness = DurableWitness::open(
            &self.witness.0,
            owner(&self.witness.0),
            &[22; 32],
            writer,
            "node".into(),
            bootstrap,
        )
        .unwrap();
        let public = witness.public_key();
        PinnedWitness::new(Box::new(witness), public, WitnessDurability::FileSynced).unwrap()
    }
}

#[test]
fn sealed_owner_retains_both_locks_and_repairs_only_missing_external_checkpoint() {
    use humanymous_runtime::audit::{
        log::{SealedLog, UnlockCredential},
        record::Record,
        storage::{TailRecovery, Wal, WalOptions},
    };
    use zeroize::Zeroizing;
    let fixture = SealedFixture::new();
    let original = fs::read(fixture.keys.0.join("keys.sealed")).unwrap();
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    let mut log = SealedLog::open(
        &fixture.options(None),
        UnlockCredential::Bytes(&mut password),
        Some(fixture.client(None)),
    )
    .unwrap();
    assert!(password.is_empty());
    assert!(Custody::acquire(&fixture.keys.0, owner(&fixture.keys.0)).is_err());
    assert!(Wal::open(&fixture.wal.0, WalOptions::default()).is_err());
    for _ in 0..2 {
        log.append(Record {
            event_type: "test.synthetic".into(),
            ..Default::default()
        })
        .unwrap();
    }
    log.flush().unwrap();
    let checkpoints = log.checkpoints().unwrap();
    let head = checkpoints.last().unwrap().clone();
    drop(log);
    let checkpoint_path = fixture.wal.0.join("checkpoints.jsonl");
    let bytes = fs::read(&checkpoint_path).unwrap();
    let prefix_end = bytes.iter().position(|byte| *byte == b'\n').unwrap() + 1;
    fs::write(&checkpoint_path, &bytes[..prefix_end]).unwrap();
    let options = fixture.options(Some(head.clone()));
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    let repaired = SealedLog::recover_pinned(
        &options,
        UnlockCredential::Bytes(&mut password),
        fixture.client(Some(&head)),
        TailRecovery::RequireCovered,
    )
    .unwrap();
    assert_eq!(repaired.sequence(), 2);
    assert_eq!(repaired.checkpoints().unwrap(), checkpoints);
    let receipt = repaired.recovery_receipt().unwrap();
    assert!(receipt.checkpoint_appended);
    assert!(receipt.persistence.is_some());
    assert_eq!(receipt.coverage.ambiguous_tail_len, 0);
    assert!(receipt.coverage.fully_witnessed);
    assert_eq!(
        fs::read(fixture.keys.0.join("keys.sealed")).unwrap(),
        original
    );
}

#[test]
fn sealed_recovery_preserves_ambiguous_complete_tail_without_checkpoint_acknowledgement() {
    use humanymous_runtime::audit::{
        log::{SealedLog, UnlockCredential},
        record::Record,
        storage::TailRecovery,
    };
    use zeroize::Zeroizing;
    let fixture = SealedFixture::new();
    let original = fs::read(fixture.keys.0.join("keys.sealed")).unwrap();
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    let mut log = SealedLog::open(
        &fixture.options(None),
        UnlockCredential::Bytes(&mut password),
        Some(fixture.client(None)),
    )
    .unwrap();
    for _ in 0..2 {
        log.append(Record {
            event_type: "test.synthetic".into(),
            ..Default::default()
        })
        .unwrap();
    }
    log.flush().unwrap();
    let head = log.checkpoints().unwrap().last().unwrap().clone();
    assert_eq!(head.tree_size, 2);
    log.append(Record {
        event_type: "test.synthetic".into(),
        ..Default::default()
    })
    .unwrap();
    let records = log.records().unwrap();
    drop(log);
    let options = fixture.options(Some(head.clone()));
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    assert!(
        SealedLog::open(
            &options,
            UnlockCredential::Bytes(&mut password),
            Some(fixture.client(Some(&head)))
        )
        .is_err()
    );
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    assert!(
        SealedLog::recover_pinned(
            &options,
            UnlockCredential::Bytes(&mut password),
            fixture.client(Some(&head)),
            TailRecovery::RequireCovered
        )
        .is_err()
    );
    let mut password = Zeroizing::new(b"synthetic-sealed-passphrase".to_vec());
    let recovered = SealedLog::recover_pinned(
        &options,
        UnlockCredential::Bytes(&mut password),
        fixture.client(Some(&head)),
        TailRecovery::PreserveAmbiguousTail,
    )
    .unwrap();
    assert_eq!(recovered.sequence(), 3);
    assert_eq!(
        serde_json::to_vec(&recovered.records().unwrap()).unwrap(),
        serde_json::to_vec(&records).unwrap()
    );
    let receipt = recovered.recovery_receipt().unwrap();
    assert!(!receipt.checkpoint_appended);
    assert!(receipt.persistence.is_none());
    assert_eq!(receipt.coverage.ambiguous_tail_len, 1);
    assert!(!receipt.coverage.fully_witnessed);
    assert_eq!(
        fs::read(fixture.keys.0.join("keys.sealed")).unwrap(),
        original
    );
}

#[test]
fn retained_grant_requires_independent_anchor_and_exact_restart_pin() {
    let local = Dir::new();
    let independent = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let initial = authority.anchor().clone();
    let public = authority.public_key();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        public,
        &initial,
        true,
    )
    .unwrap();
    let original = fs::read(independent.0.join("control-anchor.json")).unwrap();
    let proposal = authority
        .propose(
            Action::KillSwitch,
            target_commitment(b"activate-only"),
            10,
            100,
        )
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    let latest = grant.retention().anchor().clone();
    assert_eq!(retainer.current().unwrap(), latest);
    let mut effects = 0;
    grant
        .execute(&scope(), Action::KillSwitch, &proposal.target, 20, |_| {
            effects += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(effects, 1);
    assert!(
        authority
            .authorize_retained_action(
                request(&proposal, &signatures),
                &mut retainer,
                RetentionRequirement::FileSynced
            )
            .is_err()
    );
    drop(retainer);
    assert!(
        NativeAnchorRetainer::open(
            &independent.0,
            owner(&independent.0),
            scope(),
            public,
            &initial,
            false
        )
        .is_err()
    );
    let restored = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        public,
        &latest,
        false,
    )
    .unwrap();
    drop(restored);
    fs::write(independent.0.join("control-anchor.json"), original).unwrap();
    assert!(
        NativeAnchorRetainer::open(
            &independent.0,
            owner(&independent.0),
            scope(),
            public,
            &latest,
            false
        )
        .is_err()
    );
}

struct UnavailableRetainer {
    scope: Scope,
    public: [u8; 32],
    anchor: LedgerAnchor,
}
impl ControlAnchorRetainer for UnavailableRetainer {
    fn scope(&self) -> &Scope {
        &self.scope
    }
    fn authority_public(&self) -> [u8; 32] {
        self.public
    }
    fn current(&mut self) -> std::io::Result<LedgerAnchor> {
        Ok(self.anchor.clone())
    }
    fn retain(
        &mut self,
        _: &LedgerAnchor,
        _: &LedgerAnchor,
    ) -> std::io::Result<RetainedControlAnchor> {
        Err(std::io::Error::other(
            "synthetic unavailable independent retention peer",
        ))
    }
}

#[test]
fn external_retention_failure_spends_nonce_without_issuing_effect_grant() {
    let local = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut unavailable = UnavailableRetainer {
        scope: scope(),
        public: authority.public_key(),
        anchor: authority.anchor().clone(),
    };
    let proposal = authority
        .propose(
            Action::KillSwitch,
            target_commitment(b"activate-only"),
            10,
            100,
        )
        .unwrap();
    let signatures = approvals(&proposal);
    assert!(
        authority
            .authorize_retained_action(
                request(&proposal, &signatures),
                &mut unavailable,
                RetentionRequirement::FileSynced
            )
            .is_err()
    );
    assert_eq!(authority.anchor().revision, 1);
    authority.verify_consumed(&proposal).unwrap();
    assert!(
        authority
            .retained_action_state(&mut unavailable, Action::KillSwitch, &proposal.target)
            .is_err()
    );
    assert!(
        authority
            .authorize_action(request(&proposal, &signatures))
            .is_err()
    );
    let latest = authority.anchor().clone();
    drop(authority);
    let mut reopened = local.authority(LedgerBootstrap::Pinned(&latest)).unwrap();
    assert!(
        reopened
            .authorize_action(request(&proposal, &signatures))
            .is_err()
    );
}

#[test]
fn independent_retainer_recovers_only_exact_authenticated_pending_transition() {
    let local = Dir::new();
    let independent = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let previous = authority.anchor().clone();
    let public = authority.public_key();
    let retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        public,
        &previous,
        true,
    )
    .unwrap();
    drop(retainer);
    let proposal = authority
        .propose(
            Action::KillSwitch,
            target_commitment(b"activate-only"),
            10,
            100,
        )
        .unwrap();
    let next = authority
        .consume(&proposal, &approvals(&proposal), 20)
        .unwrap()
        .anchor;
    let pending = serde_json::json!({"scope":scope(), "authority_public":public, "anchor":next});
    let pending_path = independent.0.join("control-anchor.pending");
    fs::copy(independent.0.join("control-anchor.json"), &pending_path).unwrap();
    fs::write(&pending_path, serde_json::to_vec(&pending).unwrap()).unwrap();
    assert!(
        NativeAnchorRetainer::open(
            &independent.0,
            owner(&independent.0),
            scope(),
            public,
            &previous,
            false
        )
        .is_err()
    );
    assert!(
        NativeAnchorRetainer::recover_pending(
            &independent.0,
            owner(&independent.0),
            scope(),
            public,
            &next,
            &previous
        )
        .is_err()
    );
    let (mut recovered, acknowledgement) = NativeAnchorRetainer::recover_pending(
        &independent.0,
        owner(&independent.0),
        scope(),
        public,
        &previous,
        &next,
    )
    .unwrap();
    assert_eq!(acknowledgement.anchor(), &next);
    assert_eq!(recovered.current().unwrap(), next);
    assert!(!pending_path.exists());
}

#[test]
fn prepared_rotation_applies_exact_retained_destination_and_preserves_identity() {
    let local = Dir::new();
    let independent = Dir::new();
    let keys = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let mut custody = keys.custody();
    let vault = Vault::new_scoped(Mode::Hmac, 20, scope()).unwrap();
    vault.pseudonymize("subject-a", "synthetic-value").unwrap();
    let snapshot = vault.snapshot().unwrap();
    custody
        .load_or_create(b"old-synthetic-passphrase", || {
            KeyMaterial::new([7; 32], vec![8; 32], Some(snapshot.to_vec()), Some([9; 32]))
        })
        .unwrap();
    assert!(
        custody
            .prepare_rotation(b"wrong", b"new-synthetic-passphrase")
            .is_err()
    );
    assert!(
        custody
            .prepare_rotation(b"old-synthetic-passphrase", b"")
            .is_err()
    );
    let prepared = custody
        .prepare_rotation(b"old-synthetic-passphrase", b"new-synthetic-passphrase")
        .unwrap();
    let proposal = authority
        .propose(Action::RotatePassphrase, prepared.target().into(), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    custody
        .apply_prepared_rotation(prepared, grant, &scope(), 20)
        .unwrap();
    assert!(
        custody
            .load_or_create(b"old-synthetic-passphrase", material)
            .is_err()
    );
    let (reopened, created) = custody
        .load_or_create(b"new-synthetic-passphrase", || {
            panic!("rotation never regenerates identity")
        })
        .unwrap();
    assert!(created.is_none());
    assert_eq!(reopened.signing_seed(), &[7; 32]);
    assert_eq!(reopened.hmac_key(), &[8; 32]);
    assert_eq!(reopened.vault(), Some(snapshot.as_slice()));
    assert_eq!(reopened.witness_seed(), Some(&[9; 32]));
}

#[test]
fn prepared_rotation_rejects_substituted_destination_and_stale_source() {
    let local = Dir::new();
    let independent = Dir::new();
    let keys = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let mut retainer = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        authority.public_key(),
        authority.anchor(),
        true,
    )
    .unwrap();
    let mut custody = keys.custody();
    custody
        .load_or_create(b"old-synthetic-passphrase", material)
        .unwrap();
    let original = fs::read(keys.0.join("keys.sealed")).unwrap();
    let approved = custody
        .prepare_rotation(b"old-synthetic-passphrase", b"approved-destination")
        .unwrap();
    let substituted = custody
        .prepare_rotation(b"old-synthetic-passphrase", b"unapproved-destination")
        .unwrap();
    assert_ne!(approved.target(), substituted.target());
    let proposal = authority
        .propose(Action::RotatePassphrase, approved.target().into(), 10, 100)
        .unwrap();
    let signatures = approvals(&proposal);
    let grant = authority
        .authorize_retained_action(
            request(&proposal, &signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    assert!(
        custody
            .apply_prepared_rotation(substituted, grant, &scope(), 20)
            .is_err()
    );
    assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), original);
    assert!(!keys.0.join("keys.pending").exists());
    authority.verify_consumed(&proposal).unwrap();
    let retry = authority
        .propose(Action::RotatePassphrase, approved.target().into(), 10, 100)
        .unwrap();
    let retry_signatures = approvals(&retry);
    let grant = authority
        .authorize_retained_action(
            request(&retry, &retry_signatures),
            &mut retainer,
            RetentionRequirement::FileSynced,
        )
        .unwrap();
    custody
        .rotate(b"old-synthetic-passphrase", b"newer-local-generation")
        .unwrap();
    let newer = fs::read(keys.0.join("keys.sealed")).unwrap();
    assert!(
        custody
            .apply_prepared_rotation(approved, grant, &scope(), 20)
            .is_err()
    );
    assert_eq!(fs::read(keys.0.join("keys.sealed")).unwrap(), newer);
    assert!(!keys.0.join("keys.pending").exists());
}

#[test]
fn retention_service_real_child_persists_restarts_and_rejects_stale_pin() {
    use humanymous_platform::testing::process::capture_with_stderr;
    use humanymous_runtime::audit::{
        checkpoint::service::write_frame,
        control::retention_service::{
            RetentionServiceClient, RetentionServiceConfig, RetentionServiceRequest,
        },
    };
    use std::{io::Cursor, process::Command, time::Duration};
    let local = Dir::new();
    let independent = Dir::new();
    let mut authority = local.authority(LedgerBootstrap::New).unwrap();
    let initial = authority.anchor().clone();
    let public = authority.public_key();
    let proposal = authority
        .propose(
            Action::KillSwitch,
            target_commitment(b"activate-only"),
            10,
            100,
        )
        .unwrap();
    let latest = authority
        .consume(&proposal, &approvals(&proposal), 20)
        .unwrap()
        .anchor;
    let owner_sid = match owner(&independent.0) {
        #[cfg(unix)]
        Owner::CurrentUser => None,
        #[cfg(windows)]
        Owner::WindowsSid(sid) => Some(sid),
    };
    let mut config = RetentionServiceConfig {
        directory: independent.0.clone(),
        owner_sid,
        scope: scope(),
        authority_public: public,
        pin: initial.clone(),
        provision: true,
    };
    let mut input = Vec::new();
    write_frame(&mut input, &config).unwrap();
    write_frame(&mut input, &RetentionServiceRequest::State).unwrap();
    write_frame(
        &mut input,
        &RetentionServiceRequest::Retain {
            previous: initial.clone(),
            next: latest.clone(),
        },
    )
    .unwrap();
    write_frame(&mut input, &RetentionServiceRequest::State).unwrap();
    write_frame(&mut input, &RetentionServiceRequest::Shutdown).unwrap();
    let binary = env!("CARGO_BIN_EXE_audit-control-retainer");
    let captured = capture_with_stderr(
        &mut Command::new(binary),
        &input,
        Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(captured.status.success());
    assert!(captured.stderr.is_empty());
    assert_eq!(
        captured
            .stdout
            .iter()
            .filter(|byte| **byte == b'\n')
            .count(),
        5
    );
    // Verify the actual child's complete transcript using the shared client.
    // This is captured-protocol validation, not a live interactive deadline test.
    let mut client = RetentionServiceClient::from_ready(
        Cursor::new(captured.stdout),
        Vec::new(),
        scope(),
        public,
        &initial,
    )
    .unwrap();
    assert_eq!(client.current().unwrap(), initial);
    assert_eq!(client.retain(&initial, &latest).unwrap().anchor(), &latest);
    assert_eq!(client.current().unwrap(), latest);
    client.shutdown().unwrap();

    config.pin = latest.clone();
    config.provision = false;
    let mut input = Vec::new();
    write_frame(&mut input, &config).unwrap();
    write_frame(&mut input, &RetentionServiceRequest::State).unwrap();
    write_frame(&mut input, &RetentionServiceRequest::Shutdown).unwrap();
    let captured = capture_with_stderr(
        &mut Command::new(binary),
        &input,
        Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(captured.status.success());
    assert!(captured.stderr.is_empty());
    assert_eq!(
        captured
            .stdout
            .iter()
            .filter(|byte| **byte == b'\n')
            .count(),
        3
    );
    let mut client = RetentionServiceClient::from_ready(
        Cursor::new(captured.stdout),
        Vec::new(),
        scope(),
        public,
        &latest,
    )
    .unwrap();
    assert_eq!(client.current().unwrap(), latest);
    client.shutdown().unwrap();

    config.pin = initial;
    let mut input = Vec::new();
    write_frame(&mut input, &config).unwrap();
    let captured = capture_with_stderr(
        &mut Command::new(binary),
        &input,
        Duration::from_secs(60),
        64 * 1024,
    )
    .unwrap();
    assert!(!captured.status.success());
    assert!(captured.stdout.is_empty());
    assert_eq!(captured.stderr, b"control retention service unavailable\n");
    let mut restored = NativeAnchorRetainer::open(
        &independent.0,
        owner(&independent.0),
        scope(),
        public,
        &latest,
        false,
    )
    .unwrap();
    assert_eq!(restored.current().unwrap(), latest);
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
    let ledger = directory.0.join("control.json");
    let saved = directory.0.join("control.saved-for-membership-loss");
    fs::rename(&ledger, &saved).unwrap();
    assert!(restored.verify_consumed(&proposal).is_err());
    fs::rename(&saved, &ledger).unwrap();
    fs::write(&ledger, b"corrupt authenticated ledger probe").unwrap();
    assert!(restored.verify_consumed(&proposal).is_err());
    fs::write(&ledger, &original).unwrap();
    assert!(restored.verify_consumed(&proposal).is_err());
    fs::write(&ledger, &committed).unwrap();
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
            .recover_erasure_pending(b"pass", b"pass", &permit_b, &scope())
            .is_err()
    );
    custody
        .recover_erasure_pending(b"pass", b"pass", &permit_a, &scope())
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
    use humanymous_runtime::audit::{
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
