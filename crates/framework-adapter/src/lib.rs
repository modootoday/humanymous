//! Bounded framework observations; all security decisions belong to authority.
mod configuration;
mod request;
pub use configuration::Configuration;
pub use request::{Observation, Pending, internal_header};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Static, non-sensitive host boundary failure. Never contains credentials/body.
#[derive(Debug, Clone, Copy)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// Process-wide body/decision admission; hold until origin handoff or rejection.
pub struct Admission;
impl Admission {
    /// Refuse overload before allocating a complete body.
    pub fn acquire() -> Result<Self, Error> {
        ACTIVE
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 64).then_some(n + 1)
            })
            .map(|_| Self)
            .map_err(|_| Error("adapter capacity unavailable"))
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Reject malformed CGI framing without interpreting a missing length as empty.
pub fn body_length(value: &str, maximum: usize) -> Result<usize, Error> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error("explicit decimal content length required"));
    }
    let n = value
        .parse::<usize>()
        .map_err(|_| Error("invalid content length"))?;
    if n > maximum {
        return Err(Error("body exceeds limit"));
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_ambiguous_and_oversized_lengths_reject() {
        for value in ["", "-1", "+1", "1,1", " 1", "1\n", "42949672960"] {
            assert!(body_length(value, 8).is_err());
        }
        assert!(body_length("9", 8).is_err());
        assert_eq!(body_length("0", 8).unwrap(), 0);
        assert_eq!(body_length("8", 8).unwrap(), 8);
    }
}
