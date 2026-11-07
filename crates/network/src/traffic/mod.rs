//! Legacy temporal observations, never connection provenance or serving health.
mod consistency;
mod log;
mod record;
use crate::history::Unavailable;
pub use consistency::{Coverage, Outcome, check};
pub use log::{Association, Config, Log};
pub use record::{Record, hash_order, hash_ua, subnet_key};

/// Explicit finite limits. No production defaults or activation.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    max_sessions: usize,
    max_addresses: usize,
    max_field_bytes: usize,
    max_payload_bytes: usize,
}
impl Limits {
    pub fn new(
        sessions: usize,
        addresses: usize,
        field_bytes: usize,
        payload_bytes: usize,
    ) -> Result<Self, Unavailable> {
        if [sessions, addresses, field_bytes, payload_bytes].contains(&0) {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            max_sessions: sessions,
            max_addresses: addresses,
            max_field_bytes: field_bytes,
            max_payload_bytes: payload_bytes,
        })
    }
    fn validate(&self, record: &Record) -> Result<(), Unavailable> {
        if record
            .fields()
            .iter()
            .any(|field| field.len() > self.max_field_bytes)
        {
            return Err(Unavailable::Budget);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
