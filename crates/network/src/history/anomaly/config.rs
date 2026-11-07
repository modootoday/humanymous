use super::Unavailable;

/// Explicit caller-owned resource limits. No production defaults are inferred.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub(super) max_keys: usize,
    pub(super) max_window: usize,
    pub(super) max_key_bytes: usize,
    pub(super) max_payload_bytes: usize,
}
impl Limits {
    /// Payload accounting reserves a full sample window plus UTF-8 key bytes
    /// per series. Allocator/map metadata and bounded sorting scratch are extra.
    pub fn new(
        max_keys: usize,
        max_window: usize,
        max_key_bytes: usize,
        max_payload_bytes: usize,
    ) -> Result<Self, Unavailable> {
        if [max_keys, max_window, max_key_bytes, max_payload_bytes].contains(&0) {
            return Err(Unavailable::InvalidConfiguration);
        }
        max_window
            .checked_mul(size_of::<f64>())
            .ok_or(Unavailable::InvalidConfiguration)?;
        Ok(Self {
            max_keys,
            max_window,
            max_key_bytes,
            max_payload_bytes,
        })
    }
}

/// Explicit measurement parameters; changing a consuming detector is a policy
/// decision. This type neither supplies defaults nor changes frozen consumers.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub(super) window: usize,
    pub(super) k: f64,
    pub(super) warmup: usize,
    pub(super) mad_floor: f64,
    pub(super) limits: Limits,
    pub(super) sample_bytes: usize,
}
impl Config {
    /// Warmup greater than window is allowed: that history never becomes warm.
    pub fn new(
        window: usize,
        k: f64,
        warmup: usize,
        mad_floor: f64,
        limits: Limits,
    ) -> Result<Self, Unavailable> {
        let sample_bytes = window
            .checked_mul(size_of::<f64>())
            .ok_or(Unavailable::InvalidConfiguration)?;
        if window == 0
            || window > limits.max_window
            || warmup == 0
            || !k.is_finite()
            || k <= 0.0
            || !mad_floor.is_finite()
            || mad_floor < 0.0
            || sample_bytes > limits.max_payload_bytes
        {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self {
            window,
            k,
            warmup,
            mad_floor,
            limits,
            sample_bytes,
        })
    }
}
