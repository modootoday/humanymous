//! Go `internal/audit/chain.go` at 981d25c: the chain a `Log` seals from the
//! same inputs and keys, and the verifier's result on that chain and on
//! tampered copies of it (`tests/fixtures/audit/chain-go-golden.json`).
use humanymous_platform::testing::assert_json_equivalent;
use humanymous_runtime::audit::{
    checkpoint::{Checkpoint, Witness},
    log::{Config, Log},
    record::Record,
    storage::{Wal, WalOptions},
    verification::{verify, verify_with_witness},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Deserialize)]
struct Golden {
    chains: Vec<Chain>,
    verification: Vec<VerifyCase>,
}

#[derive(Deserialize)]
struct Chain {
    name: String,
    node_id: String,
    hmac_key: String,
    signing_seed: String,
    witness_seed: String,
    checkpoint_every: u64,
    force_checkpoint: bool,
    inputs: Vec<Record>,
    sealed: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    public: String,
    witness_public: String,
    self_verify: Value,
}

#[derive(Deserialize)]
struct VerifyCase {
    name: String,
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    key: String,
    public: String,
    witness: String,
    expected: Value,
}

fn golden() -> Golden {
    serde_json::from_str(include_str!("../fixtures/audit/chain-go-golden.json")).unwrap()
}

fn key32(text: &str) -> [u8; 32] {
    hex::decode(text).unwrap().try_into().unwrap()
}

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Private scratch directory for one WAL, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-chain-parity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
    }
}

fn report(
    records: &[Record],
    checkpoints: &[Checkpoint],
    key: &[u8],
    public: &[u8; 32],
    witness: Option<&[u8; 32]>,
) -> Value {
    let report = match witness {
        None => verify(records, checkpoints, key, Some(public)),
        Some(witness) => verify_with_witness(records, checkpoints, key, Some(public), witness),
    };
    serde_json::to_value(report).unwrap()
}

/// Go `NewLog` + `Append` + `Checkpoint`: from the same pre-stamped inputs,
/// node id, HMAC key, writer seed, cadence and witness seed, the WAL-backed
/// Rust `Log` seals byte-identical records (seq, node id, prev_hash,
/// record_hash, HMAC, defaulted event version and empty rule and signal
/// lists) and identical signed heads (Merkle root, chained root, previous
/// head, writer and witness signatures), and `SelfVerify` agrees.
///
/// Go also ran without a WAL (ephemeral memory chain); Rust always opens over
/// the authenticated WAL (owner decision 2026-11-16, DIFF), which does not
/// change a sealed byte.
#[test]
fn sealed_records_and_signed_heads_match_go() {
    let chains = golden().chains;
    assert_eq!(chains.len(), 3);
    for chain in chains {
        let scratch = Scratch::new();
        let seed = key32(&chain.signing_seed);
        let writer = ed25519_dalek::SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes();
        assert_eq!(hex::encode(writer), chain.public, "{}", chain.name);
        let witness = (!chain.witness_seed.is_empty()).then(|| {
            Witness::from_seed(&key32(&chain.witness_seed), writer, chain.node_id.clone())
        });
        if let Some(witness) = &witness {
            assert_eq!(hex::encode(witness.public_key()), chain.witness_public);
        }
        let key = hex::decode(&chain.hmac_key).unwrap();
        let mut log = Log::open(
            Wal::open(&scratch.0, WalOptions::default()).unwrap(),
            Config {
                node_id: chain.node_id.clone(),
                hmac_key: key.clone(),
                signing_seed: seed,
                checkpoint_every: chain.checkpoint_every,
                witness,
            },
            None,
            true,
        )
        .unwrap();
        for (input, expected) in chain.inputs.into_iter().zip(&chain.sealed) {
            assert_eq!(&log.append(input).unwrap(), expected, "{}", chain.name);
        }
        if chain.force_checkpoint {
            log.checkpoint().unwrap();
        }
        let records = log.records().unwrap();
        let checkpoints = log.checkpoints().unwrap();
        assert_eq!(records, chain.sealed, "{}", chain.name);
        assert_eq!(checkpoints, chain.checkpoints, "{}", chain.name);
        let witness_public =
            (!chain.witness_public.is_empty()).then(|| key32(&chain.witness_public));
        assert_json_equivalent(
            &report(
                &records,
                &checkpoints,
                &key,
                &writer,
                witness_public.as_ref(),
            ),
            &chain.self_verify,
        );
    }
}

/// Go `Verify`/`SelfVerify` on the sealed chain and on tampered copies of it:
/// altered content, a forged HMAC, a wrong HMAC key, a deleted, relinked or
/// reordered record, truncation with and without its heads, a dropped,
/// swapped or rewritten head, a wrong writer key, a rebuilt history under the
/// old heads, an empty chain, and missing, replaced or wrongly keyed witness
/// signatures. Each yields Go's class, sequence and detail.
#[test]
fn tamper_detection_matches_go() {
    let cases = golden().verification;
    assert_eq!(cases.len(), 20);
    for case in cases {
        let key = hex::decode(&case.key).unwrap();
        let witness = (!case.witness.is_empty()).then(|| key32(&case.witness));
        let actual = report(
            &case.records,
            &case.checkpoints,
            &key,
            &key32(&case.public),
            witness.as_ref(),
        );
        assert_eq!(
            actual["Class"], case.expected["Class"],
            "{}: {actual} != {}",
            case.name, case.expected
        );
        assert_json_equivalent(&actual, &case.expected);
    }
}
