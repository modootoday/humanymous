//! Persist-before-reply witness state, anchored by an independently retained head.
//! Separate process/key protection remains a deployment responsibility.
use super::{Checkpoint, Witness};
use crate::audit::{
    keys::custody::{Durability, Owner},
    merkle::Hash,
    storage::PrivateState,
};
use std::{io, path::Path};
pub mod service;

const MAX_CHECKPOINT_BYTES: usize = 64 * 1024;

/// Explicit provisioning or an externally authenticated restart anchor.
/// A local checkpoint alone cannot detect restoration of the entire directory.
pub enum WitnessBootstrap<'a> {
    /// Explicit first provisioning; refuses any committed or pending state.
    /// Never select this automatically after losing a previously provisioned head.
    CreateNew,
    /// Minimum previously accepted checkpoint retained outside this directory.
    Pinned {
        /// Authenticated writer and witness checkpoint supplied by the supervisor.
        minimum: &'a Checkpoint,
        /// Append-only proof from minimum to the committed checkpoint, empty for
        /// an identical head. For recovery without committed state this is empty.
        consistency_proof: &'a [Hash],
    },
}

/// Successful file replacement and its exact native durability strength.
/// Windows namespace power-loss durability is deliberately not implied.
pub struct WitnessReceipt {
    /// Writer and witness authenticated checkpoint now present in local storage.
    pub checkpoint: Checkpoint,
    /// Native persistence result; consumers must apply their required strength.
    pub durability: Durability,
}

/// A co-signer boundary that can be implemented by a separately supervised IPC
/// client. The log consumer needs no witness seed and must verify the returned
/// signature and durability policy before releasing a protected operation.
pub trait CheckpointWitness: Send {
    /// Provisioned verification identity, independent of the audit writer key.
    fn public_key(&self) -> [u8; 32];
    /// Latest independently accepted size; unavailable state is an error.
    fn last_size(&self) -> io::Result<u64>;
    /// Refuse a local head that is absent from or differs from the independently
    /// persisted latest commitment; this must never restore witness state back.
    fn validate_local_head(&self, checkpoint: &Checkpoint) -> io::Result<()>;
    /// Persist an append-only co-signed commitment before returning its receipt.
    fn counter_sign(
        &mut self,
        checkpoint: &Checkpoint,
        proof: &[Hash],
    ) -> io::Result<WitnessReceipt>;
}

/// A separately provisioned witness key and lifetime-locked latest-head store.
/// No seed is written to the head store; no signature escapes before commit.
/// Local disk protection plus external pinning does not establish process/HSM
/// independence, host compromise resistance or remote-filesystem semantics.
pub struct DurableWitness {
    witness: Witness,
    state: PrivateState,
    head: Option<Checkpoint>,
    poisoned: bool,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl DurableWitness {
    /// Acquire a preprovisioned private directory and authenticate restart state.
    /// Existing state requires an external minimum; stale, partial, pending,
    /// unowned or unauthenticated state fails closed without replacing identity.
    pub fn open(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        writer: [u8; 32],
        node: String,
        bootstrap: WitnessBootstrap<'_>,
    ) -> io::Result<Self> {
        let witness = provision(seed, writer, node)?;
        let state = PrivateState::open(directory, owner, "witness")?;
        let committed = state.read()?.map(|bytes| decode(&bytes)).transpose()?;
        restore(&witness, committed.as_ref(), bootstrap, false)?;
        Ok(Self {
            witness,
            state,
            head: committed,
            poisoned: false,
        })
    }

    /// Public verification identity; no signing authority or seed is exposed.
    pub fn public_key(&self) -> [u8; 32] {
        self.witness.public_key()
    }

    /// Latest locally committed checkpoint, suitable for independent pin storage.
    /// An external pin must itself be durably authenticated outside this store.
    pub fn last_checkpoint(&self) -> io::Result<Option<&Checkpoint>> {
        self.healthy()?;
        Ok(self.head.as_ref())
    }

    /// Verify monotonic append-only extension using the existing Witness engine,
    /// then sync and replace local state before returning the co-signature.
    /// Persistence errors poison this instance; do not retry a logical action.
    pub fn counter_sign(
        &mut self,
        checkpoint: &Checkpoint,
        proof: &[Hash],
    ) -> io::Result<WitnessReceipt> {
        self.healthy()?;
        bound(checkpoint, proof)?;
        let mut checkpoint = checkpoint.clone();
        checkpoint.witness_sig = self
            .witness
            .counter_sign(&checkpoint, proof)
            .map_err(io::Error::other)?;
        // Once the Witness advances, every subsequent error must poison this
        // wrapper so no in-memory-only accepted head becomes usable.
        self.poisoned = true;
        let bytes = encode(&checkpoint)?;
        let durability = self.state.commit(&bytes)?;
        self.head = Some(checkpoint.clone());
        self.poisoned = false;
        Ok(WitnessReceipt {
            checkpoint,
            durability,
        })
    }

    /// Explicitly authenticate and promote an interrupted transaction. Old state
    /// is anchored first; pending must prove an append-only extension. The method
    /// retains the lifetime writer lock in the returned object. Invalid pending
    /// bytes are never discarded or converted into an empty witness.
    pub fn recover_pending(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        writer: [u8; 32],
        node: String,
        bootstrap: WitnessBootstrap<'_>,
        pending_proof: &[Hash],
    ) -> io::Result<(Self, WitnessReceipt)> {
        let witness = provision(seed, writer, node)?;
        let mut state = PrivateState::open(directory, owner, "witness")?;
        let committed = state.committed()?.map(|bytes| decode(&bytes)).transpose()?;
        restore(&witness, committed.as_ref(), bootstrap, true)?;
        let pending = state
            .pending()?
            .ok_or_else(|| invalid("no pending witness transaction"))?;
        let pending = decode(&pending)?;
        authenticate_extension(&witness, &pending, pending_proof)?;
        let durability = state.promote_pending()?;
        Ok((
            Self {
                witness,
                state,
                head: Some(pending.clone()),
                poisoned: false,
            },
            WitnessReceipt {
                checkpoint: pending,
                durability,
            },
        ))
    }

    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            Err(invalid(
                "witness requires authenticated restart after failed commit",
            ))
        } else {
            Ok(())
        }
    }
}

impl CheckpointWitness for DurableWitness {
    fn public_key(&self) -> [u8; 32] {
        DurableWitness::public_key(self)
    }
    fn last_size(&self) -> io::Result<u64> {
        Ok(self.last_checkpoint()?.map_or(0, |head| head.tree_size))
    }
    fn validate_local_head(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        if self.last_checkpoint()? == Some(checkpoint) {
            Ok(())
        } else {
            Err(invalid(
                "local checkpoint differs from persisted witness head",
            ))
        }
    }
    fn counter_sign(
        &mut self,
        checkpoint: &Checkpoint,
        proof: &[Hash],
    ) -> io::Result<WitnessReceipt> {
        DurableWitness::counter_sign(self, checkpoint, proof)
    }
}

fn provision(seed: &[u8; 32], writer: [u8; 32], node: String) -> io::Result<Witness> {
    if node.is_empty() || node.len() > 4096 {
        return Err(invalid("invalid witness node identity"));
    }
    let witness = Witness::from_seed(seed, writer, node);
    if witness.public_key() == writer {
        return Err(invalid(
            "writer key cannot also be the independent witness key",
        ));
    }
    Ok(witness)
}

fn restore(
    witness: &Witness,
    committed: Option<&Checkpoint>,
    bootstrap: WitnessBootstrap<'_>,
    recovering: bool,
) -> io::Result<()> {
    match bootstrap {
        WitnessBootstrap::CreateNew => {
            if committed.is_some() {
                return Err(invalid("existing witness requires an external pinned head"));
            }
        }
        WitnessBootstrap::Pinned {
            minimum,
            consistency_proof,
        } => {
            bound(minimum, consistency_proof)?;
            witness.restore(minimum).map_err(io::Error::other)?;
            if let Some(committed) = committed {
                authenticate_extension(witness, committed, consistency_proof)?;
            } else if !recovering || !consistency_proof.is_empty() {
                return Err(invalid("pinned witness head absent from committed storage"));
            }
        }
    }
    Ok(())
}

fn authenticate_extension(
    witness: &Witness,
    checkpoint: &Checkpoint,
    proof: &[Hash],
) -> io::Result<()> {
    bound(checkpoint, proof)?;
    if !checkpoint.verify_witness(&witness.public_key()) {
        return Err(invalid("persisted witness signature invalid"));
    }
    witness
        .counter_sign(checkpoint, proof)
        .map_err(io::Error::other)?;
    Ok(())
}

fn bound(checkpoint: &Checkpoint, proof: &[Hash]) -> io::Result<()> {
    if proof.len() > 128 {
        return Err(invalid("witness proof size limit"));
    }
    encode(checkpoint).map(|_| ())
}
fn encode(checkpoint: &Checkpoint) -> io::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(checkpoint).map_err(io::Error::other)?;
    if bytes.len() > MAX_CHECKPOINT_BYTES {
        return Err(invalid("witness checkpoint size limit"));
    }
    Ok(bytes)
}
fn decode(bytes: &[u8]) -> io::Result<Checkpoint> {
    if bytes.len() > MAX_CHECKPOINT_BYTES {
        return Err(invalid("witness checkpoint size limit"));
    }
    serde_json::from_slice(bytes).map_err(|_| invalid("invalid persisted witness checkpoint"))
}
