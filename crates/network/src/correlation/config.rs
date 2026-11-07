use super::Unavailable;
use crate::history::anomaly;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
/// Finite retained key, distinct-set, string and payload limits.
pub struct Limits {
    pub(super) max_keys: usize,
    pub(super) max_fanout_keys: usize,
    pub(super) max_distinct: usize,
    pub(super) max_field_bytes: usize,
    pub(super) max_payload_bytes: usize,
}
impl Limits {
    /// Reject zero bounds; values are supplied by the owning configuration.
    pub fn new(
        keys: usize,
        fanout: usize,
        distinct: usize,
        field_bytes: usize,
        payload_bytes: usize,
    ) -> Result<Self, Unavailable> {
        if [keys, fanout, distinct, field_bytes, payload_bytes].contains(&0) {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            max_keys: keys,
            max_fanout_keys: fanout,
            max_distinct: distinct,
            max_field_bytes: field_bytes,
            max_payload_bytes: payload_bytes,
        })
    }
    pub(super) fn fields(&self, values: &[&str]) -> Result<(), Unavailable> {
        if values
            .iter()
            .any(|value| value.len() > self.max_field_bytes)
        {
            return Err(Unavailable::Budget);
        }
        Ok(())
    }
}
/// Explicit TTL and bounded population measurement configuration.
pub struct Config {
    pub(super) ttl: Duration,
    pub(super) limits: Limits,
    pub(super) population: anomaly::Config,
}
impl Config {
    /// The frozen population parameters are not generic anomaly defaults.
    pub fn new(
        ttl: Duration,
        limits: Limits,
        mad_limits: anomaly::Limits,
    ) -> Result<Self, Unavailable> {
        if ttl.is_zero() {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            ttl,
            limits,
            population: anomaly::Config::new(256, 6.0, 20, 1.0, mad_limits)?,
        })
    }
}
