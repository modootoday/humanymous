//! Retained byte transport, not readiness, peer authority or process-tree proof.
//! Spawn/OS calls/final joins can block; independent outer supervision is required.
//! No Drop wait/kill/join: abandoning an owner forfeits settlement.
use super::{
    CaptureDeadlines, CaptureIssue, capture_attempt::worker::Worker, owned_child::OwnedDirectChild,
};
use std::{
    io,
    process::{Command, ExitStatus},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    time::Instant,
};
#[cfg(test)]
mod tests;
mod transport;
const CHUNK: usize = 4096;
const OUTPUT_SLOTS: usize = 16;
const STARTUP_MAX: usize = 65536;
const CONTROL_MAX: usize = 1024;
const CONTROL_SLOTS: usize = 64;

/// Closed queue/chunk profile; caller may choose smaller positive stream caps.
pub struct InteractiveLimits {
    stdout: usize,
    stderr: usize,
}
impl InteractiveLimits {
    /// Reject zero or lifetime stream caps above256KiB before spawn.
    pub fn new(stdout: usize, stderr: usize) -> io::Result<Self> {
        if stdout == 0 || stderr == 0 || stdout > 262144 || stderr > 262144 {
            return Err(io::Error::other("interactive limits refused"));
        }
        Ok(Self { stdout, stderr })
    }
}
/// Individual byte order is retained; no cross-stream ordering is implied.
#[derive(Clone, Copy)]
pub enum Stream {
    /// Child standard output.
    Stdout,
    /// Private child standard error.
    Stderr,
}
/// One raw bounded chunk. No raw Debug output or parser/authentication claim.
pub struct OutputChunk {
    storage: Box<[u8]>,
    used: usize,
}
impl OutputChunk {
    /// Actual received bytes, excluding unused fixed4096byte storage.
    pub fn bytes(&self) -> &[u8] {
        &self.storage[..self.used]
    }
}
/// Queue acceptance is not delivery. Each refusal returns the SAME input object.
#[must_use]
pub enum EnqueueResult {
    /// Accepted by the bounded writer queue; not necessarily written.
    Accepted,
    /// Queue full; sticky failure, never a retry-as-success path.
    Full(Box<[u8]>),
    /// Input explicitly closed; ownership not transferred.
    Closed(Box<[u8]>),
    /// Wrong order/size, expired run budget, prior input failure or writer unavailable.
    Failed(Box<[u8]>),
}
struct Input {
    startup: Option<SyncSender<Box<[u8]>>>,
    control: Option<SyncSender<Box<[u8]>>>,
    cancel: Arc<AtomicBool>,
    written: Arc<AtomicUsize>,
    worker: Option<Worker<()>>,
    startup_accepted: bool,
    accepted: usize,
    failed: bool,
    closed: bool,
}
impl Input {
    fn new() -> Self {
        Self {
            startup: None,
            control: None,
            cancel: Arc::new(AtomicBool::new(false)),
            written: Arc::new(AtomicUsize::new(0)),
            worker: None,
            startup_accepted: false,
            accepted: 0,
            failed: false,
            closed: false,
        }
    }
    fn offer(&mut self, bytes: Box<[u8]>, startup: bool, timely: bool) -> EnqueueResult {
        if self.closed {
            return EnqueueResult::Closed(bytes);
        }
        let order = if startup {
            !self.startup_accepted
        } else {
            self.startup_accepted
        };
        let cap = if startup { STARTUP_MAX } else { CONTROL_MAX };
        if self.failed
            || !timely
            || !order
            || bytes.is_empty()
            || bytes.len() > cap
            || self.accepted == usize::MAX
        {
            self.failed = true;
            return EnqueueResult::Failed(bytes);
        }
        let sender = if startup {
            &self.startup
        } else {
            &self.control
        };
        let result = match sender {
            Some(sender) => sender.try_send(bytes),
            None => Err(TrySendError::Disconnected(bytes)),
        };
        match result {
            Ok(()) => {
                self.accepted += 1;
                self.startup_accepted |= startup;
                EnqueueResult::Accepted
            }
            Err(TrySendError::Full(bytes)) => {
                self.failed = true;
                EnqueueResult::Full(bytes)
            }
            Err(TrySendError::Disconnected(bytes)) => {
                self.failed = true;
                EnqueueResult::Failed(bytes)
            }
        }
    }
    fn close(&mut self) {
        self.closed = true;
        drop(self.startup.take());
        drop(self.control.take());
    }
}
struct Output {
    worker: Worker<()>,
    chunks: Receiver<OutputChunk>,
}

/// Distinct real observations; queue emptiness, EOF, completion and join differ.
pub struct InteractiveProgress {
    /// Actual held child status was observed.
    pub direct_child_exited: bool,
    /// New offers disabled, independently of writer completion.
    pub input_closed: bool,
    /// Queue acceptances including startup, never a delivery claim.
    pub input_messages_accepted: usize,
    /// Entire messages actually written and flushed, including startup.
    pub input_messages_written: usize,
    /// Writer returned successful close/drain, distinct from join.
    pub input_drained: bool,
    /// Actual successful read EOF, not inferred from child status.
    pub stdout_eof: bool,
    /// Actual successful read EOF, not inferred from queue emptiness.
    pub stderr_eof: bool,
    /// Started worker count, including partial setup.
    pub workers_started: u8,
    /// Received outcomes including failures, not actual joins.
    pub workers_completed: u8,
    /// Actual consumed JoinHandles, including joined panics.
    pub workers_joined: u8,
    /// Finite sticky redacted failures; no raw content.
    pub issues: Vec<CaptureIssue>,
}
/// Prelaunch refusal and retained partial startup are distinct.
#[must_use]
pub enum InteractiveStartFailure {
    /// OS spawn did not return an actual child.
    NotStarted(io::Error),
    /// Keep the actual child and every started worker.
    Retained(RetainedInteractiveFailure),
}
/// Same owner on partial setup, execution error or unsettled finalization.
#[must_use]
pub struct RetainedInteractiveFailure {
    owner: Box<InteractiveAttempt>,
}
impl RetainedInteractiveFailure {
    /// Recover unchanged custody and original clocks.
    pub fn into_owner(self) -> InteractiveAttempt {
        *self.owner
    }
}
/// ONE existing native child owner plus every started retained IO worker.
#[must_use]
pub struct InteractiveAttempt {
    child: OwnedDirectChild,
    run_until: Instant,
    input: Input,
    stdout: Option<Output>,
    stderr: Option<Output>,
}
impl InteractiveAttempt {
    /// Start may block in OS. Caller establishes admission/fixed env/cwd and
    /// independent supervision. Startup is deferred until an explicit offer.
    pub fn start(
        command: &mut Command,
        deadlines: CaptureDeadlines,
        limits: InteractiveLimits,
    ) -> Result<Self, InteractiveStartFailure> {
        let run_until = deadlines.run_until();
        let child = OwnedDirectChild::spawn(command, deadlines)
            .map_err(InteractiveStartFailure::NotStarted)?;
        let mut owner = Self {
            child,
            run_until,
            input: Input::new(),
            stdout: None,
            stderr: None,
        };
        if owner.start_workers(limits).is_err() {
            owner.child.record(CaptureIssue::SetupFailed);
            owner.cancel_input();
            return Err(InteractiveStartFailure::Retained(owner.retained()));
        }
        owner.poll();
        Ok(owner)
    }
    fn start_workers(&mut self, limits: InteractiveLimits) -> io::Result<()> {
        self.stdout = Some(transport::output(
            self.child.take_stdout().ok_or_else(missing)?,
            limits.stdout,
        )?);
        self.stderr = Some(transport::output(
            self.child.take_stderr().ok_or_else(missing)?,
            limits.stderr,
        )?);
        let stdin = self.child.take_input().ok_or_else(missing)?;
        let (startup, startup_rx) = mpsc::sync_channel(1);
        let (control, control_rx) = mpsc::sync_channel(CONTROL_SLOTS);
        self.input.startup = Some(startup);
        self.input.control = Some(control);
        let cancel = self.input.cancel.clone();
        let written = self.input.written.clone();
        let until = self.run_until;
        self.input.worker = Some(Worker::start("interactive-stdin", move || {
            transport::input(stdin, startup_rx, control_rx, cancel, written, until)
        })?);
        Ok(())
    }
    /// Actual held child PID. Observation, never peer/native authority.
    pub fn pid(&self) -> u32 {
        self.child.pid()
    }
    /// One opaque startup up to65536bytes. UTF8/framing/accepted-peer checks are
    /// the consumer's responsibility. Boxed input has no excess Vec capacity.
    pub fn try_startup(&mut self, bytes: Box<[u8]>) -> EnqueueResult {
        self.offer(bytes, true)
    }
    /// FIFO controls up to1024bytes,64pending, after startup acceptance. Other
    /// stream/child issues do not automatically cancel accepted input; consumers
    /// must inspect poll issues and explicitly cancel_input on protocol failure.
    pub fn try_control(&mut self, bytes: Box<[u8]>) -> EnqueueResult {
        self.offer(bytes, false)
    }
    fn offer(&mut self, bytes: Box<[u8]>, startup: bool) -> EnqueueResult {
        self.poll();
        let result = self
            .input
            .offer(bytes, startup, Instant::now() < self.run_until);
        if self.input.failed {
            self.child.record(CaptureIssue::InputFailed);
        }
        result
    }
    /// Disable new offers and disconnect senders without a queue sentinel.
    /// Accepted input drains FIFO before stdin closes; this is not drain proof.
    pub fn close_input(&mut self) {
        self.input.close();
    }
    /// Independent cancellation flag; an in-flight OS write is not preempted.
    /// Incomplete accepted input remains a sticky failure, never delivery.
    pub fn cancel_input(&mut self) {
        self.input.failed = true;
        self.input.cancel.store(true, Ordering::Release);
        self.input.close();
        self.child.record(CaptureIssue::InputFailed);
    }
    /// One owned raw chunk. None means unavailable, never EOF. Poll and final
    /// settlement never consume or silently discard payload chunks.
    pub fn take_output(&mut self, stream: Stream) -> Option<OutputChunk> {
        match stream {
            Stream::Stdout => &self.stdout,
            Stream::Stderr => &self.stderr,
        }
        .as_ref()
        .and_then(|output| output.chunks.try_recv().ok())
    }
    /// Observe child and independent fixed result channels only. No payload
    /// drain, wait, mutex, write, stop, join or caller-supplied clock.
    pub fn poll(&mut self) -> InteractiveProgress {
        self.child.poll_child();
        if let Some(worker) = &mut self.input.worker {
            worker.poll();
            if worker.failed() {
                self.input.failed = true;
            }
        }
        if self.input.failed {
            self.child.record(CaptureIssue::InputFailed);
        }
        for (output, issue) in [
            (&mut self.stdout, CaptureIssue::StdoutFailed),
            (&mut self.stderr, CaptureIssue::StderrFailed),
        ] {
            if let Some(output) = output {
                output.worker.poll();
                if output.worker.failed() {
                    self.child.record(issue);
                }
            }
        }
        let complete = self.child.status().is_some()
            && self.input.worker.as_ref().is_some_and(Worker::succeeded)
            && self.stdout.as_ref().is_some_and(|o| o.worker.succeeded())
            && self.stderr.as_ref().is_some_and(|o| o.worker.succeeded());
        let (started, completed, joined) = self.worker_counts();
        let settled = self.child.status().is_some()
            && started == joined
            && self.child.retained_pipes() == [false; 3];
        self.child.observe_completion(complete, settled);
        InteractiveProgress {
            direct_child_exited: self.child.status().is_some(),
            input_closed: self.input.closed,
            input_messages_accepted: self.input.accepted,
            input_messages_written: self.input.written.load(Ordering::Acquire),
            input_drained: self.input.worker.as_ref().is_some_and(Worker::succeeded),
            stdout_eof: self.stdout.as_ref().is_some_and(|o| o.worker.succeeded()),
            stderr_eof: self.stderr.as_ref().is_some_and(|o| o.worker.succeeded()),
            workers_started: started,
            workers_completed: completed,
            workers_joined: joined,
            issues: self.child.issues().to_vec(),
        }
    }
    fn worker_counts(&self) -> (u8, u8, u8) {
        let mut counts = (0, 0, 0);
        for worker in [
            self.input.worker.as_ref(),
            self.stdout.as_ref().map(|o| &o.worker),
            self.stderr.as_ref().map(|o| &o.worker),
        ]
        .into_iter()
        .flatten()
        {
            counts.0 += 1;
            counts.1 += u8::from(worker.succeeded() || worker.failed());
            counts.2 += u8::from(worker.joined());
        }
        counts
    }
    /// Borrowed direct-child stop request, potentially blocking. Not exit/tree
    /// proof; independent outer supervision remains necessary.
    pub fn request_stop(&mut self) -> io::Result<()> {
        self.child.request_stop()
    }
    fn retained(self) -> RetainedInteractiveFailure {
        RetainedInteractiveFailure {
            owner: Box::new(self),
        }
    }
    /// Pending child/workers return SAME owner. Final joins may still block even
    /// after is_finished. Unread chunks remain owned by the settled receipt.
    pub fn settle_joined(mut self) -> Result<SettledInteractiveChild, RetainedInteractiveFailure> {
        self.poll();
        if self.child.status().is_none()
            || [
                self.input.worker.as_ref(),
                self.stdout.as_ref().map(|o| &o.worker),
                self.stderr.as_ref().map(|o| &o.worker),
            ]
            .into_iter()
            .flatten()
            .any(|worker| !worker.join_ready())
        {
            return Err(self.retained());
        }
        for worker in [
            self.input.worker.as_mut(),
            self.stdout.as_mut().map(|o| &mut o.worker),
            self.stderr.as_mut().map(|o| &mut o.worker),
        ]
        .into_iter()
        .flatten()
        {
            if worker.join() {
                self.child.record(CaptureIssue::WorkerPanicked);
            }
        }
        let _closed = self.child.close_pipes_after_exit();
        let progress = self.poll();
        let Some(status) = self.child.status() else {
            return Err(self.retained());
        };
        Ok(SettledInteractiveChild {
            pid: self.pid(),
            status,
            progress,
            stdout: self.stdout.map(|o| o.chunks),
            stderr: self.stderr.map(|o| o.chunks),
        })
    }
}
/// Actual joined direct-child receipt retaining unread raw output. No tree proof.
pub struct SettledInteractiveChild {
    pid: u32,
    status: ExitStatus,
    progress: InteractiveProgress,
    stdout: Option<Receiver<OutputChunk>>,
    stderr: Option<Receiver<OutputChunk>>,
}
impl SettledInteractiveChild {
    /// Actual direct-child PID observed at spawn, not authorization.
    pub fn pid(&self) -> u32 {
        self.pid
    }
    /// Actual observed OS status.
    pub fn status(&self) -> ExitStatus {
        self.status
    }
    /// Final separate EOF/result/join facts and sticky issues.
    pub fn progress(&self) -> &InteractiveProgress {
        &self.progress
    }
    /// Drain retained output after join. None is queue exhaustion only.
    pub fn take_output(&mut self, stream: Stream) -> Option<OutputChunk> {
        match stream {
            Stream::Stdout => &self.stdout,
            Stream::Stderr => &self.stderr,
        }
        .as_ref()
        .and_then(|rx| rx.try_recv().ok())
    }
}
fn missing() -> io::Error {
    io::Error::other("interactive pipe unavailable")
}
