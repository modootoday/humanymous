use super::*;

pub(super) struct Tracker {
    last_tracking_tick: Option<Duration>,
    losses: [Option<Loss>; 4],
    expansion_until: Option<Duration>,
    abandoned: bool,
    clock_fault: bool,
}
pub(super) struct Frame<'a> {
    tracker: &'a mut Tracker,
    now: Duration,
    records: [Option<Record>; SLOTS],
    finished: bool,
}
pub(super) struct Receipt {
    pub(super) records: [Option<Record>; SLOTS],
}
#[derive(Debug)]
pub(super) struct View {
    pub(super) losses: [Option<Loss>; 4],
    pub(super) expansion_until: Option<Duration>,
    pub(super) abandoned: bool,
    pub(super) clock_fault: bool,
}
impl Tracker {
    pub(super) fn new() -> Self {
        Self {
            last_tracking_tick: None,
            losses: [None; 4],
            expansion_until: None,
            abandoned: false,
            clock_fault: false,
        }
    }
    pub(super) fn begin(&mut self, now: Duration) -> Result<Frame<'_>, Error> {
        if self.last_tracking_tick.is_some_and(|last| now < last) {
            self.clock_fault = true;
            return Err(Error::ClockRegression);
        }
        // A bookkeeping tick is NOT a provider's committed clock/history tick.
        self.last_tracking_tick = Some(now);
        Ok(Frame {
            tracker: self,
            now,
            records: [None; SLOTS],
            finished: false,
        })
    }
    pub(super) fn view(&self, now: Duration) -> View {
        View {
            losses: self.losses,
            expansion_until: self.expansion_until.filter(|until| now < *until),
            abandoned: self.abandoned,
            clock_fault: self.clock_fault,
        }
    }
    fn latch(&mut self, domain: Domain, at: Duration, reason: Gap) {
        self.losses[domain.index()].get_or_insert(Loss { at, reason });
    }
}
impl Frame<'_> {
    // Only trusted private provider calls belong here, never I/O or authority
    // constructors. Check duplicate admission BEFORE invoking the lower call.
    pub(super) fn apply<T>(
        &mut self,
        operation: Operation,
        call: impl FnOnce() -> Result<T, Unavailable>,
        classify: impl FnOnce(&T) -> (Effect, Detail),
    ) -> Result<T, Error> {
        let index = operation.index();
        if self.records[index].is_some() {
            return Err(Error::Duplicate(operation));
        }
        self.records[index] = Some(Record {
            operation,
            effect: Effect::Pending,
            detail: Detail::Pending,
        });
        match call() {
            Ok(value) => {
                let (effect, detail) = classify(&value);
                self.records[index] = Some(Record {
                    operation,
                    effect,
                    detail,
                });
                match detail {
                    Detail::MissingIdentity => {
                        self.tracker
                            .latch(operation.domain(), self.now, Gap::MissingIdentity)
                    }
                    Detail::Anomaly { evicted: true, .. } => {
                        self.tracker
                            .latch(operation.domain(), self.now, Gap::Evicted)
                    }
                    Detail::Rate(rate::Coverage::WindowExpanded { complete_after }) => {
                        self.tracker.expansion_until = Some(
                            self.tracker
                                .expansion_until
                                .map_or(complete_after, |old| old.max(complete_after)),
                        );
                    }
                    _ => {}
                }
                Ok(value)
            }
            Err(reason) => {
                self.records[index] = Some(Record {
                    operation,
                    effect: Effect::Refused,
                    detail: Detail::Refused(reason),
                });
                self.tracker
                    .latch(operation.domain(), self.now, Gap::Provider(reason));
                Err(Error::Provider(reason))
            }
        }
    }
    pub(super) fn finish(mut self) -> Receipt {
        self.finished = true;
        Receipt {
            records: self.records,
        }
    }
}
impl Drop for Frame<'_> {
    fn drop(&mut self) {
        if !self.finished {
            for record in self.records.iter().flatten() {
                if record.effect == Effect::Pending {
                    self.tracker
                        .latch(record.operation.domain(), self.now, Gap::Interrupted);
                }
                if matches!(record.effect, Effect::Committed | Effect::Pending) {
                    self.tracker.abandoned = true;
                }
            }
        }
    }
}
