//! Local-only, fail-closed adapter decision composition.
pub mod configuration;
pub mod credentials;
pub mod decision;
pub mod protocol;
pub mod transport;

/// Static errors are safe for the wire; request values and secrets are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejection {
    /// Protocol HTTP error status.
    pub status: u16,
    /// Bounded, non-sensitive diagnostic code.
    pub code: &'static str,
}
impl Rejection {
    /// Construct a protocol rejection without interpolating sensitive values.
    pub const fn new(status: u16, code: &'static str) -> Self {
        Self { status, code }
    }
}

/// Fallible wall clock shared by transport and decision deadlines.
pub fn now_ms() -> Result<u64, Rejection> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .filter(|&ms| ms <= 9_007_199_254_740_991)
        .ok_or(Rejection::new(503, "clock_unavailable"))
}
