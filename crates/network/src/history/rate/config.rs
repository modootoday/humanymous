use super::Unavailable;
use std::time::Duration;

/// Immutable caller-supplied resource bounds, with no production defaults.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub(super) max_keys: usize,
    pub(super) max_hits_per_key: usize,
    pub(super) max_key_bytes: usize,
    pub(super) max_window: Duration,
    pub(super) max_payload_bytes: usize,
    pub(super) gc_scan_budget: usize,
}
impl Limits {
    /// Payload is full reserved timestamp capacity plus key bytes per series;
    /// allocator/map metadata and the bounded cursor clone are extra.
    pub fn new(
        max_keys: usize,
        max_hits_per_key: usize,
        max_key_bytes: usize,
        max_window: Duration,
        max_payload_bytes: usize,
        gc_scan_budget: usize,
    ) -> Result<Self, Unavailable> {
        if [
            max_keys,
            max_hits_per_key,
            max_key_bytes,
            max_payload_bytes,
            gc_scan_budget,
        ]
        .contains(&0)
            || max_window.is_zero()
            || gc_scan_budget > 4096
            || max_hits_per_key
                .checked_mul(size_of::<Duration>())
                .is_none()
        {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            max_keys,
            max_hits_per_key,
            max_key_bytes,
            max_window,
            max_payload_bytes,
            gc_scan_budget,
        })
    }
}

/// Explicit policy; no activation, signals or verdict rules are supplied here.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub(super) window: Duration,
    pub(super) soft: usize,
    pub(super) hard: usize,
    pub(super) ttl: Duration,
    pub(super) limits: Limits,
    pub(super) sample_bytes: usize,
}
impl Config {
    /// Positive ordered cutpoints match the existing Gate settings contract.
    pub fn new(
        window: Duration,
        soft: usize,
        hard: usize,
        limits: Limits,
    ) -> Result<Self, Unavailable> {
        let ttl = window
            .checked_mul(6)
            .ok_or(Unavailable::InvalidConfiguration)?;
        let sample_bytes = limits
            .max_hits_per_key
            .checked_mul(size_of::<Duration>())
            .ok_or(Unavailable::InvalidConfiguration)?;
        if window.is_zero()
            || window > limits.max_window
            || soft == 0
            || hard < soft
            || sample_bytes > limits.max_payload_bytes
        {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            window,
            soft,
            hard,
            ttl,
            limits,
            sample_bytes,
        })
    }
}
