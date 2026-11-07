//! Bounded native child lifetime and private readiness transport for pilots.
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

mod capture_attempt;
pub use capture_attempt::{
    CaptureAttempt, CaptureDeadlines, CaptureIssue, CaptureLimits, CaptureProgress,
    CaptureStartFailure, CapturedDirectChild, RetainedCaptureFailure,
};

fn piped_child(command: &mut Command, stderr: bool) -> io::Result<Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(if stderr {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()
}

/// Owns and reaps a native child even after a readiness/validation error.
pub struct ManagedChild(Child);

/// Bounded process output. Callers must not publish credential-bearing output.
pub struct Captured {
    /// Actual direct-child OS PID captured immediately after spawn. This does not
    /// identify descendants, authorize a caller or prevent PID reuse after exit.
    pub pid: u32,
    /// Observed OS exit status, never a shell's guessed exit value.
    pub status: ExitStatus,
    /// Complete stdout, limited to the caller's declared byte budget.
    pub stdout: Vec<u8>,
    /// Complete stderr only when explicitly requested; private evidence only.
    pub stderr: Vec<u8>,
}
impl std::fmt::Debug for Captured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Captured")
            .field("status", &self.status)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .finish()
    }
}

impl ManagedChild {
    /// Launch a command with private inherited input/stdout and no stderr dump.
    pub fn spawn(command: &mut Command, input: &[u8]) -> io::Result<Self> {
        Self::spawn_with_stderr(command, input, false)
    }
    fn spawn_with_stderr(command: &mut Command, input: &[u8], stderr: bool) -> io::Result<Self> {
        if input.len() > 4096 {
            return Err(io::Error::other("private startup input budget exceeded"));
        }
        let mut child = Self::piped(command, stderr)?;
        let mut stdin = child
            .0
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("private stdin missing"))?;
        stdin.write_all(input)?;
        drop(stdin);
        Ok(child)
    }
    fn piped(command: &mut Command, stderr: bool) -> io::Result<Self> {
        Ok(Self(piped_child(command, stderr)?))
    }
    /// Keep private stdin/stdout open for a typed interactive protocol. Caller
    /// owns framing, per-frame byte/deadline limits and closing these pipes;
    /// this guard still kills/reaps the direct child. Stderr is suppressed.
    pub fn spawn_duplex(command: &mut Command) -> io::Result<(Self, ChildStdin, ChildStdout)> {
        let mut child = Self::piped(command, false)?;
        let stdin = child
            .0
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("private stdin missing"))?;
        let stdout = child
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("private stdout missing"))?;
        Ok((child, stdin, stdout))
    }
    /// Return one direct-child guard and all three owned pipes. This performs no
    /// startup write, reader spawn, framing or descendant supervision. The caller
    /// must apply verified cwd/env before launch, bound input/output, retain and
    /// join readers, and independently establish owned-tree containment. Returning
    /// pipes or dropping the guard is not EOF/reader-join/Job-empty proof.
    pub fn spawn_duplex_with_stderr(
        command: &mut Command,
    ) -> io::Result<(Self, ChildStdin, ChildStdout, ChildStderr)> {
        let mut child = Self::piped(command, true)?;
        let stdin = child
            .0
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("private stdin missing"))?;
        let stdout = child
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("private stdout missing"))?;
        let stderr = child
            .0
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("private stderr missing"))?;
        Ok((child, stdin, stdout, stderr))
    }
    /// Observe the actual direct-child status without blocking a frame loop.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.0.try_wait()
    }

    /// Read one bounded readiness frame with a deadline. Timeout kills/reaps.
    pub fn readiness(&mut self, timeout: Duration, limit: usize) -> io::Result<Vec<u8>> {
        let output = self
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout already consumed"))?;
        let (send, receive) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = BufReader::new(output)
                .take(limit as u64 + 1)
                .read_until(b'\n', &mut bytes)
                .and_then(|_| {
                    if bytes.is_empty() || bytes.len() > limit || bytes.last() != Some(&b'\n') {
                        Err(io::Error::other("missing/oversized readiness frame"))
                    } else {
                        Ok(bytes)
                    }
                });
            let _ = send.send(result);
        });
        match receive.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => {
                self.stop()?;
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "readiness deadline",
                ))
            }
        }
    }

    /// Wait for actual exit, killing/reaping rather than leaving timed-out work.
    pub fn wait(&mut self, timeout: Duration) -> io::Result<ExitStatus> {
        let until = Instant::now() + timeout;
        loop {
            if let Some(status) = self.0.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= until {
                self.stop()?;
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "child exit deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Kill a still-running owned child and always reap its process handle.
    pub fn stop(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.0.try_wait()? {
            return Ok(status);
        }
        self.0.kill()?;
        self.0.wait()
    }

    /// OS PID for identity binding, not proof of user authorization.
    pub fn id(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Execute and collect bounded stdout while concurrently draining the OS pipe.
pub fn capture(
    command: &mut Command,
    input: &[u8],
    timeout: Duration,
    limit: usize,
) -> io::Result<Captured> {
    capture_streams(command, input, timeout, limit, false)
}
/// Concurrent bounded stdout/stderr capture for private build/pilot evidence.
/// Each stream has its own limit. Overflow or pipe EOF beyond the overall
/// deadline is failure, never truncated success. Supervisors of descendants
/// must own their process group/job; this helper owns the direct child only.
pub fn capture_with_stderr(
    command: &mut Command,
    input: &[u8],
    timeout: Duration,
    limit: usize,
) -> io::Result<Captured> {
    capture_streams(command, input, timeout, limit, true)
}
fn reader(output: impl Read + Send + 'static, limit: usize) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (|| {
            let mut bytes = Vec::new();
            output.take(limit as u64 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                return Err(io::Error::other("child output limit exceeded"));
            }
            Ok::<_, io::Error>(bytes)
        })();
        let _ = send.send(result);
    });
    receive
}
fn capture_streams(
    command: &mut Command,
    input: &[u8],
    timeout: Duration,
    limit: usize,
    stderr: bool,
) -> io::Result<Captured> {
    let until = Instant::now() + timeout;
    let mut child = ManagedChild::spawn_with_stderr(command, input, stderr)?;
    let pid = child.id();
    let stdout = reader(
        child
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout missing"))?,
        limit,
    );
    let stderr = child.0.stderr.take().map(|pipe| reader(pipe, limit));
    let status = child.wait(timeout)?;
    let receive = |channel: mpsc::Receiver<io::Result<Vec<u8>>>| {
        channel
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "child output EOF deadline"))?
    };
    let stdout = receive(stdout)?;
    let stderr = match stderr {
        Some(channel) => receive(channel)?,
        None => Vec::new(),
    };
    Ok(Captured {
        pid,
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "subprocess entry point; owning parent tests verify execution"]
    fn subprocess_fixture() {
        match std::env::var("HUMANYMOUS_SUPPORT_FIXTURE").as_deref() {
            Ok("hang") => std::thread::sleep(Duration::from_secs(5)),
            Ok("fail") => panic!("expected fixture refusal"),
            Ok("flood") => println!("{}", "x".repeat(8192)),
            Ok("both") => {
                println!("stdout fixture");
                eprintln!("stderr fixture");
            }
            Ok("stderr-flood") => eprintln!("{}", "x".repeat(8192)),
            Ok("identity") => println!("\nchild-pid:{}", std::process::id()),
            Ok("duplex") | Ok("duplex-both") => {
                let mut input = String::new();
                std::io::stdin()
                    .take(4096)
                    .read_to_string(&mut input)
                    .unwrap();
                println!("{input}");
                if std::env::var("HUMANYMOUS_SUPPORT_FIXTURE").as_deref() == Ok("duplex-both") {
                    eprintln!("owned-duplex-stderr");
                }
            }
            _ => (),
        }
    }
    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "process::tests::subprocess_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("HUMANYMOUS_SUPPORT_FIXTURE", mode);
        command
    }
    #[test]
    fn actual_exit_and_output_are_observed() {
        assert!(
            capture(&mut fixture("ok"), b"", Duration::from_secs(5), 4096)
                .unwrap()
                .status
                .success()
        );
        assert!(
            !capture(&mut fixture("fail"), b"", Duration::from_secs(5), 4096)
                .unwrap()
                .status
                .success()
        );
    }
    #[test]
    fn captured_pid_matches_the_actual_direct_child_not_the_parent() {
        let output =
            capture_with_stderr(&mut fixture("identity"), b"", Duration::from_secs(5), 4096)
                .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let text = std::str::from_utf8(&output.stdout).unwrap();
        let observed = text
            .lines()
            .find_map(|line| line.strip_prefix("child-pid:"))
            .unwrap()
            .parse::<u32>()
            .unwrap();
        assert_eq!(output.pid, observed);
        assert_ne!(output.pid, std::process::id());
    }
    #[test]
    fn timeout_is_failure_and_child_is_reaped() {
        assert_eq!(
            capture(&mut fixture("hang"), b"", Duration::from_millis(50), 4096)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }
    #[test]
    fn output_limit_is_not_truncated_success() {
        assert!(capture(&mut fixture("flood"), b"", Duration::from_secs(5), 128).is_err());
    }
    #[test]
    fn explicit_stderr_is_concurrent_bounded_and_not_in_debug() {
        let output =
            capture_with_stderr(&mut fixture("both"), b"", Duration::from_secs(5), 4096).unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("stdout fixture"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("stderr fixture"));
        assert!(!format!("{output:?}").contains("stderr fixture"));
        assert!(
            capture_with_stderr(
                &mut fixture("stderr-flood"),
                b"",
                Duration::from_secs(5),
                128
            )
            .is_err()
        );
    }
    #[test]
    fn duplex_caller_owns_pipes_and_guard_owns_lifetime() {
        let (mut child, mut input, output) =
            ManagedChild::spawn_duplex(&mut fixture("duplex")).unwrap();
        input.write_all(b"owned-native-duplex").unwrap();
        drop(input);
        let mut bytes = Vec::new();
        output.take(4096).read_to_end(&mut bytes).unwrap();
        assert!(child.wait(Duration::from_secs(5)).unwrap().success());
        assert!(String::from_utf8_lossy(&bytes).contains("owned-native-duplex"));
        assert!(child.try_wait().unwrap().unwrap().success());

        let (mut child, mut input, output, error) =
            ManagedChild::spawn_duplex_with_stderr(&mut fixture("duplex-both")).unwrap();
        let stdout = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            output.take(4097).read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            error.take(4097).read_to_end(&mut bytes).map(|_| bytes)
        });
        input.write_all(b"owned-all-three-pipes").unwrap();
        drop(input);
        let status = child.wait(Duration::from_secs(5));
        let stdout = stdout.join().unwrap().unwrap();
        let stderr = stderr.join().unwrap().unwrap();
        assert!(status.unwrap().success());
        assert!(stdout.len() <= 4096 && stderr.len() <= 4096);
        assert!(String::from_utf8_lossy(&stdout).contains("owned-all-three-pipes"));
        assert_eq!(stderr, b"owned-duplex-stderr\n");
    }
}
