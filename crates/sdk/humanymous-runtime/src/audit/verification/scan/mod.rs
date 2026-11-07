//! Private S1 diagnostics; no operational owner, repair or trusted-state token.
mod checkpoints;
mod frames;
mod limits;
mod metadata;
mod recovery_transition;
mod source;
mod verify;

use super::{Report, incremental::Work};
use crate::audit::storage::RecoveryPolicy;
use source::inspect_native;
use std::io;

// Trusted resource reservation only; never an identity or successful-scan token.
struct NativeBudget {
    max_entries: usize,
    max_handles: usize,
    other_handles: usize,
}
impl NativeBudget {
    fn check(&self, segments: usize) -> io::Result<()> {
        let peak = segments
            .checked_add(7)
            .and_then(|value| value.checked_add(self.other_handles))
            .ok_or_else(|| limits::resource("native descriptor budget overflow"))?;
        if segments > 4096
            || self.max_entries == 0
            || self.max_entries > 4100
            || self.max_handles == 0
            || self.max_handles == usize::MAX
            || self.other_handles == usize::MAX
            || peak > self.max_handles
        {
            return Err(limits::resource(
                "native source descriptor admission refused",
            ));
        }
        Ok(())
    }
}

struct Context<'scan, 'keys, 'cancel> {
    policy: &'scan RecoveryPolicy<'keys>,
    limits: &'scan limits::Limits,
    control: &'scan limits::Control<'cancel>,
}

struct Diagnostic {
    pub report: Option<Report>,
    pub work: Work,
}

fn inspect(
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
