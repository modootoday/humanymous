use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{marker::PhantomData, rc::Rc};

/// Apache process model, provisioned at child initialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mpm {
    /// One request at a time in a non-threaded child.
    Prefork,
    /// Bounded worker threads.
    Worker,
    /// Event MPM; this alone grants no asynchronous host capability.
    Event,
}
impl Mpm {
    /// Bind a pinned host identity to its independently queried capabilities.
    /// Missing async support is accepted only for the two explicitly known
    /// synchronous MPMs; it is never a fallback for unknown or event models.
    pub fn identify(
        name: &str,
        threaded: bool,
        asynchronous: Option<bool>,
        threads: usize,
    ) -> Result<Self, Error> {
        match (name, threaded, asynchronous, threads) {
            ("prefork", false, None, 1) => Ok(Self::Prefork),
            ("worker", true, None, 1..) => Ok(Self::Worker),
            ("event", true, Some(true), 1..) => Ok(Self::Event),
            _ => Err(Error::ChildUnavailable),
        }
    }
}
/// Static failure without request bytes, credentials or client identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Child identity, capacity, generation or settings are unavailable.
    ChildUnavailable,
    /// Body framing, completeness or bound was violated.
    BodyUnavailable,
    /// Cancellation, deadline or lifecycle prohibits further effects.
    RequestClosed,
}
struct State {
    accepting: bool,
    active: usize,
}
struct Owner {
    pid: u32,
    generation: u64,
    capacity: usize,
    state: Mutex<State>,
}
/// Child-scoped admission owner. Construct inside the child, never before fork.
#[derive(Clone)]
pub struct Child(Arc<Owner>);
impl Child {
    /// Establish actual current PID and a nonzero supervisor generation.
    pub fn initialize(generation: u64, mpm: Mpm, capacity: usize) -> Result<Self, Error> {
        if generation == 0
            || capacity == 0
            || capacity > 1024
            || (mpm == Mpm::Prefork && capacity != 1)
        {
            return Err(Error::ChildUnavailable);
        }
        Ok(Self(Arc::new(Owner {
            pid: std::process::id(),
            generation,
            capacity,
            state: Mutex::new(State {
                accepting: true,
                active: 0,
            }),
        })))
    }
    /// Close admission and invalidate outstanding requests on graceful teardown.
    pub fn shutdown(&self) -> Result<(), Error> {
        // Check PID before touching an inherited potentially locked mutex.
        self.process()?;
        self.0
            .state
            .lock()
            .map_err(|_| Error::ChildUnavailable)?
            .accepting = false;
        Ok(())
    }
    fn process(&self) -> Result<(), Error> {
        if self.0.pid != std::process::id() {
            return Err(Error::ChildUnavailable);
        }
        Ok(())
    }
    /// Admit before consuming host input; no subrequest/redirect decision reuse.
    pub fn begin(
        &self,
        generation: u64,
        initial: bool,
        length: usize,
        maximum: usize,
        budget: Duration,
    ) -> Result<Request, Error> {
        self.process()?;
        if generation != self.0.generation || !initial {
            return Err(Error::ChildUnavailable);
        }
        if maximum > 1_048_576 || length > maximum {
            return Err(Error::BodyUnavailable);
        }
        if budget.is_zero() || budget > Duration::from_secs(30) {
            return Err(Error::RequestClosed);
        }
        let until = Instant::now()
            .checked_add(budget)
            .ok_or(Error::RequestClosed)?;
        let mut state = self.0.state.lock().map_err(|_| Error::ChildUnavailable)?;
        if !state.accepting || state.active >= self.0.capacity {
            return Err(Error::ChildUnavailable);
        }
        state.active += 1;
        Ok(Request {
            child: self.clone(),
            until,
            length,
            bytes: Vec::new(),
            phase: Phase::Reading,
            cancellation: Cancellation(Arc::new(AtomicBool::new(false))),
            host_thread: PhantomData,
        })
    }
}
#[derive(PartialEq)]
enum Phase {
    Reading,
    Complete,
    Closed,
}
/// Cleanup signal retained independently by Apache's request-pool cleanup.
/// It contains no request memory and cannot authorize or revive a request.
#[derive(Clone)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    /// Idempotently revoke outstanding work, including an authority await.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}
/// Owned input and permit; dropping always releases same-process admission.
/// No Apache pointer is accepted or retained by this safe boundary.
/// This owner is deliberately !Send and !Sync: only owned IPC observations may
/// cross a worker boundary. Final body replay stays on the Apache request thread.
pub struct Request {
    child: Child,
    until: Instant,
    length: usize,
    bytes: Vec<u8>,
    phase: Phase,
    cancellation: Cancellation,
    host_thread: PhantomData<Rc<()>>,
}
impl Request {
    fn live(&self) -> Result<(), Error> {
        self.child.process()?;
        if self.phase == Phase::Closed
            || self.cancellation.0.load(Ordering::Acquire)
            || Instant::now() >= self.until
        {
            return Err(Error::RequestClosed);
        }
        if !self
            .child
            .0
            .state
            .lock()
            .map_err(|_| Error::ChildUnavailable)?
            .accepting
        {
            return Err(Error::RequestClosed);
        }
        Ok(())
    }
    /// Copy bounded transfer-decoded bytes on the host request thread.
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.live()?;
        if self.phase != Phase::Reading || bytes.len() > self.length - self.bytes.len() {
            self.cancel();
            return Err(Error::BodyUnavailable);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    /// Only actual end-of-input establishes a complete body, including empty.
    pub fn finish(&mut self) -> Result<(), Error> {
        self.live()?;
        if self.phase != Phase::Reading || self.bytes.len() != self.length {
            self.cancel();
            return Err(Error::BodyUnavailable);
        }
        self.phase = Phase::Complete;
        Ok(())
    }
    /// Borrow complete copied input for shared normalized encoding/authority IPC.
    pub fn body(&self) -> Result<&[u8], Error> {
        self.live()?;
        if self.phase != Phase::Complete {
            return Err(Error::BodyUnavailable);
        }
        Ok(&self.bytes)
    }
    /// Remaining original budget; callers cannot restart input/IPC deadlines.
    pub fn remaining(&self) -> Result<Duration, Error> {
        self.live()?;
        self.until
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(Error::RequestClosed)
    }
    /// Pool cleanup/abort closes input; cancellation never grants origin effects.
    pub fn cancel(&mut self) {
        self.cancellation.cancel();
        self.phase = Phase::Closed;
        self.bytes.clear();
    }
    /// Register this signal with native pool cleanup before any host body read.
    /// Registration and actual cleanup invocation require the native ABI pilot.
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        // A forked child must not lock or mutate the parent's copied mutex.
        if self.child.process().is_ok()
            && let Ok(mut state) = self.child.0.state.lock()
        {
            state.active -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn child() -> Child {
        Child::initialize(7, Mpm::Prefork, 1).unwrap()
    }
    fn begin(c: &Child, n: usize) -> Result<Request, Error> {
        c.begin(7, true, n, 8, Duration::from_secs(1))
    }
    #[test]
    fn admission_retained_until_drop_and_shutdown_invalidates_input() {
        let c = child();
        let mut r = begin(&c, 0).unwrap();
        assert!(begin(&c, 0).is_err());
        r.finish().unwrap();
        assert!(begin(&c, 0).is_err());
        c.shutdown().unwrap();
        assert_eq!(r.body(), Err(Error::RequestClosed));
        drop(r);
        assert!(begin(&c, 0).is_err());
        assert_eq!(c.0.state.lock().unwrap().active, 0);
    }
    #[test]
    fn overflow_and_truncation_poison_instead_of_authorizing_prefix() {
        for overflow in [true, false] {
            let c = child();
            let mut r = begin(&c, 2).unwrap();
            r.append(&[0]).unwrap();
            if overflow {
                assert!(r.append(&[1, 2]).is_err());
            } else {
                assert!(r.finish().is_err());
            }
            assert!(r.append(&[1]).is_err());
            assert!(r.body().is_err());
            drop(r);
            assert!(begin(&c, 0).is_ok());
        }
    }
    #[test]
    fn complete_binary_bytes_and_cancellation() {
        let c = child();
        let mut r = begin(&c, 2).unwrap();
        assert!(r.body().is_err());
        r.append(&[0, 255]).unwrap();
        r.finish().unwrap();
        assert_eq!(r.body().unwrap(), [0, 255]);
        r.cancel();
        assert!(r.body().is_err());
        assert!(begin(&c, 0).is_err());
        drop(r);
        assert!(begin(&c, 0).is_ok());
    }
    #[test]
    fn rejects_generation_subrequest_fork_and_prefork_concurrency() {
        let c = child();
        assert!(c.begin(8, true, 0, 8, Duration::from_secs(1)).is_err());
        assert!(c.begin(7, false, 0, 8, Duration::from_secs(1)).is_err());
        assert!(Child::initialize(1, Mpm::Prefork, 2).is_err());
        let inherited = Child(Arc::new(Owner {
            pid: std::process::id().wrapping_add(1),
            generation: 7,
            capacity: 1,
            state: Mutex::new(State {
                accepting: true,
                active: 0,
            }),
        }));
        assert!(begin(&inherited, 0).is_err());
        assert!(inherited.shutdown().is_err());
    }
    #[test]
    fn expired_body_cannot_progress() {
        let c = child();
        let mut r = begin(&c, 0).unwrap();
        r.until = Instant::now();
        assert!(r.finish().is_err());
        assert!(r.remaining().is_err());
    }
    #[test]
    fn detached_cleanup_signal_revokes_complete_body_and_keeps_admission() {
        let c = child();
        let mut r = begin(&c, 0).unwrap();
        let cleanup = r.cancellation();
        r.finish().unwrap();
        cleanup.cancel();
        cleanup.cancel();
        assert!(r.body().is_err());
        assert!(r.remaining().is_err());
        assert!(begin(&c, 0).is_err());
        drop(r);
        assert!(begin(&c, 0).is_ok());
        cleanup.cancel();
        assert!(begin(&c, 0).is_ok());
    }
    #[test]
    fn known_mpm_identity_requires_matching_independent_capabilities() {
        assert_eq!(Mpm::identify("prefork", false, None, 1), Ok(Mpm::Prefork));
        assert_eq!(Mpm::identify("worker", true, None, 25), Ok(Mpm::Worker));
        assert_eq!(Mpm::identify("event", true, Some(true), 25), Ok(Mpm::Event));
    }
    #[test]
    fn unknown_or_inconsistent_mpm_capabilities_never_fall_back() {
        for (name, threaded, asynchronous, threads) in [
            ("unknown", false, None, 1),
            ("event", true, None, 25),
            ("event", true, Some(false), 25),
            ("event", false, Some(true), 25),
            ("worker", false, None, 25),
            ("worker", true, Some(true), 25),
            ("prefork", true, None, 1),
            ("prefork", false, None, 2),
        ] {
            assert!(Mpm::identify(name, threaded, asynchronous, threads).is_err());
        }
    }
    #[test]
    fn no_mpm_can_admit_with_missing_thread_capacity() {
        for (name, threaded, asynchronous) in [
            ("prefork", false, None),
            ("worker", true, None),
            ("event", true, Some(true)),
        ] {
            assert!(Mpm::identify(name, threaded, asynchronous, 0).is_err());
        }
    }
}
