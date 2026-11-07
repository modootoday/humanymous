use crate::audit::storage::framing::preflight;
use std::{
    io,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
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
            std::mem::size_of::<crate::audit::record::Signal>() + std::mem::size_of::<String>(),
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
    deadline: Deadline<'a>,
}
enum Deadline<'a> {
    System(Instant),
    Fixture { clock: &'a ManualClock, end: u64 },
}
// Test-compiled only, like this entire scanner. The fixture clock can only
// advance; it is borrowed by the original Control and cannot replace it later.
#[derive(Default)]
pub(super) struct ManualClock(AtomicU64);
impl ManualClock {
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    pub fn advance(&self, elapsed: Duration) -> io::Result<()> {
        let ticks =
            u64::try_from(elapsed.as_nanos()).map_err(|_| invalid("fixture clock overflow"))?;
        self.0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |now| {
                now.checked_add(ticks)
            })
            .map(|_| ())
            .map_err(|_| invalid("fixture clock overflow"))
    }
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
            deadline: Deadline::System(deadline),
        })
    }
    pub fn with_clock(
        cancelled: &'a AtomicBool,
        timeout: Duration,
        clock: &'a ManualClock,
    ) -> io::Result<Self> {
        let ticks =
            u64::try_from(timeout.as_nanos()).map_err(|_| invalid("fixture deadline overflow"))?;
        if ticks == 0 {
            return Err(invalid("zero scan deadline"));
        }
        let end = clock
            .0
            .load(Ordering::Acquire)
            .checked_add(ticks)
            .ok_or_else(|| invalid("fixture deadline overflow"))?;
        Ok(Self {
            cancelled,
            deadline: Deadline::Fixture { clock, end },
        })
    }
    pub fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "audit scan cancelled",
            ));
        }
        let expired = match &self.deadline {
            Deadline::System(end) => Instant::now() >= *end,
            Deadline::Fixture { clock, end } => clock.0.load(Ordering::Acquire) >= *end,
        };
        if expired {
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
            deadline: Deadline::System(Instant::now()),
        };
        assert_eq!(control.check().unwrap_err().kind(), io::ErrorKind::TimedOut);
    }
    #[test]
    fn original_manual_clock_advances_without_replacing_control() {
        let cancel = AtomicBool::new(false);
        let clock = ManualClock::new();
        let control = Control::with_clock(&cancel, Duration::from_nanos(2), &clock).unwrap();
        clock.advance(Duration::from_nanos(1)).unwrap();
        assert!(control.check().is_ok());
        clock.advance(Duration::from_nanos(1)).unwrap();
        assert_eq!(control.check().unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(clock.advance(Duration::MAX).is_err());
        assert_eq!(control.check().unwrap_err().kind(), io::ErrorKind::TimedOut);
    }
}
