//! Private S1 diagnostics; no operational owner, repair or trusted-state token.
mod checkpoints;
mod frames;
mod limits;
mod metadata;
mod source;
mod verify;

use super::{Report, incremental::Work};
use crate::storage::RecoveryPolicy;
use std::io;

struct Context<'scan, 'keys, 'cancel> {
    policy: &'scan RecoveryPolicy<'keys>,
    limits: &'scan limits::Limits,
    control: &'scan limits::Control<'cancel>,
}

pub(super) struct Diagnostic {
    pub report: Option<Report>,
    pub work: Work,
}

pub(super) fn inspect(
    source: &impl source::Source,
    policy: &RecoveryPolicy<'_>,
    limits: &limits::Limits,
    control: &limits::Control<'_>,
) -> io::Result<Diagnostic> {
    let metadata = metadata::inspect(source, policy, limits, control)?;
    verify::finish(
        source,
        Context {
            policy,
            limits,
            control,
        },
        metadata,
    )
}

#[cfg(test)]
mod tests;
