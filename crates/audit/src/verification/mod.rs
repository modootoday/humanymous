//! Offline checks expose their verification scope; unanchored hashes alone are
//! not independent authentication of the writer's history.
use crate::{
    checkpoint::Checkpoint,
    merkle::Tree,
    record::{Record, record_hash, verify_record_mac},
};
use serde::Serialize;
use subtle::ConstantTimeEq;

/// Stable mismatch classes returned by the historical offline verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Mismatch {
    /// All requested checks passed.
    #[serde(rename = "ok")]
    Ok,
    /// Canonical content hash changed.
    #[serde(rename = "hash-break")]
    HashBreak,
    /// Secondary authentication failed.
    #[serde(rename = "hmac-invalid")]
    HmacInvalid,
    /// Per-node sequence is discontinuous.
    #[serde(rename = "seq-gap")]
    SequenceGap,
    /// Previous-hash linkage changed.
    #[serde(rename = "linkage-break")]
    LinkageBreak,
    /// Signed checkpoint or its commitments disagree.
    #[serde(rename = "checkpoint-mismatch")]
    CheckpointMismatch,
    /// Required witness signature missing/invalid.
    #[serde(rename = "witness-invalid")]
    WitnessInvalid,
    /// Empty history never passes verification.
    #[serde(rename = "empty-chain")]
    EmptyChain,
    /// Public-key-only verification omitted the secondary HMAC layer.
    #[serde(rename = "hmac-unchecked")]
    HmacUnchecked,
    /// An expected cluster node is absent.
    #[serde(rename = "node-missing")]
    NodeMissing,
}

/// Actual work performed, kept separately from the compatibility wire report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scope {
    /// Number of content/linkage checked records.
    pub records: usize,
    /// Secondary HMAC was required and checked for those records.
    pub hmac_checked: bool,
    /// Valid writer checkpoints checked against the actual records.
    pub checkpoints: usize,
    /// Checkpoints with an independently supplied witness verification key.
    pub witnessed_checkpoints: usize,
}

/// First mismatch, with the historical JSON fields plus explicit native scope.
/// ok means requested checks passed; inspect scope before claiming anchoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Whether the requested checks passed.
    #[serde(rename = "OK")]
    pub ok: bool,
    /// Mismatch or intentionally unchecked layer.
    #[serde(rename = "Class")]
    pub class: Mismatch,
    /// First divergent sequence/tree size.
    #[serde(rename = "AtSeq")]
    pub at_seq: u64,
    /// Stable diagnostic without raw record contents.
    #[serde(rename = "Detail")]
    pub detail: String,
    /// Inspectable verification coverage, not part of the old wire report.
    #[serde(skip)]
    pub scope: Scope,
}
impl Report {
    fn failure(class: Mismatch, at_seq: u64, detail: impl Into<String>, scope: Scope) -> Self {
        Self {
            ok: false,
            class,
            at_seq,
            detail: detail.into(),
            scope,
        }
    }
}

/// Verify content, sequence, linkage, optional HMAC and every supplied writer
/// checkpoint. A missing public key fails if a checkpoint needs checking.
/// Callers needing independent anchoring must require/check checkpoint scope;
/// no supplied checkpoints is not proof of an external anchor.
pub fn verify(
    records: &[Record],
    checkpoints: &[Checkpoint],
    hmac_key: &[u8],
    public: Option<&[u8; 32]>,
) -> Report {
    let mut scope = Scope {
        hmac_checked: !hmac_key.is_empty(),
        ..Default::default()
    };
    if records.is_empty() {
        return Report::failure(Mismatch::EmptyChain, 0, "empty chain is not a pass", scope);
    }
    let mut previous = "";
    let mut expected = 1_u64;
    for record in records {
        if record.seq != expected {
            return Report::failure(
                Mismatch::SequenceGap,
                record.seq,
                format!("expected seq {expected}"),
                scope,
            );
        }
        expected = expected.wrapping_add(1);
        if record.prev_hash != previous {
            return Report::failure(
                Mismatch::LinkageBreak,
                record.seq,
                "prev_hash does not match chain head",
                scope,
            );
        }
        let wanted = record_hash(record, previous);
        if !bool::from(wanted.as_bytes().ct_eq(record.record_hash.as_bytes())) {
            return Report::failure(
                Mismatch::HashBreak,
                record.seq,
                "record content was altered",
                scope,
            );
        }
        if scope.hmac_checked && !verify_record_mac(&record.record_hash, &record.hmac, hmac_key) {
            return Report::failure(Mismatch::HmacInvalid, record.seq, "hmac invalid", scope);
        }
        previous = &record.record_hash;
        scope.records += 1;
    }
    let leaves = records
        .iter()
        .map(|r| r.record_hash.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let mut tree = Tree::new(&leaves);
    let mut previous_signature = "";
    for checkpoint in checkpoints {
        let bad = |detail| {
            Report::failure(
                Mismatch::CheckpointMismatch,
                checkpoint.tree_size,
                detail,
                scope,
            )
        };
        if !public.is_some_and(|key| checkpoint.verify_writer(key)) {
            return bad("checkpoint signature invalid");
        }
        if checkpoint.prev_checkpoint_hash != previous_signature {
            return bad("checkpoint chain broken");
        }
        let index = usize::try_from(checkpoint.tree_size).ok();
        let root = index.and_then(|size| {
            if size == 0 {
                Some("")
            } else {
                records.get(size - 1).map(|r| r.record_hash.as_str())
            }
        });
        if root != Some(checkpoint.root.as_str()) {
            return bad("checkpoint root does not match records (truncation/rollback)");
        }
        if !checkpoint.merkle_root.is_empty() {
            let Some(root) = index.and_then(|size| tree.root_at(size)) else {
                return bad("checkpoint merkle root: fewer records than tree size");
            };
            if hex::encode(root) != checkpoint.merkle_root {
                return bad(
                    "checkpoint merkle root does not match records (STH committed a different leaf set)",
                );
            }
        }
        previous_signature = &checkpoint.sig;
        scope.checkpoints += 1;
    }
    if scope.hmac_checked {
        Report {
            ok: true,
            class: Mismatch::Ok,
            at_seq: 0,
            detail: String::new(),
            scope,
        }
    } else {
        Report {
            ok: true,
            class: Mismatch::HmacUnchecked,
            at_seq: 0,
            detail: "hmac layer skipped (no key); hash chain and STH signatures verified".into(),
            scope,
        }
    }
}

/// Require all supplied witness signatures after the base verification succeeds.
/// Scope still distinguishes an empty checkpoint list from a witnessed anchor.
pub fn verify_with_witness(
    records: &[Record],
    checkpoints: &[Checkpoint],
    hmac_key: &[u8],
    public: Option<&[u8; 32]>,
    witness_public: &[u8; 32],
) -> Report {
    let mut report = verify(records, checkpoints, hmac_key, public);
    if !report.ok {
        return report;
    }
    for checkpoint in checkpoints {
        if !checkpoint.verify_witness(witness_public) {
            return Report::failure(
                Mismatch::WitnessInvalid,
                checkpoint.tree_size,
                "witness co-signature missing or invalid",
                report.scope,
            );
        }
        report.scope.witnessed_checkpoints += 1;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::record_mac;
    fn sealed() -> Record {
        let mut r = Record {
            seq: 1,
            node_id: "node".into(),
            ..Default::default()
        };
        r.record_hash = record_hash(&r, "");
        r.hmac = record_mac(&r.record_hash, b"key").unwrap();
        r
    }
    #[test]
    fn absence_is_not_anchoring_and_error_priority_is_preserved() {
        assert_eq!(verify(&[], &[], b"key", None).class, Mismatch::EmptyChain);
        let mut record = sealed();
        let report = verify(std::slice::from_ref(&record), &[], b"key", None);
        assert!(report.ok);
        assert_eq!(report.scope.checkpoints, 0);
        assert_eq!(report.scope.records, 1);
        assert_eq!(
            verify(std::slice::from_ref(&record), &[], b"", None).class,
            Mismatch::HmacUnchecked
        );
        record.seq = 2;
        record.prev_hash = "bad".into();
        assert_eq!(
            verify(&[record], &[], b"key", None).class,
            Mismatch::SequenceGap
        );
    }
}
