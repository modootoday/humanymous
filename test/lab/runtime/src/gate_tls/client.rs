//! Ingress-only TLS client for the full Gate TLS/H2/capture stage.
//!
//! It trusts only the lab CA the Gate generated, records the digest of every
//! server certificate it was presented, and runs each row in its own Gate
//! session over the attack catalog's private control protocol (start, stop,
//! measured effects). Catalog rows reuse the attacker catalog's own steps and
//! expectations through a TLS H1 transport; capture rows speak TLS H1, TLS H2
//! (hand-framed, stream 1) or plaintext to the TLS ingress.
use super::{
    CAPTURE, FRONTEND_PORT, GATE_CATALOG, GATE_EDGE, ORIGIN_PORT, REPORT_SCHEMA, SERVER_NAME,
};
use crate::attack_catalog::client::{Operator, Pipes, Remote, configuration, session};
use crate::origin_audit::{emit, read_line};
use httlib_hpack::{Decoder, Encoder};
use humanymous_attacker::{
    network::{self, NetworkRow, Step, Target},
    report::Status,
};
use humanymous_defender::lab::{LabError, Response, SnapshotDelta};
use humanymous_runtime::gateway::http::Request;
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, StreamOwned,
    pki_types::{CertificateDer, ServerName, pem::PemObject},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::{self, BufReader, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

const CONNECT: Duration = Duration::from_secs(5);
const READ: Duration = Duration::from_secs(10);
const RESPONSE_LIMIT: usize = 81_920;
const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const STREAM: u32 = 1;
const END_STREAM: u8 = 0x1;
const END_HEADERS: u8 = 0x4;
/// Truthful synthetic native identity: a Rust/rustls client says so.
pub const NATIVE_AGENT: &str = "HumanymousLabClient/1.0 Rustls";

/// Trust store holding exactly the PEM certificates in `ca_pem`.
pub fn roots(ca_pem: &str) -> Result<RootCertStore, LabError> {
    let mut roots = RootCertStore::empty();
    for certificate in CertificateDer::pem_slice_iter(ca_pem.as_bytes()) {
        roots
            .add(certificate.map_err(|_| LabError("lab CA PEM"))?)
            .map_err(|_| LabError("lab CA certificate"))?;
    }
    if roots.is_empty() {
        return Err(LabError("lab CA absent"));
    }
    Ok(roots)
}

/// TLS client bound to the lab CA; records every presented server certificate.
pub struct TlsClient {
    h1: Arc<ClientConfig>,
    h2: Arc<ClientConfig>,
    presented: Mutex<BTreeSet<String>>,
    handshakes: Mutex<u64>,
}

type Tls = StreamOwned<ClientConnection, TcpStream>;

impl TlsClient {
    /// Verify the server against `ca_pem` only; offer exactly one ALPN each.
    pub fn new(ca_pem: &str) -> Result<Self, LabError> {
        let roots = Arc::new(roots(ca_pem)?);
        let config = |alpn: &[u8]| {
            let mut config = ClientConfig::builder()
                .with_root_certificates(roots.clone())
                .with_no_client_auth();
            config.alpn_protocols = vec![alpn.to_vec()];
            Arc::new(config)
        };
        Ok(Self {
            h1: config(b"http/1.1"),
            h2: config(b"h2"),
            presented: Mutex::new(BTreeSet::new()),
            handshakes: Mutex::new(0),
        })
    }

    /// Distinct server certificate digests presented, and handshake count.
    pub fn presented(&self) -> (Vec<String>, u64) {
        let digests = self
            .presented
            .lock()
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        (digests, self.handshakes.lock().map_or(0, |count| *count))
    }

    fn connect(
        &self,
        address: SocketAddr,
        config: &Arc<ClientConfig>,
        alpn: &[u8],
    ) -> Result<Tls, LabError> {
        let socket = TcpStream::connect_timeout(&address, CONNECT)
            .map_err(|_| LabError("Gate TLS ingress connect"))?;
        socket
            .set_read_timeout(Some(READ))
            .and_then(|_| socket.set_write_timeout(Some(CONNECT)))
            .map_err(|_| LabError("socket timeout"))?;
        let name = ServerName::try_from(SERVER_NAME).map_err(|_| LabError("server name"))?;
        let connection =
            ClientConnection::new(config.clone(), name).map_err(|_| LabError("ClientHello"))?;
        let mut stream = StreamOwned::new(connection, socket);
        while stream.conn.is_handshaking() {
            stream
                .conn
                .complete_io(&mut stream.sock)
                .map_err(|_| LabError("TLS handshake"))?;
        }
        if stream.conn.alpn_protocol() != Some(alpn) {
            return Err(LabError("negotiated ALPN differs from the offer"));
        }
        let leaf = stream
            .conn
            .peer_certificates()
            .and_then(|chain| chain.first())
            .ok_or(LabError("server certificate absent"))?;
        let digest = format!("{:x}", Sha256::digest(leaf.as_ref()));
        if let (Ok(mut set), Ok(mut count)) = (self.presented.lock(), self.handshakes.lock()) {
            set.insert(digest);
            *count += 1;
        }
        Ok(stream)
    }

    /// One HTTP/1.1 exchange on a fresh TLS connection (ALPN http/1.1).
    pub fn exchange_h1(&self, address: SocketAddr, bytes: &[u8]) -> Result<Vec<u8>, LabError> {
        let mut stream = self.connect(address, &self.h1, b"http/1.1")?;
        stream
            .write_all(bytes)
            .and_then(|_| stream.flush())
            .map_err(|_| LabError("request write"))?;
        read_bounded(&mut stream)
    }

    /// One HTTP/2 request on stream 1 of a fresh TLS connection (ALPN h2).
    /// `request.headers` must already be in the server's view: lower-case,
    /// `host` first; `host` travels as `:authority`.
    pub fn exchange_h2(
        &self,
        address: SocketAddr,
        request: &Request,
    ) -> Result<Response, LabError> {
        let wire = encode_h2(request)?;
        let mut stream = self.connect(address, &self.h2, b"h2")?;
        stream
            .write_all(&wire)
            .and_then(|_| stream.flush())
            .map_err(|_| LabError("H2 request write"))?;
        decode_h2(&read_bounded(&mut stream)?)
    }
}

fn read_bounded(stream: &mut impl Read) -> Result<Vec<u8>, LabError> {
    let mut response = Vec::new();
    let mut part = [0; 16_384];
    loop {
        match stream.read(&mut part) {
            Ok(0) => break,
            Ok(count) => {
                response.extend_from_slice(&part[..count]);
                if response.len() > RESPONSE_LIMIT {
                    return Err(LabError("response budget"));
                }
            }
            // The runtime closes its epoch socket after the single response;
            // strict framing below detects any truncation.
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => break,
            Err(_) => return Err(LabError("response read")),
        }
    }
    Ok(response)
}

fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let length = payload.len() as u32;
    let mut out = vec![
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
        kind,
        flags,
    ];
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn encode_h2(request: &Request) -> Result<Vec<u8>, LabError> {
    let authority = request.header("host").ok_or(LabError("H2 authority"))?;
    if request.headers.first().map(|(name, _)| name.as_str()) != Some("host")
        || request
            .headers
            .iter()
            .any(|(name, _)| name.bytes().any(|b| b.is_ascii_uppercase()))
    {
        return Err(LabError("H2 request must be in the server's header view"));
    }
    let mut encoder = Encoder::default();
    let mut block = Vec::new();
    let pseudo = [
        (":method", request.method.as_str()),
        (":scheme", "https"),
        (":authority", authority),
        (":path", request.target.as_str()),
    ];
    let fields = pseudo.into_iter().chain(
        request
            .headers
            .iter()
            .skip(1)
            .map(|(name, value)| (name.as_str(), value.as_str())),
    );
    for (name, value) in fields {
        encoder
            .encode(
                (
                    name.as_bytes().to_vec(),
                    value.as_bytes().to_vec(),
                    Encoder::NEVER_INDEXED,
                ),
                &mut block,
            )
            .map_err(|_| LabError("HPACK encode"))?;
    }
    let mut wire = PREFACE.to_vec();
    wire.extend(frame(4, 0, 0, &[]));
    if request.body.is_empty() {
        wire.extend(frame(1, END_HEADERS | END_STREAM, STREAM, &block));
    } else {
        wire.extend(frame(1, END_HEADERS, STREAM, &block));
        wire.extend(frame(0, END_STREAM, STREAM, &request.body));
    }
    Ok(wire)
}

/// Stream-1 response: `:status`, regular fields and DATA through END_STREAM.
fn decode_h2(bytes: &[u8]) -> Result<Response, LabError> {
    let bad = LabError("H2 response framing");
    let mut offset = 0;
    let mut status = None;
    let mut headers = Vec::new();
    let mut body = Vec::new();
    let mut decoder = Decoder::default();
    while offset + 9 <= bytes.len() {
        let length = (usize::from(bytes[offset]) << 16)
            | (usize::from(bytes[offset + 1]) << 8)
            | usize::from(bytes[offset + 2]);
        let (kind, flags) = (bytes[offset + 3], bytes[offset + 4]);
        let stream = u32::from_be_bytes(bytes[offset + 5..offset + 9].try_into().map_err(|_| bad)?)
            & 0x7fff_ffff;
        let start = offset + 9;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or(bad)?;
        let payload = &bytes[start..end];
        offset = end;
        match (kind, stream) {
            (4 | 8, 0) => continue,
            (7, 0) | (3, STREAM) => return Err(LabError("H2 stream refused")),
            (1, STREAM) if status.is_none() => {
                if flags & END_HEADERS == 0 || flags & 0x28 != 0 {
                    return Err(bad);
                }
                let mut block = payload.to_vec();
                let mut fields = Vec::new();
                decoder
                    .decode(&mut block, &mut fields)
                    .map_err(|_| LabError("HPACK decode"))?;
                for (name, value, _) in fields {
                    let name = String::from_utf8(name).map_err(|_| bad)?;
                    let value = String::from_utf8(value).map_err(|_| bad)?;
                    if name == ":status" {
                        status = Some(value.parse::<u16>().map_err(|_| bad)?);
                    } else {
                        headers.push((name, value));
                    }
                }
            }
            (0, STREAM) if status.is_some() => {
                if flags & 0x8 != 0 {
                    return Err(bad);
                }
                body.extend_from_slice(payload);
            }
            _ => return Err(bad),
        }
        if flags & END_STREAM != 0 && matches!(kind, 0 | 1) {
            let status = status.ok_or(bad)?;
            if !(200..600).contains(&status) {
                return Err(bad);
            }
            return Ok(Response {
                status,
                headers,
                body,
            });
        }
    }
    Err(LabError("H2 response incomplete"))
}

/// Plaintext bytes to the TLS ingress; returns whatever the peer sent back.
pub fn plaintext(address: SocketAddr, bytes: &[u8]) -> Result<Vec<u8>, LabError> {
    let mut stream =
        TcpStream::connect_timeout(&address, CONNECT).map_err(|_| LabError("ingress connect"))?;
    stream
        .set_read_timeout(Some(READ))
        .and_then(|_| stream.set_write_timeout(Some(CONNECT)))
        .map_err(|_| LabError("socket timeout"))?;
    stream
        .write_all(bytes)
        .map_err(|_| LabError("request write"))?;
    read_bounded(&mut stream)
}

/// A request in the truthful native identity (no browser claims).
pub fn native(path: &str, body: Vec<u8>) -> Request {
    Request {
        method: "POST".into(),
        target: path.into(),
        headers: vec![
            ("Host".into(), SERVER_NAME.into()),
            ("Content-Length".into(), body.len().to_string()),
            ("Content-Type".into(), "application/json".into()),
            ("User-Agent".into(), NATIVE_AGENT.into()),
            ("Accept".into(), "*/*".into()),
            ("Accept-Encoding".into(), "gzip, deflate, br, zstd".into()),
        ],
        body,
    }
}

/// The same request as an H2 server reconstructs it: lower-case, `host` first.
fn h2_view(request: &Request) -> Request {
    Request {
        method: request.method.clone(),
        target: request.target.clone(),
        headers: request
            .headers
            .iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
            .collect(),
        body: request.body.clone(),
    }
}

/// Native-identity client report bound to the session (fixture, not a human).
fn native_report(session: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"sessionId":session,"userAgent":NATIVE_AGENT,
        "environment":{"probed":true},"behavior":{},
        "signals":[{"id":"l1.navigator.webdriver","value":false,"verdict":"OK","confidence":1,"collected":"wasm"}]}))
    .unwrap_or_default()
}

fn step(steps: &mut Vec<Step>, id: &str, role: &str, outcome: Result<Response, LabError>) {
    let (http_status, response_verdict, error) = match outcome {
        Ok(response) => (Some(response.status), response.verdict(), None),
        Err(error) => (None, None, Some(error.to_string())),
    };
    steps.push(Step {
        id: id.into(),
        role: role.into(),
        http_status,
        response_verdict,
        error,
    });
}

/// The transport a capture row uses for its protected request.
#[derive(Clone, Copy)]
enum Wire {
    H1,
    H2,
}

/// Native collect over TLS H1, then one protected request on `path` over `wire`.
fn collect_then<O: Operator>(
    target: &mut Remote<'_, O>,
    steps: &mut Vec<Step>,
    path: &str,
    wire: Wire,
    browser_claim: bool,
) -> Result<(), LabError> {
    let issued = target.issue()?;
    let session = issued.session.to_string();
    let mut collect = if browser_claim {
        target.request(
            "/_humanymous/collect",
            crate::client::report(&json!({"session":session})),
        )
    } else {
        native("/_humanymous/collect", native_report(&session))
    };
    target.sign(&mut collect, &issued, 1)?;
    let outcome = target.exchange(&collect);
    step(steps, "collect-asserted-fixture", "setup", outcome);
    let body = b"effect".to_vec();
    let mut request = if browser_claim {
        target.request(path, body)
    } else {
        native(path, body)
    };
    let outcome = match wire {
        Wire::H1 => {
            target.sign(&mut request, &issued, 2)?;
            target.exchange(&request)
        }
        Wire::H2 => {
            let mut request = h2_view(&request);
            target.sign(&mut request, &issued, 2)?;
            let tls = target.tls.ok_or(LabError("TLS transport"))?;
            tls.exchange_h2(target.address, &request)
        }
    };
    step(steps, "protected-request", "probe", outcome);
    Ok(())
}

fn statuses(steps: &[Step]) -> Vec<Option<u16>> {
    steps.iter().map(|step| step.http_status).collect()
}

fn verdicts(delta: &SnapshotDelta) -> Vec<&str> {
    delta
        .audit
        .iter()
        .map(|event| event.verdict.as_str())
        .collect()
}

fn typed(delta: &SnapshotDelta) -> bool {
    delta.audit.iter().all(|event| {
        event.event_type
            == if event.verdict == "COLLECTED" {
                "runtime.collect"
            } else {
                "runtime.decision"
            }
    })
}

/// Explicit expectation of one TLS edge or capture row against its effects.
pub fn expected(id: &str, steps: &[Step], delta: &SnapshotDelta) -> Option<bool> {
    let clean = steps.iter().all(|step| step.error.is_none());
    // A route requiring raw TLS / original H2 is *decided* (not refused 422)
    // only when the runtime captured the ClientHello / H2 frames. Under the
    // frozen scoring a truthful synthetic rustls client is challenged, never
    // forwarded (Go `test/gate/e2e.mjs` `synthetic-baseline-challenged`).
    let challenged = clean
        && statuses(steps) == [Some(200), Some(403)]
        && delta.origin_effects == 0
        && verdicts(delta) == ["COLLECTED", "CHALLENGE"]
        && typed(delta);
    Some(match id {
        "tls-edge-rustls-native-challenged"
        | "tls-h1-required-raw-tls"
        | "tls-h2-required-original-h2" => challenged,
        // The exact signed bytes replayed on a new TLS connection: refused,
        // never forwarded.
        "tls-edge-replay-refused" => {
            clean
                && statuses(steps) == [Some(200), Some(403), Some(403)]
                && delta.origin_effects == 0
                // The replay is refused before any second decision is recorded.
                && verdicts(delta) == ["COLLECTED", "CHALLENGE"]
                && typed(delta)
        }
        // Duplicate Content-Length over TLS H1: the connection is closed with
        // no HTTP response, no decision and no origin effect.
        "tls-edge-ambiguous-framing-refused" => {
            steps.len() == 1
                && steps[0].http_status.is_none()
                && steps[0].error.is_some()
                && delta.origin_effects == 0
                && delta.audit.is_empty()
        }
        // original_h2 is never satisfied by TLS H1: refused before any decision.
        "tls-h1-required-original-h2-refused" => {
            clean
                && statuses(steps) == [Some(200), Some(422)]
                && delta.origin_effects == 0
                && verdicts(delta) == ["COLLECTED"]
                && typed(delta)
        }
        // A Chrome claim over a rustls ClientHello is decided, never forwarded.
        "tls-chrome-claim-rustls-hello" => {
            clean
                && statuses(steps) == [Some(200), Some(403)]
                && delta.origin_effects == 0
                && delta.audit.len() == 2
                && delta.audit[0].verdict == "COLLECTED"
                && matches!(delta.audit[1].verdict.as_str(), "DENY" | "CHALLENGE")
                && typed(delta)
        }
        // No plaintext fallback: no HTTP response, no audit, no origin effect.
        "plaintext-on-tls-ingress-refused" => {
            steps.len() == 1
                && steps[0].http_status.is_none()
                && steps[0].error.as_deref() == Some("no HTTP response on TLS ingress")
                && delta.origin_effects == 0
                && delta.audit.is_empty()
        }
        _ => return None,
    })
}

fn row(id: &str, tier: &str, class: &str) -> NetworkRow {
    NetworkRow {
        id: id.into(),
        tier: tier.into(),
        class: class.into(),
        status: Status::Error,
        steps: Vec::new(),
        processes: Vec::new(),
        effects: None,
        expectation_matches: None,
        reason: None,
    }
}

fn edge_or_capture<O: Operator>(
    target: &mut Remote<'_, O>,
    id: &str,
    steps: &mut Vec<Step>,
) -> Result<(), LabError> {
    match id {
        "tls-edge-rustls-native-challenged" => {
            collect_then(target, steps, "/effect", Wire::H1, false)
        }
        "tls-edge-ambiguous-framing-refused" => {
            // The catalog's duplicate-length bytes, over TLS H1.
            let raw = format!(
                "POST /effect HTTP/1.1\r\nHost: {SERVER_NAME}\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n"
            );
            let outcome = target.exchange_raw(raw.as_bytes());
            step(steps, "ambiguous-framing", "attack", outcome);
            Ok(())
        }
        "tls-h1-required-raw-tls" => collect_then(target, steps, "/tls", Wire::H1, false),
        "tls-h2-required-original-h2" => collect_then(target, steps, "/h2", Wire::H2, false),
        "tls-h1-required-original-h2-refused" => {
            collect_then(target, steps, "/h2", Wire::H1, false)
        }
        "tls-chrome-claim-rustls-hello" => collect_then(target, steps, "/effect", Wire::H1, true),
        "tls-edge-replay-refused" => {
            let issued = target.issue()?;
            let session = issued.session.to_string();
            let mut collect = native("/_humanymous/collect", native_report(&session));
            target.sign(&mut collect, &issued, 1)?;
            let outcome = target.exchange(&collect);
            step(steps, "collect-asserted-fixture", "setup", outcome);
            let mut request = native("/effect", b"effect".to_vec());
            target.sign(&mut request, &issued, 2)?;
            let outcome = target.exchange(&request);
            step(steps, "protected-request", "synthetic_control", outcome);
            let outcome = target.exchange(&request);
            step(steps, "replayed-request", "attack", outcome);
            Ok(())
        }
        "plaintext-on-tls-ingress-refused" => {
            let bytes = crate::attack_catalog::client::wire(&native("/effect", b"{}".to_vec()));
            let outcome = plaintext(target.address, &bytes).and_then(|reply| {
                if reply.starts_with(b"HTTP/") {
                    crate::attack_catalog::client::parse(&reply)
                } else {
                    Err(LabError("no HTTP response on TLS ingress"))
                }
            });
            step(steps, "plaintext-request", "attack", outcome);
            Ok(())
        }
        _ => Err(LabError("unknown TLS row")),
    }
}

/// Every row in its own Gate session, then the topology and certificate facts.
pub fn report(
    operator: &mut impl Operator,
    address: SocketAddr,
    origin: SocketAddr,
    ca_pem: &str,
) -> Value {
    let tls = match TlsClient::new(ca_pem) {
        Ok(tls) => tls,
        Err(error) => {
            return json!({"schema":REPORT_SCHEMA,"scoped_passed":false,"error":error.to_string(),
                "full_product_qualified":false});
        }
    };
    let mut rows = Vec::new();
    for id in GATE_CATALOG {
        let entry = network::entries(0)
            .into_iter()
            .find(|entry| entry.id == *id);
        let (tier, class) = entry.as_ref().map_or(("", ""), |e| (e.tier, e.class));
        rows.push(session(
            operator,
            id,
            row(id, tier, class),
            |operator, steps| {
                let mut target = Remote {
                    address,
                    config: configuration()?,
                    operator,
                    tls: Some(&tls),
                };
                network::execute_case(&mut target, id, steps)
            },
            |steps, delta| network::expectation_matches(id, steps, delta),
        ));
    }
    for (ids, class) in [(GATE_EDGE, "tls_edge"), (CAPTURE, "tls_capture")] {
        for id in ids {
            rows.push(session(
                operator,
                id,
                row(id, "runtime", class),
                |operator, steps| {
                    let mut target = Remote {
                        address,
                        config: configuration()?,
                        operator,
                        tls: Some(&tls),
                    };
                    edge_or_capture(&mut target, id, steps)
                },
                |steps, delta| expected(id, steps, delta),
            ));
        }
    }
    // Go `origin-direct-hit-blocked`: the origin is not reachable from ingress.
    let origin_direct_refused = TcpStream::connect_timeout(&origin, CONNECT).is_err();
    let (presented, handshakes) = tls.presented();
    let count = |status: Status| rows.iter().filter(|row| row.status == status).count();
    let mismatches = rows
        .iter()
        .filter(|row| row.status == Status::Evaluated && row.expectation_matches != Some(true))
        .count();
    let evaluated = count(Status::Evaluated);
    let errors = count(Status::Error);
    let scoped_passed = evaluated == rows.len()
        && errors == 0
        && mismatches == 0
        && origin_direct_refused
        && presented.len() == 1;
    json!({"schema":REPORT_SCHEMA,"evidence_type":"docker-internal-network-gate-runtime-tls",
        "human_executed":false,"full_product_qualified":false,
        "totals":{"scheduled":rows.len(),"evaluated":evaluated,"errors":errors,
            "expectation_mismatches":mismatches},
        "origin_direct_refused":origin_direct_refused,
        "presented_certificates":presented,"tls_handshakes":handshakes,
        "scoped_passed":scoped_passed,"rows":rows})
}

pub fn run() {
    let mut input = BufReader::new(io::stdin());
    let outcome = (|| -> Result<Value, Box<dyn std::error::Error>> {
        let control = read_line(&mut input, 1 << 16)?;
        let gate: IpAddr = control["gate"].as_str().ok_or("gate address")?.parse()?;
        let origin: IpAddr = control["origin"]
            .as_str()
            .ok_or("origin address")?
            .parse()?;
        let ca_pem = control["ca_pem"].as_str().ok_or("lab CA")?.to_owned();
        let mut pipes = Pipes { input: &mut input };
        Ok(report(
            &mut pipes,
            SocketAddr::new(gate, FRONTEND_PORT),
            SocketAddr::new(origin, ORIGIN_PORT),
            &ca_pem,
        ))
    })();
    let report = outcome.unwrap_or_else(|error| {
        json!({"schema":REPORT_SCHEMA,"scoped_passed":false,"error":error.to_string(),
            "full_product_qualified":false})
    });
    let passed = report["scoped_passed"] == true;
    emit(&json!({"op":"report","report":report})).expect("TLS client report output");
    if !passed {
        std::process::exit(1);
    }
}
