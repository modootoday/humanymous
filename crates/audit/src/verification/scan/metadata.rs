//! Complete framing/node/progression/pin pass before cryptographic diagnostics.
use super::{
    frames::{Budget, Frames},
    limits::{Control, Limits, invalid, resource},
    source::Source,
};
use crate::{checkpoint::Checkpoint, record::Record, storage::RecoveryPolicy};
use sha2::Digest;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Torn {
    pub segment: usize,
    pub complete: u64,
    pub bytes: usize,
}
pub(super) struct Metadata {
    pub segments: usize,
    pub records: usize,
    pub checkpoints: usize,
    pub record_bytes: u64,
    pub checkpoint_bytes: u64,
    pub record_digest: [u8; 32],
    pub checkpoint_digest: [u8; 32],
    pub has_sidecar: bool,
    pub torn: Option<Torn>,
    pub physical_latest: Option<Checkpoint>,
}

pub(super) fn inspect(
    source: &impl Source,
    policy: &RecoveryPolicy<'_>,
    limits: &Limits,
    control: &Control<'_>,
) -> io::Result<Metadata> {
    limits.validate()?;
    control.check()?;
    if policy.node_id.is_empty() || policy.hmac_key.is_empty() {
        return Err(invalid("missing audit recovery identity or HMAC key"));
    }
    if policy.node_id.len() > limits.node_bytes {
        return Err(resource("audit node identity limit exceeded"));
    }
    let segments = source.segment_count()?;
    if segments > limits.segments {
        return Err(resource("audit segment count exceeded"));
    }
    source.unchanged()?;
    let mut records = Budget::new(limits.record_bytes, limits.records);
    let mut bad_node = false;
    let mut torn = None;
    for index in 1..=segments {
        control.check()?;
        records.digest.update(b"audit-record-segment-v1\0");
        records.digest.update(
            u64::try_from(index)
                .map_err(|_| resource("audit segment counter overflow"))?
                .to_be_bytes(),
        );
        let mut frames = Frames::new(source.segment(index)?, limits.frame, index == segments)?;
        while let Some(record) = frames.next::<Record>(&mut records, limits, control)? {
            bad_node |= record.node_id != policy.node_id;
        }
        let end = frames.finish()?;
        records.digest.update(end.complete.to_be_bytes());
        records.digest.update(
            u64::try_from(end.torn)
                .map_err(|_| resource("audit torn byte overflow"))?
                .to_be_bytes(),
        );
        if end.torn != 0 {
            torn = Some(Torn {
                segment: index,
                complete: end.complete,
                bytes: end.torn,
            });
        }
    }
    let mut checkpoints = Budget::new(limits.checkpoint_bytes, limits.checkpoints);
    checkpoints.digest.update(b"audit-checkpoint-sidecar-v1\0");
    let sidecar = source.checkpoints()?;
    let has_sidecar = sidecar.is_some();
    checkpoints.digest.update([u8::from(has_sidecar)]);
    let mut bad_progression = false;
    let mut previous = 0;
    let mut trusted_present = false;
    let mut physical_latest = None;
    if let Some(reader) = sidecar {
        let mut frames = Frames::new(reader, limits.frame, false)?;
        while let Some(checkpoint) = frames.next::<Checkpoint>(&mut checkpoints, limits, control)? {
            bad_node |= checkpoint.node_id != policy.node_id;
            bad_progression |= checkpoint.tree_size <= previous
                || checkpoint.merkle_root.len() != 64
                || !checkpoint
                    .merkle_root
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            previous = checkpoint.tree_size;
            trusted_present |= policy.trusted_head.is_some_and(|head| head == &checkpoint);
            physical_latest = Some(checkpoint);
        }
        frames.finish()?;
    }
    control.check()?;
    source.unchanged()?;
    if source.segment_count()? != segments {
        return Err(invalid("audit source membership changed"));
    }
    control.check()?;
    if records.values == 0 {
        if !policy.allow_new || checkpoints.values != 0 || policy.trusted_head.is_some() {
            return Err(invalid("unexpected empty audit history"));
        }
    } else {
        if bad_node {
            return Err(invalid("audit history belongs to another node"));
        }
        if bad_progression {
            return Err(invalid(
                "invalid audit checkpoint progression or Merkle commitment",
            ));
        }
        if policy.trusted_head.is_some() && !trusted_present {
            return Err(invalid("trusted audit head missing: rollback or fork"));
        }
    }
    Ok(Metadata {
        segments,
        records: records.values,
        checkpoints: checkpoints.values,
        record_bytes: records.bytes,
        checkpoint_bytes: checkpoints.bytes,
        record_digest: records.digest.finalize().into(),
        checkpoint_digest: checkpoints.digest.finalize().into(),
        has_sidecar,
        torn,
        physical_latest,
    })
}
