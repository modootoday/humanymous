//! TLS/H2 driver for the shipped gateway's TLS listener (profile `tls`).
//!
//! Every case is judged on what this client observed: the negotiated ALPN,
//! the TLS records or HTTP/2 frames the Gate sent back, and the origin's own
//! records. Positive traffic uses a real TLS client (rustls) that trusts only
//! the per-run lab root the runner relays from the Gate's readiness line.
//! Raw ClientHellos are the owned parrots from `humanymous-attacker`'s
//! `wire::clienthello`, and H2 requests are encoded by its `wire::h2` framer;
//! neither can complete a handshake by itself (their key shares are filler),
//! so they measure the listener's refusal, not a parrot session.
//!
//! Scope: the client runs in its own container and network namespace on the
//! internal `tls` network and reaches the Gate's public TLS edge listener
//! (`public_listen`, owner decision B01 #1) by its network address, as an
//! original client would. The origin stays on the Gate's loopback; only its
//! read-only observation endpoint is bound beside the Gate.
use super::{
    driver::{body, forbidden, header_names, records_at, status, wait_for},
    report::{self, Captured},
};
use crate::client::{request, sign, sign_h2, wire};
use humanymous_attacker::wire::{
    clienthello::{ClientHelloBuilder, Profile},
    h2::{H2Profile, H2Request},
};
use humanymous_core::{
    scoring::engine::Engine,
    signals::{
        observation::{Signal, Source, Verdict},
        report::{ClientReport, SessionReport},
    },
};
use humanymous_runtime::{
    capture::tls::ClientHelloCapture,
    gateway::{configuration::Configuration, http::Request},
    network::{
        http::HeaderInfo,
        report::{Observation, build},
        tls::{engine_from_client_hello, ja4},
    },
};
use rustls::{
    CertificateError, ClientConfig, ClientConnection, RootCertStore, StreamOwned,
    pki_types::{CertificateDer, ServerName, pem::PemObject},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, Read, Write},
    net::{Shutdown, TcpStream},
    sync::Arc,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Wrapper schema of the public lab root the runner writes on stdin.
pub const LAB_CA_SCHEMA: &str = "humanymous.product-e2e-lab-ca/1";
const SCOPE: &str = "cross-namespace original client: the driver runs in its own network namespace and reaches the Gate's public TLS edge listener over the internal tls network";
const RESPONSE_LIMIT: u64 = 1 << 20;
const H2: &[u8] = b"h2";
const H1: &[u8] = b"http/1.1";

/// Verify the relayed root: schema, PEM, and its own SHA-256.
fn lab_root(line: &str) -> Result<(CertificateDer<'static>, String)> {
    let wrapper: Value = serde_json::from_str(line.trim_end())?;
    if wrapper["schema"] != LAB_CA_SCHEMA {
        return Err("lab CA wrapper schema".into());
    }
    let pem = wrapper["certificate_pem"]
        .as_str()
        .ok_or("lab CA certificate absent")?;
    let root = CertificateDer::from_pem_slice(pem.as_bytes())?;
    let digest = hex::encode(Sha256::digest(root.as_ref()));
    if wrapper["sha256"] != digest.as_str() {
        return Err("lab CA digest mismatch".into());
    }
    Ok((root, digest))
}

fn client(root: &CertificateDer<'static>, alpn: &[&[u8]]) -> Result<Arc<ClientConfig>> {
    let mut roots = RootCertStore::empty();
    roots.add(root.clone())?;
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
    config.alpn_protocols = alpn.iter().map(|protocol| protocol.to_vec()).collect();
    Ok(Arc::new(config))
}

/// JA4 and engine class of one ClientHello record as the runtime's own
/// capture reads it.
fn fingerprint(record: &[u8]) -> Value {
    let mut capture = ClientHelloCapture::default();
    let parsed = capture
        .feed(record)
        .ok()
        .filter(|_| capture.is_complete())
        .and_then(|_| capture.finish().ok());
    match parsed {
        Some(hello) => json!({"ja4":ja4(&hello).0,"engine":engine_from_client_hello(Some(&hello)),
            "alpn_offered":hello.alpn.iter().map(|p| String::from_utf8_lossy(p).into_owned()).collect::<Vec<_>>(),"sni_present":hello.sni_present}),
        None => json!({"ja4":null,"capture":"refused"}),
    }
}

/// What one TLS connection observed.
#[derive(Default)]
struct Exchange {
    hello: Value,
    /// The client's first flight (its ClientHello record) as sent.
    record: Vec<u8>,
    handshake: Option<String>,
    /// The rustls certificate verification class when the handshake failed
    /// on the server's certificate.
    certificate_error: Option<&'static str>,
    alpn: Option<String>,
    response: Vec<u8>,
    end: &'static str,
}

impl Exchange {
    fn handshaken(&self) -> bool {
        self.handshake.is_none() && !self.hello.is_null()
    }
    fn summary(&self) -> Value {
        json!({"client_hello":self.hello,"handshake_error":self.handshake,
            "certificate_error":self.certificate_error,
            "negotiated_alpn":self.alpn,"response_bytes":self.response.len(),"stream_end":self.end})
    }
}

/// The certificate verification class of a failed client handshake, read
/// from the typed rustls error (never from its display text).
fn certificate_error(error: &std::io::Error) -> Option<&'static str> {
    let rustls::Error::InvalidCertificate(certificate) =
        error.get_ref()?.downcast_ref::<rustls::Error>()?
    else {
        return None;
    };
    Some(match certificate {
        CertificateError::UnknownIssuer => "UnknownIssuer",
        CertificateError::BadSignature => "BadSignature",
        CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
            "NotValidForName"
        }
        _ => "other",
    })
}

/// The frozen L5/L6 inputs the Gate scores for one signed request, rebuilt
/// by this driver from what it sent: the collector report it posted, the
/// exact header list, its own ClientHello record and the negotiated ALPN,
/// scored by `Engine::default()` (the engine the shipped binary constructs).
/// A reconstruction, not the Gate's own trace: it explains a local verdict
/// and is required to agree with it.
fn reconstruct(report: &[u8], headers: &[(String, String)], record: &[u8], alpn: &str) -> Value {
    let rebuilt = (|| -> Result<Value> {
        let mut client: ClientReport = serde_json::from_slice(report)?;
        // The Gate's collect seam re-derives registry metadata the same way.
        for signal in client.signals.iter_mut().flatten() {
            *signal = Signal::new(
                &signal.id,
                signal.value.clone(),
                signal.verdict.clone(),
                signal.confidence,
                signal.collected.clone(),
                "",
            );
        }
        let mut capture = ClientHelloCapture::default();
        capture.feed(record)?;
        let hello = capture.finish()?;
        let value = |name: &str| {
            headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map_or(String::new(), |(_, v)| v.clone())
        };
        let header = HeaderInfo {
            method: "POST".into(),
            version: "11".into(),
            names: headers.iter().map(|(n, _)| n.clone()).collect(),
            order_reliable: true,
            casing_reliable: true,
            user_agent: value("user-agent"),
            accept_language: value("accept-language"),
            accept_encoding: value("accept-encoding"),
            ..Default::default()
        };
        let mut session = SessionReport {
            client,
            network: build(&Observation {
                header,
                hello: Some(hello),
                ..Default::default()
            }),
            ..Default::default()
        };
        session.network.negotiated_alpn = alpn.into();
        session
            .network
            .signals
            .get_or_insert_default()
            .push(Signal::new(
                "l5.rit.ok",
                json!(true),
                Verdict::Ok,
                1.0,
                Source::Server,
                "RIT valid",
            ));
        let raised: Vec<Value> = session
            .network
            .signals
            .iter()
            .flatten()
            .filter(|s| matches!(s.verdict, Verdict::Bot | Verdict::Suspicious))
            .map(|s| json!({"id":s.id,"verdict":s.verdict}))
            .collect();
        let (result, trace) = Engine::default().score_with_trace(&mut session)?;
        Ok(
            json!({"kind":"driver-side reconstruction with Engine::default(); not the Gate's own trace",
            "verdict":result.verdict,"risk_score":result.risk_score,
            "hard_rule_fired":result.hard_rule_fired,
            "policy":{"challenge_at":trace.policy.challenge_at,"deny_at":trace.policy.deny_at},
            "contributions":trace.contributions.iter().map(|c| json!({"id":c.id,"layer":c.layer,"score":c.score})).collect::<Vec<_>>(),
            "raised_network_signals":raised,
            "client_user_agent":session.client.user_agent,
            "ja4_engine":session.network.ja4_engine}),
        )
    })();
    rebuilt.unwrap_or_else(|error| json!({"error":error.to_string()}))
}

/// One real TLS connection to the Gate: handshake, write `bytes`, optionally
/// send close_notify, then read until the peer ends the stream.
fn tls(config: Arc<ClientConfig>, name: &str, bytes: &[u8], close: bool) -> Exchange {
    let mut exchange = Exchange::default();
    let outcome = (|| -> Result<()> {
        let mut socket =
            TcpStream::connect_timeout(&super::address(super::TLS_GATE), Duration::from_secs(5))?;
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        socket.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut connection = ClientConnection::new(config, ServerName::try_from(name.to_owned())?)?;
        let mut first = Vec::new();
        connection.write_tls(&mut first)?;
        exchange.hello = fingerprint(&first);
        exchange.record.clone_from(&first);
        socket.write_all(&first)?;
        while connection.is_handshaking() {
            if let Err(error) = connection.complete_io(&mut socket) {
                exchange.certificate_error = certificate_error(&error);
                exchange.handshake = Some(error.to_string());
                return Ok(());
            }
        }
        exchange.alpn = connection
            .alpn_protocol()
            .map(|protocol| String::from_utf8_lossy(protocol).into_owned());
        let mut stream = StreamOwned::new(connection, socket);
        stream.write_all(bytes)?;
        if close {
            stream.conn.send_close_notify();
        }
        stream.flush()?;
        let mut limited = (&mut stream).take(RESPONSE_LIMIT);
        let mut buffer = [0; 16_384];
        exchange.end = loop {
            match limited.read(&mut buffer) {
                Ok(0) => break "close_notify",
                Ok(n) => exchange.response.extend_from_slice(&buffer[..n]),
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break "eof_without_close_notify";
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break "read_timeout";
                }
                Err(_) => break "read_error",
            }
        };
        Ok(())
    })();
    if let Err(error) = outcome {
        exchange.handshake.get_or_insert(error.to_string());
    }
    exchange
}

/// What a raw TCP peer observed after sending bytes that are not a complete
/// TLS client flight.
struct Raw {
    response: Vec<u8>,
    closed_by_gate: bool,
    elapsed_ms: u128,
    error: Option<String>,
}

fn raw(bytes: &[u8], half_close: bool) -> Raw {
    let started = Instant::now();
    let mut response = Vec::new();
    let mut closed = false;
    let outcome = (|| -> Result<()> {
        let mut socket =
            TcpStream::connect_timeout(&super::address(super::TLS_GATE), Duration::from_secs(5))?;
        socket.set_read_timeout(Some(Duration::from_secs(12)))?;
        socket.set_write_timeout(Some(Duration::from_secs(5)))?;
        socket.write_all(bytes)?;
        if half_close {
            socket.shutdown(Shutdown::Write)?;
        }
        let mut buffer = [0; 16_384];
        loop {
            match socket.read(&mut buffer) {
                Ok(0) => {
                    closed = true;
                    return Ok(());
                }
                Ok(n) if response.len() < RESPONSE_LIMIT as usize => {
                    response.extend_from_slice(&buffer[..n]);
                }
                Ok(_) => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
                    closed = true;
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
            }
        }
    })();
    Raw {
        response,
        closed_by_gate: closed,
        elapsed_ms: started.elapsed().as_millis(),
        error: outcome.err().map(|error| error.to_string()),
    }
}

/// First TLS record the Gate wrote, classified from its header bytes.
fn first_record(bytes: &[u8]) -> Value {
    match bytes {
        [] => json!({"kind":"none"}),
        [b'H', b'T', b'T', b'P', b'/', ..] => json!({"kind":"plaintext_http_response"}),
        [0x15, major, minor, _, _, level, description, ..] => {
            json!({"kind":"alert","version":format!("{major}.{minor}"),"level":level,"description":description})
        }
        [0x16, major, minor, _, _, kind, ..] => {
            json!({"kind":"handshake","version":format!("{major}.{minor}"),"handshake_type":kind})
        }
        [kind, ..] => json!({"kind":"other","content_type":kind}),
    }
}

// HPACK static table names (RFC 7541 Appendix A), index 1..=61.
const STATIC_NAMES: [&str; 61] = [
    ":authority",
    ":method",
    ":method",
    ":path",
    ":path",
    ":scheme",
    ":scheme",
    ":status",
    ":status",
    ":status",
    ":status",
    ":status",
    ":status",
    ":status",
    "accept-charset",
    "accept-encoding",
    "accept-language",
    "accept-ranges",
    "accept",
    "access-control-allow-origin",
    "age",
    "allow",
    "authorization",
    "cache-control",
    "content-disposition",
    "content-encoding",
    "content-language",
    "content-length",
    "content-location",
    "content-range",
    "content-type",
    "cookie",
    "date",
    "etag",
    "expect",
    "expires",
    "from",
    "host",
    "if-match",
    "if-modified-since",
    "if-none-match",
    "if-range",
    "if-unmodified-since",
    "last-modified",
    "link",
    "location",
    "max-forwards",
    "proxy-authenticate",
    "proxy-authorization",
    "range",
    "referer",
    "refresh",
    "retry-after",
    "server",
    "set-cookie",
    "strict-transport-security",
    "transfer-encoding",
    "user-agent",
    "vary",
    "via",
    "www-authenticate",
];
const STATIC_STATUS: [&str; 7] = ["200", "204", "206", "304", "400", "404", "500"];

fn hpack_integer(block: &[u8], at: &mut usize, prefix: u8) -> Option<usize> {
    let mask = (1u16 << prefix) as usize - 1;
    let mut value = usize::from(*block.get(*at)?) & mask;
    *at += 1;
    if value < mask {
        return Some(value);
    }
    let mut shift = 0;
    loop {
        let byte = *block.get(*at)?;
        *at += 1;
        value = value.checked_add(usize::from(byte & 0x7f).checked_shl(shift)?)?;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

fn hpack_string(block: &[u8], at: &mut usize) -> std::result::Result<String, &'static str> {
    let huffman = block.get(*at).ok_or("hpack truncated")? & 0x80 != 0;
    let length = hpack_integer(block, at, 7).ok_or("hpack length")?;
    let bytes = block.get(*at..*at + length).ok_or("hpack truncated")?;
    *at += length;
    if huffman {
        return Err("hpack huffman string not decoded");
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| "hpack utf8")
}

/// Minimal HPACK decoder for the response header block (literal forms,
/// static and dynamic indexing). Huffman strings are reported, not guessed.
fn hpack(
    block: &[u8],
    table: &mut Vec<(String, String)>,
) -> std::result::Result<Vec<(String, String)>, &'static str> {
    let entry = |index: usize, table: &Vec<(String, String)>| -> Option<(String, String)> {
        match index {
            1..=61 => Some((
                STATIC_NAMES[index - 1].to_owned(),
                if (8..=14).contains(&index) {
                    STATIC_STATUS[index - 8].to_owned()
                } else {
                    String::new()
                },
            )),
            _ => table.get(index.checked_sub(62)?).cloned(),
        }
    };
    let mut fields = Vec::new();
    let mut at = 0;
    while at < block.len() {
        let first = block[at];
        if first & 0x80 != 0 {
            let index = hpack_integer(block, &mut at, 7).ok_or("hpack index")?;
            fields.push(entry(index, table).ok_or("hpack index")?);
            continue;
        }
        if first & 0xe0 == 0x20 {
            hpack_integer(block, &mut at, 5).ok_or("hpack table size")?;
            continue;
        }
        let (prefix, indexing) = if first & 0xc0 == 0x40 {
            (6, true)
        } else {
            (4, false)
        };
        let index = hpack_integer(block, &mut at, prefix).ok_or("hpack name index")?;
        let name = if index == 0 {
            hpack_string(block, &mut at)?
        } else {
            entry(index, table).ok_or("hpack name index")?.0
        };
        let value = hpack_string(block, &mut at)?;
        if indexing {
            table.insert(0, (name.clone(), value.clone()));
        }
        fields.push((name, value));
    }
    Ok(fields)
}

/// The Gate's HTTP/2 frames as this client observed them.
#[derive(Debug, Default, PartialEq)]
struct H2Response {
    status: Option<u16>,
    body: Vec<u8>,
    settings: Vec<(u16, u32)>,
    settings_ack: bool,
    goaway_error: Option<u32>,
    reset_error: Option<u32>,
    frame_types: Vec<u8>,
    decode_error: Option<&'static str>,
}

fn h2_response(bytes: &[u8]) -> H2Response {
    let mut parsed = H2Response::default();
    let mut table = Vec::new();
    let mut rest = bytes;
    while rest.len() >= 9 {
        let length = usize::from(rest[0]) << 16 | usize::from(rest[1]) << 8 | usize::from(rest[2]);
        let (kind, flags) = (rest[3], rest[4]);
        let stream = u32::from_be_bytes([rest[5] & 0x7f, rest[6], rest[7], rest[8]]);
        let Some(payload) = rest.get(9..9 + length) else {
            parsed.decode_error = Some("truncated frame");
            break;
        };
        parsed.frame_types.push(kind);
        let word = |at: usize| {
            payload
                .get(at..at + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        };
        match (kind, stream) {
            (0, 1) => parsed.body.extend_from_slice(payload),
            (1, 1) => match hpack(payload, &mut table) {
                Ok(fields) => {
                    parsed.status = fields
                        .iter()
                        .find(|(name, _)| name == ":status")
                        .and_then(|(_, value)| value.parse().ok());
                }
                Err(error) => parsed.decode_error = Some(error),
            },
            (3, _) => parsed.reset_error = word(0),
            (4, 0) if flags & 1 == 1 => parsed.settings_ack = true,
            (4, 0) => parsed
                .settings
                .extend(payload.as_chunks::<6>().0.iter().map(|pair| {
                    (
                        u16::from_be_bytes([pair[0], pair[1]]),
                        u32::from_be_bytes([pair[2], pair[3], pair[4], pair[5]]),
                    )
                })),
            (7, 0) => parsed.goaway_error = word(4),
            _ => {}
        }
        rest = &rest[9 + length..];
    }
    parsed
}

fn h2_frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).expect("bounded test frame");
    let mut out = length.to_be_bytes()[1..].to_vec();
    out.extend_from_slice(&[kind, flags]);
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Rewrite every frame on stream `from` to stream `to` after the preface.
fn retarget(encoded: &[u8], from: u32, to: u32) -> Vec<u8> {
    let preface = humanymous_attacker::wire::h2::PREFACE.len();
    let mut out = encoded[..preface].to_vec();
    let mut rest = &encoded[preface..];
    while rest.len() >= 9 {
        let length = usize::from(rest[0]) << 16 | usize::from(rest[1]) << 8 | usize::from(rest[2]);
        let mut header = rest[..9].to_vec();
        if u32::from_be_bytes([rest[5] & 0x7f, rest[6], rest[7], rest[8]]) == from {
            header[5..9].copy_from_slice(&to.to_be_bytes());
        }
        out.extend_from_slice(&header);
        out.extend_from_slice(&rest[9..9 + length]);
        rest = &rest[9 + length..];
    }
    out
}

/// An H2 request on stream 1 whose header list is exactly `req`'s (the Gate
/// rebuilds `host` from `:authority` at index 0).
fn h2_bytes(req: &Request) -> Result<Vec<u8>> {
    let authority = req.header("host").ok_or("host")?;
    let mut h2 = H2Request::new(H2Profile::Chrome, &req.method, authority, &req.target)?;
    for (name, value) in req.headers.iter().skip(1) {
        h2 = h2.header(name, value);
    }
    Ok(h2.body(req.body.clone()).encode())
}

/// The plaintext request headers lowercased, as an H2 client must send them.
fn h2_request(path: &str, payload: &[u8]) -> Request {
    let mut req = request(path, payload.to_vec());
    for header in &mut req.headers {
        header.0 = header.0.to_ascii_lowercase();
    }
    req
}

/// Local verdicts the Gate's scoring path answers with 403 (application.rs
/// accepts only ALLOW, DENY and CHALLENGE from the engine; ALLOW forwards).
const LOCAL_VERDICTS: [&str; 2] = ["CHALLENGE", "DENY"];

/// The scoring verdict of a Gate-local 403 body: exactly `{"verdict":V}`
/// with V a local verdict. Any `error` body (request_integrity, deadline,
/// counter_exhausted, authority, report_session, …) is a refusal before
/// scoring, never a verdict.
fn local_verdict(local: &Option<Value>) -> Option<&str> {
    let body = local.as_ref()?.as_object()?;
    let verdict = body.get("verdict")?.as_str()?;
    (body.len() == 1 && LOCAL_VERDICTS.contains(&verdict)).then_some(verdict)
}

/// A signed Gate decision is coherent when 200 forwarded exactly one request,
/// or a 403 forwarded none and carried a scoring verdict.
fn coherent(status: Option<u16>, local: &Option<Value>, delta: usize) -> bool {
    match status {
        Some(200) => delta == 1,
        Some(403) => delta == 0 && local_verdict(local).is_some(),
        _ => false,
    }
}

struct Driver {
    config: Configuration,
    root: CertificateDer<'static>,
    root_sha256: String,
    report: std::result::Result<Captured, String>,
    cases: Vec<Value>,
    collects: Vec<Value>,
}

impl Driver {
    fn config(&self, alpn: &[&[u8]]) -> Result<Arc<ClientConfig>> {
        client(&self.root, alpn)
    }

    fn h1(&self, bytes: &[u8]) -> Result<Exchange> {
        Ok(tls(
            self.config(&[H1])?,
            super::lab_ca::SERVER_NAME,
            bytes,
            false,
        ))
    }

    fn h2(&self, bytes: &[u8], close: bool) -> Result<Exchange> {
        Ok(tls(
            self.config(&[H2, H1])?,
            super::lab_ca::SERVER_NAME,
            bytes,
            close,
        ))
    }

    fn session(&mut self) -> Result<Value> {
        let exchange = self.h1(&wire(&request("/_humanymous/session", vec![])))?;
        let issued = body(&exchange.response).ok_or("session body over TLS")?;
        if status(&exchange.response) != Some(200) || !issued["session"].is_string() {
            return Err("session issue over TLS refused".into());
        }
        Ok(issued)
    }

    fn collected(&mut self) -> Result<Value> {
        let issued = self.session()?;
        let entity = match &self.report {
            Ok(captured) => captured.bind(issued["session"].as_str().ok_or("session id")?),
            Err(error) => return Err(format!("no real collector report: {error}").into()),
        };
        let mut collect = request("/_humanymous/collect", entity);
        sign(&self.config, &mut collect, &issued, 1);
        let exchange = self.h1(&wire(&collect))?;
        let observed = status(&exchange.response);
        self.collects.push(
            json!({"transport":"tls http/1.1","observed_status":observed,
            "gate_verdict":body(&exchange.response).map_or(Value::Null, |b| b["verdict"].clone())}),
        );
        if observed != Some(200) {
            return Err(format!("collect over TLS refused: {observed:?}").into());
        }
        Ok(issued)
    }

    /// Run one exchange and return it with the origin records it caused.
    fn observed(
        &mut self,
        send: impl FnOnce(&Self) -> Result<Exchange>,
    ) -> Result<(Exchange, Vec<Value>)> {
        let before = records_at(super::TLS_OBSERVE)?.len();
        let exchange = send(self)?;
        Ok((exchange, records_at(super::TLS_OBSERVE)?[before..].to_vec()))
    }

    /// Negotiation and an unsigned session issue under one ALPN offer.
    fn negotiation(&mut self, id: &str, offer: &[&[u8]], expected: Option<&str>) -> Result<()> {
        let config = self.config(offer)?;
        let exchange = if offer.first() == Some(&H2) {
            let req = h2_request("/_humanymous/session", b"");
            tls(config, super::lab_ca::SERVER_NAME, &h2_bytes(&req)?, false)
        } else {
            tls(
                config,
                super::lab_ca::SERVER_NAME,
                &wire(&request("/_humanymous/session", vec![])),
                false,
            )
        };
        let (observed, issued) = if exchange.alpn.as_deref() == Some("h2") {
            let parsed = h2_response(&exchange.response);
            let issued = serde_json::from_slice::<Value>(&parsed.body).ok();
            (parsed.status, issued)
        } else {
            (status(&exchange.response), body(&exchange.response))
        };
        let issued_session = issued.as_ref().is_some_and(|b| b["session"].is_string());
        let pass = exchange.handshaken()
            && exchange.alpn.as_deref() == expected
            && observed == Some(200)
            && issued_session;
        self.cases.push(json!({"id":id,"alpn_offered":offer.iter().map(|p| String::from_utf8_lossy(p)).collect::<Vec<_>>(),
            "expected_alpn":expected,"tls":exchange.summary(),"observed_status":observed,
            "session_issued":issued_session,"pass":pass}));
        Ok(())
    }

    /// The client refuses a chain it cannot anchor; the Gate is not trusted
    /// by name or by a different root.
    fn client_refusal(
        &mut self,
        id: &str,
        root: Option<CertificateDer<'static>>,
        name: &str,
        accepted: &[&str],
    ) -> Result<()> {
        let config = client(root.as_ref().unwrap_or(&self.root), &[H1])?;
        let exchange = tls(
            config,
            name,
            &wire(&request("/_humanymous/session", vec![])),
            false,
        );
        let pass = exchange.handshake.is_some()
            && exchange
                .certificate_error
                .is_some_and(|class| accepted.contains(&class))
            && exchange.response.is_empty();
        self.cases.push(json!({"id":id,"server_name":name,"tls":exchange.summary(),
            "expected_certificate_error":accepted,
            "expectation":"the client's certificate verification refuses the handshake with this class","pass":pass}));
        Ok(())
    }

    /// A signed request whose verdict is the Gate's own decision.
    fn signed(&mut self, id: &str, h2: bool, path: &str, expect_status: Option<u16>) -> Result<()> {
        // A session that cannot be issued or collected is this case's failure,
        // recorded with its reason; the remaining cases still run.
        let issued = match self.collected() {
            Ok(issued) => issued,
            Err(error) => {
                self.cases.push(json!({"id":id,"path":path,
                    "precondition_error":error.to_string(),"pass":false}));
                return Ok(());
            }
        };
        let (bytes, headers) = if h2 {
            let mut req = h2_request(path, b"effect");
            sign_h2(&self.config, &mut req, &issued, 2);
            (h2_bytes(&req)?, req.headers)
        } else {
            let mut req = request(path, b"effect".to_vec());
            sign(&self.config, &mut req, &issued, 2);
            (wire(&req), req.headers)
        };
        let (exchange, new) = self.observed(|driver| {
            if h2 {
                driver.h2(&bytes, false)
            } else {
                driver.h1(&bytes)
            }
        })?;
        let (observed, local, frames) = if h2 {
            let parsed = h2_response(&exchange.response);
            let local = serde_json::from_slice::<Value>(&parsed.body).ok();
            let frames = json!({"types":parsed.frame_types,"server_settings":parsed.settings,
                "settings_ack":parsed.settings_ack,"goaway_error":parsed.goaway_error,
                "reset_error":parsed.reset_error,"decode_error":parsed.decode_error});
            (parsed.status, local, frames)
        } else {
            (
                status(&exchange.response),
                body(&exchange.response),
                Value::Null,
            )
        };
        let leaked: Vec<String> = new
            .iter()
            .flat_map(header_names)
            .filter(|n| forbidden(n))
            .collect();
        let alpn = if h2 { "h2" } else { "http/1.1" };
        let alpn_ok = exchange.alpn.as_deref() == Some(alpn);
        // A scored decision must be the one the frozen engine derives from
        // these exact inputs: ALLOW for a forward, the body's verdict for 403.
        let reconstruction = match (&self.report, expect_status) {
            (Ok(captured), None) => {
                let entity = captured.bind(issued["session"].as_str().unwrap_or_default());
                reconstruct(&entity, &headers, &exchange.record, alpn)
            }
            _ => Value::Null,
        };
        let decided = match observed {
            Some(200) => Some("ALLOW"),
            Some(403) => local_verdict(&local),
            _ => None,
        };
        let pass = alpn_ok
            && leaked.is_empty()
            && match expect_status {
                Some(want) => observed == Some(want) && new.is_empty(),
                None => {
                    coherent(observed, &local, new.len())
                        && decided.is_some_and(|v| reconstruction["verdict"] == v)
                }
            };
        self.cases.push(json!({"id":id,"path":path,"tls":exchange.summary(),"h2_frames":frames,
            "expected_status":expect_status.map_or(json!("200 forwarded, or 403 with a scoring verdict the frozen engine reproduces; never an error body, never 422"),|s|json!(s)),
            "scoring_reconstruction":reconstruction,
            "observed_status":observed,"origin_delta":new.len(),
            "gate_local_body":if new.is_empty() { local } else { None },
            "origin_method":new.first().map(|r| r["method"].clone()),
            "origin_target":new.first().map(|r| r["target"].clone()),
            "origin_body_length":new.first().map(|r| r["body_length"].clone()),
            "forbidden_headers_at_origin":leaked,"pass":pass}));
        Ok(())
    }

    /// Unsigned H2 request to a protected route: refused before the origin.
    fn h2_unsigned(&mut self) -> Result<()> {
        let bytes = h2_bytes(&h2_request("/effect", b"effect"))?;
        let (exchange, new) = self.observed(|driver| driver.h2(&bytes, false))?;
        let parsed = h2_response(&exchange.response);
        let pass =
            exchange.alpn.as_deref() == Some("h2") && parsed.status == Some(401) && new.is_empty();
        self.cases.push(
            json!({"id":"tls_h2_no_session_protected_route","tls":exchange.summary(),
            "expected_status":401,"observed_status":parsed.status,"origin_delta":new.len(),
            "gate_local_body":serde_json::from_slice::<Value>(&parsed.body).ok(),"pass":pass}),
        );
        Ok(())
    }

    /// Malformed H2 over a real TLS session: no status, no origin effect.
    fn h2_malformed(&mut self, id: &str, bytes: &[u8], close: bool) -> Result<()> {
        let (exchange, new) = self.observed(|driver| driver.h2(bytes, close))?;
        let parsed = h2_response(&exchange.response);
        let pass = exchange.alpn.as_deref() == Some("h2")
            && !parsed.status.is_some_and(|code| (200..300).contains(&code))
            && new.is_empty();
        self.cases.push(json!({"id":id,"tls":exchange.summary(),
            "observed_status":parsed.status,"frame_types":parsed.frame_types,
            "goaway_error":parsed.goaway_error,"reset_error":parsed.reset_error,
            "origin_delta":new.len(),"expectation":"no 2xx status and no origin effect","pass":pass}));
        Ok(())
    }

    /// Raw bytes on the TLS listener that are not a complete client flight.
    fn raw_case(&mut self, id: &str, bytes: &[u8], half_close: bool, extra: Value) -> Result<()> {
        let before = records_at(super::TLS_OBSERVE)?.len();
        let outcome = raw(bytes, half_close);
        let new = records_at(super::TLS_OBSERVE)?.len() - before;
        let plaintext = outcome.response.starts_with(b"HTTP/");
        let pass = !plaintext && new == 0 && outcome.closed_by_gate;
        let mut case = json!({"id":id,"sent_bytes":bytes.len(),"half_close":half_close,
            "gate_first_record":first_record(&outcome.response),"gate_bytes":outcome.response.len(),
            "closed_by_gate":outcome.closed_by_gate,"elapsed_ms":outcome.elapsed_ms,
            "transport_error":outcome.error,"plaintext_http_response":plaintext,"origin_delta":new,
            "expectation":"no plaintext fallback, no origin effect, connection ended by the Gate","pass":pass});
        if let (Some(case), Some(extra)) = (case.as_object_mut(), extra.as_object()) {
            case.extend(extra.clone());
        }
        self.cases.push(case);
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        // Negotiation as the Gate offers it (h2 first, then http/1.1).
        self.negotiation("tls_alpn_h2_selected", &[H2, H1], Some("h2"))?;
        self.negotiation("tls_alpn_http11_only", &[H1], Some("http/1.1"))?;
        self.negotiation("tls_alpn_not_offered", &[], None)?;
        let other = super::lab_ca::issue()?;
        let other = CertificateDer::from_pem_slice(other.root_pem.as_bytes())?;
        // The other root carries the same subject as the lab root, so webpki
        // reaches signature verification (BadSignature); an unrelated issuer
        // name would be UnknownIssuer. Both mean the anchor did not verify.
        self.client_refusal(
            "tls_client_refuses_other_root",
            Some(other),
            super::lab_ca::SERVER_NAME,
            &["UnknownIssuer", "BadSignature"],
        )?;
        self.client_refusal(
            "tls_client_refuses_foreign_name",
            None,
            "foreign.test",
            &["NotValidForName"],
        )?;
        // Capability gates: the raw ClientHello capture exists on TLS (the
        // plaintext Gate answers 422 for the same route), the original H2
        // observation only on H2.
        self.signed("tls_h1_signed_effect", false, "/effect", None)?;
        self.signed("tls_h1_raw_tls_route_admitted", false, "/tls", None)?;
        self.signed(
            "tls_h1_original_h2_route_refused",
            false,
            super::H2_ROUTE,
            Some(422),
        )?;
        self.signed("tls_h2_signed_effect", true, "/effect", None)?;
        self.signed("tls_h2_raw_tls_route_admitted", true, "/tls", None)?;
        self.signed(
            "tls_h2_original_h2_route_admitted",
            true,
            super::H2_ROUTE,
            None,
        )?;
        self.h2_unsigned()?;
        // Fail-closed listener inputs.
        self.raw_case(
            "plaintext_http_on_tls_listener",
            &wire(&request("/_humanymous/session", vec![])),
            false,
            json!({}),
        )?;
        for (name, profile) in [
            ("chrome", Profile::Chrome),
            ("chrome100", Profile::Chrome100),
            ("firefox", Profile::Firefox),
            ("stock_go", Profile::Stock),
        ] {
            let record = ClientHelloBuilder::new(profile).record();
            self.raw_case(
                &format!("raw_parrot_clienthello_{name}"),
                &record,
                false,
                json!({"oracle_preset":profile.oracle_preset(),"parrot":fingerprint(&record),
                    "note":"owned parrot bytes with filler key shares; no client can finish this handshake"}),
            )?;
        }
        let record = ClientHelloBuilder::new(Profile::Chrome).record();
        self.raw_case(
            "malformed_clienthello_truncated",
            &record[..record.len() / 2],
            true,
            json!({}),
        )?;
        let mut wrong = record.clone();
        wrong[5] = 2;
        self.raw_case(
            "malformed_clienthello_wrong_handshake_type",
            &wrong,
            false,
            json!({}),
        )?;
        let mut oversized = vec![0x16, 3, 1, 0xff, 0xff];
        oversized.extend_from_slice(&record[5..]);
        self.raw_case(
            "malformed_record_length_over_limit",
            &oversized,
            true,
            json!({}),
        )?;
        // Malformed H2 after a real handshake that negotiated h2.
        let settings = h2_frame(4, 0, 0, &[]);
        let mut bad_preface = b"PRI * HTTP/2.0\r\n\r\nXX\r\n\r\n".to_vec();
        bad_preface.extend_from_slice(&settings);
        self.h2_malformed("h2_bad_preface", &bad_preface, false)?;
        let mut over = humanymous_attacker::wire::h2::PREFACE.to_vec();
        over.extend_from_slice(&settings);
        over.extend_from_slice(&h2_frame(0, 1, 1, &vec![0; 16_385]));
        self.h2_malformed("h2_frame_over_budget", &over, false)?;
        let unsigned = h2_bytes(&h2_request("/effect", b""))?;
        self.h2_malformed("h2_request_on_stream_3", &retarget(&unsigned, 1, 3), false)?;
        let mut settings_on_stream = humanymous_attacker::wire::h2::PREFACE.to_vec();
        settings_on_stream.extend_from_slice(&h2_frame(4, 0, 1, &[0, 4, 0, 0, 0xff, 0xff]));
        self.h2_malformed("h2_settings_on_stream_1", &settings_on_stream, false)?;
        let mut orphan = humanymous_attacker::wire::h2::PREFACE.to_vec();
        orphan.extend_from_slice(&settings);
        orphan.extend_from_slice(&h2_frame(9, 4, 1, &[0x82]));
        self.h2_malformed("h2_continuation_without_headers", &orphan, false)?;
        let with_body = h2_bytes(&h2_request("/effect", b"effect"))?;
        let cut = with_body.len() - (9 + 6);
        self.h2_malformed("h2_truncated_before_end_stream", &with_body[..cut], true)?;
        // The listener still serves after every refused input.
        let after = self.session().is_ok();
        self.cases
            .push(json!({"id":"tls_gate_serves_after_refused_inputs",
            "session_issued_over_tls":after,"pass":after}));
        Ok(())
    }
}

/// Why no signed TLS request reached the origin, from the cases' own
/// reconstructions. A forwarded effect is reported only from an observed 200
/// with origin delta 1.
fn effect_summary(cases: &[Value]) -> Value {
    let scored: Vec<&Value> = cases
        .iter()
        .filter(|c| c["scoring_reconstruction"].is_object())
        .collect();
    let forwarded: Vec<&Value> = scored
        .iter()
        .filter(|c| c["observed_status"] == 200 && c["origin_delta"] == 1)
        .map(|c| &c["id"])
        .collect();
    let mut causes = std::collections::BTreeSet::new();
    for case in &scored {
        let rebuilt = &case["scoring_reconstruction"];
        for contribution in rebuilt["contributions"].as_array().into_iter().flatten() {
            causes.insert(format!(
                "{} (+{})",
                contribution["id"].as_str().unwrap_or_default(),
                contribution["score"]
            ));
        }
        if let Some(rule) = rebuilt["hard_rule_fired"]
            .as_str()
            .filter(|r| !r.is_empty())
        {
            causes.insert(format!("hard rule {rule}"));
        }
    }
    json!({"forwarded_cases":forwarded,
        "scored_cases":scored.len(),
        "reconstructed_causes":causes,
        "why_no_honest_forward": if forwarded.is_empty() && !scored.is_empty() {
            json!("the only client that can complete a handshake here is rustls: no GREASE (l5.tls.grease_absent, crates/sdk/humanymous-runtime/src/network/report/assembly.rs) and a JA4 engine of go, while the only genuine collector report is Firefox's, so the L6 cross-check x.ua_vs_ja4 (crates/sdk/humanymous-core/src/scoring/consistency/crosschecks.rs) is inconsistent (a non-browser UA would raise x.non_browser_ua instead, an empty report hard rule HR-10); the owned parrots cannot finish a handshake and the browser cannot reach the loopback TLS listener (B01 #1). A report edited to hide the mismatch would be a forged signal, so no case requires a TLS forward")
        } else { Value::Null }})
}

fn read_line(limit: u64) -> Result<String> {
    let mut line = String::new();
    std::io::stdin().lock().take(limit).read_line(&mut line)?;
    Ok(line)
}

/// Read the collector report and lab root, drive every case, print one
/// receipt line, and fail on any failed assertion.
pub fn run() -> Result<()> {
    let report = report::from_stdin();
    let (root, root_sha256) = lab_root(&read_line(64 * 1024)?)?;
    wait_for(super::TLS_OBSERVE)?;
    wait_for(super::TLS_GATE)?;
    let mut driver = Driver {
        config: serde_json::from_value(super::tls_configuration(
            "/unused", "/unused", "unused", "/unused", "/unused",
        ))?,
        root,
        root_sha256,
        report,
        cases: Vec::new(),
        collects: Vec::new(),
    };
    let outcome = driver.run();
    let failed = driver.cases.iter().filter(|c| c["pass"] != true).count();
    let client_report = match &driver.report {
        Ok(captured) => json!({"kind":"real Rust WASM collector report replayed",
            "provenance":captured.provenance,"collect_observations":driver.collects}),
        Err(error) => json!({"kind":"absent","error":error,"fixture_fallback":false}),
    };
    let tls_effect = effect_summary(&driver.cases);
    let receipt = json!({"schema":"humanymous.product-e2e-tls-driver/1","scope":SCOPE,
        "tls_effect":tls_effect,
        "trusted_lab_root_sha256":driver.root_sha256,
        "client":"rustls 0.23.46 (ring) with only the per-run lab root; raw parrots from humanymous-attacker wire::clienthello; H2 frames from wire::h2",
        "cases":driver.cases,"passed":driver.cases.len()-failed,"failed":failed,
        "driver_error":outcome.as_ref().err().map(|e|e.to_string()),"client_report":client_report});
    println!("{receipt}");
    if failed > 0 || outcome.is_err() {
        return Err("product e2e TLS assertions failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_root_wrapper_is_verified_against_its_own_digest() {
        let issued = super::super::lab_ca::issue().unwrap();
        let line = json!({"schema":LAB_CA_SCHEMA,"certificate_pem":issued.root_pem,"sha256":issued.root_sha256}).to_string();
        assert_eq!(lab_root(&line).unwrap().1, issued.root_sha256);
        let forged =
            json!({"schema":LAB_CA_SCHEMA,"certificate_pem":issued.root_pem,"sha256":"00"})
                .to_string();
        assert!(lab_root(&forged).is_err());
        let other =
            json!({"schema":"x","certificate_pem":issued.root_pem,"sha256":issued.root_sha256})
                .to_string();
        assert!(lab_root(&other).is_err());
    }

    #[test]
    fn h2_response_reads_status_body_settings_and_errors() {
        // :status 403 as a never-indexed literal with static name index 8.
        let mut block = vec![0x18, 3];
        block.extend_from_slice(b"403");
        block.extend_from_slice(&[0x10, 12]);
        block.extend_from_slice(b"content-type");
        block.extend_from_slice(&[16]);
        block.extend_from_slice(b"application/json");
        let mut bytes = h2_frame(4, 0, 0, &[0, 4, 0, 0, 1, 0]);
        bytes.extend(h2_frame(4, 1, 0, &[]));
        bytes.extend(h2_frame(1, 4, 1, &block));
        bytes.extend(h2_frame(0, 1, 1, b"{\"verdict\":\"DENY\"}"));
        bytes.extend(h2_frame(7, 0, 0, &[0, 0, 0, 1, 0, 0, 0, 2]));
        let parsed = h2_response(&bytes);
        assert_eq!(parsed.status, Some(403));
        assert_eq!(parsed.body, b"{\"verdict\":\"DENY\"}");
        assert_eq!(parsed.settings, vec![(4, 256)]);
        assert!(parsed.settings_ack);
        assert_eq!(parsed.goaway_error, Some(2));
        assert_eq!(parsed.frame_types, vec![4, 4, 1, 0, 7]);
        // Fully indexed :status 200, and a Huffman value is reported.
        assert_eq!(h2_response(&h2_frame(1, 4, 1, &[0x88])).status, Some(200));
        let huffman = h2_response(&h2_frame(1, 4, 1, &[0x18, 0x82, 0x10, 0x01]));
        assert_eq!(
            huffman.decode_error,
            Some("hpack huffman string not decoded")
        );
        assert_eq!(
            h2_response(&[0, 0, 9, 1, 4, 0, 0, 0, 1]).decode_error,
            Some("truncated frame")
        );
    }

    #[test]
    fn hpack_incremental_indexing_feeds_the_dynamic_table() {
        let mut table = Vec::new();
        let mut block = vec![0x40, 1, b'x', 1, b'y'];
        block.push(0x80 | 62);
        let fields = hpack(&block, &mut table).unwrap();
        assert_eq!(
            fields,
            vec![("x".into(), "y".into()), ("x".into(), "y".into())]
        );
        assert!(hpack(&[0x80 | 70], &mut table).is_err());
    }

    #[test]
    fn retarget_moves_only_the_request_stream() {
        let encoded = h2_bytes(&h2_request("/effect", b"")).unwrap();
        let moved = retarget(&encoded, 1, 3);
        assert_eq!(moved.len(), encoded.len());
        let mut rest = &moved[humanymous_attacker::wire::h2::PREFACE.len()..];
        let mut streams = Vec::new();
        while rest.len() >= 9 {
            let length =
                usize::from(rest[0]) << 16 | usize::from(rest[1]) << 8 | usize::from(rest[2]);
            streams.push((
                rest[3],
                u32::from_be_bytes([rest[5], rest[6], rest[7], rest[8]]),
            ));
            rest = &rest[9 + length..];
        }
        assert!(
            streams
                .iter()
                .any(|(kind, stream)| *kind == 1 && *stream == 3)
        );
        assert!(streams.iter().all(|(_, stream)| *stream != 1));
    }

    #[test]
    fn h2_request_carries_the_signed_header_list_without_host() {
        let req = h2_request("/effect", b"effect");
        assert_eq!(req.headers[0], ("host".into(), "owned.test".into()));
        assert!(
            req.headers
                .iter()
                .all(|(name, _)| *name == name.to_ascii_lowercase())
        );
        let bytes = h2_bytes(&req).unwrap();
        assert!(bytes.starts_with(humanymous_attacker::wire::h2::PREFACE));
        assert!(!bytes.windows(4).any(|w| w == b"host"));
        assert!(bytes.ends_with(b"effect"));
    }

    #[test]
    fn coherent_signed_outcomes_tie_status_to_origin_effect() {
        assert!(coherent(Some(200), &None, 1));
        assert!(!coherent(Some(200), &None, 0));
        assert!(coherent(Some(403), &Some(json!({"verdict":"DENY"})), 0));
        assert!(!coherent(Some(403), &Some(json!({"verdict":"DENY"})), 1));
        assert!(!coherent(Some(403), &None, 0));
        assert!(!coherent(
            Some(422),
            &Some(json!({"error":"required_capability"})),
            0
        ));
        assert!(!coherent(None, &None, 0));
    }

    #[test]
    fn a_403_is_coherent_only_with_an_exact_scoring_verdict() {
        for verdict in ["CHALLENGE", "DENY"] {
            assert!(coherent(Some(403), &Some(json!({"verdict":verdict})), 0));
        }
        // Refusals before scoring never pass as a decision.
        for error in [
            "request_integrity",
            "deadline",
            "counter_exhausted",
            "authority",
            "report_session",
        ] {
            assert!(!coherent(Some(403), &Some(json!({"error":error})), 0));
        }
        for body in [
            json!({"verdict":"CHALLENGE","error":"request_integrity"}),
            json!({"verdict":"challenge"}),
            json!({"verdict":"ALLOW"}),
            json!({"verdict":"BLOCK"}),
            json!({"verdict":1}),
            json!("CHALLENGE"),
        ] {
            assert!(!coherent(Some(403), &Some(body), 0));
        }
    }

    #[test]
    fn certificate_refusals_are_classified_from_the_typed_error() {
        let wrap = |error: CertificateError| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                rustls::Error::InvalidCertificate(error),
            )
        };
        assert_eq!(
            certificate_error(&wrap(CertificateError::BadSignature)),
            Some("BadSignature")
        );
        assert_eq!(
            certificate_error(&wrap(CertificateError::UnknownIssuer)),
            Some("UnknownIssuer")
        );
        assert_eq!(
            certificate_error(&wrap(CertificateError::NotValidForName)),
            Some("NotValidForName")
        );
        assert_eq!(
            certificate_error(&wrap(CertificateError::Expired)),
            Some("other")
        );
        let other =
            std::io::Error::new(std::io::ErrorKind::InvalidData, rustls::Error::DecryptError);
        assert_eq!(certificate_error(&other), None);
        assert_eq!(certificate_error(&std::io::Error::other("text")), None);
    }

    #[test]
    fn a_real_handshake_refusal_carries_the_certificate_class() {
        use rustls::{ServerConfig, ServerConnection};
        let issued = super::super::lab_ca::issue().unwrap();
        let chain: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(issued.chain_pem.as_bytes())
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let key =
            rustls::pki_types::PrivateKeyDer::from_pem_slice(issued.key_pem.as_bytes()).unwrap();
        let server = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(chain, key)
                .unwrap(),
        );
        let other = super::super::lab_ca::issue().unwrap();
        let other = CertificateDer::from_pem_slice(other.root_pem.as_bytes()).unwrap();
        let own = CertificateDer::from_pem_slice(issued.root_pem.as_bytes()).unwrap();
        let refuse = |root: &CertificateDer<'static>, name: &str| {
            let mut client = ClientConnection::new(
                client(root, &[H1]).unwrap(),
                ServerName::try_from(name.to_owned()).unwrap(),
            )
            .unwrap();
            let mut server = ServerConnection::new(server.clone()).unwrap();
            loop {
                let mut bytes = Vec::new();
                client.write_tls(&mut bytes).unwrap();
                server.read_tls(&mut bytes.as_slice()).unwrap();
                server.process_new_packets().unwrap();
                let mut bytes = Vec::new();
                server.write_tls(&mut bytes).unwrap();
                client.read_tls(&mut bytes.as_slice()).unwrap();
                if let Err(error) = client.process_new_packets() {
                    return certificate_error(&std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error,
                    ));
                }
                assert!(client.is_handshaking(), "handshake must not complete");
            }
        };
        assert_eq!(
            refuse(&other, super::super::lab_ca::SERVER_NAME),
            Some("BadSignature")
        );
        assert_eq!(refuse(&own, "foreign.test"), Some("NotValidForName"));
    }

    /// The frozen engine, fed a rustls ClientHello and a Firefox-UA report,
    /// challenges on the UA/JA4 cross-check; the same inputs carry no TLS
    /// contribution without a hello. Pins the receipt's stated cause.
    #[test]
    fn reconstruction_names_the_ua_vs_ja4_cross_check() {
        let issued = super::super::lab_ca::issue().unwrap();
        let root = CertificateDer::from_pem_slice(issued.root_pem.as_bytes()).unwrap();
        let report = serde_json::to_vec(&json!({"sessionId":"s",
            "userAgent":"Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
            "signals":[{"id":"l1.navigator.webdriver","value":true,"verdict":"BOT","confidence":1,"collected":"wasm"}]}))
        .unwrap();
        for (alpn, offer) in [("http/1.1", &[H1][..]), ("h2", &[H2, H1][..])] {
            let mut connection = ClientConnection::new(
                client(&root, offer).unwrap(),
                ServerName::try_from(super::super::lab_ca::SERVER_NAME).unwrap(),
            )
            .unwrap();
            let mut first = Vec::new();
            connection.write_tls(&mut first).unwrap();
            let headers = if alpn == "h2" {
                h2_request("/effect", b"effect").headers
            } else {
                request("/effect", b"effect".to_vec()).headers
            };
            let rebuilt = reconstruct(&report, &headers, &first, alpn);
            assert_eq!(rebuilt["verdict"], "CHALLENGE", "{rebuilt}");
            assert_eq!(rebuilt["ja4_engine"], "go");
            let ids: Vec<&str> = rebuilt["contributions"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|c| c["id"].as_str())
                .collect();
            assert!(ids.contains(&"x.ua_vs_ja4"), "{rebuilt}");
            assert!(ids.contains(&"l5.tls.grease_absent"), "{rebuilt}");
        }
        assert!(reconstruct(&report, &[], b"\x16", "h2")["error"].is_string());
        let summary = effect_summary(&[json!({"id":"a","observed_status":403,"origin_delta":0,
            "scoring_reconstruction":{"contributions":[{"id":"x.ua_vs_ja4","score":50.0}],"hard_rule_fired":"HR-24"}})]);
        assert_eq!(summary["forwarded_cases"], json!([]));
        assert!(summary["why_no_honest_forward"].is_string());
        assert_eq!(
            summary["reconstructed_causes"],
            json!(["hard rule HR-24", "x.ua_vs_ja4 (+50.0)"])
        );
    }

    #[test]
    fn first_record_classifies_alerts_handshakes_and_plaintext() {
        assert_eq!(first_record(&[])["kind"], "none");
        assert_eq!(
            first_record(b"HTTP/1.1 400")["kind"],
            "plaintext_http_response"
        );
        let alert = first_record(&[0x15, 3, 3, 0, 2, 2, 50]);
        assert_eq!(alert["kind"], "alert");
        assert_eq!(alert["description"], 50);
        assert_eq!(first_record(&[0x16, 3, 3, 0, 9, 2])["handshake_type"], 2);
        assert_eq!(first_record(&[0x17])["kind"], "other");
    }

    #[test]
    fn rustls_client_hello_is_read_by_the_runtime_capture() {
        let issued = super::super::lab_ca::issue().unwrap();
        let root = CertificateDer::from_pem_slice(issued.root_pem.as_bytes()).unwrap();
        let mut connection = ClientConnection::new(
            client(&root, &[H2, H1]).unwrap(),
            ServerName::try_from(super::super::lab_ca::SERVER_NAME).unwrap(),
        )
        .unwrap();
        let mut first = Vec::new();
        connection.write_tls(&mut first).unwrap();
        let hello = fingerprint(&first);
        assert!(
            hello["ja4"]
                .as_str()
                .is_some_and(|ja4| ja4.starts_with("t13d"))
        );
        assert_eq!(hello["alpn_offered"], json!(["h2", "http/1.1"]));
        let parrot = fingerprint(&ClientHelloBuilder::new(Profile::Chrome).record());
        assert_eq!(parrot["engine"], "chrome");
        assert_eq!(fingerprint(b"\x16\x03\x01\x00")["capture"], "refused");
    }
}
