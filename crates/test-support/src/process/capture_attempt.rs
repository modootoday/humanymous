//! Retained direct-child capture under an independent caller/host supervisor.
//! Start, stop requests and final joins may block in the OS. Deadlines are
//! observation/request thresholds, never universal wall-clock return guarantees.
//! No timeout path discards workers. Callers must retain returned owners and
//! explicitly settle them; abandoning an owner is Unknown, never cleanup proof.
use std::{
    io::{self, Read, Write},
    process::{Child, Command, ExitStatus},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread::{self, JoinHandle},
    time::Instant,
};

mod worker;
use worker::Worker;

/// One pair of absolute original deadlines, with no relative-time renewal.
pub struct CaptureDeadlines {
    run: Instant,
    cleanup: Instant,
    run_met: bool,
    cleanup_met: bool,
}
impl CaptureDeadlines {
    /// Reject reversed deadlines. These values are caller claims, not authority.
    pub fn from_original(run: Instant, cleanup: Instant) -> io::Result<Self> {
        if cleanup < run {
            return Err(io::Error::other("reversed capture deadlines"));
        }
        Ok(Self {
            run,
            cleanup,
            run_met: false,
            cleanup_met: false,
        })
    }

    fn observe(
        &mut self,
        now: Instant,
        completed: bool,
        settled: bool,
    ) -> [Option<CaptureIssue>; 2] {
        self.run_met |= now < self.run && completed;
        self.cleanup_met |= now < self.cleanup && settled;
        [
            (now >= self.run && !self.run_met).then_some(CaptureIssue::RunExpired),
            (now >= self.cleanup && !self.cleanup_met).then_some(CaptureIssue::CleanupExpired),
        ]
    }
}

/// Finite per-stream and aggregate allocation limits, checked before launch.
pub struct CaptureLimits {
    stdout: usize,
    stderr: usize,
}
impl CaptureLimits {
    /// Both streams must have a nonzero finite limit, including overflow sentinel.
    /// Caller additionally controls its overall memory and concurrent-attempt budget.
    pub fn new(stdout: usize, stderr: usize) -> io::Result<Self> {
        if stdout == 0
            || stderr == 0
            || stdout
                .checked_add(stderr)
                .and_then(|sum| sum.checked_add(4098))
                .is_none_or(|sum| sum > isize::MAX as usize)
        {
            return Err(io::Error::other("invalid capture byte limits"));
        }
        Ok(Self { stdout, stderr })
    }
}

/// Sticky redacted execution/observation failures; no raw paths or pipe content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureIssue {
    /// Original run threshold elapsed before full child/IO completion was observed.
    RunExpired,
    /// Original cleanup threshold elapsed before actual final joins were observed.
    CleanupExpired,
    /// Partial pipe or worker creation failed after the actual child was retained.
    SetupFailed,
    /// Input write failed, or its worker outcome was unavailable.
    InputFailed,
    /// Stdout failed or exceeded its declared byte limit.
    StdoutFailed,
    /// Stderr failed or exceeded its declared byte limit.
    StderrFailed,
    /// An actual joined worker panicked.
    WorkerPanicked,
    /// Direct-child status observation failed.
    ChildObservationFailed,
    /// A stop request was made; even an eventual exit0 is not natural success.
    ForcedStop,
    /// Actual direct-child kill request failed.
    StopFailed,
    /// The observed direct-child exit was nonzero/unsuccessful.
    UnsuccessfulExit,
}

/// Observations only. Child exit is independent of EOF, joins and descendants.
pub struct CaptureProgress {
    /// Actual direct child has an observed exit status.
    pub direct_child_exited: bool,
    /// A complete successful stdin writer result was received.
    pub input_completed: bool,
    /// Successful EOF within each cap was received for both output streams.
    pub output_eof: bool,
    /// Every worker that was started has actually been joined.
    pub workers_joined: bool,
    /// Errors remain sticky across later successful OS observations.
    pub issues: Vec<CaptureIssue>,
}

/// Actual settled direct-child result; this never certifies process-tree cleanup.
pub struct CapturedDirectChild {
    /// Actual direct-child PID captured from the owned Child, not authorization.
    pid: u32,
    /// Actual observed direct-child status.
    status: ExitStatus,
    /// Bounded private stdout; empty if that stream failed.
    stdout: Vec<u8>,
    /// Bounded private stderr; empty if that stream failed.
    stderr: Vec<u8>,
    /// Sticky failures, including forced stop and missed original thresholds.
    issues: Vec<CaptureIssue>,
}
impl CapturedDirectChild {
    /// Actual captured PID; not a reusable native identity or authorization.
    pub fn pid(&self) -> u32 {
        self.pid
    }
    /// Actual observed direct-child exit status.
    pub fn status(&self) -> ExitStatus {
        self.status
    }
    /// Complete bounded private stdout, or empty on stream failure.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    /// Complete bounded private stderr, or empty on stream failure.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
    /// Sticky redacted failures; caller cannot replace actual receipt fields.
    pub fn issues(&self) -> &[CaptureIssue] {
        &self.issues
    }
    /// Natural observed successful execution with successful IO and actual joins.
    /// Descendant/tree coverage remains Unknown, including when this returns true.
    pub fn execution_succeeded(&self) -> bool {
        self.status.success() && self.issues.is_empty()
    }
}

/// A prelaunch refusal owns no spawned child; partial starts retain their owner.
#[must_use]
pub enum CaptureStartFailure {
    /// Input/budget/spawn refused before a child was returned by the OS.
    NotStarted(io::Error),
    /// Actual child and every started worker must be retained by the caller.
    Retained(RetainedCaptureFailure),
}

/// Keep the SAME owner on partial setup, status error or unsettled finalization.
#[must_use]
pub struct RetainedCaptureFailure {
    owner: Box<CaptureAttempt>,
}
impl RetainedCaptureFailure {
    /// Recover the actual child/worker owner; no timeout or authority is renewed.
    pub fn into_owner(self) -> CaptureAttempt {
        *self.owner
    }
}

/// Actual Child and all started IO JoinHandles, including failed/expired attempts.
/// No Drop wait/kill/join and no timeout detachment. Caller abandonment forfeits
/// settlement; safe Rust cannot enforce linear consumption of this must_use owner.
#[must_use]
pub struct CaptureAttempt {
    child: Child,
    pid: u32,
    status: Option<ExitStatus>,
    input: Option<Worker<()>>,
    stdout: Option<Worker<Vec<u8>>>,
    stderr: Option<Worker<Vec<u8>>>,
    deadlines: CaptureDeadlines,
    issues: Vec<CaptureIssue>,
}
impl CaptureAttempt {
    /// Synchronous OS spawn can block: caller MUST provide independent supervision.
    /// Drainers start before the bounded stdin worker. Partial errors retain owner.
    /// This always requests all three pipes and performs no synchronous input write.
    pub fn start(
        command: &mut Command,
        input: Vec<u8>,
        limits: CaptureLimits,
        deadlines: CaptureDeadlines,
    ) -> Result<Self, CaptureStartFailure> {
        if input.len() > 4096 || Instant::now() >= deadlines.run {
            return Err(CaptureStartFailure::NotStarted(io::Error::other(
                "capture input or original run budget refused",
            )));
        }
        let child = super::piped_child(command, true).map_err(CaptureStartFailure::NotStarted)?;
        let mut owner = Self {
            pid: child.id(),
            child,
            status: None,
            input: None,
            stdout: None,
            stderr: None,
            deadlines,
            issues: Vec::new(),
        };
        if owner.start_workers(input, limits).is_err() {
            owner.record(CaptureIssue::SetupFailed);
            return Err(CaptureStartFailure::Retained(RetainedCaptureFailure {
                owner: Box::new(owner),
            }));
        }
        owner.poll();
        Ok(owner)
    }

    fn start_workers(&mut self, input: Vec<u8>, limits: CaptureLimits) -> io::Result<()> {
        let stdout = self.child.stdout.take().ok_or_else(missing_pipe)?;
        self.stdout = Some(Worker::start("capture-stdout", move || {
            read_capped(stdout, limits.stdout)
        })?);
        let stderr = self.child.stderr.take().ok_or_else(missing_pipe)?;
        self.stderr = Some(Worker::start("capture-stderr", move || {
            read_capped(stderr, limits.stderr)
        })?);
        let mut stdin = self.child.stdin.take().ok_or_else(missing_pipe)?;
        self.input = Some(Worker::start("capture-stdin", move || {
            stdin.write_all(&input)
        })?);
        Ok(())
    }

    fn record(&mut self, issue: CaptureIssue) {
        if !self.issues.contains(&issue) {
            self.issues.push(issue);
        }
    }

    /// Observe child status and worker channels only. No wait/join/write/kill,
    /// blocking mutex, automatic stop or fresh deadline is performed here. The
    /// observation clock is sampled internally AFTER gathering the actual facts.
    pub fn poll(&mut self) -> CaptureProgress {
        if self.status.is_none() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        self.record(CaptureIssue::UnsuccessfulExit);
                    }
                    self.status = Some(status);
                }
                Ok(None) => (),
                Err(_) => self.record(CaptureIssue::ChildObservationFailed),
            }
        }
        if let Some(worker) = &mut self.input {
            worker.poll();
            if worker.failed() {
                self.record(CaptureIssue::InputFailed);
            }
        }
        if let Some(worker) = &mut self.stdout {
            worker.poll();
            if worker.failed() {
                self.record(CaptureIssue::StdoutFailed);
            }
        }
        if let Some(worker) = &mut self.stderr {
            worker.poll();
            if worker.failed() {
                self.record(CaptureIssue::StderrFailed);
            }
        }
        let completed = self.status.is_some()
            && self.input.as_ref().is_some_and(Worker::succeeded)
            && self.stdout.as_ref().is_some_and(Worker::succeeded)
            && self.stderr.as_ref().is_some_and(Worker::succeeded);
        self.observe_deadlines(Instant::now(), completed);
        self.progress()
    }

    fn observe_deadlines(&mut self, now: Instant, completed: bool) {
        let settled = self.status.is_some()
            && self.workers_joined()
            && self.child.stdin.is_none()
            && self.child.stdout.is_none()
            && self.child.stderr.is_none();
        for issue in self
            .deadlines
            .observe(now, completed, settled)
            .into_iter()
            .flatten()
        {
            self.record(issue);
        }
    }

    fn workers_joined(&self) -> bool {
        self.input.as_ref().is_none_or(Worker::joined)
            && self.stdout.as_ref().is_none_or(Worker::joined)
            && self.stderr.as_ref().is_none_or(Worker::joined)
    }

    fn progress(&self) -> CaptureProgress {
        CaptureProgress {
            direct_child_exited: self.status.is_some(),
            input_completed: self.input.as_ref().is_some_and(Worker::succeeded),
            output_eof: self.stdout.as_ref().is_some_and(Worker::succeeded)
                && self.stderr.as_ref().is_some_and(Worker::succeeded),
            workers_joined: self.workers_joined(),
            issues: self.issues.clone(),
        }
    }

    /// Request direct-child termination through the actual Child. The OS call may
    /// block: invoke only under independent supervision. This is NOT exit/reap,
    /// reader settlement or descendant termination. Error retains this borrowed owner.
    pub fn request_stop(&mut self) -> io::Result<()> {
        self.record(CaptureIssue::ForcedStop);
        if self.status.is_some() {
            return Ok(());
        }
        if let Err(error) = self.child.kill() {
            self.record(CaptureIssue::StopFailed);
            return Err(error);
        }
        Ok(())
    }

    /// Final joins may block even after is_finished; caller MUST keep independent
    /// outer supervision. If the child/workers are still pending, returns SAME
    /// owner immediately. No Child::wait, new timeout or discarded JoinHandle.
    /// Successful settlement returns actual joined facts, possibly failed execution.
    pub fn settle_joined(mut self) -> Result<CapturedDirectChild, RetainedCaptureFailure> {
        self.poll();
        let ready = self.status.is_some()
            && self.input.as_ref().is_none_or(Worker::join_ready)
            && self.stdout.as_ref().is_none_or(Worker::join_ready)
            && self.stderr.as_ref().is_none_or(Worker::join_ready);
        if !ready {
            return Err(RetainedCaptureFailure {
                owner: Box::new(self),
            });
        }
        let panicked = self.input.as_mut().is_some_and(Worker::join)
            | self.stdout.as_mut().is_some_and(Worker::join)
            | self.stderr.as_mut().is_some_and(Worker::join);
        if panicked {
            self.record(CaptureIssue::WorkerPanicked);
        }
        let status = match self.status {
            Some(status) => status,
            None => {
                return Err(RetainedCaptureFailure {
                    owner: Box::new(self),
                });
            }
        };
        // Partial setup remains a failed execution. Pipes without workers are
        // closed only after the actual child is observed exited, never as EOF proof.
        drop(self.child.stdin.take());
        drop(self.child.stdout.take());
        drop(self.child.stderr.take());
        self.poll();
        Ok(CapturedDirectChild {
            pid: self.pid,
            status,
            stdout: self
                .stdout
                .and_then(Worker::into_output)
                .unwrap_or_default(),
            stderr: self
                .stderr
                .and_then(Worker::into_output)
                .unwrap_or_default(),
            issues: self.issues,
        })
    }
}

fn missing_pipe() -> io::Error {
    io::Error::other("capture pipe unavailable")
}
fn read_capped(output: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    output.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::other("capture stream limit exceeded"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
