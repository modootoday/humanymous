//! Authenticated local durable export. No remote/object-store delivery is implied.
use crate::{
    checkpoint::Checkpoint,
    keys::custody::{Durability, Owner},
    record::Record,
    storage::{PrivateState, RecoveryPolicy, verify_history},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io, path::Path};
use zeroize::Zeroizing;
/// Provisioned sink identities, obtained independently of exported data.
pub struct SinkPolicy {
    /// Exact owning node.
    pub node: String,
    /// Record authentication secret; moved into a zeroized owned buffer at open.
    pub hmac_key: Vec<u8>,
    /// Pinned writer verification key.
    pub writer: [u8; 32],
    /// Required independent witness when configured.
    pub witness: Option<[u8; 32]>,
}
/// Explicit new destination or exact independently retained latest receipt.
pub enum SinkBootstrap<'a> {
    /// Only when destination state is absent; never automatic reset.
    New,
    /// External latest checkpoint expected on every existing-state reopen.
    Pinned(&'a Checkpoint),
}
/// Exact delivered commitment, distinct from network/service acknowledgement.
pub struct SinkReceipt {
    /// Complete writer/witness authenticated latest checkpoint.
    pub checkpoint: Checkpoint,
    /// SHA256 over exact persisted JSON bytes, without raw record output.
    pub digest: String,
    /// Native filesystem strength of persistence.
    pub durability: Durability,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
}
/// Single-writer bounded export destination. Inputs must already be pseudonymized;
/// Record and this sink do not sanitize raw personal data or provide WORM media.
pub struct DurableSink {
    store: PrivateState,
    node: String,
    hmac: Zeroizing<Vec<u8>>,
    writer: [u8; 32],
    witness: Option<[u8; 32]>,
    head: Option<Checkpoint>,
}
fn invalid() -> io::Error {
    io::Error::other("sink authentication or progression failure")
}
impl DurableSink {
    /// Acquire a private destination and verify its entire persisted export.
    pub fn open(
        dir: &Path,
        owner: Owner,
        policy: SinkPolicy,
        bootstrap: SinkBootstrap<'_>,
    ) -> io::Result<Self> {
        if policy.node.is_empty()
            || policy.hmac_key.is_empty()
            || policy.witness == Some(policy.writer)
        {
            return Err(invalid());
        }
        let store = PrivateState::open(dir, owner, "sink")?;
        let mut sink = Self {
            store,
            node: policy.node,
            hmac: Zeroizing::new(policy.hmac_key),
            writer: policy.writer,
            witness: policy.witness,
            head: None,
        };
        match (sink.store.read()?, bootstrap) {
            (None, SinkBootstrap::New) => {}
            (Some(bytes), SinkBootstrap::Pinned(pin)) => {
                let export = sink.validate(&bytes, Some(pin))?;
                if export.checkpoints.last() != Some(pin) {
                    return Err(invalid());
                }
                sink.head = Some(pin.clone());
            }
            _ => return Err(invalid()),
        }
        Ok(sink)
    }
    fn validate(&self, bytes: &[u8], trusted: Option<&Checkpoint>) -> io::Result<Export> {
        let export: Export = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        let last = export.checkpoints.last().ok_or_else(invalid)?;
        if last.tree_size != export.records.len() as u64 {
            return Err(invalid());
        }
        verify_history(
            export.records.clone(),
            export.checkpoints.clone(),
            RecoveryPolicy {
                node_id: &self.node,
                hmac_key: &self.hmac,
                writer_public: &self.writer,
                witness_public: self.witness.as_ref(),
                trusted_head: trusted,
                allow_new: false,
            },
        )
        .map_err(|_| invalid())?;
        Ok(export)
    }
    /// Authenticate a fully checkpoint-covered history and extend the previous
    /// receipt before atomically storing it. Failures never report delivery.
    pub fn publish(
        &mut self,
        records: Vec<Record>,
        checkpoints: Vec<Checkpoint>,
    ) -> io::Result<SinkReceipt> {
        let bytes = serde_json::to_vec(&Export {
            records,
            checkpoints,
        })
        .map_err(|_| invalid())?;
        let export = self.validate(&bytes, self.head.as_ref())?;
        let checkpoint = export.checkpoints.last().ok_or_else(invalid)?.clone();
        let durability = self.store.commit(&bytes)?;
        self.head = Some(checkpoint.clone());
        Ok(SinkReceipt {
            checkpoint,
            digest: hex::encode(Sha256::digest(bytes)),
            durability,
        })
    }
    /// Current exact export anchor for independent persistence by the supervisor.
    pub fn head(&self) -> Option<&Checkpoint> {
        self.head.as_ref()
    }
    /// Authenticate and promote a staged export against the exact external
    /// committed pin. Partial bytes and missing old anchors are never repaired.
    pub fn recover_pending(
        dir: &Path,
        owner: Owner,
        policy: SinkPolicy,
        pin: &Checkpoint,
    ) -> io::Result<(Self, SinkReceipt)> {
        if policy.node.is_empty()
            || policy.hmac_key.is_empty()
            || policy.witness == Some(policy.writer)
        {
            return Err(invalid());
        }
        let store = PrivateState::open(dir, owner, "sink")?;
        let mut sink = Self {
            store,
            node: policy.node,
            hmac: Zeroizing::new(policy.hmac_key),
            writer: policy.writer,
            witness: policy.witness,
            head: Some(pin.clone()),
        };
        let old = sink.validate(&sink.store.committed()?.ok_or_else(invalid)?, Some(pin))?;
        if old.checkpoints.last() != Some(pin) {
            return Err(invalid());
        }
        let bytes = sink.store.pending()?.ok_or_else(invalid)?;
        let export = sink.validate(&bytes, Some(pin))?;
        let checkpoint = export.checkpoints.last().ok_or_else(invalid)?.clone();
        let durability = sink.store.promote_pending()?;
        sink.head = Some(checkpoint.clone());
        let receipt = SinkReceipt {
            checkpoint,
            digest: hex::encode(Sha256::digest(bytes)),
            durability,
        };
        Ok((sink, receipt))
    }
}
