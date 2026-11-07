//! RFC 6962 domain-separated Merkle commitments over audit record hashes.
use sha2::{Digest, Sha256};
use std::collections::HashMap;
mod frontier;
mod ranges;

/// SHA-256 commitment; fixed width rejects malformed proof nodes at decoding.
pub type Hash = [u8; 32];

fn leaf_hash(data: &[u8]) -> Hash {
    let mut digest = Sha256::new();
    digest.update([0]);
    digest.update(data);
    digest.finalize().into()
}
fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let mut digest = Sha256::new();
    digest.update([1]);
    digest.update(left);
    digest.update(right);
    digest.finalize().into()
}
fn split(size: usize) -> usize {
    debug_assert!(size >= 2);
    1 << (usize::BITS - 1 - (size - 1).leading_zeros())
}

/// Immutable leaf view with memoized subtree commitments for proof generation.
/// Audit leaves contain record-hash bytes, not record JSON or private contents.
pub struct Tree<'a> {
    leaves: &'a [Vec<u8>],
    memo: HashMap<(usize, usize), Hash>,
}
impl<'a> Tree<'a> {
    /// Borrow a stable leaf sequence; changing it requires a new tree view.
    pub fn new(leaves: &'a [Vec<u8>]) -> Self {
        Self {
            leaves,
            memo: HashMap::new(),
        }
    }

    /// Root of the complete view, including SHA256(empty) for no leaves.
    pub fn root(&mut self) -> Hash {
        let key = (0, self.leaves.len());
        if let Some(root) = self.memo.get(&key) {
            return *root;
        }
        let mut frontier = frontier::Frontier::new();
        for leaf in self.leaves {
            frontier
                .append(leaf)
                .expect("in-memory leaf count fits u64");
        }
        debug_assert_eq!(frontier.count(), self.leaves.len() as u64);
        let root = frontier.root();
        self.memo.insert(key, root);
        root
    }

    /// Root of a checked prefix using the same subtree cache; None for a size
    /// beyond the borrowed leaf view. Zero is the standard empty-tree root.
    pub fn root_at(&mut self, size: usize) -> Option<Hash> {
        (size <= self.leaves.len()).then(|| self.subtree(0, size))
    }

    /// Leaf-to-root inclusion path, or None for an out-of-range index.
    pub fn inclusion_proof(&mut self, index: usize) -> Option<Vec<Hash>> {
        if index >= self.leaves.len() {
            return None;
        }
        let plan = ranges::Plan::inclusion(
            u64::try_from(index).ok()?,
            u64::try_from(self.leaves.len()).ok()?,
        )?;
        let mut proof = vec![[0; 32]; plan.len()];
        for (index, range) in plan.visits() {
            proof[index] = self.subtree(
                usize::try_from(range.start).ok()?,
                usize::try_from(range.size).ok()?,
            );
        }
        Some(proof)
    }

    /// Append-only consistency path for a nonempty strict prefix.
    pub fn consistency_proof(&mut self, old_size: usize) -> Option<Vec<Hash>> {
        if old_size == 0 || old_size >= self.leaves.len() {
            return None;
        }
        let plan = ranges::Plan::consistency(
            u64::try_from(old_size).ok()?,
            u64::try_from(self.leaves.len()).ok()?,
        )?;
        let mut proof = vec![[0; 32]; plan.len()];
        for (index, range) in plan.visits() {
            proof[index] = self.subtree(
                usize::try_from(range.start).ok()?,
                usize::try_from(range.size).ok()?,
            );
        }
        Some(proof)
    }

    fn subtree(&mut self, offset: usize, size: usize) -> Hash {
        if size == 0 {
            return Sha256::digest([]).into();
        }
        if size == 1 {
            return leaf_hash(&self.leaves[offset]);
        }
        if let Some(hash) = self.memo.get(&(offset, size)) {
            return *hash;
        }
        let k = split(size);
        let left = self.subtree(offset, k);
        let right = self.subtree(offset + k, size - k);
        let hash = node_hash(&left, &right);
        self.memo.insert((offset, size), hash);
        hash
    }
}

/// Verify an exact RFC 6962 inclusion path against a trusted tree root/size.
/// Extra path nodes are rejected even if a caller supplies their fabricated root.
pub fn verify_inclusion(
    leaf: &[u8],
    index: usize,
    size: usize,
    proof: &[Hash],
    root: &Hash,
) -> bool {
    if index >= size {
        return false;
    }
    let (mut at, mut length, mut depth) = (index, size, 0);
    while length > 1 {
        let k = split(length);
        if at < k {
            length = k;
        } else {
            at -= k;
            length -= k;
        }
        depth += 1;
    }
    if proof.len() != depth {
        return false;
    }
    let mut node = leaf_hash(leaf);
    let (mut position, mut last) = (index, size - 1);
    for hash in proof {
        if position == last || position % 2 == 1 {
            node = node_hash(hash, &node);
            while position % 2 == 0 && position != 0 {
                position >>= 1;
                last >>= 1;
            }
        } else {
            node = node_hash(&node, hash);
        }
        position >>= 1;
        last >>= 1;
    }
    last == 0 && node == *root
}

/// Verify that an authenticated old root is a prefix of an authenticated new
/// root. Empty old trees are outside this log's proof API, as in the Go contract.
pub fn verify_consistency(
    old: usize,
    new: usize,
    proof: &[Hash],
    old_root: &Hash,
    new_root: &Hash,
) -> bool {
    if old == 0 || old > new || proof.len() > 65 {
        return false;
    }
    if old == new {
        return proof.is_empty() && old_root == new_root;
    }
    let mut path = [[0; 32]; 66];
    let mut length = 0;
    if old.is_power_of_two() {
        path[0] = *old_root;
        length = 1;
    }
    path[length..length + proof.len()].copy_from_slice(proof);
    length += proof.len();
    let path = &path[..length];
    let Some(&first) = path.first() else {
        return false;
    };
    let (mut position, mut last) = (old - 1, new - 1);
    while position % 2 == 1 {
        position >>= 1;
        last >>= 1;
    }
    let (mut old_hash, mut new_hash) = (first, first);
    for hash in &path[1..] {
        if last == 0 {
            return false;
        }
        if position % 2 == 1 || position == last {
            old_hash = node_hash(hash, &old_hash);
            new_hash = node_hash(hash, &new_hash);
            while position % 2 == 0 && position != 0 {
                position >>= 1;
                last >>= 1;
            }
        } else {
            new_hash = node_hash(&new_hash, hash);
        }
        position >>= 1;
        last >>= 1;
    }
    last == 0 && old_hash == *old_root && new_hash == *new_root
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_leaf_and_prefix_verifies_across_unbalanced_trees() {
        for size in 1..=65 {
            let leaves = (0..size)
                .map(|n| format!("record-{n}").into_bytes())
                .collect::<Vec<_>>();
            let mut tree = Tree::new(&leaves);
            let root = tree.root();
            for (index, leaf) in leaves.iter().enumerate() {
                let proof = tree.inclusion_proof(index).unwrap();
                assert!(verify_inclusion(leaf, index, size, &proof, &root));
                assert!(!verify_inclusion(b"tampered", index, size, &proof, &root));
                let mut extra = proof.clone();
                extra.push([0; 32]);
                assert!(!verify_inclusion(leaf, index, size, &extra, &root));
            }
            for old in 1..size {
                let old_root = Tree::new(&leaves[..old]).root();
                let proof = tree.consistency_proof(old).unwrap();
                assert!(verify_consistency(old, size, &proof, &old_root, &root));
                assert!(!verify_consistency(old, size, &proof, &[0; 32], &root));
            }
            assert!(tree.inclusion_proof(size).is_none());
            assert!(tree.consistency_proof(0).is_none());
            assert!(tree.consistency_proof(size).is_none());
        }
    }
    #[test]
    fn empty_singleton_domain_separation_and_malformed_paths() {
        assert_eq!(Tree::new(&[]).root(), Sha256::digest([]).as_slice());
        let leaf = b"leaf";
        let root = leaf_hash(leaf);
        assert!(verify_inclusion(leaf, 0, 1, &[], &root));
        let sibling = [1; 32];
        let forged = node_hash(&sibling, &root);
        assert!(!verify_inclusion(leaf, 0, 1, &[sibling], &forged));
        assert!(!verify_inclusion(leaf, 0, 0, &[], &root));
        assert!(verify_consistency(1, 1, &[], &root, &root));
        assert!(!verify_consistency(1, 1, &[sibling], &root, &root));
        assert!(!verify_consistency(0, 1, &[], &root, &root));
        assert_ne!(leaf_hash(leaf), Sha256::digest(leaf).as_slice());
    }
}
