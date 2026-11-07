//! Offline checks expose their verification scope; unanchored hashes alone are
//! not independent authentication of the writer's history.
use crate::audit::{checkpoint::Checkpoint, merkle::Tree, record::Record};
use serde::Serialize;
mod checks;
// S1 diagnostic accumulation remains private and test-compiled until the
// read-only scanner owns complete EOF, resource and source-identity checks.
#[cfg(test)]
mod incremental;
#[cfg(test)]
mod scan;

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

/// Successful-prefix coverage of the compatibility verifier. Interleaved scan
/// work must be counted separately, without changing this historical scope.
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

    fn success(scope: Scope) -> Self {
        Self {
            ok: true,
            class: if scope.hmac_checked {
                Mismatch::Ok
            } else {
                Mismatch::HmacUnchecked
            },
            at_seq: 0,
            detail: if scope.hmac_checked {
                String::new()
            } else {
                "hmac layer skipped (no key); hash chain and STH signatures verified".into()
            },
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
        if let Some(failure) =
            checks::record(record, expected, previous, hmac_key, usize::MAX, usize::MAX)
                .expect("compatibility canonical fields fit address space")
        {
            return failure.report(scope);
        }
        expected = expected.wrapping_add(1);
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
        let index = usize::try_from(checkpoint.tree_size).ok();
        let root = index.and_then(|size| {
            if size == 0 {
                Some("")
            } else {
                records.get(size - 1).map(|r| r.record_hash.as_str())
            }
        });
        if let Some(failure) = checks::checkpoint(
            checkpoint,
            previous_signature,
            root,
            || index.and_then(|size| tree.root_at(size)),
            public,
            usize::MAX,
        )
        .expect("compatibility checkpoint signing bytes fit address space")
        {
            return failure.report(scope);
        }
        previous_signature = &checkpoint.sig;
        scope.checkpoints += 1;
    }
    Report::success(scope)
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
            return checks::Failure::witness(checkpoint.tree_size).report(report.scope);
        }
        report.scope.witnessed_checkpoints += 1;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::record::{record_hash, record_mac};
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
