//! Pure shutdown configuration; actual blocking I/O supervision belongs to CLI.
use std::{io, time::Duration};

/// Validated bounded drain request; no OS handle or credential crosses this API.
#[derive(Clone, Copy)]
pub struct ShutdownRequest {
    drain_ms: u64,
}
impl ShutdownRequest {
    /// Component drain bound. Use StopBudget for a managed service's deadlines.
    pub fn new(drain_ms: u64) -> io::Result<Self> {
        if !(1..=30_000).contains(&drain_ms) {
            return Err(io::Error::other("shutdown drain budget"));
        }
        Ok(Self { drain_ms })
    }
    pub(crate) fn drain_timeout(self) -> Duration {
        Duration::from_millis(self.drain_ms)
    }
}

/// A validated relationship, not a claim of synchronous I/O preemption.
#[derive(Clone, Copy)]
pub struct StopBudget {
    request: ShutdownRequest,
    supervisor_reserve_ms: u64,
    manager_stop_ms: u64,
}
impl StopBudget {
    /// Leave strict slack before the manager can force termination. The caller
    /// must enforce actual I/O deadlines and reap any owned child on failure.
    pub fn new(
        drain_ms: u64,
        supervisor_reserve_ms: u64,
        manager_stop_ms: u64,
    ) -> io::Result<Self> {
        let request = ShutdownRequest::new(drain_ms)?;
        if supervisor_reserve_ms == 0
            || manager_stop_ms == 0
            || drain_ms
                .checked_add(supervisor_reserve_ms)
                .is_none_or(|total| total >= manager_stop_ms)
        {
            return Err(io::Error::other("service supervisor budget"));
        }
        Ok(Self {
            request,
            supervisor_reserve_ms,
            manager_stop_ms,
        })
    }
    /// Request sent only after the platform adapter observes a stop event.
    pub fn request(self) -> ShutdownRequest {
        self.request
    }
    /// Reserved interval for supervised synchronous work and child cleanup.
    pub fn supervisor_reserve(self) -> Duration {
        Duration::from_millis(self.supervisor_reserve_ms)
    }
    /// External manager deadline that the adapter must actually enforce.
    pub fn manager_stop(self) -> Duration {
        Duration::from_millis(self.manager_stop_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manager_budget_has_strict_slack_and_rejects_overflow() {
        assert!(StopBudget::new(4000, 1000, 5000).is_err());
        assert!(StopBudget::new(4000, 1001, 5000).is_err());
        assert!(StopBudget::new(4000, u64::MAX, u64::MAX).is_err());
        assert!(StopBudget::new(4000, 0, 5000).is_err());
        assert!(StopBudget::new(0, 1000, 5000).is_err());
        let linux = StopBudget::new(3000, 1000, 5000).unwrap();
        assert_eq!(linux.request().drain_timeout(), Duration::from_secs(3));
        assert_eq!(linux.supervisor_reserve(), Duration::from_secs(1));
        assert_eq!(linux.manager_stop(), Duration::from_secs(5));
        let windows = StopBudget::new(4000, 1000, 6000).unwrap();
        assert_eq!(windows.manager_stop(), Duration::from_secs(6));
    }
}
