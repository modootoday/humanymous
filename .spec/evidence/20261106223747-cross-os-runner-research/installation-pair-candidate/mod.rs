//! Private review candidate: native observations before protected admission.
#![forbid(unsafe_code)]

#[cfg(windows)]
mod installation;
#[cfg(windows)]
mod pair;
#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_service;

use std::time::{Duration, Instant};

/// Static refusal labels only; never raw native identities or paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    InvalidSelection,
    Deadline,
    Query,
    Changed,
    WrongCaller,
    Descriptor,
    /// No original installer/native selection manufacturer is implemented.
    OriginalInstallerProducerUnavailable,
    /// Provider/file identity and ancestor in-place mutation admission are absent.
    HeldNamespaceAdmissionUnavailable,
    /// Separate reviewed self-query ACL initializer has not run/qualified.
    QueryRightsUnavailable,
    UnsupportedOs,
}

/// One original observation-attempt clock, not a native supervisor capability.
/// No caller Instant, wire reconstruction, Clone or renewal method.
struct QueryClock {
    started: Instant,
    deadline: Instant,
}
impl QueryClock {
    fn start(cap: Duration) -> Result<Self, Refusal> {
        let started = Instant::now();
        Self::from_start(started, cap)
    }
    fn from_start(started: Instant, cap: Duration) -> Result<Self, Refusal> {
        if cap.is_zero() || cap > Duration::from_secs(30) {
            return Err(Refusal::InvalidSelection);
        }
        let deadline = started.checked_add(cap).ok_or(Refusal::Deadline)?;
        Ok(Self { started, deadline })
    }
    fn check(&self) -> Result<(), Refusal> {
        if Instant::now() >= self.deadline { Err(Refusal::Deadline) } else { Ok(()) }
    }
}

/// No native operation or provider fallback exists on these OS branches.
#[cfg(not(windows))]
pub(super) fn native_observation_available() -> Result<(), Refusal> {
    Err(Refusal::UnsupportedOs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_clock_has_no_zero_or_oversize_profile() {
        assert!(QueryClock::start(Duration::ZERO).is_err());
        assert!(QueryClock::start(Duration::from_secs(31)).is_err());
        let clock = QueryClock::start(Duration::from_secs(1)).unwrap();
        assert_eq!(clock.deadline.duration_since(clock.started), Duration::from_secs(1));
        clock.check().unwrap();
    }
}
