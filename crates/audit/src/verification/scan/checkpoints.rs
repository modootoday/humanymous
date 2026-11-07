use super::{
    Context,
    frames::{Budget, Frames},
    limits::invalid,
    source::Source,
};
use crate::{
    checkpoint::Checkpoint,
    merkle::Hash,
    verification::{checks, incremental::Accumulator},
};
use std::io::{self, BufRead};

pub(super) struct Prefix<'a> {
    pub linear: Option<&'a str>,
    pub merkle: Option<Hash>,
}
pub(super) struct Checkpoints<R> {
    frames: Option<Frames<R>>,
    pub budget: Budget,
    pub pending: Option<Checkpoint>,
    pub has_sidecar: bool,
    previous_signature: String,
    last_size: u64,
}
impl<R: BufRead> Checkpoints<R> {
    pub fn open(
        source: &impl Source<Reader = R>,
        context: &Context<'_, '_, '_>,
    ) -> io::Result<Self> {
        context.control.check()?;
        let reader = source.checkpoints()?;
        let has_sidecar = reader.is_some();
        let frames = reader
            .map(|reader| Frames::new(reader, context.limits.frame, false))
            .transpose()?;
        let mut budget = Budget::new(context.limits.checkpoint_bytes, context.limits.checkpoints);
        budget.sidecar(has_sidecar);
        let mut stream = Self {
            frames,
            budget,
            pending: None,
            has_sidecar,
            previous_signature: String::new(),
            last_size: 0,
        };
        stream.advance(context)?;
        Ok(stream)
    }

    fn advance(&mut self, context: &Context<'_, '_, '_>) -> io::Result<()> {
        self.pending = match &mut self.frames {
            Some(frames) => frames.next(&mut self.budget, context.limits, context.control)?,
            None => None,
        };
        if self
            .pending
            .as_ref()
            .is_some_and(|checkpoint| checkpoint.tree_size <= self.last_size)
        {
            return Err(invalid(
                "audit checkpoint progression changed between passes",
            ));
        }
        Ok(())
    }

    pub fn consume(
        &mut self,
        prefix: Prefix<'_>,
        context: &Context<'_, '_, '_>,
        accumulator: &mut Accumulator,
    ) -> io::Result<()> {
        let mut checkpoint = self
            .pending
            .take()
            .ok_or_else(|| invalid("missing audit checkpoint cursor value"))?;
        context.control.check()?;
        accumulator.checkpoint(checks::checkpoint(
            &checkpoint,
            &self.previous_signature,
            prefix.linear,
            || prefix.merkle,
            Some(context.policy.writer_public),
            context.limits.signing_bytes,
        )?)?;
        context.control.check()?;
        if let Some(witness) = context.policy.witness_public {
            let valid = checkpoint.verify_signature_bounded(
                witness,
                &checkpoint.witness_sig,
                context.limits.signing_bytes,
            )?;
            accumulator.witness(checkpoint.tree_size, valid)?;
            context.control.check()?;
        }
        self.last_size = checkpoint.tree_size;
        // Preserve exact malformed link text too; Frames bounds its allocation.
        // This is separate from the fixed64-byte cryptographic signature decode.
        self.previous_signature = std::mem::take(&mut checkpoint.sig);
        drop(checkpoint);
        self.advance(context)
    }

    pub fn finish(self) -> io::Result<(Budget, bool)> {
        if self.pending.is_some() {
            return Err(invalid("audit checkpoint cursor did not finish"));
        }
        if let Some(frames) = self.frames {
            frames.finish()?;
        }
        Ok((self.budget, self.has_sidecar))
    }
}
