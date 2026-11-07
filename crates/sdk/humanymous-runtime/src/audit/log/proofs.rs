use super::Log;
use crate::audit::{
    checkpoint::Checkpoint,
    merkle::{Hash, Tree, verify_inclusion},
};
use std::io;

/// RFC 6962 leaf-to-root proof over ASCII record-hash text, not decoded hash
/// bytes or record JSON. An unsigned root alone does not authenticate history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProof {
    /// Exact committed record-hash text bytes.
    pub leaf_data: Vec<u8>,
    /// Zero-based record position.
    pub leaf_index: usize,
    /// Number of leaves in the selected prefix.
    pub tree_size: usize,
    /// Leaf-to-root sibling commitments.
    pub path: Vec<Hash>,
}
impl InclusionProof {
    /// Verify the path against a caller-supplied root. Authentication of that
    /// root is a separate requirement; this does not inspect the record body.
    pub fn verify(&self, root: &Hash) -> bool {
        verify_inclusion(
            &self.leaf_data,
            self.leaf_index,
            self.tree_size,
            &self.path,
            root,
        )
    }
}

/// Inclusion bound to one actually persisted signed head. Offline verifiers
/// must provision trusted writer/witness keys separately from this bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointInclusion {
    /// Merkle path and exact leaf data.
    pub inclusion: InclusionProof,
    /// Signed head committing the selected tree prefix.
    pub checkpoint: Checkpoint,
}
impl CheckpointInclusion {
    /// Check signature, size and path. Some(witness) requires the co-signature;
    /// None authenticates only the writer and must not imply independence.
    pub fn verify(&self, writer: &[u8; 32], witness: Option<&[u8; 32]>) -> bool {
        if !self.checkpoint.verify_writer(writer)
            || witness.is_some_and(|key| !self.checkpoint.verify_witness(key))
            || usize::try_from(self.checkpoint.tree_size).ok() != Some(self.inclusion.tree_size)
        {
            return false;
        }
        let Ok(root) = hex::decode(&self.checkpoint.merkle_root) else {
            return false;
        };
        let Ok(root): Result<Hash, _> = root.try_into() else {
            return false;
        };
        self.inclusion.verify(&root)
    }
}

impl Log {
    /// Current unsigned Merkle tip. Use a persisted signed checkpoint when
    /// authentication or independent anchoring is required.
    pub fn merkle_root(&self) -> io::Result<Hash> {
        self.healthy()?;
        Ok(Tree::new(&self.leaves).root())
    }

    /// Inclusion at the current, possibly not-yet-checkpointed tree tip.
    pub fn inclusion_proof(&self, sequence: u64) -> io::Result<Option<InclusionProof>> {
        self.inclusion_proof_at(sequence, self.sequence)
    }

    /// Inclusion at a checked prefix; None for zero/out-of-range sequence/size.
    /// This method alone does not require that the prefix was checkpointed.
    pub fn inclusion_proof_at(
        &self,
        sequence: u64,
        size: u64,
    ) -> io::Result<Option<InclusionProof>> {
        self.healthy()?;
        if sequence == 0 || sequence > size {
            return Ok(None);
        }
        let (Ok(index), Ok(size)) = (usize::try_from(sequence - 1), usize::try_from(size)) else {
            return Ok(None);
        };
        let Some(leaves) = self.leaves.get(..size) else {
            return Ok(None);
        };
        let Some(leaf) = leaves.get(index) else {
            return Ok(None);
        };
        Ok(Tree::new(leaves)
            .inclusion_proof(index)
            .map(|path| InclusionProof {
                leaf_data: leaf.clone(),
                leaf_index: index,
                tree_size: size,
                path,
            }))
    }

    /// Export inclusion only for an exact head in the persisted authenticated
    /// checkpoint chain. A same-size caller-supplied replacement is refused.
    pub fn checkpointed_inclusion(
        &self,
        sequence: u64,
        checkpoint: &Checkpoint,
    ) -> io::Result<Option<CheckpointInclusion>> {
        self.healthy()?;
        if !self.checkpoints.contains(checkpoint) {
            return Ok(None);
        }
        Ok(self
            .inclusion_proof_at(sequence, checkpoint.tree_size)?
            .map(|inclusion| CheckpointInclusion {
                inclusion,
                checkpoint: checkpoint.clone(),
            }))
    }

    /// Append-only proof from a nonempty strict prefix to the current tip.
    /// Authenticate old/new roots independently before trusting this proof.
    pub fn consistency_proof(&self, old_size: u64) -> io::Result<Option<Vec<Hash>>> {
        self.healthy()?;
        let Ok(size) = usize::try_from(old_size) else {
            return Ok(None);
        };
        Ok(Tree::new(&self.leaves).consistency_proof(size))
    }
}
