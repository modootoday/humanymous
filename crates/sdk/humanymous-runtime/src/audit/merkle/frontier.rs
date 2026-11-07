//! Fixed-size RFC6962 append frontier; no record/leaf history is retained.
use super::{Hash, leaf_hash, node_hash};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::audit) struct Frontier {
    count: u64,
    peaks: [Option<Hash>; 64],
}
#[derive(Debug, PartialEq, Eq)]
pub(in crate::audit) struct Full;

impl Frontier {
    pub fn new() -> Self {
        Self {
            count: 0,
            peaks: [None; 64],
        }
    }
    pub fn count(&self) -> u64 {
        self.count
    }
    pub fn append(&mut self, leaf: &[u8]) -> Result<(), Full> {
        let next = self.count.checked_add(1).ok_or(Full)?;
        let mut node = leaf_hash(leaf);
        let mut occupied = self.count;
        let mut height = 0;
        while occupied & 1 == 1 {
            let left = self.peaks[height]
                .take()
                .expect("frontier count matches occupied peaks");
            node = node_hash(&left, &node);
            occupied >>= 1;
            height += 1;
        }
        self.peaks[height] = Some(node);
        self.count = next;
        Ok(())
    }
    pub fn root(&self) -> Hash {
        let mut right = None;
        for left in self.peaks.iter().flatten() {
            right = Some(match right {
                Some(right) => node_hash(left, &right),
                None => *left,
            });
        }
        right.unwrap_or_else(|| Sha256::digest([]).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::merkle::Tree;
    #[test]
    fn every_prefix_matches_unchanged_recursive_roots_and_large_boundaries() {
        let leaves = (0..4097)
            .map(|n| format!("record-{n}").into_bytes())
            .collect::<Vec<_>>();
        let mut reference = Tree::new(&leaves);
        let mut frontier = Frontier::new();
        assert_eq!(frontier.root(), reference.root_at(0).unwrap());
        for (index, leaf) in leaves.iter().enumerate() {
            frontier.append(leaf).unwrap();
            assert_eq!(frontier.count(), index as u64 + 1);
            if !(257..4094).contains(&index) {
                assert_eq!(frontier.root(), reference.root_at(index + 1).unwrap());
            }
        }
        assert!(std::mem::size_of::<Frontier>() <= 4096);
    }
    #[test]
    fn overflow_refuses_before_any_peak_or_count_mutation() {
        let mut frontier = Frontier {
            count: u64::MAX,
            peaks: [Some([7; 32]); 64],
        };
        let before = frontier.clone();
        assert_eq!(frontier.append(b"cannot append"), Err(Full));
        assert_eq!(frontier, before);
    }
    #[test]
    fn text_leaf_bytes_are_not_silently_decoded_as_hex() {
        let mut text = Frontier::new();
        text.append("00".repeat(32).as_bytes()).unwrap();
        let mut binary = Frontier::new();
        binary.append(&[0; 32]).unwrap();
        assert_ne!(text.root(), binary.root());
        assert_eq!(text.root(), leaf_hash("00".repeat(32).as_bytes()));
    }
}
