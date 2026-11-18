//! L4 consumer of the sole shared interactive owner. Launch requires the
//! creator-channel admission (SoT dotnet-protected-host-launch 3.2).
use super::protocol::{Control, Frame, Ingress};
use humanymous_platform::testing::process::{
    CaptureDeadlines, EnqueueResult, InteractiveAttempt, InteractiveLimits, InteractiveProgress,
    InteractiveStartFailure, SettledInteractiveChild, Stream,
};
use serde_json::Value;
use std::{
    io::{self, ErrorKind},
    path::Path,
    process::Command,
    sync::mpsc::Receiver,
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// Path variables that are never admission. Present values must equal the
/// admitted path, or start refuses.
const PATH_VARIABLES: [&str; 2] = ["HUMANYMOUS_DOTNET_PILOT", "HUMANYMOUS_DOTNET_AUDIT_DIR"];
/// INTEGRATION.md stream contract: 256 KiB per stream.
const STREAM_LIMIT: usize = 262_144;

/// Harness transport custody only; outer Job/scope cleanup stays with the provider.
pub struct Host {
    owner: InteractiveAttempt,
    ingress: Ingress,
    frames: Receiver<Frame>,
    run_until: Instant,
    cleanup_until: Instant,
    startup_sent: bool,
    stopped: bool,
    stdout_ended: bool,
    failed: bool,
    stop_requested: bool,
    pending_http: Option<JoinHandle<Vec<u8>>>,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("retained host custody; cleanup Unknown")
    }
}

impl Host {
    /// Borrow the one process-wide admission and refuse BEFORE `Command` on any
    /// missing gate. No current-user, ambient TEMP or environment-path fallback.
    pub fn start() -> io::Result<Self> {
        let admission = super::launch::admission()?;
        let (run_until, cleanup_until) = admission.deadlines()?;
        for (name, admitted) in PATH_VARIABLES
            .into_iter()
            .zip([admission.apphost(), admission.audit_root()])
        {
            if std::env::var_os(name).is_some_and(|value| Path::new(&value) != admitted) {
                return Err(io::Error::new(
                    ErrorKind::PermissionDenied,
                    "path variable differs from admission",
                ));
            }
        }
        // Lexical containment was admitted; also refuse a link that escapes it.
        let apphost = admission.apphost().canonicalize()?;
        let root = admission.apphost_root().canonicalize()?;
        if apphost == root || !apphost.starts_with(&root) {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "apphost resolves outside its admitted root",
            ));
        }
        if Instant::now() >= run_until {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                "original run budget exhausted",
            ));
        }
        let mut command = Command::new(admission.apphost());
        command
            .env_clear()
            .envs(admission.fixed_environment())
            .current_dir(admission.private_work());
        let deadlines = CaptureDeadlines::from_original(run_until, cleanup_until)?;
        let limits = InteractiveLimits::new(STREAM_LIMIT, STREAM_LIMIT)?;
        match InteractiveAttempt::start(&mut command, deadlines, limits) {
            Ok(owner) => Ok(Self::from_retained(owner)),
            Err(InteractiveStartFailure::NotStarted(error)) => Err(error),
            // Partial start keeps its owner: the same Host is returned already
            // failed, so the caller's cancel/finish path retains custody.
            Err(InteractiveStartFailure::Retained(retained)) => {
                let mut host = Self::from_retained(retained.into_owner());
                host.failed = true;
                Ok(host)
            }
        }
    }

    /// Consume an already owned native transport and borrow its exact thresholds.
    /// The caller must separately retain actual admission/Job/cleanup custody.
    pub fn from_retained(owner: InteractiveAttempt) -> Self {
        let (run_until, cleanup_until) = owner.original_deadlines();
        let (ingress, frames) = Ingress::new(owner.pid());
        Self {
            owner,
            ingress,
            frames,
            run_until,
            cleanup_until,
            startup_sent: false,
            stopped: false,
            stdout_ended: false,
            failed: false,
            stop_requested: false,
            pending_http: None,
        }
    }

    /// Admitted private audit root; never an environment locator.
    pub fn audit_root(&self) -> io::Result<&'static Path> {
        Ok(super::launch::admission()?.audit_root())
    }

    pub fn pid(&self) -> u32 {
        self.owner.pid()
    }

    pub fn run_deadline(&self) -> Instant {
        self.run_until
    }

    fn fail(&mut self, reason: &'static str) -> io::Error {
        self.failed = true;
        self.owner.cancel_input();
        io::Error::other(reason)
    }

    /// Poll bounded chunks without waiting, joining, logging stderr or losing owner.
    pub fn poll(&mut self) -> io::Result<InteractiveProgress> {
        let mut stdout_empty = false;
        for _ in 0..16 {
            let Some(chunk) = self.owner.take_output(Stream::Stdout) else {
                stdout_empty = true;
                break;
            };
            if self.ingress.offer_chunk(chunk.bytes()).is_err() {
                return Err(self.fail("host output protocol refused"));
            }
        }
        // The native worker enforces the lifetime bound. Private diagnostics are
        // intentionally consumed without publication; failures stay sticky there.
        for _ in 0..16 {
            if self.owner.take_output(Stream::Stderr).is_none() {
                break;
            }
        }
        let progress = self.owner.poll();
        if self.failed || !progress.issues.is_empty() {
            return Err(self.fail("native host transport failed"));
        }
        // A concurrent worker may publish its final chunk after our first drain.
        // Observe result first, then probe the now-completed producer's queue.
        if progress.stdout_eof && stdout_empty && !self.stdout_ended {
            while let Some(chunk) = self.owner.take_output(Stream::Stdout) {
                if self.ingress.offer_chunk(chunk.bytes()).is_err() {
                    return Err(self.fail("host final output refused"));
                }
            }
            if self.ingress.eof().is_err() {
                return Err(self.fail("host EOF before complete stop"));
            }
            self.stdout_ended = true;
        }
        Ok(progress)
    }

    /// Original absolute deadline, never a fresh per-frame timeout.
    pub fn next_frame(&mut self) -> io::Result<Option<Value>> {
        self.poll()?;
        if Instant::now() >= self.run_until {
            return Err(self.fail("original host run budget exhausted"));
        }
        self.frames
            .try_recv()
            .ok()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|_| self.fail("host frame projection failed"))
    }

    /// One bounded startup followed only by the closed control vocabulary.
    pub fn try_send(&mut self, value: Value) -> io::Result<()> {
        if self.failed || self.stopped || Instant::now() >= self.run_until {
            return Err(self.fail("host input unavailable"));
        }
        let stop = if self.startup_sent {
            let control: Control = serde_json::from_value(value.clone())
                .map_err(|_| self.fail("unknown host control"))?;
            matches!(control, Control::Stop)
        } else {
            false
        };
        let mut bytes =
            serde_json::to_vec(&value).map_err(|_| self.fail("host input encoding failed"))?;
        bytes.push(b'\n');
        let result = if self.startup_sent {
            self.owner.try_control(bytes.into_boxed_slice())
        } else {
            self.owner.try_startup(bytes.into_boxed_slice())
        };
        if !matches!(result, EnqueueResult::Accepted) {
            return Err(self.fail("host input rejected without retry"));
        }
        self.startup_sent = true;
        if stop {
            self.stopped = true;
            self.owner.close_input();
        }
        Ok(())
    }

    /// Cleanup requests retain custody and do not establish exit/Job settlement.
    pub fn cancel(&mut self) {
        self.failed = true;
        self.owner.cancel_input();
    }

    pub fn request_stop(&mut self) -> io::Result<()> {
        if self.stop_requested {
            return Ok(());
        }
        self.stop_requested = true;
        self.owner.request_stop()
    }

    pub fn frame(&mut self, kind: &str) -> Value {
        loop {
            if let Some(frame) = self.next_frame().expect("retained host frame failed") {
                if frame["kind"] == kind {
                    if kind == "ready" {
                        let port = frame["address"]
                            .as_str()
                            .and_then(|address| address.strip_prefix("http://127.0.0.1:"))
                            .and_then(|port| port.parse::<u16>().ok());
                        assert!(port.is_some_and(|port| port != 0), "host endpoint refused");
                    }
                    return frame;
                }
                assert_eq!(
                    frame["kind"], "application-contact",
                    "unexpected host frame"
                );
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    pub fn send(&mut self, value: Value) {
        self.try_send(value).expect("retained host input failed");
    }

    pub fn counters(&mut self) -> Value {
        self.send(serde_json::json!({"operation":"counters"}));
        self.frame("counters")
    }

    /// The GC-overlap request belongs to this Host, including assertion failures.
    pub fn spawn_http(
        &mut self,
        operation: impl FnOnce() -> Vec<u8> + Send + 'static,
    ) -> io::Result<()> {
        if self.pending_http.is_some() || self.failed || Instant::now() >= self.run_until {
            return Err(self.fail("HTTP worker unavailable"));
        }
        let worker = std::thread::Builder::new()
            .name("dotnet-host-http".into())
            .spawn(operation)
            .map_err(|_| self.fail("HTTP worker start failed"))?;
        self.pending_http = Some(worker);
        Ok(())
    }

    /// Whether the HTTP worker finished by `until` (capped by the original run
    /// deadline). Never joins; a still-running worker stays in custody.
    pub fn http_finished_by(&mut self, until: Instant) -> io::Result<bool> {
        let until = until.min(self.run_until);
        loop {
            let Some(worker) = self.pending_http.as_ref() else {
                return Err(self.fail("HTTP worker missing"));
            };
            if worker.is_finished() {
                return Ok(true);
            }
            if Instant::now() >= until {
                return Ok(false);
            }
            self.poll()?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn join_http(&mut self) -> io::Result<Vec<u8>> {
        loop {
            let Some(worker) = self.pending_http.as_ref() else {
                return Err(self.fail("HTTP worker missing"));
            };
            if worker.is_finished() {
                let Some(worker) = self.pending_http.take() else {
                    return Err(self.fail("HTTP worker custody missing"));
                };
                let result = worker.join();
                // Joining is an actual fact even when adjudication is too late.
                // Never return bytes as a timely stage after the original run.
                if Instant::now() >= self.run_until {
                    return Err(self.fail("original HTTP worker budget exhausted after join"));
                }
                return result.map_err(|_| self.fail("HTTP worker panicked"));
            }
            if Instant::now() >= self.run_until {
                return Err(self.fail("original HTTP worker budget exhausted"));
            }
            self.poll()?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Poll/settle only against the original cleanup threshold; failure retains
    /// custody. This proves direct worker facts, never outer Job/tree cleanup.
    pub fn finish(mut self) -> Result<SettledInteractiveChild, Box<Self>> {
        loop {
            if self.poll().is_err() {
                self.cancel();
                // Stop is a request; errors remain on the retained native owner.
                let _ = self.request_stop();
            }
            // A failed case may leave its HTTP worker unconsumed. Join only a
            // finished worker; otherwise retain it through the original cleanup
            // threshold. No scoped implicit join or detached failure path.
            if self
                .pending_http
                .as_ref()
                .is_some_and(JoinHandle::is_finished)
                && let Some(worker) = self.pending_http.take()
            {
                let _result = worker.join();
                let _error = self.fail("HTTP worker result was not consumed");
            }
            match self.settle() {
                Ok(receipt) => return Ok(receipt),
                Err(retained) => self = *retained,
            }
            if Instant::now() >= self.cleanup_until {
                return Err(Box::new(self));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// A failure returns SAME harness and native owner, including parser state.
    /// Successful resource settlement still requires checking the receipt's facts.
    pub fn settle(self) -> Result<SettledInteractiveChild, Box<Self>> {
        if self.pending_http.is_some() || (!self.failed && !self.stdout_ended) {
            return Err(Box::new(self));
        }
        let Self {
            owner,
            ingress,
            frames,
            run_until,
            cleanup_until,
            startup_sent,
            stopped,
            stdout_ended,
            failed,
            stop_requested,
            pending_http,
        } = self;
        match owner.settle_joined() {
            Ok(receipt) => Ok(receipt),
            Err(retained) => Err(Box::new(Self {
                owner: retained.into_owner(),
                ingress,
                frames,
                run_until,
                cleanup_until,
                startup_sent,
                stopped,
                stdout_ended,
                failed,
                stop_requested,
                pending_http,
            })),
        }
    }
}
