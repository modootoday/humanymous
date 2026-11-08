//! Safe concrete owner. No generic provider, raw PID or symbolic proof input.
#![forbid(unsafe_code)]

use super::{launch::{PreparedLaunch, QualificationHelper}, native::{self, JobObservation, ProcessObservation, Query}, state::{BudgetLimits, FirstFailure, Issue, Phase, State}};
use std::{path::Path, time::Instant};

struct Owner {
    // Drop closes sole Job handle first; kill-on-close is NOT a cleanup receipt.
    job: Option<native::Job>,
    process: Option<native::Process>,
    launch: Option<PreparedLaunch>,
    state: State,
    create_entered: bool,
    membership: Option<bool>,
    exit: Option<u32>,
    population: Option<u32>,
}

/// Owns real Job/process/thread/image guards and original local clock.
/// The independent caller/outer budget and admitted source remain preconditions.
/// The public low-level signature is usable without importing raw native fields:
/// ```no_run
/// use humanymous_platform::windows_supervisor::{OwnedTree, RetainedTree, QualificationHelper, BudgetLimits};
/// let _: fn(&std::path::Path, &std::path::Path, QualificationHelper, BudgetLimits)
///     -> Result<OwnedTree, RetainedTree> = OwnedTree::launch;
/// ```
/// Caller DATA cannot construct an actual owning tree:
/// ```compile_fail,E0451
/// use humanymous_platform::windows_supervisor::OwnedTree;
/// let _forged = OwnedTree { owner: todo!() };
/// ```
#[must_use = "retain the actual tree until explicitly settled or reported Unknown"]
pub struct OwnedTree { owner: Box<Owner> }

/// Actual partial owner on error; move recovery never renews the original pair.
/// ```compile_fail,E0616
/// use humanymous_platform::windows_supervisor::RetainedTree;
/// fn forbidden(owner: RetainedTree) { let _ = owner.tree; }
/// ```
#[must_use = "failure retains the actual tree; Drop is not cleanup success"]
pub struct RetainedTree { tree: OwnedTree }
impl RetainedTree {
    /// Move the SAME actual owner into in-process recovery, without PID reopening.
    pub fn into_owner(self) -> OwnedTree { self.tree }
    /// Restricted diagnostic view, not permission to launch/activate anything.
    pub fn observation(&self) -> TreeObservation { self.tree.observation() }
}

/// Fully observed minimum-profile resource closure, with execution kept separate.
/// This is not provider, original invocation, worker/capture or NativeProof status.
pub struct TerminalTree { tree: OwnedTree }
impl TerminalTree {
    /// Natural exit0, original run observation and no sticky failure.
    pub fn execution_succeeded(&self) -> bool {
        self.tree.owner.exit == Some(0) && self.tree.owner.state.first.is_none()
    }
    /// Actual component diagnostic facts; this does not expose raw handles.
    pub fn observation(&self) -> TreeObservation { self.tree.observation() }
}

/// Read-only observations; callers cannot supply them to manufacture completion.
#[derive(Clone, Copy)]
pub struct TreeObservation {
    phase: Phase,
    exit: Option<u32>,
    population: Option<u32>,
    first: Option<FirstFailure>,
    started: Instant,
    run: Instant,
    cleanup: Instant,
    effective_cleanup: Instant,
}
impl TreeObservation {
    /// Last owning phase.
    pub fn phase(&self) -> Phase { self.phase }
    /// Actual process signal followed by exit-code query, including valid259.
    pub fn direct_exit(&self) -> Option<u32> { self.exit }
    /// Separately queried Job population; direct exit is not Job emptiness.
    pub fn active_processes(&self) -> Option<u32> { self.population }
    /// Sticky first issue and observation time.
    pub fn first_failure(&self) -> Option<FirstFailure> { self.first }
    /// Original local start/run/cleanup DATA; no outer supervisor is inferred.
    pub fn original_instants(&self) -> (Instant, Instant, Instant) { (self.started, self.run, self.cleanup) }
    /// Sticky min(original cleanup, first loss plus accepted finite drain).
    pub fn effective_cleanup(&self) -> Instant { self.effective_cleanup }
}

fn exact_policy(flags: u32) -> bool { flags == 0x2000 }

impl OwnedTree {
    /// Start ONE fixed Rust x64/CUI qualification-helper mode. The caller must
    /// separately admit this exact helper artifact/source/namespace and retain an
    /// independent ORIGINAL outer watchdog across every potentially blocking call.
    /// No inherited handles, stdio, arbitrary argv, shell or environment input.
    pub fn launch(executable: &Path, cwd: &Path, helper: QualificationHelper, limits: BudgetLimits) -> Result<Self, RetainedTree> {
        let started = Instant::now(); // FIRST entry action, before image/native I/O.
        let state = match State::new(started, limits) {
            Ok(state) => state,
            Err(issue) => State::refused(started, issue),
        };
        // Allocate the retained-owner box BEFORE any actual native process exists.
        let mut tree = Self { owner: Box::new(Owner { job: None, process: None, launch: None,
            state, create_entered: false, membership: None, exit: None, population: None }) };
        if tree.owner.state.first.is_some() { return Err(RetainedTree { tree }); }
        if let Err(issue) = tree.start(executable, cwd, helper) {
            tree.owner.state.observe_time(Instant::now(), false);
            tree.owner.state.record(issue, Instant::now());
            tree.owner.state.phase = Phase::Closing;
            return Err(RetainedTree { tree });
        }
        Ok(tree)
    }

    fn start(&mut self, executable: &Path, cwd: &Path, helper: QualificationHelper) -> Result<(), Issue> {
        self.start_allowed()?;
        self.owner.launch = Some(PreparedLaunch::prepare(executable, cwd, helper)?);
        self.start_allowed()?;
        self.owner.job = Some(native::create_job_owned().map_err(|_| Issue::NativeCall)?);
        self.owner.state.phase = Phase::JobCreated;
        self.start_allowed()?;
        let job = self.owner.job.as_ref().ok_or(Issue::NativeCall)?;
        native::configure_job(job).map_err(|_| Issue::NativeCall)?;
        match native::query_job(job, Query::Policy).map_err(|_| Issue::NativeCall)? {
            JobObservation::Policy(flags) if exact_policy(flags) => (),
            _ => return Err(Issue::Containment),
        }
        self.owner.state.phase = Phase::Configured;
        self.start_allowed()?;
        let job = self.owner.job.as_ref().ok_or(Issue::NativeCall)?;
        let attributes = native::make_job_attribute_list(job).map_err(|_| Issue::NativeCall)?;
        if !self.owner.state.permits_start(Instant::now()) { return Err(Issue::RunExpired); }
        self.owner.create_entered = true;
        self.owner.state.phase = Phase::CreateEntered;
        let result = native::create_process_in_job(&attributes, self.owner.launch.as_mut().ok_or(Issue::InvalidInput)?);
        // Attribute backing and borrowed Job stay live through creation and Delete.
        drop(attributes);
        self.owner.process = Some(result.map_err(|_| Issue::NativeCall)?);
        self.owner.state.phase = Phase::Created;
        self.start_allowed()?;
        let process = self.owner.process.as_ref().ok_or(Issue::NativeCall)?;
        if !process.complete() { return Err(Issue::NativeCall); }
        let job = self.owner.job.as_ref().ok_or(Issue::NativeCall)?;
        self.owner.membership = Some(native::is_owned_process_in_job(job, process).map_err(|_| Issue::NativeCall)?);
        if self.owner.membership != Some(true) { return Err(Issue::Containment); }
        match native::query_job(job, Query::Policy).map_err(|_| Issue::NativeCall)? {
            JobObservation::Policy(flags) if exact_policy(flags) => (),
            _ => return Err(Issue::Containment),
        }
        self.owner.state.phase = Phase::Running;
        self.start_allowed()
    }

    fn start_allowed(&mut self) -> Result<(), Issue> {
        let now = Instant::now();
        self.owner.state.observe_time(now, false);
        if !self.owner.state.permits_start(now) { return Err(Issue::RunExpired); }
        Ok(())
    }

    /// Query SAME actual owners. Zero-time waits are not universal hard bounds.
    /// Errors borrow this owner; they cannot detach or discard its resources.
    pub fn poll(&mut self) -> Result<TreeObservation, Issue> {
        let result = self.poll_inner();
        if let Err(issue) = result {
            self.owner.state.observe_time(Instant::now(), false);
            self.owner.state.record(issue, Instant::now());
            self.owner.state.phase = Phase::Closing;
        }
        result.map(|()| self.observation())
    }

    fn poll_inner(&mut self) -> Result<(), Issue> {
        let previously_complete = self.native_complete();
        self.owner.state.observe_time(Instant::now(), previously_complete);
        if self.owner.exit.is_none()
            && let Some(process) = self.owner.process.as_ref()
        {
            match native::poll_owned_process(process).map_err(|_| Issue::NativeCall)? {
                ProcessObservation::Pending => self.owner.state.observe_time(Instant::now(), false),
                ProcessObservation::Exited(code) => {
                    self.owner.exit = Some(code);
                    self.owner.state.direct_exit_observed(code, Instant::now());
                }
            }
        }
        if let Some(job) = self.owner.job.as_ref() {
            match native::query_job(job, Query::Population).map_err(|_| Issue::NativeCall)? {
                JobObservation::Population(active) => self.owner.population = Some(active),
                _ => return Err(Issue::NativeCall),
            }
        }
        let complete = self.native_complete();
        self.owner.state.observe_time(Instant::now(), complete);
        Ok(())
    }

    fn native_complete(&self) -> bool {
        if !self.owner.create_entered {
            return self.owner.job.is_none() || self.owner.population == Some(0);
        }
        self.owner.membership == Some(true) && self.owner.exit.is_some() && self.owner.population == Some(0)
    }

    /// Request OWN containment stop once; request success never implies exit/Job0.
    pub fn request_stop(&mut self) -> Result<(), Issue> {
        let now = Instant::now();
        self.owner.state.observe_time(now, false);
        self.owner.state.phase = Phase::Closing;
        if now >= self.owner.state.effective_cleanup { return Err(Issue::CleanupExpired); }
        if self.owner.state.stop_requested { return Ok(()); }
        self.owner.state.stop_requested = true;
        self.owner.state.record(Issue::StopRequested, now);
        let mut failed = false;
        if let Some(job) = self.owner.job.as_ref() { failed |= native::terminate_owned_job(job).is_err(); }
        if self.owner.membership != Some(true) && self.owner.exit.is_none()
            && let Some(process) = self.owner.process.as_ref()
        { failed |= native::terminate_owned_process(process).is_err(); }
        if failed { self.owner.state.record(Issue::NativeCall, Instant::now()); return Err(Issue::NativeCall); }
        Ok(())
    }

    /// No caller facts accepted: query again after closing new launches, require
    /// actual direct signal AND Job0 within original cleanup. NO workers exist
    /// in this minimum profile; this never certifies a nonempty capture request.
    pub fn settle(mut self) -> Result<TerminalTree, RetainedTree> {
        self.owner.state.phase = Phase::Closing;
        if self.poll().is_err() || !self.native_complete() || self.owner.state.cleanup_missed {
            return Err(RetainedTree { tree: self });
        }
        Ok(TerminalTree { tree: self })
    }

    /// Snapshot only. No activation, cleanup or original invocation authority.
    pub fn observation(&self) -> TreeObservation {
        TreeObservation { phase: self.owner.state.phase, exit: self.owner.exit, population: self.owner.population,
            first: self.owner.state.first, started: self.owner.state.started, run: self.owner.state.run, cleanup: self.owner.state.cleanup,
            effective_cleanup: self.owner.state.effective_cleanup }
    }

    #[cfg(test)]
    pub(super) fn reject_after_actual_create(mut self) -> RetainedTree {
        // Controlled validation refusal ONLY; not an injected Windows API failure.
        self.owner.state.record(Issue::Containment, Instant::now());
        self.owner.state.phase = Phase::Closing;
        RetainedTree { tree: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_accepts_only_kill_on_close_and_not_breakaway_or_other_flags() {
        assert!(exact_policy(0x2000));
        for flags in [0, 0x800, 0x1000, 0x2800, 0x3000, 0x2001] { assert!(!exact_policy(flags)); }
    }
}
