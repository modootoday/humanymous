//! Native IPC security, bounded handler lifecycle and shared local services.
mod permissions;
mod service;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
use crate::{
    Rejection,
    configuration::{Configuration, Peer},
    decision::{Authority, NativePeer},
    protocol,
};
pub use permissions::private_directory;
pub use service::{Limits, LocalContext, LocalHandler, LocalRequest, LocalResponse};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    sync::Semaphore,
};
#[cfg(unix)]
pub(crate) use unix::connect;
#[cfg(windows)]
pub(crate) use windows::connect;
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
/// Restricted native listener; no TCP, DNS or remote-UNC fallback.
pub struct Listener {
    #[cfg(unix)]
    inner: unix::Listener,
    #[cfg(windows)]
    inner: windows::Listener,
}
impl Listener {
    /// Bind the legacy decision service using its provisioned endpoint/principal.
    pub fn bind(config: &Configuration) -> io::Result<Self> {
        Self::bind_ipc(&config.endpoint, &config.peer)
    }
    /// Bind an independently configured local service, with the same actual native
    /// ACL/peer checks. This does not authenticate any administrative credential.
    pub fn bind_ipc(endpoint: &str, peer: &Peer) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                inner: unix::Listener::bind(endpoint, peer)?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                inner: windows::Listener::bind(endpoint, peer)?,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (endpoint, peer);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported IPC platform",
            ))
        }
    }
    /// Legacy decision-only service. Its handler still refuses every admin path.
    pub async fn serve(
        self,
        authority: Authority,
        capacity: usize,
        rate: u32,
        timeout_ms: u64,
    ) -> io::Result<()> {
        self.serve_handler(
            DecisionHandler(Arc::new(Mutex::new(authority))),
            Limits {
                capacity,
                rate_per_second: rate,
                timeout_ms,
            },
        )
        .await
    }
    /// Bounded shared transport for a separately authorized local handler.
    pub async fn serve_handler<H: LocalHandler>(
        self,
        handler: H,
        limits: Limits,
    ) -> io::Result<()> {
        self.serve_handler_until(handler, limits, std::future::pending(), 30_000)
            .await
    }
    /// Stop acceptance, cancel/close tracked connections and wait for every
    /// admission-owning worker to end. Timeout is an error, never a graceful
    /// acknowledgement. Dropping this future cancels but cannot acknowledge drain.
    pub async fn serve_handler_until<
        H: LocalHandler,
        F: std::future::Future<Output = ()> + Send,
    >(
        self,
        handler: H,
        limits: Limits,
        shutdown: F,
        drain_timeout_ms: u64,
    ) -> io::Result<()> {
        let limits = limits.validate()?;
        if !(1..=30_000).contains(&drain_timeout_ms) {
            return Err(io::Error::other("invalid IPC drain deadline"));
        }
        let handler = Arc::new(handler);
        let slots = Arc::new(Semaphore::new(limits.capacity));
        let mut admission = Admission::new(limits.rate_per_second);
        let stopping = Arc::new(AtomicBool::new(false));
        let _stop_on_drop = CancelOnDrop(stopping.clone());
        let mut tasks = tokio::task::JoinSet::new();
        tokio::pin!(shutdown);
        let outcome = loop {
            let accepted = tokio::select! {
                _=&mut shutdown=>break Ok(()),
                joined=tasks.join_next(),if !tasks.is_empty()=>{
                    if let Some(Err(_))=joined{break Err(io::Error::other("IPC connection task failed"));}
                    continue;
                },
                accepted=self.inner.accept()=>accepted,
            };
            let (stream, peer) = match accepted {
                Ok(value) => value,
                Err(error) => break Err(error),
            };
            let Some(peer) = peer else {
                continue;
            };
            let permit = slots.clone().try_acquire_owned();
            if !admission.admit() || permit.is_err() {
                let mut stream = stream;
                let error = Rejection::new(429, "ipc_capacity");
                let _ = tokio::time::timeout(Duration::from_millis(10), async {
                    let _ = protocol::discard(&mut stream).await;
                    if protocol::write(&mut stream, error.status, &protocol::error_body(error))
                        .await
                        .is_ok()
                    {
                        finish(&mut stream).await;
                    }
                })
                .await;
                continue;
            }
            let permit = Arc::new(permit.expect("checked permit"));
            let handler = handler.clone();
            let until = Instant::now() + Duration::from_millis(limits.timeout_ms);
            let stopping = stopping.clone();
            tasks.spawn(async move {
                let _ = tokio::time::timeout_at(
                    until.into(),
                    connection(stream, peer, handler, permit, until, stopping),
                )
                .await;
            });
        };
        stopping.store(true, Ordering::Release);
        tasks.abort_all();
        drop(self);
        tokio::time::timeout(Duration::from_millis(drain_timeout_ms), async {
            while tasks.join_next().await.is_some() {}
            let _all = slots
                .acquire_many(limits.capacity as u32)
                .await
                .map_err(|_| io::Error::other("IPC admission closed"))?;
            Ok::<(), io::Error>(())
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "IPC drain deadline exceeded"))??;
        outcome
    }
}
struct DecisionHandler(Arc<Mutex<Authority>>);
impl LocalHandler for DecisionHandler {
    async fn handle(
        &self,
        request: LocalRequest,
        context: LocalContext,
    ) -> Result<LocalResponse, Rejection> {
        if request.method != "POST" || request.target != "/adapter/v0.1/decision" {
            return Err(Rejection::new(403, "decision_endpoint_only"));
        }
        let authority = self.0.clone();
        let response = tokio::task::spawn_blocking(move || {
            context.check()?;
            authority
                .lock()
                .map_err(|_| Rejection::new(503, "state_poisoned"))?
                .decide(
                    context.peer(),
                    &request.authorization,
                    &request.body,
                    || context.check(),
                )
        })
        .await
        .map_err(|_| Rejection::new(503, "decision_worker"))??;
        LocalResponse::json(200, &response)
    }
}
async fn connection<H: LocalHandler>(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    peer: NativePeer,
    handler: Arc<H>,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
    until: Instant,
    stopping: Arc<AtomicBool>,
) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel = CancelOnDrop(cancelled.clone());
    let result = match protocol::read_local(&mut stream).await {
        Err(error) => Err(error),
        Ok(request) => {
            match LocalContext::new(peer, cancelled.clone(), stopping, until, permit.clone()) {
                Err(error) => Err(error),
                Ok(context) => {
                    let future = handler.handle(request, context.clone());
                    tokio::pin!(future);
                    let mut extra = [0u8; 1];
                    tokio::select! {
                        result=&mut future=>result.and_then(|response|{context.check()?;response.validate()?;Ok(response)}),
                        read=stream.read(&mut extra)=>{
                            cancelled.store(true,Ordering::Release);
                            Err(if matches!(read,Ok(1)){Rejection::new(400,"trailing_frame")}else{Rejection::new(503,"request_cancelled")})
                        }
                    }
                }
            }
        }
    };
    let (status, bytes) = match result {
        Ok(response) => (response.status, response.body),
        Err(error) => (error.status, protocol::error_body(error)),
    };
    // A partial/failed write is closed. Never append another response or error.
    if protocol::write(&mut stream, status, &bytes).await.is_ok() {
        finish(&mut stream).await;
    }
    drop(permit);
}
async fn finish(_stream: &mut (impl AsyncRead + Unpin)) {
    #[cfg(windows)]
    {
        let mut closed = [0u8; 1];
        let _ = _stream.read(&mut closed).await;
    }
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
    #[test]
    fn invalid_limits_and_unbounded_or_non_json_response_reject() {
        assert!(
            Limits {
                capacity: 0,
                rate_per_second: 1,
                timeout_ms: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            LocalResponse {
                status: 200,
                body: vec![b' '; 65_537]
            }
            .validate()
            .is_err()
        );
        assert!(
            LocalResponse {
                status: 200,
                body: b"not-json".to_vec()
            }
            .validate()
            .is_err()
        );
    }
}
