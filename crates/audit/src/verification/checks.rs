//! Shared mismatch rules for compatibility and bounded incremental consumers.
use super::{Mismatch, Report, Scope};
use crate::{
    checkpoint::Checkpoint,
    merkle::Hash,
    record::{Record, record_hash_bounded, verify_record_mac},
};
use std::io;
use subtle::ConstantTimeEq;

pub(super) struct Failure {
    class: Mismatch,
    at: u64,
    detail: String,
}
impl Failure {
    pub fn report(self, scope: Scope) -> Report {
        Report::failure(self.class, self.at, self.detail, scope)
    }
    pub fn witness(at: u64) -> Self {
        Self {
            class: Mismatch::WitnessInvalid,
            at,
            detail: "witness co-signature missing or invalid".into(),
        }
    }
}

pub(super) fn record(
    record: &Record,
    expected: u64,
    previous: &str,
    hmac_key: &[u8],
    bytes: usize,
    items: usize,
) -> io::Result<Option<Failure>> {
    let failure = |class, detail| {
        Some(Failure {
            class,
            at: record.seq,
            detail,
        })
    };
    if record.seq != expected {
        return Ok(failure(
            Mismatch::SequenceGap,
            format!("expected seq {expected}"),
        ));
    }
    if record.prev_hash != previous {
        return Ok(failure(
            Mismatch::LinkageBreak,
            "prev_hash does not match chain head".into(),
        ));
    }
    let wanted = record_hash_bounded(record, previous, bytes, items)?;
    if !bool::from(wanted.as_bytes().ct_eq(record.record_hash.as_bytes())) {
        return Ok(failure(
            Mismatch::HashBreak,
            "record content was altered".into(),
        ));
    }
    if !hmac_key.is_empty() && !verify_record_mac(&record.record_hash, &record.hmac, hmac_key) {
        return Ok(failure(Mismatch::HmacInvalid, "hmac invalid".into()));
    }
    Ok(None)
}

pub(super) fn checkpoint(
    checkpoint: &Checkpoint,
    previous_signature: &str,
    linear: Option<&str>,
    merkle: impl FnOnce() -> Option<Hash>,
    public: Option<&[u8; 32]>,
    signing_bytes: usize,
) -> io::Result<Option<Failure>> {
    let failure = |detail: &str| {
        Some(Failure {
            class: Mismatch::CheckpointMismatch,
            at: checkpoint.tree_size,
            detail: detail.into(),
        })
    };
    let valid = match public {
        Some(key) => checkpoint.verify_signature_bounded(key, &checkpoint.sig, signing_bytes)?,
        None => false,
    };
    if !valid {
        return Ok(failure("checkpoint signature invalid"));
    }
    if checkpoint.prev_checkpoint_hash != previous_signature {
        return Ok(failure("checkpoint chain broken"));
    }
    if linear != Some(checkpoint.root.as_str()) {
        return Ok(failure(
            "checkpoint root does not match records (truncation/rollback)",
        ));
    }
    if !checkpoint.merkle_root.is_empty() {
        let Some(root) = merkle() else {
            return Ok(failure(
                "checkpoint merkle root: fewer records than tree size",
            ));
        };
        if hex::encode(root) != checkpoint.merkle_root {
            return Ok(failure(
                "checkpoint merkle root does not match records (STH committed a different leaf set)",
            ));
        }
    }
    Ok(None)
}
