//! A node-bound independent key holder preserves a monotonic witnessed head.
use super::Checkpoint;
use crate::audit::merkle::{Hash, verify_consistency};
use ed25519_dalek::{Signer, SigningKey};
use std::sync::Mutex;

/// Witness refusal never mutates the last accepted head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessError {
    /// Invalid size/hash framing.
    InvalidCheckpoint,
    /// The configured node differs from the checkpoint owner.
    WrongNode,
    /// Writer signature cannot authenticate this checkpoint.
    InvalidWriter,
    /// A smaller tree cannot replace the witnessed head, even if seen before.
    Regress,
    /// Either commitment changed at the already-witnessed size.
    Rewrite,
    /// The new tree does not consistency-extend the previous witnessed root.
    Fork,
    /// Restored state lacks a valid signature from this witness.
    InvalidRestore,
    /// A failed thread poisoned the state lock; do not silently reset it.
    Poisoned,
}
impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCheckpoint => "witness: invalid checkpoint framing",
            Self::WrongNode => "witness: wrong node",
            Self::InvalidWriter => "witness: writer signature invalid",
            Self::Regress => "witness: tree size regressed (rollback)",
            Self::Rewrite => {
                "witness: commitment changed at an already-witnessed tree size (rewrite)"
            }
            Self::Fork => "witness: new tree is not an append-only extension (fork/rewrite)",
            Self::InvalidRestore => "witness: restore signature invalid",
            Self::Poisoned => "witness: state unavailable",
        })
    }
}
impl std::error::Error for WitnessError {}

#[derive(Clone)]
struct Head {
    size: u64,
    root: String,
    merkle: Hash,
}

/// Separate signing-key authority for one node/writer. The reference stores its
/// key in this object; actual process/HSM independence is a deployment property.
/// Durable callers must persist/restore the accepted signed head before serving.
pub struct Witness {
    key: SigningKey,
    writer: [u8; 32],
    node: String,
    head: Mutex<Option<Head>>,
}
impl Witness {
    /// Reconstruct a stable witness identity from an explicitly valid seed and
    /// provisioned writer/node. No malformed-seed fallback changes key identity.
    pub fn from_seed(seed: &[u8; 32], writer: [u8; 32], node: String) -> Self {
        Self {
            key: SigningKey::from_bytes(seed),
            writer,
            node,
            head: Mutex::new(None),
        }
    }
    /// Public verification key, with no signing authority.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    /// Last accepted tree size, or zero before a first trusted checkpoint.
    pub fn last_size(&self) -> Result<u64, WitnessError> {
        let head = self.head.lock().map_err(|_| WitnessError::Poisoned)?;
        Ok(head.as_ref().map(|head| head.size).unwrap_or(0))
    }

    fn validate(&self, checkpoint: &Checkpoint) -> Result<Head, WitnessError> {
        if checkpoint.node_id != self.node {
            return Err(WitnessError::WrongNode);
        }
        if !checkpoint.verify_writer(&self.writer) {
            return Err(WitnessError::InvalidWriter);
        }
        let canonical_hash = |text: &str| -> Option<Hash> {
            if text.len() != 64
                || text
                    .bytes()
                    .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
            {
                return None;
            }
            hex::decode(text).ok()?.try_into().ok()
        };
        if checkpoint.tree_size == 0 || canonical_hash(&checkpoint.root).is_none() {
            return Err(WitnessError::InvalidCheckpoint);
        }
        let merkle =
            canonical_hash(&checkpoint.merkle_root).ok_or(WitnessError::InvalidCheckpoint)?;
        Ok(Head {
            size: checkpoint.tree_size,
            root: checkpoint.root.clone(),
            merkle,
        })
    }

    /// Restore an authenticated previously co-signed head. Boot recovery cannot
    /// reset the state to an unsigned or smaller commitment.
    pub fn restore(&self, checkpoint: &Checkpoint) -> Result<(), WitnessError> {
        let restored = self.validate(checkpoint)?;
        if !checkpoint.verify_witness(&self.public_key()) {
            return Err(WitnessError::InvalidRestore);
        }
        let mut state = self.head.lock().map_err(|_| WitnessError::Poisoned)?;
        if let Some(previous) = state.as_ref() {
            check_same_or_newer(previous, &restored)?;
        }
        *state = Some(restored);
        Ok(())
    }

    /// Counter-sign only authenticated same-root repetitions or append-only
    /// extensions. Reject missing Merkle roots and old-head replay; a refusal
    /// cannot weaken the next consistency requirement.
    pub fn counter_sign(
        &self,
        checkpoint: &Checkpoint,
        proof: &[Hash],
    ) -> Result<String, WitnessError> {
        let next = self.validate(checkpoint)?;
        let mut state = self.head.lock().map_err(|_| WitnessError::Poisoned)?;
        if let Some(previous) = state.as_ref() {
            check_same_or_newer(previous, &next)?;
            if next.size > previous.size {
                let old =
                    usize::try_from(previous.size).map_err(|_| WitnessError::InvalidCheckpoint)?;
                let new =
                    usize::try_from(next.size).map_err(|_| WitnessError::InvalidCheckpoint)?;
                if !verify_consistency(old, new, proof, &previous.merkle, &next.merkle) {
                    return Err(WitnessError::Fork);
                }
            } else if !proof.is_empty() {
                return Err(WitnessError::InvalidCheckpoint);
            }
        } else if !proof.is_empty() {
            return Err(WitnessError::InvalidCheckpoint);
        }
        let signature = hex::encode(self.key.sign(&checkpoint.signing_bytes()).to_bytes());
        *state = Some(next);
        Ok(signature)
    }
}
fn check_same_or_newer(previous: &Head, next: &Head) -> Result<(), WitnessError> {
    if next.size < previous.size {
        return Err(WitnessError::Regress);
    }
    if next.size == previous.size && (next.root != previous.root || next.merkle != previous.merkle)
    {
        return Err(WitnessError::Rewrite);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::merkle::Tree;
    fn checkpoint(leaves: &[Vec<u8>], writer: &SigningKey) -> Checkpoint {
        let mut checkpoint = Checkpoint {
            node_id: "node".into(),
            tree_size: leaves.len() as u64,
            root: hex::encode(sha2::Sha256::digest(leaves.last().unwrap())),
            merkle_root: hex::encode(Tree::new(leaves).root()),
            ..Default::default()
        };
        checkpoint.sign(writer);
        checkpoint
    }
    use sha2::Digest;

    #[test]
    fn rollback_same_size_merkle_fork_and_restart_cannot_weaken_the_head() {
        let writer = SigningKey::from_bytes(&[1; 32]);
        let seed = [2; 32];
        let witness = Witness::from_seed(&seed, writer.verifying_key().to_bytes(), "node".into());
        let leaves = [
            b"one".to_vec(),
            b"two".to_vec(),
            b"three".to_vec(),
            b"four".to_vec(),
        ];
        let mut first = checkpoint(&leaves[..2], &writer);
        first.witness_sig = witness.counter_sign(&first, &[]).unwrap();
        let mut next = checkpoint(&leaves[..3], &writer);
        let proof = Tree::new(&leaves[..3]).consistency_proof(2).unwrap();
        next.witness_sig = witness.counter_sign(&next, &proof).unwrap();
        assert!(next.verify_witness(&witness.public_key()));
        assert_eq!(
            witness.counter_sign(&first, &[]),
            Err(WitnessError::Regress)
        );
        assert_eq!(witness.last_size().unwrap(), 3);
        let mut changed = next.clone();
        changed.merkle_root = "00".repeat(32);
        changed.sign(&writer);
        assert_eq!(
            witness.counter_sign(&changed, &[]),
            Err(WitnessError::Rewrite)
        );
        changed = next.clone();
        changed.merkle_root.clear();
        changed.sign(&writer);
        assert_eq!(
            witness.counter_sign(&changed, &[]),
            Err(WitnessError::InvalidCheckpoint)
        );
        let mut forged = leaves.clone();
        forged[0] = b"rewrite".to_vec();
        let fork = checkpoint(&forged, &writer);
        assert_eq!(
            witness.counter_sign(&fork, &Tree::new(&forged).consistency_proof(3).unwrap()),
            Err(WitnessError::Fork)
        );
        let restored = Witness::from_seed(&seed, writer.verifying_key().to_bytes(), "node".into());
        restored.restore(&next).unwrap();
        assert_eq!(
            restored.counter_sign(&first, &[]),
            Err(WitnessError::Regress)
        );
        let mut unsigned = next;
        unsigned.witness_sig.clear();
        assert_eq!(
            restored.restore(&unsigned),
            Err(WitnessError::InvalidRestore)
        );
    }
}
