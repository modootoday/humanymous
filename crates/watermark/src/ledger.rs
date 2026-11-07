//! Bounded volatile issuance owner with linear, automatically cancelled reservations.
//!
//! The runtime supplies trusted monotonic instants from a single epoch. No wall
//! clock, HTTP identity, filesystem or scoring decision is acquired here.

mod reservation;
mod state;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{Error, payload::Payload};
use state::{Slot, State};

pub use reservation::Reservation;

/// Maximum admitted live entries, including pending reservations.
pub const MAX_ENTRIES: usize = 65_536;
const MAX_MIME_BYTES: usize = 256;
const MAX_CHANNEL_BYTES: usize = 64;
const MAX_CHANNELS: usize = 4;
const MAX_PSEUDONYM_BYTES: usize = 128;

/// Explicit retention configuration; no universal/default TTL is supplied.
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    /// Maximum committed-record age; exact TTL boundary remains live.
    pub record_ttl: Duration,
    /// Maximum pending-reservation age; independent from committed retention.
    pub reservation_ttl: Duration,
    /// Live records plus reservations, in 1..=MAX_ENTRIES.
    pub capacity: usize,
}

impl Configuration {
    fn validate(self) -> Result<Self, Error> {
        if self.record_ttl.is_zero()
            || self.reservation_ttl.is_zero()
            || !(1..=MAX_ENTRIES).contains(&self.capacity)
        {
            return Err(Error::InvalidConfiguration);
        }
        Ok(self)
    }
}

/// Metadata offered only after successful resource encoding, before exposure.
/// No raw IP address belongs in this object.
pub struct Issuance {
    /// Payload whose tag ID must match the linear reservation.
    pub payload: Payload,
    /// Declared MIME recorded by the runtime.
    pub mime: String,
    /// Actual encoded channels, not a speculative composite label.
    pub channels: Vec<String>,
    /// Optional keyed pseudonym, still personal data subject to retention.
    pub ip_pseudonym: Option<String>,
}

impl Issuance {
    fn validate(&self) -> Result<(), Error> {
        self.payload.validate_issuance()?;
        if self.mime.is_empty()
            || self.mime.len() > MAX_MIME_BYTES
            || self.channels.is_empty()
            || self.channels.len() > MAX_CHANNELS
            || self
                .channels
                .iter()
                .any(|channel| channel.is_empty() || channel.len() > MAX_CHANNEL_BYTES)
            || self
                .ip_pseudonym
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > MAX_PSEUDONYM_BYTES)
        {
            return Err(Error::InvalidMetadata);
        }
        for (index, channel) in self.channels.iter().enumerate() {
            if self.channels[..index].contains(channel) {
                return Err(Error::InvalidMetadata);
            }
        }
        Ok(())
    }
}

/// Immutable committed issuance; Debug intentionally cannot expose identifiers.
#[derive(Clone)]
pub struct Record {
    payload: Payload,
    mime: String,
    channels: Vec<String>,
    ip_pseudonym: Option<String>,
    issued_at: Duration,
}

impl Record {
    /// Trusted issued payload for authenticated operator verification.
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// MIME recorded at issuance.
    pub fn mime(&self) -> &str {
        &self.mime
    }

    /// Channels actually reported by encoding.
    pub fn channels(&self) -> &[String] {
        &self.channels
    }

    /// Optional keyed IP pseudonym; never anonymous data.
    pub fn ip_pseudonym(&self) -> Option<&str> {
        self.ip_pseudonym.as_deref()
    }

    /// Monotonic commit instant, not a serializable wall-clock timestamp.
    pub fn issued_at(&self) -> Duration {
        self.issued_at
    }
}

/// Lookup outcomes never constitute an authenticated trace by themselves.
pub enum Lookup {
    /// A live issuance; its carrier HMAC must still be checked.
    Live(Record),
    /// A known committed issuance was beyond retention and was removed.
    Expired,
    /// No committed issuance is available, including a pending reservation.
    Unknown,
}

/// Shared synchronized bounded owner. Clone shares state and the same ID space.
#[derive(Clone)]
pub struct Ledger {
    inner: Arc<Mutex<State>>,
}

impl Ledger {
    /// Construct a volatile ledger at the trusted epoch's initial instant.
    pub fn new(configuration: Configuration, now: Duration) -> Result<Self, Error> {
        let configuration = configuration.validate()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(State::new(configuration, now))),
        })
    }

    /// Reserve a unique nonzero ID and capacity. Drop or cancel releases capacity;
    /// the consumed ID is never recycled, even after expiry or failure.
    pub fn reserve(&self, now: Duration) -> Result<Reservation, Error> {
        let mut state = self.inner.lock().map_err(|_| Error::Poisoned)?;
        state.observe(now)?;
        state.gc(now);
        if state.slots.len() >= state.configuration.capacity {
            return Err(Error::Capacity);
        }
        let id = u32::try_from(state.next_id).map_err(|_| Error::IdExhausted)?;
        state.next_id += 1;
        state.slots.insert(id, Slot::Pending { reserved_at: now });
        Ok(Reservation::new(Arc::clone(&self.inner), id))
    }

    /// Read a live record, enforcing expiry without waiting for a GC task.
    /// Returned data is privileged and must not be exposed before operator auth.
    pub fn lookup(&self, id: u32, now: Duration) -> Result<Lookup, Error> {
        let mut state = self.inner.lock().map_err(|_| Error::Poisoned)?;
        state.observe(now)?;
        match state.slots.get(&id) {
            Some(Slot::Committed(record)) => {
                if now.saturating_sub(record.issued_at) > state.configuration.record_ttl {
                    state.slots.remove(&id);
                    Ok(Lookup::Expired)
                } else {
                    Ok(Lookup::Live(record.clone()))
                }
            }
            _ => Ok(Lookup::Unknown),
        }
    }

    /// Remove expired entries in a bounded scan (at most configured capacity).
    /// A runtime-owned periodic call is required to enforce physical retention
    /// when there are no lookups or new issuances. It does not authorize erasure.
    pub fn gc(&self, now: Duration) -> Result<usize, Error> {
        let mut state = self.inner.lock().map_err(|_| Error::Poisoned)?;
        state.observe(now)?;
        Ok(state.gc(now))
    }
}

#[cfg(test)]
mod tests;
