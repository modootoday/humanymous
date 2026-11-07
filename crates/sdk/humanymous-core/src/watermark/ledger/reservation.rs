//! A reservation is linear and cancelled automatically on every uncommitted path.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    Issuance, Record,
    state::{Slot, State},
};
use crate::watermark::Error;

/// A unique reserved ID. It cannot be cloned or committed twice.
#[must_use = "dropping a reservation cancels issuance without recycling its ID"]
pub struct Reservation {
    inner: Arc<Mutex<State>>,
    id: u32,
    finished: bool,
}

impl Reservation {
    pub(super) fn new(inner: Arc<Mutex<State>>, id: u32) -> Self {
        Self {
            inner,
            id,
            finished: false,
        }
    }

    /// Reserved nonzero ID, consumed forever even if issuance is cancelled.
    pub fn tag_id(&self) -> u32 {
        self.id
    }

    /// Commit only after successful encoding and before returning resource bytes.
    /// Validation failure, expiration or cancellation leaves no committed record.
    pub fn commit(mut self, issuance: Issuance, now: Duration) -> Result<(), Error> {
        issuance.validate()?;
        if issuance.payload.tag_id != self.id {
            return Err(Error::ReservationMismatch);
        }
        let mut state = self.inner.lock().map_err(|_| Error::Poisoned)?;
        state.observe(now)?;
        let Some(Slot::Pending { reserved_at }) = state.slots.get(&self.id) else {
            return Err(Error::ReservationExpired);
        };
        if now.saturating_sub(*reserved_at) > state.configuration.reservation_ttl {
            state.slots.remove(&self.id);
            return Err(Error::ReservationExpired);
        }
        state.slots.insert(
            self.id,
            Slot::Committed(Record {
                payload: issuance.payload,
                mime: issuance.mime,
                channels: issuance.channels,
                ip_pseudonym: issuance.ip_pseudonym,
                issued_at: now,
            }),
        );
        self.finished = true;
        Ok(())
    }

    /// Cancel explicitly. Drop has the same capacity-release effect on failure.
    pub fn cancel(mut self) -> Result<(), Error> {
        let mut state = self.inner.lock().map_err(|_| Error::Poisoned)?;
        if matches!(state.slots.get(&self.id), Some(Slot::Pending { .. })) {
            state.slots.remove(&self.id);
        }
        self.finished = true;
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        // Recovery permits deletion only. Public operations remain fail-closed
        // on poison and can never use recovered records for attribution.
        let mut state = match self.inner.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        if matches!(state.slots.get(&self.id), Some(Slot::Pending { .. })) {
            state.slots.remove(&self.id);
        }
    }
}
