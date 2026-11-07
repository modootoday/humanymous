//! Absolute lifetime for owned service fixtures, including blocked pipe writes.
use humanymous_test_support::process::ManagedChild;
use std::{
    io,
    process::{ChildStdin, ChildStdout, Command, ExitStatus},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct ChildLifetime {
    child: Arc<Mutex<ManagedChild>>,
    expired: Arc<AtomicBool>,
    cancel: mpsc::Sender<()>,
    watcher: Option<JoinHandle<()>>,
}
impl ChildLifetime {
    pub fn spawn(command: &mut Command) -> io::Result<(Self, ChildStdin, ChildStdout)> {
        Self::spawn_with_timeout(command, Duration::from_secs(60))
    }
    pub fn spawn_with_timeout(
        command: &mut Command,
        timeout: Duration,
    ) -> io::Result<(Self, ChildStdin, ChildStdout)> {
        let (child, input, output) = ManagedChild::spawn_duplex(command)?;
        let child = Arc::new(Mutex::new(child));
        let expired = Arc::new(AtomicBool::new(false));
        let (cancel, receive) = mpsc::channel();
        let owned = Arc::clone(&child);
        let timed_out = Arc::clone(&expired);
        let watcher = thread::spawn(move || {
            if matches!(
                receive.recv_timeout(timeout),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                let mut child = owned.lock().unwrap_or_else(|poison| poison.into_inner());
                if !matches!(child.try_wait(), Ok(Some(_))) {
                    timed_out.store(true, Ordering::Release);
                    let _ = child.stop();
                }
            }
        });
        Ok((
            Self {
                child,
                expired,
                cancel,
                watcher: Some(watcher),
            },
            input,
            output,
        ))
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            let status = self
                .child
                .lock()
                .map_err(|_| io::Error::other("fixture child lock poisoned"))?
                .try_wait()?;
            if self.expired.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned service fixture lifetime exceeded",
                ));
            }
            if let Some(status) = status {
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
    /// Deliberate crash probe, never a global PID lookup or foreign process kill.
    pub fn kill(&mut self) -> io::Result<()> {
        self.child
            .lock()
            .map_err(|_| io::Error::other("fixture child lock poisoned"))?
            .stop()?;
        Ok(())
    }
}
impl Drop for ChildLifetime {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
        let _ = self
            .child
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .stop();
    }
}
