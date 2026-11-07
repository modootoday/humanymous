//! Safe Rust request ownership for a future ASP.NET Core host binding.
//!
//! No managed object, pointer, callback or GC handle crosses this API. This is
//! not an installed Kestrel middleware or a qualified P/Invoke export.
#![forbid(unsafe_code)]

use humanymous_authority::client::{BoundResponse, Verdict};
use humanymous_framework_adapter::{
    Admission, Configuration, Error, Observation, Pending, internal_header,
};
use humanymous_integrity::adapter::AdapterProfile;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Reading,
    Sealed,
    Cancelled,
    HandedOff,
    Closed,
}

#[derive(PartialEq, Eq)]
struct Binding {
    method: String,
    target: String,
    authority: String,
    headers: Vec<[String; 2]>,
}

impl Binding {
    fn observed(o: &Observation) -> Self {
        Self {
            method: o.method.clone(),
            target: o.target.clone(),
            authority: o.authority.clone(),
            headers: o
                .headers
                .iter()
                .filter(|[name, _]| !internal_header(name))
                .cloned()
                .collect(),
        }
    }

    fn matches(&self, o: &Observation) -> bool {
        self.method == o.method
            && self.target == o.target
            && self.authority == o.authority
            && self
                .headers
                .iter()
                .eq(o.headers.iter().filter(|[name, _]| !internal_header(name)))
    }
}

/// Current request-path host state, sampled immediately before handoff.
pub struct HostState<'a> {
    /// Actual current normalized request; never client-provided serialized state.
    pub observation: &'a Observation,
    /// Actual RequestAborted cancellation status.
    pub aborted: bool,
    /// Actual HttpResponse.HasStarted value.
    pub response_started: bool,
    /// Actual host upgrade feature; upgrades are unsupported.
    pub upgrade: bool,
}

/// Request-specific cancellation; clones never cancel a subsequent request.
#[derive(Clone)]
pub struct Cancellation(Arc<Mutex<Phase>>);

impl Cancellation {
    /// Linearize cancellation against handoff. False means handoff already won.
    /// A poisoned lock is an error and cannot grant origin access.
    pub fn cancel(&self) -> Result<bool, Error> {
        let mut phase = self.0.lock().map_err(|_| Error("lifecycle unavailable"))?;
        match *phase {
            Phase::HandedOff | Phase::Closed => Ok(false),
            _ => {
                *phase = Phase::Cancelled;
                Ok(true)
            }
        }
    }
}

/// Owned complete-body request. Dropping it releases process admission.
/// The host must start this before reading and before any application effect.
pub struct Request {
    body: Vec<u8>,
    maximum: usize,
    declared: Option<usize>,
    pending: Pending,
    decision: Option<BoundResponse>,
    decision_started: bool,
    binding: Option<Binding>,
    cancellation: Cancellation,
    _admission: Admission,
}

impl Request {
    /// Reserve bounded process capacity and one total deadline before input.
    /// Unknown length is supported only when the host later observes actual EOF.
    pub fn begin(c: &Configuration, declared: Option<usize>) -> Result<Self, Error> {
        if declared.is_some_and(|n| n > c.max_body) {
            return Err(Error("body exceeds limit"));
        }
        let admission = Admission::acquire()?;
        let pending = Pending::new(c)?;
        Ok(Self {
            body: Vec::new(),
            maximum: c.max_body,
            declared,
            pending,
            decision: None,
            decision_started: false,
            binding: None,
            cancellation: Cancellation(Arc::new(Mutex::new(Phase::Reading))),
            _admission: admission,
        })
    }

    /// Register this token with RequestAborted before the first host read.
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }

    /// Copy bytes while the host read buffer is pinned/borrowed. Nothing retains
    /// the supplied slice; managed pooled buffers can be returned after this call.
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let mut phase = self
            .cancellation
            .0
            .lock()
            .map_err(|_| Error("lifecycle unavailable"))?;
        if *phase != Phase::Reading || self.pending.remaining().is_err() {
            *phase = Phase::Cancelled;
            return Err(Error("request input unavailable"));
        }
        let length = self.body.len().checked_add(bytes.len());
        if length.is_none_or(|n| n > self.maximum || self.declared.is_some_and(|d| n > d)) {
            *phase = Phase::Cancelled;
            self.body.clear();
            return Err(Error("body exceeds limit"));
        }
        self.body.try_reserve_exact(bytes.len()).map_err(|_| {
            *phase = Phase::Cancelled;
            Error("body capacity unavailable")
        })?;
        self.body.extend_from_slice(bytes);
        Ok(())
    }

    /// Seal only on actual successful EOF, never on cancellation, partial reads,
    /// declared-length exhaustion, parsed form completion or an input exception.
    pub fn eof(&mut self) -> Result<(), Error> {
        let mut phase = self
            .cancellation
            .0
            .lock()
            .map_err(|_| Error("lifecycle unavailable"))?;
        if *phase != Phase::Reading
            || self.pending.remaining().is_err()
            || self.declared.is_some_and(|n| n != self.body.len())
        {
            *phase = Phase::Cancelled;
            return Err(Error("complete body unavailable"));
        }
        *phase = Phase::Sealed;
        Ok(())
    }

    /// Borrow exact sealed entity bytes for the shared authority client. No
    /// authorization or origin permission follows from obtaining this slice.
    pub fn body(&self) -> Result<&[u8], Error> {
        let phase = self
            .cancellation
            .0
            .lock()
            .map_err(|_| Error("lifecycle unavailable"))?;
        if *phase != Phase::Sealed {
            return Err(Error("complete body unavailable"));
        }
        self.pending.remaining()?;
        Ok(&self.body)
    }

    /// Authenticate this exact complete body through the shared authority client.
    /// The host owns cancellation of this future; it must drop it on abort.
    /// A cancelled/failed decision cannot be retried on this request.
    pub async fn decide(
        &mut self,
        c: &Configuration,
        observation: &Observation,
    ) -> Result<(), Error> {
        if self.decision_started {
            return Err(Error("decision already started"));
        }
        self.body()?;
        self.decision_started = true;
        if observation.method == "CONNECT"
            || observation
                .headers
                .iter()
                .any(|[n, _]| n.eq_ignore_ascii_case("upgrade"))
        {
            self.cancellation.cancel()?;
            return Err(Error("unsupported request mode"));
        }
        let result = self
            .pending
            .decide(c, observation, &self.body, AdapterProfile::DotnetAspNetCore)
            .await;
        match result {
            Ok(decision) => {
                self.body()?;
                self.binding = Some(Binding::observed(observation));
                self.decision = Some(decision);
                Ok(())
            }
            Err(error) => {
                self.cancellation.cancel()?;
                Err(error)
            }
        }
    }

    /// Consume a real authority result exactly once and transfer the exact owned
    /// body for replay. The caller must check current host state synchronously:
    /// RequestAborted=false, HasStarted=false, identical method/raw target/host/
    /// sanitized headers, no upgrade/CONNECT and unchanged route binding.
    /// A host must not invoke next until this succeeds. Cancellation after the
    /// linearization point belongs to the running application's cancellation.
    pub fn handoff(mut self, host: HostState<'_>) -> Result<Vec<u8>, Error> {
        let host_unchanged = !host.aborted
            && !host.response_started
            && !host.upgrade
            && self
                .binding
                .as_ref()
                .is_some_and(|b| b.matches(host.observation))
            && !host.observation.raw
            && host.observation.peer.is_none()
            && host.observation.connection.is_none();
        let decision = self
            .decision
            .take()
            .ok_or(Error("authority result unavailable"))?;
        let expires = decision.expires();
        let verdict = decision
            .consume()
            .map_err(|_| Error("authority result unavailable"))?;
        self.finish(verdict, expires, host_unchanged)
    }

    fn finish(
        mut self,
        verdict: Verdict,
        expires: u64,
        host_unchanged: bool,
    ) -> Result<Vec<u8>, Error> {
        let mut phase = self
            .cancellation
            .0
            .lock()
            .map_err(|_| Error("lifecycle unavailable"))?;
        if *phase != Phase::Sealed || !host_unchanged || verdict != Verdict::Allow {
            *phase = Phase::Closed;
            return Err(Error("origin handoff refused"));
        }
        self.pending.check_handoff(expires)?;
        *phase = Phase::HandedOff;
        Ok(std::mem::take(&mut self.body))
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if let Ok(mut phase) = self.cancellation.0.lock()
            && *phase != Phase::HandedOff
        {
            *phase = Phase::Closed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_binding_detects_route_authority_auth_and_duplicate_mutation() {
        let mut observation = Observation {
            method: "POST".into(),
            target: "/api?q=1".into(),
            authority: "example.test".into(),
            headers: vec![
                ["authorization".into(), "Bearer opaque".into()],
                ["x-repeat".into(), "a".into()],
                ["x-repeat".into(), "b".into()],
            ],
            peer: None,
            connection: None,
            raw: false,
        };
        let binding = Binding::observed(&observation);
        assert!(binding.matches(&observation));
        observation.target.push('2');
        assert!(!binding.matches(&observation));
        observation.target.pop();
        observation.authority.push('2');
        assert!(!binding.matches(&observation));
        observation.authority.pop();
        observation.headers.swap(1, 2);
        assert!(!binding.matches(&observation));
        observation.headers.swap(1, 2);
        observation.headers[0][1].push('2');
        assert!(!binding.matches(&observation));
    }

    fn config() -> Configuration {
        let endpoint = if cfg!(windows) {
            r"\\.\pipe\dotnet-unit"
        } else {
            "/tmp/dotnet-unit.sock"
        };
        Configuration::parse(&format!(r#"{{"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":{},"credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{},"routes":["/api"],"timeoutMs":30000,"maxBody":4}}"#,
            serde_json::to_string(endpoint).unwrap(),
            if cfg!(windows) { r#"{"kind":"windows","sid":"S-1-5-18","pid":1}"# } else { r#"{"kind":"unix","uid":0}"# })).unwrap()
    }

    #[test]
    fn body_copies_binary_and_requires_actual_eof() {
        let mut request = Request::begin(&config(), Some(3)).unwrap();
        let mut source = [0, 255, 128];
        request.append(&source).unwrap();
        source.fill(42);
        assert!(request.body().is_err());
        request.eof().unwrap();
        assert_eq!(request.body().unwrap(), [0, 255, 128]);
        assert!(request.decision.is_none());
    }

    #[test]
    fn incomplete_oversized_and_cancelled_input_are_terminal() {
        let mut short = Request::begin(&config(), Some(3)).unwrap();
        short.append(&[1]).unwrap();
        assert!(short.eof().is_err());
        assert!(short.append(&[2, 3]).is_err());
        let mut long = Request::begin(&config(), None).unwrap();
        assert!(long.append(&[1; 5]).is_err());
        assert!(long.eof().is_err());
        let mut aborted = Request::begin(&config(), None).unwrap();
        assert!(aborted.cancellation().cancel().unwrap());
        assert!(aborted.append(&[]).is_err());
        assert!(aborted.eof().is_err());
        assert!(Request::begin(&config(), Some(5)).is_err());
    }

    #[test]
    fn cancellation_after_seal_prevents_handoff_and_old_tokens_are_isolated() {
        let mut first = Request::begin(&config(), Some(0)).unwrap();
        let old = first.cancellation();
        first.eof().unwrap();
        old.cancel().unwrap();
        assert!(first.finish(Verdict::Allow, u64::MAX, true).is_err());
        assert!(!old.cancel().unwrap());
        let mut second = Request::begin(&config(), Some(0)).unwrap();
        second.eof().unwrap();
        assert!(second.body().unwrap().is_empty());
    }

    #[test]
    fn denial_challenge_host_mutation_and_expiry_never_replay() {
        for verdict in [Verdict::Deny, Verdict::Challenge] {
            let mut request = Request::begin(&config(), Some(0)).unwrap();
            request.eof().unwrap();
            assert!(request.finish(verdict, u64::MAX, true).is_err());
        }
        for (expires, unchanged) in [(0, true), (u64::MAX, false)] {
            let mut request = Request::begin(&config(), Some(0)).unwrap();
            request.eof().unwrap();
            assert!(request.finish(Verdict::Allow, expires, unchanged).is_err());
        }
    }

    #[test]
    fn successful_core_handoff_is_single_owner() {
        let mut request = Request::begin(&config(), None).unwrap();
        let cancellation = request.cancellation();
        request.append(&[1, 2, 3, 4]).unwrap();
        request.eof().unwrap();
        assert_eq!(
            request.finish(Verdict::Allow, u64::MAX, true).unwrap(),
            [1, 2, 3, 4]
        );
        assert!(!cancellation.cancel().unwrap());
    }

    #[test]
    fn racing_cancellation_and_handoff_have_one_linearized_winner() {
        for _ in 0..16 {
            let mut request = Request::begin(&config(), Some(0)).unwrap();
            request.eof().unwrap();
            let cancellation = request.cancellation();
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let other = Arc::clone(&barrier);
            let worker = std::thread::spawn(move || {
                other.wait();
                cancellation.cancel().unwrap()
            });
            barrier.wait();
            let handed_off = request.finish(Verdict::Allow, u64::MAX, true).is_ok();
            let cancelled = worker.join().unwrap();
            assert_ne!(handed_off, cancelled);
        }
    }
}
