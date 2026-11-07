//! Real local filesystem witness qualification; synthetic keys and checkpoints.
#![cfg(any(unix, windows))]
#[path = "support/child_lifetime.rs"]
mod child_lifetime;
use ed25519_dalek::SigningKey;
use humanymous_audit::{
    checkpoint::{Checkpoint, DurableWitness, Witness, WitnessBootstrap},
    keys::custody::Owner,
    merkle::Tree,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
const SEED: [u8; 32] = [22; 32];
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-witness-{}-{}",
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
            let sd: LocalBox<SecurityDescriptor> =
                format!("D:P(A;OICI;FA;;;{sid})").parse().unwrap();
            wrappers::SetNamedSecurityInfo(
                path.as_os_str(),
                SeObjectType::SE_FILE_OBJECT,
                SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
                None,
                None,
                sd.dacl(),
                None,
            )
            .unwrap();
        }
        Self(fs::canonicalize(path).unwrap())
    }
    fn open(&self, bootstrap: WitnessBootstrap<'_>) -> std::io::Result<DurableWitness> {
        DurableWitness::open(
            &self.0,
            owner(&self.0),
            &SEED,
            writer().verifying_key().to_bytes(),
            "node".into(),
            bootstrap,
        )
    }
}

fn private_subdir(parent: &Path, name: &str) -> PathBuf {
    let path = parent.join(name);
    fs::create_dir(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    path
}

fn service_fixture(temp: &Temp) -> (PathBuf, PathBuf) {
    let keys = private_subdir(&temp.0, "keys");
    let state = private_subdir(&temp.0, "state");
    let mut custody =
        humanymous_audit::keys::custody::Custody::acquire(&keys, owner(&keys)).unwrap();
    custody
        .load_or_create(b"synthetic-witness-private-pass", || {
            humanymous_audit::keys::KeyMaterial::new(SEED, vec![44; 32], None, None)
        })
        .unwrap();
    (keys, state)
}

fn service_config(
    keys: &Path,
    state: &Path,
    minimum: Option<Checkpoint>,
) -> humanymous_audit::checkpoint::service::WitnessServiceConfig {
    use humanymous_audit::checkpoint::service::{WitnessServiceBootstrap, WitnessServiceConfig};
    #[cfg(unix)]
    let owner_sid = None;
    #[cfg(windows)]
    let Owner::WindowsSid(sid) = owner(keys);
    #[cfg(windows)]
    let owner_sid = Some(sid);
    WitnessServiceConfig {
        key_directory: keys.to_owned(),
        state_directory: state.to_owned(),
        owner_sid,
        passphrase: zeroize::Zeroizing::new(b"synthetic-witness-private-pass".to_vec()),
        writer_public: writer().verifying_key().to_bytes(),
        node_id: "node".into(),
        bootstrap: match minimum {
            Some(minimum) => WitnessServiceBootstrap::Pinned {
                minimum,
                consistency_proof: Vec::new(),
            },
            None => WitnessServiceBootstrap::CreateNew,
        },
    }
}

struct WitnessProcess {
    child: child_lifetime::ChildLifetime,
    input: std::process::ChildStdin,
    output: std::io::BufReader<std::process::ChildStdout>,
}
impl WitnessProcess {
    fn spawn(config: &humanymous_audit::checkpoint::service::WitnessServiceConfig) -> Self {
        let (child, mut input, output) = child_lifetime::ChildLifetime::spawn(&mut Command::new(
            env!("CARGO_BIN_EXE_audit-witness"),
        ))
        .unwrap();
        let output = std::io::BufReader::new(output);
        humanymous_audit::checkpoint::service::write_frame(&mut input, config).unwrap();
        Self {
            child,
            input,
            output,
        }
    }
    fn response(&mut self) -> humanymous_audit::checkpoint::service::WitnessServiceResponse {
        let bytes = humanymous_audit::checkpoint::service::read_frame(&mut self.output)
            .unwrap()
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    fn request(
        &mut self,
        request: &humanymous_audit::checkpoint::service::WitnessServiceRequest,
    ) -> humanymous_audit::checkpoint::service::WitnessServiceResponse {
        humanymous_audit::checkpoint::service::write_frame(&mut self.input, request).unwrap();
        self.response()
    }
    fn stop(mut self) {
        use humanymous_audit::checkpoint::service::{
            WitnessServiceRequest, WitnessServiceResponse,
        };
        assert!(matches!(
            self.request(&WitnessServiceRequest::Shutdown),
            WitnessServiceResponse::Stopped
        ));
        assert!(self.child.wait().unwrap().success());
    }
}

#[test]
fn independent_service_uses_private_sealed_identity_and_pinned_restart() {
    use humanymous_audit::checkpoint::service::{WitnessServiceRequest, WitnessServiceResponse};
    let temp = Temp::new();
    let (keys, state) = service_fixture(&temp);
    let mut process = WitnessProcess::spawn(&service_config(&keys, &state, None));
    let public = match process.response() {
        WitnessServiceResponse::Ready {
            witness_public,
            checkpoint: None,
            ..
        } => witness_public,
        _ => panic!("expected authenticated empty service readiness"),
    };
    assert_ne!(public, writer().verifying_key().to_bytes());
    let leaves = leaves();
    let first = match process.request(&WitnessServiceRequest::CounterSign {
        checkpoint: checkpoint(&leaves[..1]),
        consistency_proof: Vec::new(),
    }) {
        WitnessServiceResponse::Signed { checkpoint, .. } => checkpoint,
        _ => panic!("first real witness service signing refused"),
    };
    assert!(first.verify_witness(&public));
    let old_bytes = fs::read(state.join("witness.json")).unwrap();
    let next = match process.request(&WitnessServiceRequest::CounterSign {
        checkpoint: checkpoint(&leaves[..2]),
        consistency_proof: Tree::new(&leaves[..2]).consistency_proof(1).unwrap(),
    }) {
        WitnessServiceResponse::Signed { checkpoint, .. } => checkpoint,
        _ => panic!("valid extension refused"),
    };
    assert!(next.verify_writer(&writer().verifying_key().to_bytes()));
    assert!(next.verify_witness(&public));
    assert!(matches!(
        process.request(&WitnessServiceRequest::CounterSign {
            checkpoint: first,
            consistency_proof: Vec::new()
        }),
        WitnessServiceResponse::Refused { .. }
    ));
    process.child.kill().unwrap();
    process.child.wait().unwrap();
    drop(process);
    let mut restarted = WitnessProcess::spawn(&service_config(&keys, &state, Some(next.clone())));
    assert!(
        matches!(restarted.response(), WitnessServiceResponse::Ready { checkpoint: Some(head), .. } if head == next)
    );
    assert!(
        matches!(restarted.request(&WitnessServiceRequest::State), WitnessServiceResponse::State { checkpoint: Some(head) } if head == next)
    );
    restarted.stop();
    // Full local signed-state rollback is detected using the externally retained
    // latest receipt, never by trusting the old local signature by itself.
    fs::write(state.join("witness.json"), old_bytes).unwrap();
    let mut rejected = WitnessProcess::spawn(&service_config(&keys, &state, Some(next)));
    assert!(
        humanymous_audit::checkpoint::service::read_frame(&mut rejected.output)
            .unwrap()
            .is_none()
    );
    assert!(!rejected.child.wait().unwrap().success());
}

#[test]
fn service_wrong_password_and_oversized_frame_fail_without_secret_echo() {
    use humanymous_audit::checkpoint::service::{MAX_FRAME_BYTES, WitnessServiceResponse};
    use std::io::{Read, Write};
    let temp = Temp::new();
    let (keys, state) = service_fixture(&temp);
    let mut config = service_config(&keys, &state, None);
    config.passphrase = zeroize::Zeroizing::new(b"wrong-secret-synthetic".to_vec());
    let mut rejected = WitnessProcess::spawn(&config);
    assert!(
        humanymous_audit::checkpoint::service::read_frame(&mut rejected.output)
            .unwrap()
            .is_none()
    );
    assert!(!rejected.child.wait().unwrap().success());
    let mut stderr = String::new();
    rejected
        .child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert_eq!(stderr.trim(), "witness service unavailable");
    assert!(!state.join("witness.json").exists());
    drop(rejected);
    let mut process = WitnessProcess::spawn(&service_config(&keys, &state, None));
    assert!(matches!(
        process.response(),
        WitnessServiceResponse::Ready { .. }
    ));
    process
        .input
        .write_all(&vec![b' '; MAX_FRAME_BYTES + 1])
        .unwrap();
    process.input.flush().unwrap();
    assert!(matches!(
        process.response(),
        WitnessServiceResponse::Refused { .. }
    ));
    assert!(!process.child.wait().unwrap().success());
    assert!(!state.join("witness.json").exists());
}

#[test]
fn owned_service_lifetime_reaps_stalled_handshake() {
    use std::{io::BufReader, time::Duration};
    let (mut child, _input, output) = child_lifetime::ChildLifetime::spawn_with_timeout(
        &mut Command::new(env!("CARGO_BIN_EXE_audit-witness")),
        Duration::from_millis(200),
    )
    .unwrap();
    // Keep stdin open without sending startup configuration. A synchronous
    // frame read would hang forever without the independent owned-child timer.
    let frame = humanymous_audit::checkpoint::service::read_frame(&mut BufReader::new(output));
    assert!(matches!(frame, Ok(None)));
    assert_eq!(
        child.wait().unwrap_err().kind(),
        std::io::ErrorKind::TimedOut
    );
}

#[test]
fn common_typed_client_uses_real_child_and_cached_authenticated_head() {
    use humanymous_audit::checkpoint::{CheckpointWitness, service::WitnessServiceClient};
    use std::io::BufReader;
    let temp = Temp::new();
    let (keys, state) = service_fixture(&temp);
    let (mut child, input, output) = child_lifetime::ChildLifetime::spawn(&mut Command::new(env!(
        "CARGO_BIN_EXE_audit-witness"
    )))
    .unwrap();
    let expected = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let mut client = WitnessServiceClient::connect(
        BufReader::new(output),
        input,
        &service_config(&keys, &state, None),
        expected,
    )
    .unwrap();
    assert_eq!(client.last_size().unwrap(), 0);
    let leaves = leaves();
    let first = client.counter_sign(&checkpoint(&leaves[..1]), &[]).unwrap();
    assert_eq!(client.last_size().unwrap(), 1);
    client.validate_local_head(&first.checkpoint).unwrap();
    let proof = Tree::new(&leaves[..2]).consistency_proof(1).unwrap();
    let next = client
        .counter_sign(&checkpoint(&leaves[..2]), &proof)
        .unwrap();
    assert!(next.checkpoint.verify_witness(&expected));
    assert_eq!(client.last_size().unwrap(), 2);
    assert!(client.validate_local_head(&first.checkpoint).is_err());
    drop(client); // Trusted broker retains child ownership and reaps it.
    assert!(child.wait().unwrap().success());
}

#[test]
fn typed_client_rejects_forged_or_mismatched_receipts_and_poisoned_cache() {
    use humanymous_audit::checkpoint::{
        CheckpointWitness,
        service::{WitnessPersistence, WitnessServiceClient, WitnessServiceResponse, write_frame},
    };
    use std::io::Cursor;
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let leaves = leaves();
    let requested = checkpoint(&leaves[..1]);
    let signer = Witness::from_seed(&SEED, writer().verifying_key().to_bytes(), "node".into());
    let mut mismatched = checkpoint(&leaves[..2]);
    mismatched.witness_sig = signer.counter_sign(&mismatched, &[]).unwrap();
    let mut forged = requested.clone();
    forged.witness_sig = "00".repeat(64);
    for bad in [forged, mismatched] {
        let mut frames = Vec::new();
        write_frame(
            &mut frames,
            &WitnessServiceResponse::Ready {
                witness_public: public,
                checkpoint: None,
                persistence_capability: WitnessPersistence::FileSyncedNamespaceUnconfirmed,
            },
        )
        .unwrap();
        write_frame(
            &mut frames,
            &WitnessServiceResponse::Signed {
                checkpoint: bad,
                persistence: WitnessPersistence::FileSyncedNamespaceUnconfirmed,
            },
        )
        .unwrap();
        let mut client = WitnessServiceClient::from_ready(
            Cursor::new(frames),
            Vec::new(),
            public,
            writer().verifying_key().to_bytes(),
            "node".into(),
            WitnessBootstrap::CreateNew,
        )
        .unwrap();
        assert!(client.counter_sign(&requested, &[]).is_err());
        assert!(client.last_size().is_err());
        assert!(client.validate_local_head(&requested).is_err());
        assert!(client.counter_sign(&requested, &[]).is_err());
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
        let sd = wrappers::GetNamedSecurityInfo(
            _path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner,
        )
        .unwrap();
        Owner::WindowsSid(sd.owner().unwrap().to_string())
    }
}
fn writer() -> SigningKey {
    SigningKey::from_bytes(&[11; 32])
}
fn leaves() -> Vec<Vec<u8>> {
    vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]
}
fn checkpoint(leaves: &[Vec<u8>]) -> Checkpoint {
    let mut checkpoint = Checkpoint {
        node_id: "node".into(),
        tree_size: leaves.len() as u64,
        root: hex::encode(Sha256::digest(leaves.last().unwrap())),
        merkle_root: hex::encode(Tree::new(leaves).root()),
        ..Default::default()
    };
    checkpoint.sign(&writer());
    checkpoint
}
fn pin(head: &Checkpoint) -> WitnessBootstrap<'_> {
    WitnessBootstrap::Pinned {
        minimum: head,
        consistency_proof: &[],
    }
}

#[test]
fn restart_pins_latest_head_and_refuses_disk_rollback_or_missing_state() {
    let temp = Temp::new();
    let leaves = leaves();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves[..1]), &[])
        .unwrap()
        .checkpoint;
    let old_bytes = fs::read(temp.0.join("witness.json")).unwrap();
    let next = witness
        .counter_sign(
            &checkpoint(&leaves[..2]),
            &Tree::new(&leaves[..2]).consistency_proof(1).unwrap(),
        )
        .unwrap()
        .checkpoint;
    let public = witness.public_key();
    assert!(next.verify_witness(&public));
    assert!(witness.counter_sign(&first, &[]).is_err());
    drop(witness);
    assert!(temp.open(WitnessBootstrap::CreateNew).is_err());
    let reopened = temp.open(pin(&next)).unwrap();
    assert_eq!(reopened.last_checkpoint().unwrap(), Some(&next));
    drop(reopened);
    // A signed old local snapshot is still a rollback against the external pin.
    fs::write(temp.0.join("witness.json"), old_bytes).unwrap();
    assert!(temp.open(pin(&next)).is_err());
    fs::remove_file(temp.0.join("witness.json")).unwrap();
    assert!(temp.open(pin(&next)).is_err());
}

#[test]
fn external_older_pin_requires_consistency_and_fork_never_advances_head() {
    let temp = Temp::new();
    let leaves = leaves();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves[..1]), &[])
        .unwrap()
        .checkpoint;
    let mut forged = leaves.clone();
    forged[0] = b"fork".to_vec();
    assert!(
        witness
            .counter_sign(
                &checkpoint(&forged[..2]),
                &Tree::new(&forged[..2]).consistency_proof(1).unwrap()
            )
            .is_err()
    );
    assert_eq!(witness.last_checkpoint().unwrap(), Some(&first));
    let proof = Tree::new(&leaves[..2]).consistency_proof(1).unwrap();
    let next = witness
        .counter_sign(&checkpoint(&leaves[..2]), &proof)
        .unwrap()
        .checkpoint;
    drop(witness);
    assert!(temp.open(pin(&first)).is_err());
    let reopened = temp
        .open(WitnessBootstrap::Pinned {
            minimum: &first,
            consistency_proof: &proof,
        })
        .unwrap();
    assert_eq!(reopened.last_checkpoint().unwrap(), Some(&next));
}

#[test]
fn tampered_partial_and_wrong_key_state_never_reset_witness() {
    let temp = Temp::new();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves()[..1]), &[])
        .unwrap()
        .checkpoint;
    drop(witness);
    assert!(
        DurableWitness::open(
            &temp.0,
            owner(&temp.0),
            &[33; 32],
            writer().verifying_key().to_bytes(),
            "node".into(),
            pin(&first)
        )
        .is_err()
    );
    let mut tampered = first.clone();
    tampered.witness_sig = "00".repeat(64);
    fs::write(
        temp.0.join("witness.json"),
        serde_json::to_vec(&tampered).unwrap(),
    )
    .unwrap();
    assert!(temp.open(pin(&first)).is_err());
    fs::write(temp.0.join("witness.json"), b"{").unwrap();
    assert!(temp.open(pin(&first)).is_err());
    assert_eq!(fs::read(temp.0.join("witness.json")).unwrap(), b"{");
    assert!(
        DurableWitness::open(
            &temp.0,
            owner(&temp.0),
            &[11; 32],
            writer().verifying_key().to_bytes(),
            "node".into(),
            WitnessBootstrap::CreateNew
        )
        .is_err()
    );
}

#[test]
fn interrupted_state_is_never_auto_repaired_and_corrupt_pending_is_retained() {
    let temp = Temp::new();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves()[..1]), &[])
        .unwrap()
        .checkpoint;
    drop(witness);
    fs::copy(temp.0.join("witness.json"), temp.0.join("witness.pending")).unwrap();
    fs::write(temp.0.join("witness.pending"), b"{").unwrap();
    assert!(temp.open(pin(&first)).is_err());
    assert!(
        DurableWitness::recover_pending(
            &temp.0,
            owner(&temp.0),
            &SEED,
            writer().verifying_key().to_bytes(),
            "node".into(),
            pin(&first),
            &[]
        )
        .is_err()
    );
    assert_eq!(fs::read(temp.0.join("witness.pending")).unwrap(), b"{");
    let committed: Checkpoint =
        serde_json::from_slice(&fs::read(temp.0.join("witness.json")).unwrap()).unwrap();
    assert_eq!(committed, first);
}

#[test]
fn missing_committed_head_can_only_recover_against_external_anchor() {
    let temp = Temp::new();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves()[..1]), &[])
        .unwrap()
        .checkpoint;
    drop(witness);
    fs::rename(temp.0.join("witness.json"), temp.0.join("witness.pending")).unwrap();
    assert!(temp.open(pin(&first)).is_err());
    let (restored, receipt) = DurableWitness::recover_pending(
        &temp.0,
        owner(&temp.0),
        &SEED,
        writer().verifying_key().to_bytes(),
        "node".into(),
        pin(&first),
        &[],
    )
    .unwrap();
    assert_eq!(receipt.checkpoint, first);
    assert_eq!(restored.last_checkpoint().unwrap(), Some(&first));
}

#[test]
#[ignore = "subprocess helper explicitly exercised by witness_process_lifetime_and_interrupted_recovery"]
fn child_witness_probe() {
    let path = PathBuf::from(std::env::var_os("HUMANYMOUS_WITNESS_PROBE").unwrap());
    let mode = std::env::var("HUMANYMOUS_WITNESS_MODE").unwrap();
    let opened = DurableWitness::open(
        &path,
        owner(&path),
        &SEED,
        writer().verifying_key().to_bytes(),
        "node".into(),
        WitnessBootstrap::CreateNew,
    );
    if mode == "locked" {
        assert!(matches!(opened, Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        std::io::Write::write_all(&mut std::io::stdout(), b"AUDIT_WITNESS_LOCKED_OK\n").unwrap();
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    } else {
        assert_eq!(mode, "interrupted");
        let mut witness = opened.unwrap();
        witness
            .counter_sign(&checkpoint(&leaves()[..1]), &[])
            .unwrap();
        fs::copy(path.join("witness.json"), path.join("witness.pending")).unwrap();
        std::io::Write::write_all(
            &mut std::io::stdout(),
            b"AUDIT_WITNESS_INTERRUPTED_STAGED_OK\n",
        )
        .unwrap();
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        // Bypass Drop: OS releases the process lock; pending remains explicit.
        std::process::exit(0);
    }
}

#[test]
fn witness_process_lifetime_and_interrupted_recovery() {
    let temp = Temp::new();
    let witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let child = |mode| {
        humanymous_test_support::process::capture_with_stderr(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "child_witness_probe", "--ignored"])
                .env("HUMANYMOUS_WITNESS_PROBE", &temp.0)
                .env("HUMANYMOUS_WITNESS_MODE", mode),
            b"",
            std::time::Duration::from_secs(60),
            64 * 1024,
        )
        .unwrap()
    };
    let locked = child("locked");
    assert!(locked.status.success());
    assert!(locked.stderr.is_empty());
    assert!(
        locked
            .stdout
            .windows(b"AUDIT_WITNESS_LOCKED_OK\n".len())
            .any(|w| w == b"AUDIT_WITNESS_LOCKED_OK\n")
    );
    drop(witness);
    let interrupted = child("interrupted");
    assert!(interrupted.status.success());
    assert!(interrupted.stderr.is_empty());
    assert!(
        interrupted
            .stdout
            .windows(b"AUDIT_WITNESS_INTERRUPTED_STAGED_OK\n".len())
            .any(|w| w == b"AUDIT_WITNESS_INTERRUPTED_STAGED_OK\n")
    );
    let reference = Witness::from_seed(&SEED, writer().verifying_key().to_bytes(), "node".into());
    let mut first = checkpoint(&leaves()[..1]);
    first.witness_sig = reference.counter_sign(&first, &[]).unwrap();
    assert!(temp.open(pin(&first)).is_err());
    let (restored, receipt) = DurableWitness::recover_pending(
        &temp.0,
        owner(&temp.0),
        &SEED,
        writer().verifying_key().to_bytes(),
        "node".into(),
        pin(&first),
        &[],
    )
    .unwrap();
    assert_eq!(receipt.checkpoint, first);
    assert_eq!(restored.last_checkpoint().unwrap(), Some(&first));
    assert!(!temp.0.join("witness.pending").exists());
}

#[cfg(windows)]
#[test]
fn sharing_fault_returns_no_receipt_and_requires_authenticated_recovery() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = Temp::new();
    let leaves = leaves();
    let mut witness = temp.open(WitnessBootstrap::CreateNew).unwrap();
    let first = witness
        .counter_sign(&checkpoint(&leaves[..1]), &[])
        .unwrap()
        .checkpoint;
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(temp.0.join("witness.json"))
        .unwrap();
    let proof = Tree::new(&leaves[..2]).consistency_proof(1).unwrap();
    assert!(
        witness
            .counter_sign(&checkpoint(&leaves[..2]), &proof)
            .is_err()
    );
    assert!(witness.last_checkpoint().is_err());
    drop(blocker);
    drop(witness);
    assert!(temp.open(pin(&first)).is_err());
    let (recovered, receipt) = DurableWitness::recover_pending(
        &temp.0,
        owner(&temp.0),
        &SEED,
        writer().verifying_key().to_bytes(),
        "node".into(),
        pin(&first),
        &proof,
    )
    .unwrap();
    assert_eq!(receipt.checkpoint.tree_size, 2);
    assert_eq!(recovered.last_checkpoint().unwrap().unwrap().tree_size, 2);
}

#[cfg(unix)]
#[test]
fn unix_permissions_and_aliases_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = Temp::new();
    let alias = temp.0.with_extension("alias");
    symlink(&temp.0, &alias).unwrap();
    assert!(
        DurableWitness::open(
            &alias,
            Owner::CurrentUser,
            &SEED,
            writer().verifying_key().to_bytes(),
            "node".into(),
            WitnessBootstrap::CreateNew
        )
        .is_err()
    );
    fs::remove_file(alias).unwrap();
    fs::set_permissions(&temp.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(temp.open(WitnessBootstrap::CreateNew).is_err());
}

#[cfg(windows)]
#[test]
fn windows_reparse_and_foreign_owner_are_refused() {
    let temp = Temp::new();
    let alias = temp.0.join("alias");
    junction::create(&temp.0, &alias).unwrap();
    assert!(
        DurableWitness::open(
            &alias,
            owner(&temp.0),
            &SEED,
            writer().verifying_key().to_bytes(),
            "node".into(),
            WitnessBootstrap::CreateNew
        )
        .is_err()
    );
    junction::delete(&alias).unwrap();
    assert!(
        DurableWitness::open(
            &temp.0,
            Owner::WindowsSid("S-1-1-0".into()),
            &SEED,
            writer().verifying_key().to_bytes(),
            "node".into(),
            WitnessBootstrap::CreateNew
        )
        .is_err()
    );
}
