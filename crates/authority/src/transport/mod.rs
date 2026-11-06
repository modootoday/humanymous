//! Native IPC security plus bounded connection admission and lifecycle.
mod permissions;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
use crate::{
    Rejection,
    configuration::Configuration,
    decision::{Authority, NativePeer},
    protocol,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Semaphore,
};

/// A private directory must already exist and belong to the trusted principal.
pub use permissions::private_directory;

/// Peer-specific/global fixed-window admission limiter (one peer per instance).
pub struct Admission {
    start: Instant,
    count: u32,
    limit: u32,
}
impl Admission {
    /// Start a monotonic one-second admission window.
    pub fn new(limit: u32) -> Self {
        Self {
            start: Instant::now(),
            count: 0,
            limit,
        }
    }
    /// Reject excessive connections before allocating request bodies/tasks.
    pub fn admit(&mut self) -> bool {
        if self.start.elapsed() >= Duration::from_secs(1) {
            self.start = Instant::now();
            self.count = 0;
        }
        if self.count >= self.limit {
            return false;
        }
        self.count += 1;
        true
    }
}
/// Bound native listener that refuses TCP, remote UNC and namespace fallbacks.
pub struct Listener {
    #[cfg(unix)]
    inner: unix::Listener,
    #[cfg(windows)]
    inner: windows::Listener,
}
impl Listener {
    /// Create restricted IPC before publishing the credential readiness line.
    pub fn bind(config: &Configuration) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                inner: unix::Listener::bind(config)?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                inner: windows::Listener::bind(config)?,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = config;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported IPC platform",
            ))
        }
    }
    /// Run an accept loop with bounded tasks, body storage and decision work.
    /// Cancellation drops connections; already reserved nonces/audit stay spent.
    pub async fn serve(
        self,
        authority: Authority,
        capacity: usize,
        rate: u32,
        timeout_ms: u64,
    ) -> io::Result<()> {
        let authority = Arc::new(Mutex::new(authority));
        let slots = Arc::new(Semaphore::new(capacity));
        let timeout = Duration::from_millis(timeout_ms);
        let mut admission = Admission::new(rate);
        loop {
            let (stream, peer) = self.inner.accept().await?;
            let Some(peer) = peer else {
                // A client disconnecting during credential lookup must fail
                // only its own connection, not terminate the listener.
                continue;
            };
            let permit = slots.clone().try_acquire_owned();
            if !admission.admit() || permit.is_err() {
                // No task is spawned for rejected peers, keeping task count
                // bounded even if clients refuse to read the rejection.
                let mut stream = stream;
                let error = Rejection::new(429, "ipc_capacity");
                let _ = tokio::time::timeout(Duration::from_millis(10), async {
                    let _ = protocol::discard(&mut stream).await;
                    protocol::write(&mut stream, error.status, &protocol::error_body(error)).await
                })
                .await;
                continue;
            }
            let permit = Arc::new(permit.expect("checked permit"));
            let authority = authority.clone();
            tokio::spawn(async move {
                // Permit is held by the blocking decision itself after dispatch,
                // so a timed-out disk operation cannot create unbounded workers.
                let _ = tokio::time::timeout(timeout, connection(stream, peer, authority, permit))
                    .await;
            });
        }
    }
}
async fn connection(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    peer: NativePeer,
    authority: Arc<Mutex<Authority>>,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(cancelled.clone());
    let result = match protocol::read(&mut stream).await {
        Err(error) => Err(error),
        Ok(request) => {
            let worker_permit = permit.clone();
            let worker_cancelled = cancelled.clone();
            let mut worker = tokio::task::spawn_blocking(move || {
                let _permit = worker_permit;
                authority
                    .lock()
                    .map_err(|_| Rejection::new(503, "state_poisoned"))?
                    .decide(peer, &request.authorization, &request.body, || {
                        if worker_cancelled.load(Ordering::Acquire) {
                            Err(Rejection::new(503, "request_cancelled"))
                        } else {
                            crate::now_ms()
                        }
                    })
            });
            let mut extra = [0u8; 1];
            tokio::select! {
                result = &mut worker => result.unwrap_or(Err(Rejection::new(503, "decision_worker"))),
                read = tokio::io::AsyncReadExt::read(&mut stream, &mut extra) => {
                    cancelled.store(true, Ordering::Release);
                    Err(match read {
                        Ok(1) => Rejection::new(400, "trailing_frame"),
                        _ => Rejection::new(503, "request_cancelled"),
                    })
                }
            }
        }
    };
    let (status, bytes) = match result {
        Ok(response) => (
            200,
            serde_json::to_vec(&response).expect("bounded response serializes"),
        ),
        Err(error) => (error.status, protocol::error_body(error)),
    };
    let _ = protocol::write(&mut stream, status, &bytes).await;
    drop(permit);
}

struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_window_has_no_unbounded_peer_map() {
        let mut a = Admission::new(2);
        assert!(a.admit());
        assert!(a.admit());
        assert!(!a.admit());
        a.start = Instant::now() - Duration::from_secs(2);
        assert!(a.admit());
    }
}
