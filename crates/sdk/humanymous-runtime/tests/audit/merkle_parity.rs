//! Proof bytes and commitments against the actual frozen Go Merkle source.
use humanymous_runtime::audit::merkle::{Hash, Tree, verify_consistency, verify_inclusion};
use serde::Deserialize;

#[derive(Deserialize)]
struct Inclusion {
    index: usize,
    proof: Vec<String>,
}
#[derive(Deserialize)]
struct Consistency {
    old: usize,
    root: String,
    proof: Vec<String>,
}
#[derive(Deserialize)]
struct Case {
    leaves: Vec<String>,
    root: String,
    inclusion: Vec<Inclusion>,
    consistency: Vec<Consistency>,
}
fn hash(text: &str) -> Hash {
    hex::decode(text).unwrap().try_into().unwrap()
}
fn path(texts: &[String]) -> Vec<Hash> {
    texts.iter().map(|text| hash(text)).collect()
}

#[test]
fn roots_and_every_inclusion_and_consistency_path_match_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/audit/merkle.json")).unwrap();
    assert_eq!(cases.len(), 66);
    for case in cases {
        let leaves = case
            .leaves
            .iter()
            .map(|text| hex::decode(text).unwrap())
            .collect::<Vec<_>>();
        let mut tree = Tree::new(&leaves);
        let root = hash(&case.root);
        assert_eq!(tree.root(), root);
        for entry in case.inclusion {
            let proof = path(&entry.proof);
            assert_eq!(tree.inclusion_proof(entry.index).unwrap(), proof);
            assert!(verify_inclusion(
                &leaves[entry.index],
                entry.index,
                leaves.len(),
                &proof,
                &root
            ));
        }
        for entry in case.consistency {
            let proof = path(&entry.proof);
            assert_eq!(tree.consistency_proof(entry.old).unwrap(), proof);
            assert!(verify_consistency(
                entry.old,
                leaves.len(),
                &proof,
                &hash(&entry.root),
                &root
            ));
        }
    }
}
