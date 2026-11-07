//! One private actual-child/status/stop/clock owner for retained process profiles.
use super::capture_attempt::{CaptureDeadlines, CaptureIssue};
use std::{
    io::{self, Read},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus},
    time::Instant,
};

pub(super) struct OwnedDirectChild {
    child: Child,
    pid: u32,
    status: Option<ExitStatus>,
    deadlines: CaptureDeadlines,
    issues: Vec<CaptureIssue>,
}
impl OwnedDirectChild {
    pub(super) fn spawn(command: &mut Command, deadlines: CaptureDeadlines) -> io::Result<Self> {
        if !deadlines.accepts_start() {
            return Err(io::Error::other("original run budget refused"));
        }
        let child = super::piped_child(command, true)?;
        Ok(Self {
            pid: child.id(),
            child,
            status: None,
            deadlines,
            issues: Vec::new(),
        })
    }
    pub(super) fn pid(&self) -> u32 {
        self.pid
    }
    pub(super) fn original_deadlines(&self) -> (Instant, Instant) {
        self.deadlines.original_deadlines()
    }
    pub(super) fn status(&self) -> Option<ExitStatus> {
        self.status
    }
    pub(super) fn issues(&self) -> &[CaptureIssue] {
        &self.issues
    }
    pub(super) fn record(&mut self, issue: CaptureIssue) {
        if !self.issues.contains(&issue) {
            self.issues.push(issue);
        }
    }
    pub(super) fn poll_child(&mut self) {
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
    }
    // Clock sampled AFTER the composite profile gathers its real IO facts.
    pub(super) fn observe_completion(&mut self, completed: bool, settled: bool) {
        for issue in self
            .deadlines
            .observe(Instant::now(), completed, settled)
            .into_iter()
            .flatten()
        {
            self.record(issue);
        }
    }
    // May block in the kernel. No wait/reap or implicit child/tree settlement.
    pub(super) fn request_stop(&mut self) -> io::Result<()> {
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
    pub(super) fn take_input(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }
    pub(super) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }
    pub(super) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }
    pub(super) fn retained_pipes(&self) -> [bool; 3] {
        [
            self.child.stdin.is_some(),
            self.child.stdout.is_some(),
            self.child.stderr.is_some(),
        ]
    }
    pub(super) fn close_input(&mut self) -> bool {
        let input = self.child.stdin.take();
        let retained = input.is_some();
        drop(input);
        retained
    }
    pub(super) fn close_pipes_after_exit(&mut self) -> Option<u8> {
        self.status?;
        let count = self.retained_pipes().into_iter().map(u8::from).sum();
        drop(self.child.stdin.take());
        drop(self.child.stdout.take());
        drop(self.child.stderr.take());
        Some(count)
    }
    pub(super) fn read_prefix(&mut self, stderr: bool, bytes: &mut [u8]) -> io::Result<()> {
        // Borrow actual held endpoint without exposing pipe/Child ownership.
        if stderr {
            self.child
                .stderr
                .as_mut()
                .ok_or_else(missing_pipe)?
                .read_exact(bytes)
        } else {
            self.child
                .stdout
                .as_mut()
                .ok_or_else(missing_pipe)?
                .read_exact(bytes)
        }
    }
}
fn missing_pipe() -> io::Error {
    io::Error::other("retained output pipe unavailable")
}
