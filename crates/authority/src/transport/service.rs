//! Handler contracts keep peer identity and bounded work lifetime in one owner.
pub use crate::protocol::Request as LocalRequest;
use crate::{Rejection, decision::NativePeer};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Bounded admission configuration, validated before accepting any connection.
#[derive(Clone, Copy)]
pub struct Limits {
    /// Maximum live connections and detached blocking operations.
    pub capacity: usize,
    /// Maximum admissions per one-second monotonic window.
    pub rate_per_second: u32,
    /// Whole connection read/handler/write lifetime.
    pub timeout_ms: u64,
}
impl Limits {
    pub(super) fn validate(self) -> std::io::Result<Self> {
        if !(1..=128).contains(&self.capacity)
            || !(1..=10_000).contains(&self.rate_per_second)
            || !(1..=30_000).contains(&self.timeout_ms)
        {
            return Err(std::io::Error::other("invalid IPC limits"));
        }
        Ok(self)
    }
}
/// Response bytes only; the transport is the single HTTP writer. No forwarding,
/// header mutation or arbitrary socket access is exposed to handlers.
pub struct LocalResponse {
    /// Bounded protocol HTTP status.
    pub status: u16,
    /// Complete UTF-8 JSON entity, at most 65536 bytes.
    pub body: Vec<u8>,
}
impl LocalResponse {
    /// Serialize one JSON document; the transport validates this boundary again.
    pub fn json(status: u16, value: &impl serde::Serialize) -> Result<Self, Rejection> {
        let response = Self {
            status,
            body: serde_json::to_vec(value).map_err(|_| Rejection::new(503, "handler_response"))?,
        };
        response.validate()?;
        Ok(response)
    }
    pub(super) fn validate(&self) -> Result<(), Rejection> {
        if ![200, 400, 401, 403, 404, 405, 409, 413, 422, 429, 500, 503].contains(&self.status)
            || self.body.len() > 65_536
            || serde_json::from_slice::<serde_json::Value>(&self.body).is_err()
        {
            return Err(Rejection::new(503, "handler_response"));
        }
        Ok(())
    }
}
/// OS-authenticated connection context. Clone it into any detached worker so its
/// admission permit survives cancellation until that worker really terminates.
#[derive(Clone)]
pub struct LocalContext {
    peer: NativePeer,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    until: Instant,
    accepted: u64,
    _permit: Arc<tokio::sync::OwnedSemaphorePermit>,
}
impl LocalContext {
    pub(super) fn new(
        peer: NativePeer,
        cancelled: Arc<AtomicBool>,
        shutdown: Arc<AtomicBool>,
        until: Instant,
        permit: Arc<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<Self, Rejection> {
        Ok(Self {
            peer,
            cancelled,
            shutdown,
            until,
            accepted: crate::now_ms()?,
            _permit: permit,
        })
    }
    /// Identity queried from the accepted OS stream, never an HTTP header.
    pub fn peer(&self) -> NativePeer {
        self.peer
    }
    /// Trusted wall time only while cancellation/monotonic/rollback checks pass.
    /// Handlers must recheck before irreversible effects and after durable work.
    pub fn check(&self) -> Result<u64, Rejection> {
        let now = crate::now_ms()?;
        if self.cancelled.load(Ordering::Acquire)
            || self.shutdown.load(Ordering::Acquire)
            || Instant::now() >= self.until
            || now < self.accepted
        {
            return Err(Rejection::new(503, "request_cancelled_or_expired"));
        }
        Ok(now)
    }
    /// Remaining monotonic budget, with no wall-clock-derived extension.
    pub fn remaining(&self) -> Duration {
        self.until.saturating_duration_since(Instant::now())
    }
}
/// A separately configured local security service. Implementations authenticate
/// their own credential/scope/route/CSRF policy; native locality is not admin auth.
pub trait LocalHandler: Send + Sync + 'static {
    /// Dispatch one complete request. The handler never owns the transport writer.
    fn handle(
        &self,
        request: LocalRequest,
        context: LocalContext,
    ) -> impl Future<Output = Result<LocalResponse, Rejection>> + Send;
}
