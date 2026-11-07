//! Second complete pass; diagnostics are published only after transcript agreement.
use super::{
    Context, Diagnostic,
    checkpoints::{Checkpoints, Prefix},
    frames::{Budget, Frames},
    limits::{invalid, resource},
    metadata::{Metadata, Torn},
    source::Source,
};
use crate::audit::{
    merkle::frontier::Frontier,
    record::Record,
    verification::{checks, incremental::Accumulator},
};
use std::io;

pub(super) fn finish(
    source: &impl Source,
    context: Context<'_, '_, '_>,
    mut metadata: Metadata,
) -> io::Result<Diagnostic> {
    context.control.check()?;
    source.unchanged()?;
    if source.segment_count()? != metadata.segments {
        return Err(invalid("audit source membership changed"));
    }
    context.control.check()?;
    // Exact physical pin presence was checked by Phase A. Its transcript/counts
    // below must match before returning anything; the caller's pin stays borrowed
    // in this same context. S1 returns no physical head or authority token.
    drop(metadata.physical_latest.take());
    let mut records = Budget::new(context.limits.record_bytes, context.limits.records);
    let mut checkpoints = Checkpoints::open(source, &context)?;
    let mut accumulator = Accumulator::new(!context.policy.hmac_key.is_empty());
    let mut frontier = Frontier::new();
    let mut previous_raw_hash = String::new();
    let mut torn = None;
    for index in 1..=metadata.segments {
        context.control.check()?;
        records.begin_segment(index)?;
        let mut frames = Frames::new(
            source.segment(index)?,
            context.limits.frame,
            index == metadata.segments,
        )?;
        while let Some(mut record) =
            frames.next::<Record>(&mut records, context.limits, context.control)?
        {
            let position = u64::try_from(records.values)
                .map_err(|_| resource("audit sequence counter overflow"))?;
            context.control.check()?;
            accumulator.record(checks::record(
                &record,
                position,
                &previous_raw_hash,
                context.policy.hmac_key,
                context.limits.canonical_bytes,
                context.limits.json.array_items,
            )?)?;
            frontier
                .append(record.record_hash.as_bytes())
                .map_err(|_| resource("audit Merkle frontier exhausted"))?;
            context.control.check()?;
            if let Some(size) = checkpoints
                .pending
                .as_ref()
                .map(|checkpoint| checkpoint.tree_size)
            {
                if size < position {
                    return Err(invalid("audit checkpoint position changed between passes"));
                }
                if size == position {
                    checkpoints.consume(
                        Prefix {
                            linear: Some(&record.record_hash),
                            merkle: Some(frontier.root()),
                        },
                        &context,
                        &mut accumulator,
                    )?;
                }
            }
            // Move, never truncate or copy the malformed raw hash. JSON/frame
            // limits bound this separate previous-link allocation too.
            previous_raw_hash = std::mem::take(&mut record.record_hash);
        }
        let end = frames.finish()?;
        records.end_segment(&end)?;
        if end.torn != 0 {
            torn = Some(Torn {
                segment: index,
                complete: end.complete,
                bytes: end.torn,
            });
        }
    }
    while checkpoints.pending.is_some() {
        checkpoints.consume(
            Prefix {
                linear: None,
                merkle: None,
            },
            &context,
            &mut accumulator,
        )?;
    }
    let (checkpoints, has_sidecar) = checkpoints.finish()?;
    context.control.check()?;
    if records.values != metadata.records
        || records.bytes != metadata.record_bytes
        || checkpoints.values != metadata.checkpoints
        || checkpoints.bytes != metadata.checkpoint_bytes
        || records.signature() != metadata.record_digest
        || checkpoints.signature() != metadata.checkpoint_digest
        || has_sidecar != metadata.has_sidecar
        || torn != metadata.torn
    {
        return Err(invalid("audit source transcript changed between passes"));
    }
    if frontier.count()
        != u64::try_from(records.values).map_err(|_| resource("audit sequence counter overflow"))?
    {
        return Err(invalid("audit frontier count inconsistent"));
    }
    source.unchanged()?;
    if source.segment_count()? != metadata.segments {
        return Err(invalid("audit source membership changed"));
    }
    context.control.check()?;
    let (report, work) = accumulator.finish()?;
    if metadata.records == 0 {
        return Ok(Diagnostic { report: None, work });
    }
    if report.ok
        && context.policy.witness_public.is_some()
        && report.scope.witnessed_checkpoints == 0
    {
        return Err(invalid("required independent audit anchor missing"));
    }
    Ok(Diagnostic {
        report: Some(report),
        work,
    })
}
