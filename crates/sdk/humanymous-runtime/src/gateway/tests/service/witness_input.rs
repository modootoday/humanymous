//! Real OS pipes and the audit-owned witness service in a thread (C3 input).
//! Same-process placement is component evidence, not deployment independence.
use super::*;
use crate::audit::checkpoint::service::{
    WitnessServiceBootstrap, WitnessServiceConfig, serve, write_frame,
};
use crate::gateway::configuration::{WitnessInput, WitnessInputDurability};
use crate::gateway::startup::{inherited_witness, open_audit_with_witness};
use std::io::{PipeReader, PipeWriter};
use std::os::fd::AsRawFd;

const WITNESS_SEED: [u8; 32] = [8; 32];

fn public(seed: &[u8; 32]) -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(seed)
        .verifying_key()
        .to_bytes()
}

/// Supervisor role: provision witness custody, send its private first frame
/// and keep the runtime-side pipe ends as the "inherited" descriptors.
struct Supervised {
    runtime_read: PipeReader,
    runtime_write: PipeWriter,
    service: std::thread::JoinHandle<std::io::Result<()>>,
}
fn supervise(fixture: &PrivateFixture, node: &str) -> Supervised {
    let keys = fixture.0.join("witness").join("keys");
    let head = fixture.0.join("witness").join("head");
    for path in [&keys, &head] {
        std::fs::DirBuilder::new().mode(0o700).create(path).unwrap();
    }
    Custody::acquire(&keys, Owner::CurrentUser)
        .unwrap()
        .load_or_create(b"witness-fixture", || {
            KeyMaterial::new(WITNESS_SEED, vec![10; 32], None, None)
        })
        .unwrap();
    let (runtime_read, witness_write) = std::io::pipe().unwrap();
    let (witness_read, mut runtime_write) = std::io::pipe().unwrap();
    write_frame(
        &mut runtime_write,
        &WitnessServiceConfig {
            key_directory: keys,
            state_directory: head,
            owner_sid: None,
            passphrase: zeroize::Zeroizing::new(b"witness-fixture".to_vec()),
            writer_public: public(&[7; 32]),
            node_id: node.into(),
            bootstrap: WitnessServiceBootstrap::CreateNew,
        },
    )
    .unwrap();
    let service =
        std::thread::spawn(move || serve(std::io::BufReader::new(witness_read), witness_write));
    Supervised {
        runtime_read,
        runtime_write,
        service,
    }
}
fn input(read_fd: i32, write_fd: i32) -> WitnessInput {
    WitnessInput {
        read_fd: read_fd as u32,
        write_fd: write_fd as u32,
        witness_public: public(&WITNESS_SEED),
        writer_public: public(&[7; 32]),
        durability: WitnessInputDurability::FileAndDirectory,
        consistency_proof: Vec::new(),
    }
}
fn audit_is_empty(fixture: &PrivateFixture) -> bool {
    std::fs::read_dir(fixture.0.join("audit"))
        .unwrap()
        .next()
        .is_none()
}

#[test]
fn inherited_pipe_witness_pins_identity_and_cosigns_durable_audit() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.audit.require_external_witness = true;
    let supervised = supervise(&fixture, &config.audit.node_id);
    config.audit.witness_input = Some(input(
        supervised.runtime_read.as_raw_fd(),
        supervised.runtime_write.as_raw_fd(),
    ));
    assert!(config.validate().is_ok());
    let witness = inherited_witness(&config).unwrap().unwrap();
    assert_eq!(witness.durability_name(), "file_and_directory");
    let mut audit = open_audit_with_witness(&mut config, witness).unwrap();
    audit
        .append(Record {
            event_type: "runtime.decision".into(),
            verdict: "CHALLENGE".into(),
            ..Default::default()
        })
        .unwrap();
    let witnessed = audit.witnessed_through().unwrap().unwrap();
    assert!(witnessed >= 1);
    assert_eq!(witnessed, audit.sequence());
    // The witness persisted the co-signed head in its own state directory.
    assert!(
        std::fs::read_dir(fixture.0.join("witness").join("head"))
            .unwrap()
            .next()
            .is_some()
    );
    // Closing every runtime-side write end is a clean end of stream.
    drop(audit);
    drop(supervised.runtime_write);
    assert!(supervised.service.join().unwrap().is_ok());
}

#[test]
fn inherited_witness_refusals_happen_before_custody_or_unlock() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.audit.require_external_witness = true;

    // A witness key different from the pinned one refuses the handshake.
    let supervised = supervise(&fixture, &config.audit.node_id);
    let mut wrong = input(
        supervised.runtime_read.as_raw_fd(),
        supervised.runtime_write.as_raw_fd(),
    );
    wrong.witness_public = [3; 32];
    config.audit.witness_input = Some(wrong);
    assert!(inherited_witness(&config).is_err());
    drop(supervised.runtime_write);
    let _ = supervised.service.join().unwrap();

    // A regular file, a closed number and a witness that exited before its
    // ready reply (end of stream) are refused.
    let (dead_read, dead_write) = std::io::pipe().unwrap();
    drop(dead_write);
    let (_sink_read, sink_write) = std::io::pipe().unwrap();
    let regular = fixture.0.join("settings").join("not-a-pipe");
    std::fs::write(&regular, b"").unwrap();
    let file = std::fs::File::open(&regular).unwrap();
    for (read_fd, write_fd) in [
        (file.as_raw_fd(), sink_write.as_raw_fd()),
        (dead_read.as_raw_fd(), 1_000_000),
        (dead_read.as_raw_fd(), sink_write.as_raw_fd()),
    ] {
        config.audit.witness_input = Some(input(read_fd, write_fd));
        assert!(inherited_witness(&config).is_err());
    }
    // Linux checks the inherited direction before reopening.
    #[cfg(target_os = "linux")]
    {
        let (read, write) = std::io::pipe().unwrap();
        config.audit.witness_input = Some(input(write.as_raw_fd(), read.as_raw_fd()));
        assert!(inherited_witness(&config).is_err());
    }
    // A configured input is invalid without the required-witness flag.
    config.audit.require_external_witness = false;
    config.audit.witness_input = Some(input(dead_read.as_raw_fd(), sink_write.as_raw_fd()));
    assert!(config.validate().is_err());
    assert!(inherited_witness(&config).is_err());

    // No refusal opened audit state or consumed the unlock credential.
    assert!(audit_is_empty(&fixture));
    assert_eq!(config.audit.keys.passphrase.as_str(), "unit-fixture");
}
