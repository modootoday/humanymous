//! Ingress-only attacker. It executes the `humanymous-attacker` network catalog
//! with the catalog's own step logic through the Gate ingress, asks the runner
//! (over its private stdin/stdout) to start, restart and stop Gate sessions,
//! and evaluates each case with the catalog's own expectation against the
//! audit and origin effects the runner measured.
//!
//! Control protocol, one JSON line each way:
//! runner -> `{"gate":IP}`; attacker -> `{"op":"start","case":ID}`,
//! runner -> `{"op":"started","ok":bool}`; attacker -> `{"op":"restart"}`,
//! runner -> `{"op":"restarted","ok":bool}`; attacker -> `{"op":"stop","case":ID}`,
//! runner -> `{"op":"effects","effects":DELTA|null,"reason":TEXT|null}`;
//! finally attacker -> `{"op":"report","report":REPORT}`.
use super::{FRONTEND_PORT, REPORT_SCHEMA, SEED};
use crate::client::request;
use crate::origin_audit::{emit, read_line};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_attacker::{
    network::{self, NetworkRow, Step, Target},
    report::Status,
};
use humanymous_core::{
    integrity::request::{compute_token, seed},
    signals::report::ClientReport,
};
use humanymous_defender::lab::{LabError, Response, SnapshotDelta};
use humanymous_runtime::gateway::{
    configuration::Configuration, http::Request, session::canonical,
};
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

/// Volatile issued capability; never logged or serialized.
pub struct Issued {
    pub(crate) session: Zeroizing<String>,
    key: Zeroizing<Vec<u8>>,
    expires: u64,
}

/// The runner's side of the private control channel.
pub trait Operator {
    /// Send one control line and read the runner's reply.
    fn call(&mut self, message: &Value) -> Result<Value, LabError>;
}

pub(crate) struct Pipes<R: BufRead> {
    pub(crate) input: R,
}
impl<R: BufRead> Operator for Pipes<R> {
    fn call(&mut self, message: &Value) -> Result<Value, LabError> {
        emit(message).map_err(|_| LabError("control output"))?;
        read_line(&mut self.input, 1 << 20).map_err(|_| LabError("control input"))
    }
}

pub(crate) fn now_ms() -> Result<u64, LabError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| u64::try_from(value.as_millis()).ok())
        .ok_or(LabError("clock"))
}

/// Strict HTTP/1.1 response framing: exact status line and Content-Length.
pub(crate) fn parse(bytes: &[u8]) -> Result<Response, LabError> {
    let bad = LabError("response framing");
    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").ok_or(bad)?;
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| bad)?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next().ok_or(bad)?.splitn(3, ' ');
    if first.next() != Some("HTTP/1.1") {
        return Err(bad);
    }
    let code = first.next().ok_or(bad)?;
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) || first.next().is_none() {
        return Err(bad);
    }
    let status: u16 = code.parse().map_err(|_| bad)?;
    let mut headers = Vec::new();
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(bad)?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(bad);
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err(bad);
            }
            length = Some(value.parse::<usize>().map_err(|_| bad)?);
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    if !(200..600).contains(&status) || length != Some(bytes.len() - end - 4) {
        return Err(bad);
    }
    Ok(Response {
        status,
        headers,
        body: bytes[end + 4..].to_vec(),
    })
}

pub(crate) fn wire(request: &Request) -> Vec<u8> {
    let mut bytes = format!("{} {} HTTP/1.1\r\n", request.method, request.target).into_bytes();
    for (name, value) in &request.headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    bytes
}

/// The Gate in the `ac-gate` container, reached through its ingress relay;
/// with `tls`, the Gate in `tg-gate` whose runtime terminates TLS itself.
pub struct Remote<'a, O: Operator> {
    pub(crate) address: SocketAddr,
    pub(crate) config: Configuration,
    pub(crate) operator: &'a mut O,
    pub(crate) tls: Option<&'a crate::gate_tls::client::TlsClient>,
}

impl<O: Operator> Remote<'_, O> {
    fn sign_with(
        &self,
        request: &mut Request,
        issued: &Issued,
        counter: u64,
        now: u64,
        deadline: u64,
    ) -> Result<(), LabError> {
        if counter == 0
            || request.headers.iter().any(|(n, _)| {
                n.eq_ignore_ascii_case("authorization")
                    || n.to_ascii_lowercase().starts_with("x-humanymous-")
            })
        {
            return Err(LabError("request already signed or invalid counter"));
        }
        // An H2 request is signed in the server's lower-case header view.
        let lower = request
            .headers
            .iter()
            .all(|(name, _)| !name.bytes().any(|b| b.is_ascii_uppercase()));
        request.headers.extend([
            (
                if lower {
                    "authorization"
                } else {
                    "Authorization"
                }
                .into(),
                format!("Bearer {}", issued.session.as_str()),
            ),
            ("x-humanymous-counter".into(), counter.to_string()),
            ("x-humanymous-bucket".into(), (now / 10_000).to_string()),
            ("x-humanymous-deadline".into(), deadline.to_string()),
        ]);
        let text = Zeroizing::new(canonical(&self.config, request, &issued.session, deadline));
        request.headers.push((
            "x-humanymous-token".into(),
            compute_token(
                &seed(&issued.key, counter - 1),
                &text,
                counter,
                now / 10_000,
            ),
        ));
        Ok(())
    }
}

impl<O: Operator> Target for Remote<'_, O> {
    type Session = Issued;
    fn request(&self, target: &str, body: Vec<u8>) -> Request {
        request(target, body)
    }
    fn issue(&mut self) -> Result<Issued, LabError> {
        let response = self.exchange(&request("/_humanymous/session", Vec::new()))?;
        if response.status != 200 {
            return Err(LabError("session issue refused"));
        }
        let value: Value =
            serde_json::from_slice(&response.body).map_err(|_| LabError("session issue reply"))?;
        let session = value["session"].as_str().unwrap_or_default();
        let key = URL_SAFE_NO_PAD
            .decode(value["key"].as_str().unwrap_or_default())
            .map_err(|_| LabError("session key encoding"))?;
        let expires = value["expires"].as_u64().unwrap_or_default();
        if session.len() != 43 || key.len() != 32 || value["counter"] != 0 || expires <= now_ms()? {
            return Err(LabError("session issue binding"));
        }
        Ok(Issued {
            session: Zeroizing::new(session.to_owned()),
            key: Zeroizing::new(key),
            expires,
        })
    }
    fn bind_report(&self, issued: &Issued, report: &mut ClientReport) {
        report.session_id = issued.session.to_string();
    }
    fn sign(&self, request: &mut Request, issued: &Issued, counter: u64) -> Result<(), LabError> {
        let now = now_ms()?;
        let deadline = now
            .checked_add(self.config.timeout_ms.saturating_sub(1).min(4000))
            .ok_or(LabError("deadline overflow"))?
            .min(issued.expires);
        self.sign_with(request, issued, counter, now, deadline)
    }
    fn sign_expired(
        &self,
        request: &mut Request,
        issued: &Issued,
        counter: u64,
    ) -> Result<(), LabError> {
        let now = now_ms()?;
        self.sign_with(request, issued, counter, now, now.saturating_sub(1))
    }
    fn exchange(&mut self, request: &Request) -> Result<Response, LabError> {
        self.exchange_raw(&wire(request))
    }
    fn exchange_raw(&mut self, bytes: &[u8]) -> Result<Response, LabError> {
        if let Some(tls) = self.tls {
            return parse(&tls.exchange_h1(self.address, bytes)?);
        }
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(5))
            .map_err(|_| LabError("Gate ingress connect"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .and_then(|_| stream.set_read_timeout(Some(Duration::from_secs(10))))
            .map_err(|_| LabError("socket timeout"))?;
        stream
            .write_all(bytes)
            .map_err(|_| LabError("request write"))?;
        let mut response = Vec::new();
        (&mut stream)
            .take(81_921)
            .read_to_end(&mut response)
            .map_err(|_| LabError("response read"))?;
        if response.len() > 81_920 {
            return Err(LabError("response budget"));
        }
        parse(&response)
    }
    fn restart(&mut self) -> Result<(), LabError> {
        let reply = self.operator.call(&json!({"op":"restart"}))?;
        if reply["op"] == "restarted" && reply["ok"] == true {
            Ok(())
        } else {
            Err(LabError("Gate restart refused"))
        }
    }
}

/// Signing configuration: the Gate's scope and routes, placeholder custody.
pub(crate) fn configuration() -> Result<Configuration, LabError> {
    serde_json::from_value(super::document(
        "/unused",
        "/unused",
        "unused",
        false,
        Value::Null,
        65532,
    ))
    .map_err(|_| LabError("signing configuration"))
}

/// Run one supported case: Gate session, catalog steps, measured effects.
fn case(operator: &mut impl Operator, gate: IpAddr, entry: &network::Entry) -> NetworkRow {
    let mut row = NetworkRow {
        id: entry.id.into(),
        tier: entry.tier.into(),
        class: entry.class.into(),
        status: Status::Error,
        steps: Vec::new(),
        processes: Vec::new(),
        effects: None,
        expectation_matches: None,
        reason: None,
    };
    if entry.plan.is_empty() {
        // Unsupported: retained in the denominator with the catalog's reason.
        row.status = Status::Unsupported;
        row.reason = entry.reason.map(str::to_owned);
        return row;
    }
    execute(operator, gate, entry, row)
}

fn execute(
    operator: &mut impl Operator,
    gate: IpAddr,
    entry: &network::Entry,
    row: NetworkRow,
) -> NetworkRow {
    let id = entry.id;
    session(
        operator,
        id,
        row,
        |operator, steps| {
            let mut target = Remote {
                address: SocketAddr::new(gate, FRONTEND_PORT),
                config: configuration()?,
                operator,
                tls: None,
            };
            network::execute_case(&mut target, id, steps)
        },
        |steps, delta| network::expectation_matches(id, steps, delta),
    )
}

/// One Gate session around `steps`: the runner starts the session, the steps
/// run, the runner stops it and returns the measured effects, and `expect`
/// evaluates the steps against exactly those effects.
pub(crate) fn session<O: Operator>(
    operator: &mut O,
    id: &str,
    mut row: NetworkRow,
    steps: impl FnOnce(&mut O, &mut Vec<Step>) -> Result<(), LabError>,
    expect: impl FnOnce(&[Step], &SnapshotDelta) -> Option<bool>,
) -> NetworkRow {
    let started = operator.call(&json!({"op":"start","case":id}));
    if !started
        .as_ref()
        .is_ok_and(|reply| reply["op"] == "started" && reply["ok"] == true)
    {
        row.reason = Some("Gate session did not start".into());
        return row;
    }
    let execution = steps(&mut *operator, &mut row.steps);
    let effects = operator
        .call(&json!({"op":"stop","case":id}))
        .and_then(|reply| {
            if reply["op"] != "effects" {
                return Err(LabError("unexpected runner reply"));
            }
            serde_json::from_value::<Option<SnapshotDelta>>(reply["effects"].clone())
                .map_err(|_| LabError("effects encoding"))?
                .ok_or(LabError("runner could not measure effects"))
        });
    match (execution, effects) {
        (Ok(()), Ok(delta)) => {
            row.expectation_matches = expect(&row.steps, &delta);
            row.status = if row.expectation_matches.is_some() {
                Status::Evaluated
            } else {
                Status::Error
            };
            if row.expectation_matches.is_none() {
                row.reason = Some("no catalog expectation".into());
            }
            row.effects = Some(delta);
        }
        (Err(error), Ok(delta)) => {
            row.reason = Some(error.to_string());
            row.effects = Some(delta);
        }
        (Ok(()), Err(error)) => row.reason = Some(error.to_string()),
        (Err(error), Err(effects)) => {
            row.reason = Some(format!("execution: {error}; effects: {effects}"));
        }
    }
    row
}

/// The complete catalog in seed order, with totals and the scoped verdict.
pub fn report(operator: &mut impl Operator, gate: IpAddr) -> Value {
    let rows: Vec<NetworkRow> = network::entries(SEED)
        .iter()
        .map(|entry| case(&mut *operator, gate, entry))
        .collect();
    let count = |status: Status| rows.iter().filter(|row| row.status == status).count();
    let mismatches = rows
        .iter()
        .filter(|row| row.status == Status::Evaluated && row.expectation_matches != Some(true))
        .count();
    let evaluated = count(Status::Evaluated);
    let errors = count(Status::Error);
    let scoped_passed = evaluated > 0 && errors == 0 && mismatches == 0;
    json!({"schema":REPORT_SCHEMA,"seed":SEED,
        "evidence_type":"docker-internal-network-gate-http1-relay",
        "human_executed":false,"assistive_technology_executed":false,"full_product_qualified":false,
        "totals":{"scheduled":rows.len(),"evaluated":evaluated,"errors":errors,
            "unsupported":count(Status::Unsupported),"expectation_mismatches":mismatches},
        "scoped_passed":scoped_passed,"rows":rows})
}

pub fn run() {
    let mut input = BufReader::new(io::stdin());
    let outcome = (|| -> Result<Value, Box<dyn std::error::Error>> {
        let control = read_line(&mut input, 4096)?;
        let gate: IpAddr = control["gate"].as_str().ok_or("gate address")?.parse()?;
        let mut pipes = Pipes { input: &mut input };
        Ok(report(&mut pipes, gate))
    })();
    let report = outcome.unwrap_or_else(|error| {
        json!({"schema":REPORT_SCHEMA,"scoped_passed":false,"error":error.to_string(),
            "full_product_qualified":false})
    });
    let passed = report["scoped_passed"] == true;
    emit(&json!({"op":"report","report":report})).expect("attacker report output");
    if !passed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records every control line; refuses every Gate start.
    struct Refusing(Vec<Value>);
    impl Operator for Refusing {
        fn call(&mut self, message: &Value) -> Result<Value, LabError> {
            self.0.push(message.clone());
            Ok(json!({"op":"started","ok":false}))
        }
    }

    #[test]
    fn refused_gate_sessions_keep_the_whole_denominator_and_fail() {
        let mut operator = Refusing(Vec::new());
        let report = report(&mut operator, "127.0.0.1".parse().unwrap());
        assert_eq!(report["schema"], REPORT_SCHEMA);
        assert_eq!(report["totals"]["scheduled"], 26);
        assert_eq!(report["totals"]["errors"], 19);
        assert_eq!(report["totals"]["unsupported"], 7);
        assert_eq!(report["totals"]["evaluated"], 0);
        assert_eq!(report["scoped_passed"], false);
        assert_eq!(report["full_product_qualified"], false);
        // One start per supported case; nothing else is attempted.
        assert_eq!(operator.0.len(), 19);
        assert!(operator.0.iter().all(|m| m["op"] == "start"));
    }

    #[test]
    fn response_framing_is_strict() {
        let ok =
            parse(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 20\r\n\r\n{\"verdict\":\"DENY\"}  ")
                .unwrap();
        assert_eq!(ok.status, 403);
        assert_eq!(ok.verdict().as_deref(), Some("DENY"));
        for bad in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nok".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nok",
            b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nok",
            b"HTTP/1.1 2000 OK\r\nContent-Length: 2\r\n\r\nok",
            b"",
        ] {
            assert!(parse(bad).is_err());
        }
    }

    #[test]
    fn signing_matches_the_shared_verifier() {
        let mut operator = Refusing(Vec::new());
        let target = Remote {
            address: "127.0.0.1:1".parse().unwrap(),
            config: configuration().unwrap(),
            operator: &mut operator,
            tls: None,
        };
        let issued = Issued {
            session: Zeroizing::new("s".repeat(43)),
            key: Zeroizing::new(vec![7; 32]),
            expires: u64::MAX,
        };
        let mut req = target.request("/effect", b"effect".to_vec());
        target.sign_with(&mut req, &issued, 1, 1000, 5000).unwrap();
        let verify = |req: &Request| {
            humanymous_core::integrity::request::verify(
                humanymous_core::integrity::request::VerifyParams {
                    session_key: &[7; 32],
                    last_n: 0,
                    presented_n: 1,
                    presented_tb: 0,
                    current_tb: 0,
                    token: req.header("x-humanymous-token").unwrap(),
                    canonical: &canonical(&target.config, req, &issued.session, 5000),
                    first_req: false,
                },
            )
            .id
        };
        assert_eq!(verify(&req), "l5.rit.ok");
        req.body[0] = b'X';
        assert_ne!(verify(&req), "l5.rit.ok");
        assert!(target.sign_with(&mut req, &issued, 2, 1000, 5000).is_err());
    }

    #[test]
    fn exchange_against_a_local_listener_and_a_closed_port() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let _ = stream.read(&mut buffer).unwrap();
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
        });
        let mut operator = Refusing(Vec::new());
        let mut target = Remote {
            address,
            config: configuration().unwrap(),
            operator: &mut operator,
            tls: None,
        };
        let request = target.request("/effect", b"effect".to_vec());
        assert_eq!(target.exchange(&request).unwrap().status, 401);
        server.join().unwrap();
        assert!(target.exchange(&request).is_err());
        assert!(target.restart().is_err());
    }
}
