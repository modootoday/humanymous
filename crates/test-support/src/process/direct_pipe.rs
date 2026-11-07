//! Retain intentionally unread pipes around the ONE shared actual-child owner.
//! No IO workers, auto-draining, EOF, reader-join or descendant proof is provided.
use super::{
    capture_attempt::{CaptureDeadlines, CaptureIssue},
    owned_child::OwnedDirectChild,
};
use std::{
    io,
    process::{Command, ExitStatus},
};

/// Snapshot of actual direct-child facts; no external constructor or authority.
pub struct DirectChildProgress {
    status: Option<ExitStatus>,
    issues: Vec<CaptureIssue>,
    retained_pipes: [bool; 3],
    prefixes_observed: [bool; 2],
}
impl DirectChildProgress {
    /// Actual observed direct-child status, independent of pipe EOF or descendants.
    pub fn status(&self) -> Option<ExitStatus> {
        self.status
    }
    /// Sticky actual observation/request failures; nonzero may be expected by a fixture.
    pub fn issues(&self) -> &[CaptureIssue] {
        &self.issues
    }
    /// Actual owned input/stdout/stderr endpoints still held by this profile.
    pub fn retained_pipe_ends(&self) -> [bool; 3] {
        self.retained_pipes
    }
    /// Exact controlled stdout/stderr prefixes observed, without draining remainder.
    pub fn prefixes_observed(&self) -> [bool; 2] {
        self.prefixes_observed
    }
}

/// Actual exited direct child plus released parent endpoints. NOT an EOF,
/// IO-worker join, native handle-query or owned-tree cleanup receipt.
pub struct ClosedPipeChild {
    pid: u32,
    status: ExitStatus,
    released_pipe_ends: u8,
    prefixes_observed: [bool; 2],
    issues: Vec<CaptureIssue>,
}
impl ClosedPipeChild {
    /// Captured actual direct PID, never a reusable identity/authorization.
    pub fn pid(&self) -> u32 {
        self.pid
    }
    /// Actual observed direct-child exit status; compare fixture0/74 explicitly.
    pub fn status(&self) -> ExitStatus {
        self.status
    }
    /// Number of owned parent endpoints released through std's endpoint Drop.
    /// Closing them is not observation of EOF or a native CloseHandle receipt.
    pub fn released_pipe_ends(&self) -> u8 {
        self.released_pipe_ends
    }
    /// Exact prefixes observed; marker entry does not prove kernel-blocked IO.
    pub fn prefixes_observed(&self) -> [bool; 2] {
        self.prefixes_observed
    }
    /// Sticky request/observation failures, independent of expected fixture exit.
    pub fn issues(&self) -> &[CaptureIssue] {
        &self.issues
    }
}

/// Same owner returned if actual child exit remains unobserved.
#[must_use]
pub struct RetainedDirectPipeFailure {
    owner: Box<RetainedDirectPipeAttempt>,
}
impl RetainedDirectPipeFailure {
    /// Recover the actual Child and its held pipes without a renewed deadline.
    pub fn into_owner(self) -> RetainedDirectPipeAttempt {
        *self.owner
    }
}

/// Actual child plus held pipes, using the common backing, with NO IO threads.
/// No Drop wait/kill/cleanup. Caller must retain/settle; abandonment is Unknown.
#[must_use]
pub struct RetainedDirectPipeAttempt {
    child: OwnedDirectChild,
    prefix_attempted: [bool; 2],
    prefixes_observed: [bool; 2],
    released_pipe_ends: u8,
}
impl RetainedDirectPipeAttempt {
    /// Synchronous spawn may BLOCK; caller MUST provide independent outer
    /// supervision. After OS spawn succeeds, no further fallible setup is done.
    /// No output is read or input written here; all three endpoints stay owned.
    pub fn start(command: &mut Command, deadlines: CaptureDeadlines) -> io::Result<Self> {
        Ok(Self {
            child: OwnedDirectChild::spawn(command, deadlines)?,
            prefix_attempted: [false; 2],
            prefixes_observed: [false; 2],
            released_pipe_ends: 0,
        })
    }
    /// Release only the owned input endpoint, without writing, waiting or reading.
    pub fn close_input(&mut self) {
        self.released_pipe_ends += u8::from(self.child.close_input());
    }
    /// Observe child status and real internal time without reading output or joining.
    pub fn poll(&mut self) -> DirectChildProgress {
        self.child.poll_child();
        let exited = self.child.status().is_some();
        let retained_pipes = self.child.retained_pipes();
        self.child
            .observe_completion(exited, exited && retained_pipes == [false; 3]);
        DirectChildProgress {
            status: self.child.status(),
            issues: self.child.issues().to_vec(),
            retained_pipes,
            prefixes_observed: self.prefixes_observed,
        }
    }
    /// Potentially BLOCKING actual-child kill request under independent outer
    /// supervision, NOT exit/reap/EOF/descendant proof. Errors retain this owner.
    pub fn request_stop(&mut self) -> io::Result<()> {
        self.child.request_stop()
    }
    /// Read exactly one controlled stdout prefix<=256bytes, leaving its remainder
    /// unread. May BLOCK in OS; MUST have independent outer supervision. Errors
    /// retain this borrowed owner and mark failure; no reader thread is detached.
    pub fn read_stdout_prefix(&mut self, expected: &[u8]) -> io::Result<()> {
        self.read_prefix(false, expected)
    }
    /// Same bounded ONCE-per-stream, potentially BLOCKING contract for stderr.
    pub fn read_stderr_prefix(&mut self, expected: &[u8]) -> io::Result<()> {
        self.read_prefix(true, expected)
    }
    fn read_prefix(&mut self, stderr: bool, expected: &[u8]) -> io::Result<()> {
        let index = usize::from(stderr);
        let issue = if stderr {
            CaptureIssue::StderrFailed
        } else {
            CaptureIssue::StdoutFailed
        };
        if let Err(error) = check_prefix(expected, self.prefix_attempted[index]) {
            self.child.record(issue);
            return Err(error);
        }
        self.prefix_attempted[index] = true;
        let mut bytes = vec![0_u8; expected.len()];
        if let Err(error) = self.child.read_prefix(stderr, &mut bytes) {
            self.child.record(issue);
            self.poll();
            return Err(error);
        }
        self.poll();
        if bytes != expected {
            self.child.record(issue);
            return Err(io::Error::other("controlled output prefix mismatch"));
        }
        self.prefixes_observed[index] = true;
        Ok(())
    }
    /// Close held endpoints ONLY after actual child exit is observed, or return
    /// SAME owner. No draining, EOF observation, worker join or new deadline.
    /// Standard endpoint release can enter OS code; outer supervision still applies.
    pub fn settle_closing_pipes(mut self) -> Result<ClosedPipeChild, RetainedDirectPipeFailure> {
        self.poll();
        let Some(status) = self.child.status() else {
            return Err(RetainedDirectPipeFailure {
                owner: Box::new(self),
            });
        };
        let Some(released) = self.child.close_pipes_after_exit() else {
            return Err(RetainedDirectPipeFailure {
                owner: Box::new(self),
            });
        };
        self.released_pipe_ends += released;
        self.poll();
        Ok(ClosedPipeChild {
            pid: self.child.pid(),
            status,
            released_pipe_ends: self.released_pipe_ends,
            prefixes_observed: self.prefixes_observed,
            issues: self.child.issues().to_vec(),
        })
    }
}

fn check_prefix(expected: &[u8], attempted: bool) -> io::Result<()> {
    if attempted || expected.is_empty() || expected.len() > 256 {
        return Err(io::Error::other("controlled prefix budget refused"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controlled_prefix_has_one_finite_nonempty_budget() {
        assert!(check_prefix(&[], false).is_err());
        assert!(check_prefix(&[0; 256], false).is_ok());
        assert!(check_prefix(&[0; 257], false).is_err());
        assert!(check_prefix(b"entry", true).is_err());
    }
}
