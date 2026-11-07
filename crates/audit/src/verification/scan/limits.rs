use crate::storage::framing::preflight;
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) struct Limits {
    pub frame: usize,
    pub json: preflight::Limits,
    pub segments: usize,
    pub records: usize,
    pub checkpoints: usize,
    pub record_bytes: u64,
    pub checkpoint_bytes: u64,
    pub node_bytes: usize,
    pub canonical_bytes: usize,
    pub signing_bytes: usize,
}
impl Limits {
    pub fn validate(&self) -> io::Result<()> {
        let sizes = [
            self.frame,
            self.json.strings,
            self.json.array_items,
            self.json.values,
            self.json.depth,
            self.segments,
            self.records,
            self.checkpoints,
            self.node_bytes,
            self.canonical_bytes,
            self.signing_bytes,
        ];
        if sizes.contains(&0)
            || sizes.contains(&usize::MAX)
            || self.record_bytes == 0
            || self.checkpoint_bytes == 0
            || self.record_bytes == u64::MAX
            || self.checkpoint_bytes == u64::MAX
            || self.json.depth > 64
            || self.node_bytes > self.frame
            || self.json.strings > self.frame
        {
            return Err(invalid("invalid scan resource profile"));
        }
        // Address-space sanity only, not a proved whole-startup memory ledger.
        let arrays = self.json.array_items.checked_mul(
            std::mem::size_of::<crate::record::Signal>() + std::mem::size_of::<String>(),
        );
        if arrays
            .and_then(|arrays| {
                self.frame
                    .checked_mul(3)?
                    .checked_add(self.json.strings)?
                    .checked_add(arrays)?
                    .checked_add(self.canonical_bytes)?
                    .checked_add(self.signing_bytes)
            })
            .is_none()
        {
            return Err(invalid("scan resource profile arithmetic overflow"));
        }
        Ok(())
    }
}

pub(super) struct Control<'a> {
    cancelled: &'a AtomicBool,
    deadline: Instant,
}
impl<'a> Control<'a> {
    pub fn new(cancelled: &'a AtomicBool, timeout: Duration) -> io::Result<Self> {
        if timeout.is_zero() {
            return Err(invalid("zero scan deadline"));
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| invalid("scan deadline overflow"))?;
        Ok(Self {
            cancelled,
            deadline,
        })
    }
    pub fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "audit scan cancelled",
            ));
        }
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "audit scan deadline exceeded",
            ));
        }
        Ok(())
    }
}

pub(super) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub(super) fn resource(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::FileTooLarge, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn elapsed_deadline_refuses_without_an_interrupt_claim() {
        let cancel = AtomicBool::new(false);
        let control = Control {
            cancelled: &cancel,
            deadline: Instant::now(),
        };
        assert_eq!(control.check().unwrap_err().kind(), io::ErrorKind::TimedOut);
    }
}
