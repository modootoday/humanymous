//! Private synchronized state, never recovered for attribution after poisoning.

use std::collections::HashMap;
use std::time::Duration;

use super::{Configuration, Record};
use crate::watermark::Error;

pub(super) enum Slot {
    Pending { reserved_at: Duration },
    Committed(Record),
}

pub(super) struct State {
    pub(super) configuration: Configuration,
    pub(super) slots: HashMap<u32, Slot>,
    pub(super) next_id: u64,
    latest: Duration,
}

impl State {
    pub(super) fn new(configuration: Configuration, now: Duration) -> Self {
        Self {
            configuration,
            slots: HashMap::new(),
            next_id: 1,
            latest: now,
        }
    }

    pub(super) fn observe(&mut self, now: Duration) -> Result<(), Error> {
        if now < self.latest {
            return Err(Error::ClockRegression);
        }
        self.latest = now;
        Ok(())
    }

    pub(super) fn gc(&mut self, now: Duration) -> usize {
        let before = self.slots.len();
        self.slots.retain(|_, slot| match slot {
            Slot::Pending { reserved_at } => {
                now.saturating_sub(*reserved_at) <= self.configuration.reservation_ttl
            }
            Slot::Committed(record) => {
                now.saturating_sub(record.issued_at) <= self.configuration.record_ttl
            }
        });
        before - self.slots.len()
    }
}
