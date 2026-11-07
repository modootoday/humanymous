//! Pointer-free host composition over the shared native allocation registry.
use crate::{Cancellation, HostState, Request};
use humanymous_authority::client::Verdict;
use humanymous_framework_adapter::{Configuration, Error, Observation, internal_header};
use humanymous_native_abi::{Boundary, Command, Reply, Response, Status, Version};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

/// Configure one installation using an owned JSON buffer.
pub const CONFIGURE: u32 = 100;
/// Begin a request; length=u64::MAX means unknown entity length.
pub const BEGIN: u32 = 101;
/// Append at most32 inline entity bytes.
pub const APPEND: u32 = 102;
/// Observe actual successful host EOF.
pub const EOF: u32 = 103;
/// Prepare/run one authority exchange outside the bridge lock.
pub const DECIDE: u32 = 104;
/// Cancel one request without waiting for an authority exchange.
pub const CANCEL: u32 = 105;
/// Validate the current host state and consume the final origin handoff once.
pub const HANDOFF: u32 = 106;
/// Classify a header using the shared framework trust-header implementation.
pub const INTERNAL_HEADER: u32 = 107;
/// Copy approved bytes before the final liveness/once-only origin handoff.
pub const REPLAY: u32 = 108;
/// Bounded diagnostics: active16bits, in-flight16bits, initialized bytes32bits.
pub const METRICS: u32 = 109;

struct Flight(Arc<AtomicUsize>);
impl Drop for Flight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    method: String,
    target: String,
    authority: String,
    headers: Vec<[String; 2]>,
    route: [String; 3],
}
fn observation(bytes: &[u8]) -> Result<(Observation, [String; 3]), Status> {
    if bytes.len() > 65_536 {
        return Err(Status::ARGUMENT);
    }
    let input: Input = serde_json::from_slice(bytes).map_err(|_| Status::ARGUMENT)?;
    Ok((
        Observation {
            method: input.method,
            target: input.target,
            authority: input.authority,
            headers: input.headers,
            peer: None,
            connection: None,
            raw: false,
        },
        input.route,
    ))
}
struct Active {
    owner: u64,
    configuration: Arc<Configuration>,
    request: Option<Request>,
    cancellation: Cancellation,
    replay_issued: bool,
    route: Option<[String; 3]>,
}

/// One native library instance's request and allocation owner.
/// Generic handle generation/accounting belong exclusively to native-abi.
#[derive(Default)]
pub struct Bridge {
    identity: Arc<()>,
    flights: Arc<AtomicUsize>,
    boundary: Boundary,
    configurations: BTreeMap<u64, Arc<Configuration>>,
    active: BTreeMap<u64, Active>,
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.revoke();
    }
}

/// An owned bounded authority call. No managed memory or global lock is retained.
pub struct Ticket {
    flight: Flight,
    identity: Arc<()>,
    context: u64,
    configuration: Arc<Configuration>,
    request: Request,
    observation: Observation,
    route: [String; 3],
}
/// Completed request returned to its originating bridge instance only.
pub struct Completion {
    _flight: Flight,
    identity: Arc<()>,
    context: u64,
    request: Request,
    result: Result<(), Error>,
    route: [String; 3],
}
impl Ticket {
    /// Run on a Rust runtime from a managed worker, with cancellation dropping
    /// the actual shared IPC future. Polls every5ms plus scheduler delay.
    pub async fn run(mut self) -> Completion {
        let cancellation = self.request.cancellation();
        let cancelled = async {
            loop {
                let cancelled = cancellation
                    .0
                    .lock()
                    .map(|p| matches!(*p, crate::Phase::Cancelled | crate::Phase::Closed))
                    .unwrap_or(true);
                if cancelled {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };
        let result = tokio::select! {
            result = self.request.decide(&self.configuration, &self.observation) => result,
            () = cancelled => Err(Error("request cancelled")),
        };
        Completion {
            _flight: self.flight,
            identity: self.identity,
            context: self.context,
            request: self.request,
            result,
            route: self.route,
        }
    }
}

fn response(result: Result<u64, Status>) -> Response {
    Response {
        reply: match result {
            Ok(value) => Reply {
                status: 0,
                reserved: 0,
                value,
            },
            Err(status) => Reply {
                status: status.0,
                reserved: 0,
                value: 0,
            },
        },
        data: [0; 32],
    }
}
fn shape(c: &Command) -> Result<(), Status> {
    c.version.validate()?;
    if c.flags != 0 || c.offset != 0 || (c.operation != APPEND && c.data != [0; 32]) {
        return Err(Status::ARGUMENT);
    }
    Ok(())
}
impl Bridge {
    /// Revoke requests and allocations after a boundary panic; never resume.
    pub fn revoke(&mut self) {
        for active in self.active.values() {
            let _ = active.cancellation.cancel();
        }
        self.active.clear();
        self.configurations.clear();
        self.boundary.revoke();
    }

    fn close(&mut self, context: u64) -> Response {
        let children: Vec<_> = self
            .active
            .iter()
            .filter_map(|(&id, a)| (id == context || a.owner == context).then_some(id))
            .collect();
        for id in children {
            if let Some(active) = self.active.remove(&id) {
                let _ = active.cancellation.cancel();
            }
            if id != context {
                let _ = self.boundary.call(|r| r.close(id).map(|()| 0));
            }
        }
        self.configurations.remove(&context);
        self.boundary.dispatch(Command {
            operation: Command::CLOSE,
            context,
            ..Command::default()
        })
    }

    /// Dispatch synchronous input/cleanup operations. DECIDE must use prepare.
    pub fn dispatch(&mut self, c: Command) -> Response {
        if c.operation <= Command::READ {
            if c.operation == Command::CLOSE {
                if let Err(status) = c.version.validate() {
                    return response(Err(status));
                }
                if c.flags != 0
                    || c.buffer != 0
                    || c.offset != 0
                    || c.length != 0
                    || c.data != [0; 32]
                {
                    return response(Err(Status::ARGUMENT));
                }
                return self.close(c.context);
            }
            return self.boundary.dispatch(c);
        }
        if let Err(e) = shape(&c) {
            return response(Err(e));
        }
        let configs = &mut self.configurations;
        let active = &mut self.active;
        let flights = &self.flights;
        let reply = self.boundary.call(|registry| {
            registry.validate_context(c.context)?;
            match c.operation {
                METRICS if c.buffer == 0 && c.length == 0 && configs.contains_key(&c.context) => {
                    Ok(((active.len() as u64) << 48)
                        | ((flights.load(Ordering::Acquire) as u64) << 32)
                        | registry.allocated_bytes())
                }
                CONFIGURE
                    if c.length == 0
                        && !configs.contains_key(&c.context)
                        && !active.contains_key(&c.context) =>
                {
                    let bytes = registry.buffer_bytes(c.context, c.buffer)?;
                    if bytes.len() > 65_536 {
                        return Err(Status::ARGUMENT);
                    }
                    let text = std::str::from_utf8(bytes).map_err(|_| Status::ARGUMENT)?;
                    let config = Configuration::parse(text).map_err(|_| Status::ARGUMENT)?;
                    configs.insert(c.context, Arc::new(config));
                    Ok(0)
                }
                BEGIN if c.buffer == 0 => {
                    let configuration = configs.get(&c.context).ok_or(Status::HANDLE)?.clone();
                    let declared = if c.length == u64::MAX {
                        None
                    } else {
                        Some(usize::try_from(c.length).map_err(|_| Status::ARGUMENT)?)
                    };
                    let request =
                        Request::begin(&configuration, declared).map_err(|_| Status::CAPACITY)?;
                    let id = registry.open(Version::default())?;
                    let cancellation = request.cancellation();
                    active.insert(
                        id,
                        Active {
                            owner: c.context,
                            configuration,
                            request: Some(request),
                            cancellation,
                            replay_issued: false,
                            route: None,
                        },
                    );
                    Ok(id)
                }
                APPEND
                    if c.buffer == 0
                        && c.length <= 32
                        && c.data[c.length as usize..].iter().all(|&b| b == 0) =>
                {
                    active
                        .get_mut(&c.context)
                        .and_then(|a| a.request.as_mut())
                        .ok_or(Status::HANDLE)?
                        .append(&c.data[..c.length as usize])
                        .map_err(|_| Status::ARGUMENT)?;
                    Ok(0)
                }
                EOF if c.buffer == 0 && c.length == 0 => {
                    active
                        .get_mut(&c.context)
                        .and_then(|a| a.request.as_mut())
                        .ok_or(Status::HANDLE)?
                        .eof()
                        .map_err(|_| Status::ARGUMENT)?;
                    Ok(0)
                }
                CANCEL if c.buffer == 0 && c.length == 0 => {
                    active
                        .get(&c.context)
                        .ok_or(Status::HANDLE)?
                        .cancellation
                        .cancel()
                        .map_err(|_| Status::POISONED)?;
                    Ok(0)
                }
                HANDOFF if c.length <= 7 => {
                    let (observed, route) =
                        observation(registry.buffer_bytes(c.context, c.buffer)?)?;
                    let a = active.get_mut(&c.context).ok_or(Status::HANDLE)?;
                    if !a.replay_issued || a.route.as_ref() != Some(&route) {
                        return Err(Status::ARGUMENT);
                    }
                    let request = a.request.take().ok_or(Status::HANDLE)?;
                    let _body = request
                        .handoff(HostState {
                            observation: &observed,
                            aborted: c.length & 1 != 0,
                            response_started: c.length & 2 != 0,
                            upgrade: c.length & 4 != 0,
                        })
                        .map_err(|_| Status::ARGUMENT)?;
                    Ok(0)
                }
                REPLAY if c.buffer == 0 && c.length == 0 => {
                    let a = active.get_mut(&c.context).ok_or(Status::HANDLE)?;
                    if a.replay_issued {
                        return Err(Status::ARGUMENT);
                    }
                    let request = a.request.as_ref().ok_or(Status::HANDLE)?;
                    if !matches!(
                        request.decision.as_ref().map(|d| d.verdict()),
                        Some(Verdict::Allow)
                    ) {
                        return Err(Status::ARGUMENT);
                    }
                    let body = request.body().map_err(|_| Status::ARGUMENT)?;
                    let handle = registry.allocate(c.context, body.len() as u64)?;
                    if let Err(error) = registry.write(c.context, handle, 0, body) {
                        let _ = registry.free(c.context, handle);
                        return Err(error);
                    }
                    a.replay_issued = true;
                    Ok(handle)
                }
                INTERNAL_HEADER if c.length == 0 => {
                    let bytes = registry.buffer_bytes(c.context, c.buffer)?;
                    if bytes.len() > 1024 {
                        return Err(Status::ARGUMENT);
                    }
                    Ok(u64::from(internal_header(
                        std::str::from_utf8(bytes).map_err(|_| Status::ARGUMENT)?,
                    )))
                }
                _ => Err(Status::ARGUMENT),
            }
        });
        Response {
            reply,
            data: [0; 32],
        }
    }

    /// Take the request once, then release the bridge lock before any I/O.
    pub fn prepare(&mut self, c: Command) -> Result<Ticket, Status> {
        shape(&c)?;
        if c.operation != DECIDE || c.length != 0 {
            return Err(Status::ARGUMENT);
        }
        let mut ticket = None;
        let identity = self.identity.clone();
        let flights = self.flights.clone();
        let active = &mut self.active;
        let reply = self.boundary.call(|r| {
            r.validate_context(c.context)?;
            let (observation, route) = observation(r.buffer_bytes(c.context, c.buffer)?)?;
            let a = active.get_mut(&c.context).ok_or(Status::HANDLE)?;
            let request = a.request.take().ok_or(Status::HANDLE)?;
            request.body().map_err(|_| Status::ARGUMENT)?;
            ticket = Some(Ticket {
                flight: {
                    flights.fetch_add(1, Ordering::AcqRel);
                    Flight(flights.clone())
                },
                identity: identity.clone(),
                context: c.context,
                configuration: a.configuration.clone(),
                request,
                observation,
                route,
            });
            Ok(0)
        });
        if reply.status != 0 {
            return Err(Status(reply.status));
        }
        ticket.ok_or(Status::POISONED)
    }

    /// Complete only if this never-reused request context is still live.
    pub fn complete(&mut self, completion: Completion) -> Response {
        let Completion {
            identity,
            context,
            request,
            result,
            route,
            _flight,
        } = completion;
        if !Arc::ptr_eq(&identity, &self.identity) {
            return response(Err(Status::HANDLE));
        }
        if result.is_err() {
            let _ = self.close(context);
            return response(Err(Status::ARGUMENT));
        }
        let Some(active) = self.active.get_mut(&context) else {
            return response(Err(Status::HANDLE));
        };
        if active.request.is_some() || request.body().is_err() {
            return response(Err(Status::HANDLE));
        }
        let value = match request.decision.as_ref().map(|d| d.verdict()) {
            Some(Verdict::Allow) => 1,
            Some(Verdict::Deny) => 2,
            Some(Verdict::Challenge) => 3,
            None => return response(Err(Status::ARGUMENT)),
        };
        active.request = Some(request);
        active.route = Some(route);
        response(Ok(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn command(operation: u32, context: u64) -> Command {
        Command {
            operation,
            context,
            ..Command::default()
        }
    }
    fn ok(response: Response) -> u64 {
        assert_eq!(response.reply.status, 0);
        response.reply.value
    }
    fn upload(b: &mut Bridge, context: u64, bytes: &[u8]) -> u64 {
        let handle = ok(b.dispatch(Command {
            length: bytes.len() as u64,
            ..command(Command::ALLOCATE, context)
        }));
        for (n, chunk) in bytes.chunks(32).enumerate() {
            let mut c = Command {
                buffer: handle,
                offset: (n * 32) as u64,
                length: chunk.len() as u64,
                ..command(Command::WRITE, context)
            };
            c.data[..chunk.len()].copy_from_slice(chunk);
            ok(b.dispatch(c));
        }
        handle
    }
    fn configured() -> (Bridge, u64, u64) {
        let mut b = Bridge::default();
        let owner = ok(b.dispatch(command(Command::OPEN, 0)));
        let endpoint = if cfg!(windows) {
            r"\\.\pipe\dotnet-unit"
        } else {
            "/tmp/dotnet-unit.sock"
        };
        let server = if cfg!(windows) {
            json!({"kind":"windows","sid":"S-1-5-18","pid":1})
        } else {
            json!({"kind":"unix","uid":0})
        };
        let bytes=serde_json::to_vec(&json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,"credential":"a".repeat(32),"server":server,"routes":["/api"],"timeoutMs":30000,"maxBody":8})).unwrap();
        let buffer = upload(&mut b, owner, &bytes);
        ok(b.dispatch(Command {
            buffer,
            ..command(CONFIGURE, owner)
        }));
        ok(b.dispatch(Command {
            buffer,
            ..command(Command::FREE, owner)
        }));
        let request = ok(b.dispatch(command(BEGIN, owner)));
        (b, owner, request)
    }
    fn metadata(b: &mut Bridge, request: u64) -> u64 {
        upload(b,request,br#"{"method":"POST","target":"/api","authority":"example.test","headers":[],"route":["","/api",""]}"#)
    }
    #[test]
    fn closing_installation_revokes_children_and_shared_buffers() {
        let (mut b, owner, request) = configured();
        let cancellation = b.active[&request].cancellation.clone();
        let buffer = upload(&mut b, request, b"opaque");
        assert!(ok(b.dispatch(command(METRICS, owner))) > 0);
        ok(b.dispatch(command(Command::CLOSE, owner)));
        assert!(b.active.is_empty());
        assert!(b.configurations.is_empty());
        assert_eq!(
            b.dispatch(Command {
                buffer,
                ..command(Command::LENGTH, request)
            })
            .reply
            .status,
            Status::HANDLE.0
        );
        assert!(!cancellation.cancel().unwrap());
    }
    #[test]
    fn dropped_ticket_cannot_restart_and_decrements_flight_lease() {
        let (mut b, owner, request) = configured();
        ok(b.dispatch(command(EOF, request)));
        let buffer = metadata(&mut b, request);
        let c = Command {
            buffer,
            ..command(DECIDE, request)
        };
        let ticket = b.prepare(c).unwrap();
        assert_eq!(b.flights.load(Ordering::Acquire), 1);
        assert!(b.prepare(c).is_err());
        drop(ticket);
        assert_eq!(b.flights.load(Ordering::Acquire), 0);
        assert!(b.prepare(c).is_err());
        ok(b.dispatch(command(Command::CLOSE, request)));
        assert_eq!(ok(b.dispatch(command(METRICS, owner))), 0);
    }
    #[test]
    fn cross_bridge_completion_cannot_close_same_numeric_context() {
        let (mut first, _owner, request) = configured();
        ok(first.dispatch(command(EOF, request)));
        let buffer = metadata(&mut first, request);
        let ticket = first
            .prepare(Command {
                buffer,
                ..command(DECIDE, request)
            })
            .unwrap();
        let (mut other, _, other_request) = configured();
        assert_eq!(request, other_request);
        let completion = Completion {
            identity: ticket.identity,
            context: ticket.context,
            request: ticket.request,
            result: Err(Error("cancelled")),
            route: ticket.route,
            _flight: ticket.flight,
        };
        assert_eq!(other.complete(completion).reply.status, Status::HANDLE.0);
        assert!(other.active.contains_key(&other_request));
    }
    #[test]
    fn invalid_version_unknown_metadata_and_replay_without_allow_fail_closed() {
        let (mut b, owner, request) = configured();
        let mut bad = command(Command::CLOSE, owner);
        bad.flags = 1;
        assert_ne!(b.dispatch(bad).reply.status, 0);
        assert_ne!(b.dispatch(command(REPLAY, request)).reply.status, 0);
        ok(b.dispatch(command(EOF, request)));
        let buffer=upload(&mut b,request,br#"{"method":"POST","method":"GET","target":"/api","authority":"example.test","headers":[],"route":["","/api",""]}"#);
        assert!(
            b.prepare(Command {
                buffer,
                ..command(DECIDE, request)
            })
            .is_err()
        );
        assert!(b.active[&request].request.is_some());
        let mut version = command(CANCEL, request);
        version.version.major = 2;
        assert_eq!(b.dispatch(version).reply.status, Status::VERSION.0);
        ok(b.dispatch(command(Command::CLOSE, owner)));
    }
}
