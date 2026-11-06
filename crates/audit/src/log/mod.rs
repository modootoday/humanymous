//! Durable per-node sealing. The WAL is the authority; projections are not.
mod stamp;
use crate::{
    checkpoint::{Checkpoint, Witness},
    merkle::Tree,
    record::{Record, record_hash, record_mac},
    storage::{RecoveryPolicy, Wal},
};
use ed25519_dalek::SigningKey;
use std::io;

/// Stable provisioned identities. No malformed or absent key silently creates
/// an ephemeral replacement identity on restart.
pub struct Config {
    /// Exact node identifier.
    pub node_id: String,
    /// Secondary authentication secret.
    pub hmac_key: Vec<u8>,
    /// Stable writer seed; process/HSM separation remains a deployment concern.
    pub signing_seed: [u8; 32],
    /// Nonzero number of records between durable signed heads.
    pub checkpoint_every: u64,
    /// Required co-signer when present. This object does not by itself provide
    /// independent process isolation or rollback-protected head persistence.
    pub witness: Option<Witness>,
}

/// Single-owner chain state restored only after authenticated WAL recovery.
/// A configured co-signer is mandatory on every emitted checkpoint. Independent
/// key/process/head persistence must still be established by the deployment.
pub struct Log {
    config: Config,
    signer: SigningKey,
    wal: Wal,
    sequence: u64,
    head: String,
    leaves: Vec<Vec<u8>>,
    checkpoints: Vec<Checkpoint>,
    poisoned: bool,
}
impl Log {
    /// Restore an authenticated chain or explicitly initialize an empty one.
    /// A trusted head must originate outside the WAL to resist local rollback.
    pub fn open(
        wal: Wal,
        config: Config,
        trusted_head: Option<&Checkpoint>,
        allow_new: bool,
    ) -> io::Result<Self> {
        if config.checkpoint_every == 0 {
            return Err(io::Error::other("zero checkpoint cadence"));
        }
        let signer = SigningKey::from_bytes(&config.signing_seed);
        let public = signer.verifying_key().to_bytes();
        let witness_public = config.witness.as_ref().map(Witness::public_key);
        let history = wal.verified_history(RecoveryPolicy {
            node_id: &config.node_id,
            hmac_key: &config.hmac_key,
            writer_public: &public,
            witness_public: witness_public.as_ref(),
            trusted_head,
            allow_new,
        })?;
        let sequence = history.sequence();
        let head = history.head().to_owned();
        let leaves = history
            .records()
            .iter()
            .map(|r| r.record_hash.as_bytes().to_vec())
            .collect();
        let checkpoints = history.checkpoints().to_vec();
        if let Some(witness) = &config.witness {
            if let Some(last) = checkpoints.last() {
                witness.restore(last).map_err(io::Error::other)?;
            } else if witness.last_size().map_err(io::Error::other)? != 0 {
                return Err(io::Error::other(
                    "witness already has a head absent from local storage",
                ));
            }
        }
        Ok(Self {
            config,
            signer,
            wal,
            sequence,
            head,
            leaves,
            checkpoints,
            poisoned: false,
        })
    }

    /// Seal and durably append before advancing state or releasing an action.
    /// A checkpoint failure after record persistence poisons the writer and
    /// returns an error: callers must not blindly retry a logical operation.
    pub fn append(&mut self, mut record: Record) -> io::Result<Record> {
        self.healthy()?;
        if record.event_type.is_empty() {
            return Err(io::Error::other("missing audit event type"));
        }
        stamp::stamp(&mut record)?;
        record.seq = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("audit sequence exhausted"))?;
        record.node_id = self.config.node_id.clone();
        record.prev_hash = self.head.clone();
        record.triggered_rules.get_or_insert_default();
        record.top_signals.get_or_insert_default();
        record.record_hash = record_hash(&record, &self.head);
        record.hmac =
            record_mac(&record.record_hash, &self.config.hmac_key).map_err(io::Error::other)?;
        if let Err(error) = self.wal.append_record(&record) {
            self.poisoned = true;
            return Err(error);
        }
        self.sequence = record.seq;
        self.head = record.record_hash.clone();
        self.leaves.push(record.record_hash.as_bytes().to_vec());
        let last = self.checkpoints.last().map_or(0, |cp| cp.tree_size);
        if self.sequence - last >= self.config.checkpoint_every {
            self.checkpoint()?;
        }
        Ok(record)
    }

    /// Persist a head covering new records. Repeated calls reuse the existing
    /// head, avoiding duplicate-size checkpoints rejected by strict recovery.
    pub fn checkpoint(&mut self) -> io::Result<Option<Checkpoint>> {
        self.healthy()?;
        if self.sequence == 0 {
            return Ok(None);
        }
        if let Some(cp) = self
            .checkpoints
            .last()
            .filter(|cp| cp.tree_size == self.sequence)
        {
            return Ok(Some(cp.clone()));
        }
        let mut cp = Checkpoint {
            node_id: self.config.node_id.clone(),
            tree_size: self.sequence,
            root: self.head.clone(),
            merkle_root: hex::encode(Tree::new(&self.leaves).root()),
            prev_checkpoint_hash: self
                .checkpoints
                .last()
                .map_or_else(String::new, |cp| cp.sig.clone()),
            ..Default::default()
        };
        cp.sign(&self.signer);
        if let Some(witness) = &self.config.witness {
            let result = (|| {
                let last = witness.last_size().map_err(io::Error::other)?;
                let proof = if last > 0 && last < self.sequence {
                    let old = usize::try_from(last).map_err(io::Error::other)?;
                    Tree::new(&self.leaves)
                        .consistency_proof(old)
                        .ok_or_else(|| io::Error::other("witness consistency proof unavailable"))?
                } else {
                    Vec::new()
                };
                witness.counter_sign(&cp, &proof).map_err(io::Error::other)
            })();
            match result {
                Ok(signature) => cp.witness_sig = signature,
                Err(error) => {
                    self.poisoned = true;
                    return Err(error);
                }
            }
        }
        if let Err(error) = self.wal.append_checkpoint(&cp) {
            self.poisoned = true;
            return Err(error);
        }
        self.checkpoints.push(cp.clone());
        Ok(Some(cp))
    }

    /// Invoke a side effect only after the audit append and any due checkpoint
    /// succeed. This is ordering, not an atomic transaction with external state.
    pub fn emit_and_act<T>(
        &mut self,
        record: Record,
        action: impl FnOnce(&Record) -> T,
    ) -> io::Result<T> {
        let sealed = self.append(record)?;
        Ok(action(&sealed))
    }

    /// Full authenticated chain export from disk; no shadow in-memory copy.
    pub fn records(&self) -> io::Result<Vec<Record>> {
        self.healthy()?;
        self.wal.records()
    }
    /// Latest locally authenticated sequence.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            Err(io::Error::other(
                "audit writer poisoned; verified restart required",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::WalOptions;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "humanymous-log-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    fn config() -> Config {
        Config {
            node_id: "node".into(),
            hmac_key: b"secret".to_vec(),
            signing_seed: [7; 32],
            checkpoint_every: 1,
            witness: None,
        }
    }
    fn event() -> Record {
        Record {
            event_type: "gate.decision".into(),
            ..Default::default()
        }
    }
    fn clean(dir: &std::path::Path) {
        for e in fs::read_dir(dir).unwrap() {
            fs::remove_file(e.unwrap().path()).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn seals_checkpoints_restarts_and_detects_authenticated_tampering() {
        let dir = dir();
        let mut log = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            config(),
            None,
            true,
        )
        .unwrap();
        let first = log.append(event()).unwrap();
        assert_eq!(first.seq, 1);
        assert_eq!(first.event_id.as_bytes()[14], b'7');
        assert!(matches!(
            first.event_id.as_bytes()[19],
            b'8' | b'9' | b'a' | b'b'
        ));
        assert_eq!(first.event_version, "1.0.0");
        assert!(first.ts.ends_with('Z'));
        let head = log.checkpoint().unwrap().unwrap();
        assert_eq!(log.checkpoint().unwrap().unwrap(), head);
        drop(log);
        let mut log = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            config(),
            Some(&head),
            false,
        )
        .unwrap();
        let second = log
            .emit_and_act(event(), |r| (r.seq, r.prev_hash.clone()))
            .unwrap();
        assert_eq!(second, (2, first.record_hash));
        assert_eq!(log.records().unwrap().len(), 2);
        drop(log);
        let path = dir.join("audit-0000000001.log");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("gate.decision", "gate.modified");
        fs::write(path, text).unwrap();
        assert!(
            Log::open(
                Wal::open(&dir, WalOptions::default()).unwrap(),
                config(),
                None,
                false
            )
            .is_err()
        );
        clean(&dir);
    }
    #[test]
    fn checkpoint_disk_failure_prevents_action_and_poisons_writer() {
        let dir = dir();
        let mut log = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            config(),
            None,
            true,
        )
        .unwrap();
        let sidecar = dir.join("checkpoints.jsonl");
        fs::create_dir(&sidecar).unwrap();
        let acted = std::cell::Cell::new(false);
        assert!(log.emit_and_act(event(), |_| acted.set(true)).is_err());
        assert!(!acted.get());
        assert_eq!(log.sequence(), 1); // record was durable; logical retry is unsafe.
        fs::remove_dir(sidecar).unwrap();
        assert!(log.append(event()).is_err());
        drop(log);
        let mut restored = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            config(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(restored.sequence(), 1);
        restored.checkpoint().unwrap();
        drop(restored);
        clean(&dir);
    }

    fn witnessed_config(node: &str) -> Config {
        let mut config = config();
        let writer = SigningKey::from_bytes(&config.signing_seed)
            .verifying_key()
            .to_bytes();
        config.witness = Some(Witness::from_seed(&[9; 32], writer, node.into()));
        config
    }
    #[test]
    fn witnessed_heads_survive_restart_and_missing_signatures_block_boot() {
        let dir = dir();
        let mut log = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            witnessed_config("node"),
            None,
            true,
        )
        .unwrap();
        for _ in 0..3 {
            log.append(event()).unwrap();
        }
        let anchor = log.checkpoint().unwrap().unwrap();
        assert!(!anchor.witness_sig.is_empty());
        drop(log);
        let mut reopened = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            witnessed_config("node"),
            Some(&anchor),
            false,
        )
        .unwrap();
        reopened.append(event()).unwrap();
        let next = reopened.checkpoint().unwrap().unwrap();
        assert_eq!(next.tree_size, 4);
        assert!(next.verify_witness(&witnessed_config("node").witness.unwrap().public_key()));
        drop(reopened);
        let path = dir.join("checkpoints.jsonl");
        let lines = fs::read_to_string(&path).unwrap();
        let corrupted = lines
            .lines()
            .map(|line| {
                let mut cp: Checkpoint = serde_json::from_str(line).unwrap();
                cp.witness_sig.clear();
                serde_json::to_string(&cp).unwrap() + "\n"
            })
            .collect::<String>();
        fs::write(path, corrupted).unwrap();
        assert!(
            Log::open(
                Wal::open(&dir, WalOptions::default()).unwrap(),
                witnessed_config("node"),
                None,
                false
            )
            .is_err()
        );
        clean(&dir);
    }
    #[test]
    fn witness_refusal_is_not_silently_downgraded_to_writer_only() {
        let dir = dir();
        let mut log = Log::open(
            Wal::open(&dir, WalOptions::default()).unwrap(),
            witnessed_config("other-node"),
            None,
            true,
        )
        .unwrap();
        let acted = std::cell::Cell::new(false);
        assert!(log.emit_and_act(event(), |_| acted.set(true)).is_err());
        assert!(!acted.get());
        assert!(log.checkpoint().is_err());
        drop(log);
        clean(&dir);
    }
}
