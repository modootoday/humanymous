//! Safe owning bookkeeping. Pure state tests cannot manufacture native facts.
#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

/// Static issues; no executable path, identity or command text is exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Issue {
    /// Invalid finite qualification data.
    InvalidInput,
    /// The selected held image is not the fixed supported x64 console profile.
    ImageProfile,
    /// A native call failed or its result could not be trusted.
    NativeCall,
    /// Kernel policy/membership differed from the requested profile.
    Containment,
    /// Original run budget expired before complete observations.
    RunExpired,
    /// Original cleanup budget expired before complete observations.
    CleanupExpired,
    /// Stop was requested; later exit0 is not natural success.
    StopRequested,
    /// Actual signaled process returned a nonzero code.
    UnsuccessfulExit,
}

/// First local failure observation only; not a cleanup or authority receipt.
#[derive(Clone, Copy, Debug)]
pub struct FirstFailure {
    /// Immutable first reason.
    pub issue: Issue,
    /// Original local monotonic failure observation.
    pub at: Instant,
}

/// Closed low-level owner phase, not authenticated native source admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Validation/image preparation only.
    Preflight,
    /// Actual Job has been retained.
    JobCreated,
    /// Exact Job policy has been queried.
    Configured,
    /// The native create call may have had effects.
    CreateEntered,
    /// Actual returned process/thread owners are retained.
    Created,
    /// Actual membership was confirmed.
    Running,
    /// New launch is closed; cleanup observation only.
    Closing,
}

/// Finite requested durations are DATA, not an original outer supervisor proof.
pub struct BudgetLimits { run: Duration, cleanup: Duration, drain: Duration }
impl BudgetLimits {
    /// Select positive run and total cleanup durations, maximum one hour.
    /// Cleanup is measured from ORIGINAL start, not from a later failure.
    pub fn new(run: Duration, cleanup: Duration, drain: Duration) -> Result<Self, Issue> {
        if run.is_zero() || cleanup < run || cleanup > Duration::from_secs(3600)
            || drain.is_zero() || drain > cleanup || drain > Duration::from_secs(60)
        {
            return Err(Issue::InvalidInput);
        }
        Ok(Self { run, cleanup, drain })
    }
}

pub(super) struct State {
    pub(super) started: Instant,
    pub(super) run: Instant,
    pub(super) cleanup: Instant,
    pub(super) effective_cleanup: Instant,
    drain: Duration,
    pub(super) phase: Phase,
    pub(super) first: Option<FirstFailure>,
    pub(super) stop_requested: bool,
    pub(super) cleanup_missed: bool,
    run_met: bool,
    cleanup_met: bool,
}
impl State {
    pub(super) fn new(started: Instant, limits: BudgetLimits) -> Result<Self, Issue> {
        let run = started.checked_add(limits.run).ok_or(Issue::InvalidInput)?;
        let cleanup = started.checked_add(limits.cleanup).ok_or(Issue::InvalidInput)?;
        Ok(Self { started, run, cleanup, effective_cleanup: cleanup, drain: limits.drain,
            phase: Phase::Preflight, first: None, stop_requested: false, cleanup_missed: false, run_met: false, cleanup_met: false })
    }
    pub(super) fn refused(started: Instant, issue: Issue) -> Self {
        Self { started, run: started, cleanup: started, effective_cleanup: started, drain: Duration::ZERO, phase: Phase::Closing,
            first: Some(FirstFailure { issue, at: started }), stop_requested: false,
            cleanup_missed: true, run_met: false, cleanup_met: false }
    }
    pub(super) fn record(&mut self, issue: Issue, now: Instant) {
        if self.first.is_none() {
            self.first = Some(FirstFailure { issue, at: now });
            self.effective_cleanup = self.cleanup.min(now.checked_add(self.drain).unwrap_or(now));
        }
    }
    pub(super) fn observe_time(&mut self, now: Instant, complete: bool) {
        self.run_met |= complete && now < self.run;
        if now >= self.run && !self.run_met { self.record(Issue::RunExpired, self.run); self.phase = Phase::Closing; }
        self.cleanup_met |= complete && now < self.effective_cleanup;
        if now >= self.effective_cleanup && !self.cleanup_met {
            self.record(Issue::CleanupExpired, self.effective_cleanup); self.cleanup_missed = true;
        }
    }
    pub(super) fn permits_start(&self, now: Instant) -> bool {
        now < self.run && self.first.is_none() && self.phase != Phase::Closing
    }
    pub(super) fn direct_exit_observed(&mut self, code: u32, now: Instant) {
        // Native success may return late. Known expiry linearizes BEFORE a
        // subsequently observed nonzero status, preserving original_run+drain.
        self.observe_time(now, false);
        if code != 0 { self.record(Issue::UnsuccessfulExit, now); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_pair_and_first_failure_never_renew_on_recovery() {
        let start = Instant::now();
        let mut state = State::new(start, BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(1)).unwrap()).unwrap();
        state.record(Issue::NativeCall, start);
        state.observe_time(start + Duration::from_secs(3), false);
        assert_eq!(state.first.unwrap().issue, Issue::NativeCall);
        assert_eq!(state.first.unwrap().at, start);
        assert_eq!(state.run, start + Duration::from_secs(1));
        assert_eq!(state.cleanup, start + Duration::from_secs(2));
        assert!(state.cleanup_missed);
        assert!(!state.permits_start(start));
    }
    #[test]
    fn invalid_or_expired_profiles_cannot_start() {
        assert!(BudgetLimits::new(Duration::ZERO, Duration::from_secs(1), Duration::from_secs(1)).is_err());
        assert!(BudgetLimits::new(Duration::from_secs(2), Duration::from_secs(1), Duration::from_secs(1)).is_err());
        assert!(BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(3601), Duration::from_secs(1)).is_err());
        assert!(BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(2), Duration::ZERO).is_err());
        assert!(BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(120), Duration::from_secs(61)).is_err());
        assert!(BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(3)).is_err());
        let start = Instant::now();
        let state = State::new(start, BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(1)).unwrap()).unwrap();
        assert!(!state.permits_start(start + Duration::from_secs(1)));
    }
    #[test]
    fn closed_state_and_late_completion_cannot_mint_timely_success() {
        let start = Instant::now();
        let mut state = State::new(start, BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(1)).unwrap()).unwrap();
        state.phase = Phase::Closing;
        assert!(!state.permits_start(start));
        state.observe_time(start + Duration::from_secs(3), true);
        assert_eq!(state.first.unwrap().issue, Issue::RunExpired);
        assert!(state.cleanup_missed);
    }
    #[test]
    fn first_loss_drain_is_clamped_sticky_and_refuses_exact_limit() {
        let start = Instant::now();
        let mut state = State::new(start, BudgetLimits::new(Duration::from_secs(20), Duration::from_secs(30), Duration::from_secs(3)).unwrap()).unwrap();
        state.record(Issue::NativeCall, start + Duration::from_secs(2));
        assert_eq!(state.effective_cleanup, start + Duration::from_secs(5));
        state.record(Issue::StopRequested, start + Duration::from_secs(4));
        assert_eq!(state.effective_cleanup, start + Duration::from_secs(5));
        assert_eq!(state.cleanup, start + Duration::from_secs(30));
        state.observe_time(start + Duration::from_secs(5), false);
        assert!(state.cleanup_missed);
        assert_eq!(state.first.unwrap().issue, Issue::NativeCall);
        let mut earlier = State::new(start, BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(4), Duration::from_secs(3)).unwrap()).unwrap();
        earlier.record(Issue::NativeCall, start + Duration::from_secs(2));
        assert_eq!(earlier.effective_cleanup, start + Duration::from_secs(4));
    }
    #[test]
    fn delayed_nonzero_native_return_cannot_move_first_loss_past_original_run() {
        let start = Instant::now();
        let mut state = State::new(start, BudgetLimits::new(Duration::from_secs(1), Duration::from_secs(10), Duration::from_secs(3)).unwrap()).unwrap();
        state.observe_time(start + Duration::from_millis(900), false);
        state.direct_exit_observed(1, start + Duration::from_secs(3));
        assert_eq!(state.first.unwrap().issue, Issue::RunExpired);
        assert_eq!(state.first.unwrap().at, start + Duration::from_secs(1));
        assert_eq!(state.effective_cleanup, start + Duration::from_secs(4));
        assert_eq!(state.cleanup, start + Duration::from_secs(10));
    }
}
