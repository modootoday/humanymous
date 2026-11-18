//! Roadmap 2d missing-signal controls, run in the Gate namespace after the
//! real browser test ended and before the runtime drains. They talk directly
//! to the runtime loopback listener, so neither browser-proxy count changes.
//!
//! Inputs are the exact collect request the real browser sent through the Gate
//! frontend in its stripped (no guard hooks) session epoch. The only synthesized requests are an unsigned browser bootstrap
//! for a fresh session and an unsigned required-collector probe; neither can
//! carry a signature, so any admission would be a product failure. Rows hold
//! observations only (status, error code, ACK presence, finite failure label);
//! the lab runner owns the expectations. Session values never leave memory.
use crate::browser_proxy::{Receipts, fixture};
use humanymous_core::integrity::request::browser::PROTOCOL;
use serde_json::{Value, json};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub const SCHEMA: &str = "humanymous.browser-missing-signal-controls/1";
/// Exact control ledger, in execution order. The runner holds the same list.
pub const CONTROLS: &[&str] = &[
    "fresh-session-bootstrap",
    "fresh-session-required-collector",
    "replayed-collect",
    "foreign-session-collect",
    "foreign-session-required-collector-after-replay",
];
const REPLY_MAX: usize = 131_072;

#[derive(Debug, PartialEq)]
struct Reply {
    status: u16,
    ack: bool,
    body: Vec<u8>,
}

/// Status line, ACK header presence and entity of one closed HTTP/1.1 reply.
fn parse(reply: &[u8]) -> Result<Reply, &'static str> {
    let end = reply
        .windows(4)
        .position(|p| p == b"\r\n\r\n")
        .ok_or("reply head")?;
    let head = std::str::from_utf8(&reply[..end]).map_err(|_| "reply head encoding")?;
    let mut lines = head.split("\r\n");
    let start: Vec<_> = lines.next().ok_or("reply start")?.splitn(3, ' ').collect();
    if start.len() < 2 || start[0] != "HTTP/1.1" {
        return Err("reply start");
    }
    let status = start[1].parse().map_err(|_| "reply status")?;
    let ack = lines
        .filter_map(|line| line.split_once(':'))
        .any(|(name, _)| name.trim().eq_ignore_ascii_case("x-humanymous-ack"));
    Ok(Reply {
        status,
        ack,
        body: reply[end + 4..].to_vec(),
    })
}

/// The runtime's static `{"error":code}` body; anything else is not a code.
fn error_code(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let object = value.as_object().filter(|o| o.len() == 1)?;
    let code = object.get("error")?.as_str()?;
    (code.len() <= 64 && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
        .then(|| code.to_owned())
}

/// Exactly one header value by case-insensitive name.
fn header(request: &[u8], name: &str) -> Option<String> {
    let end = request.windows(4).position(|p| p == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&request[..end]).ok()?;
    let mut found = head
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim().to_owned());
    let value = found.next()?;
    found.next().is_none().then_some(value)
}

/// Replace each named header that occurs exactly once; body bytes unchanged.
fn with_headers(request: &[u8], replace: &[(&str, &str)]) -> Result<Vec<u8>, &'static str> {
    let end = request
        .windows(4)
        .position(|p| p == b"\r\n\r\n")
        .ok_or("request head")?;
    let head = std::str::from_utf8(&request[..end]).map_err(|_| "request head encoding")?;
    let mut seen = vec![0usize; replace.len()];
    let mut lines = Vec::new();
    for (index, line) in head.split("\r\n").enumerate() {
        let name = line.split_once(':').map(|(n, _)| n.trim());
        match (index, name) {
            (0, _) | (_, None) => lines.push(line.to_owned()),
            (_, Some(name)) => {
                match replace
                    .iter()
                    .position(|(r, _)| r.eq_ignore_ascii_case(name))
                {
                    Some(at) => {
                        seen[at] += 1;
                        lines.push(format!("{name}: {}", replace[at].1));
                    }
                    None => lines.push(line.to_owned()),
                }
            }
        }
    }
    if seen.iter().any(|count| *count != 1) {
        return Err("replaced header not present exactly once");
    }
    let mut bytes = lines.join("\r\n").into_bytes();
    bytes.extend_from_slice(&request[end..]);
    Ok(bytes)
}

fn scope() -> (String, String) {
    let fixture = fixture();
    (
        fixture["scope"]["authority"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        fixture["browser_origin"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

/// What a browser bootstrap carries, minus any session value.
fn bootstrap_request() -> Vec<u8> {
    let (authority, origin) = scope();
    let path = fixture()["routes"]["session"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nOrigin: {origin}\r\nContent-Type: application/json\r\nSec-Fetch-Site: same-origin\r\nSec-Fetch-Mode: same-origin\r\nSec-Fetch-Dest: empty\r\nX-Humanymous-Protocol: {PROTOCOL}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    )
    .into_bytes()
}

/// An unsigned required-collector request naming `session`. The provenance
/// gate runs before signature verification, so this reaches exactly it.
fn required_request(session: &str) -> Vec<u8> {
    let (authority, origin) = scope();
    let path = fixture()["routes"]["required_collector"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nOrigin: {origin}\r\nContent-Type: application/octet-stream\r\nSec-Fetch-Site: same-origin\r\nSec-Fetch-Mode: same-origin\r\nSec-Fetch-Dest: empty\r\nX-Humanymous-Protocol: {PROTOCOL}\r\nX-Humanymous-Session: {session}\r\nContent-Length: 6\r\nConnection: close\r\n\r\neffect"
    )
    .into_bytes()
}

async fn send(runtime: SocketAddr, request: &[u8]) -> Result<Reply, &'static str> {
    let mut socket = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(runtime))
        .await
        .map_err(|_| "connect deadline")?
        .map_err(|_| "connect")?;
    socket.write_all(request).await.map_err(|_| "write")?;
    let mut reply = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(5),
        (&mut socket)
            .take(REPLY_MAX as u64 + 1)
            .read_to_end(&mut reply),
    )
    .await
    .map_err(|_| "reply deadline")?
    .map_err(|_| "reply read")?;
    if reply.len() > REPLY_MAX {
        return Err("reply bound");
    }
    parse(&reply)
}

fn row(id: &str, reply: Result<&Reply, &'static str>) -> Value {
    match reply {
        Ok(reply) => json!({"id":id,"http":reply.status,"error":error_code(&reply.body),
            "ack":reply.ack,"failure":null}),
        Err(failure) => json!({"id":id,"http":null,"error":null,"ack":null,"failure":failure}),
    }
}

/// The epoch whose captured collect the controls replay: the stripped one.
const EPOCH: &str = "stripped";

/// Execute the ledger once; never fails, unobserved rows carry a failure label.
pub async fn run(runtime: SocketAddr, receipts: &Receipts) -> Value {
    let captured = crate::browser_expectations::EPOCHS
        .iter()
        .position(|id| *id == EPOCH)
        .map(|index| receipts.collect_requests(index))
        .unwrap_or_default();
    let mut rows = Vec::new();
    let issuing = match captured.as_slice() {
        [one] => header(one, "x-humanymous-session").filter(|id| id.len() == 43),
        _ => None,
    };
    let (Some(issuing), [collect]) = (issuing, captured.as_slice()) else {
        for id in CONTROLS {
            rows.push(row(
                id,
                Err("exactly one captured browser collect required"),
            ));
        }
        return json!({"schema":SCHEMA,"epoch":EPOCH,"captured_collect_requests":captured.len(),
            "rows":rows,"fresh_session_distinct":null});
    };
    let bootstrap = send(runtime, &bootstrap_request()).await;
    rows.push(row(CONTROLS[0], bootstrap.as_ref().map_err(|e| *e)));
    let fresh = bootstrap
        .as_ref()
        .ok()
        .filter(|reply| reply.status == 200)
        .and_then(|reply| serde_json::from_slice::<Value>(&reply.body).ok())
        .and_then(|value| value["session"].as_str().map(str::to_owned))
        .filter(|id| id.len() == 43);
    let distinct = fresh.as_ref().map(|id| *id != issuing);
    let Some(fresh) = fresh else {
        for id in &CONTROLS[1..] {
            rows.push(row(id, Err("fresh session unavailable")));
        }
        return json!({"schema":SCHEMA,"epoch":EPOCH,"captured_collect_requests":1,"rows":rows,
            "fresh_session_distinct":distinct});
    };
    let reply = send(runtime, &required_request(&fresh)).await;
    rows.push(row(CONTROLS[1], reply.as_ref().map_err(|e| *e)));
    // The exact signed bytes the real browser sent, a second time.
    let reply = send(runtime, collect).await;
    rows.push(row(CONTROLS[2], reply.as_ref().map_err(|e| *e)));
    // The same signed report moved into the fresh session at its next counter,
    // so the counter sequence admits it and only the signature, which binds the
    // issuing session (and its counter), can refuse it before consumption.
    let moved = with_headers(
        collect,
        &[
            ("x-humanymous-session", &fresh),
            ("x-humanymous-counter", "1"),
        ],
    );
    let reply = match moved {
        Ok(bytes) => send(runtime, &bytes).await,
        Err(error) => Err(error),
    };
    rows.push(row(CONTROLS[3], reply.as_ref().map_err(|e| *e)));
    let reply = send(runtime, &required_request(&fresh)).await;
    rows.push(row(CONTROLS[4], reply.as_ref().map_err(|e| *e)));
    json!({"schema":SCHEMA,"epoch":EPOCH,"captured_collect_requests":1,"rows":rows,
        "fresh_session_distinct":distinct})
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST: &[u8] = b"POST /_humanymous/browser/collect HTTP/1.1\r\nHost: 127.0.0.1:31111\r\nX-Humanymous-Session: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\nX-Humanymous-Counter: 3\r\nContent-Length: 2\r\n\r\n{}";

    #[test]
    fn replies_expose_only_status_code_and_ack_presence() {
        let reply = parse(b"HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 30\r\n\r\n{\"error\":\"required_capability\"}").unwrap();
        assert_eq!((reply.status, reply.ack), (422, false));
        assert_eq!(
            error_code(&reply.body).as_deref(),
            Some("required_capability")
        );
        let acked = parse(b"HTTP/1.1 403 Forbidden\r\nx-humanymous-ack: m\r\n\r\n{}").unwrap();
        assert!(acked.ack);
        assert_eq!(error_code(&acked.body), None);
        for bad in [
            &b"HTTP/1.0 200 OK\r\n\r\n"[..],
            b"garbage",
            b"HTTP/1.1 x\r\n\r\n",
        ] {
            assert!(parse(bad).is_err());
        }
        for body in [
            &br#"{"error":"x","extra":1}"#[..],
            br#"{"error":"Has-Upper"}"#,
            br#"{"verdict":"DENY"}"#,
        ] {
            assert_eq!(error_code(body), None);
        }
    }

    #[test]
    fn moving_a_request_rewrites_only_named_headers_once() {
        assert_eq!(header(REQUEST, "x-humanymous-session").unwrap().len(), 43);
        let moved = with_headers(
            REQUEST,
            &[("x-humanymous-session", "b"), ("x-humanymous-counter", "1")],
        )
        .unwrap();
        assert_eq!(header(&moved, "X-Humanymous-Session").as_deref(), Some("b"));
        assert_eq!(header(&moved, "x-humanymous-counter").as_deref(), Some("1"));
        assert_eq!(header(&moved, "host"), header(REQUEST, "host"));
        assert!(moved.ends_with(b"\r\n\r\n{}"));
        assert!(with_headers(REQUEST, &[("x-missing", "1")]).is_err());
        let doubled = [
            &REQUEST[..REQUEST.len() - 6],
            b"\r\nx-humanymous-counter: 4\r\n\r\n{}",
        ]
        .concat();
        assert!(with_headers(&doubled, &[("x-humanymous-counter", "1")]).is_err());
        assert_eq!(header(&doubled, "x-humanymous-counter"), None);
    }

    #[test]
    fn synthesized_requests_carry_no_signature_and_follow_the_fixture() {
        for request in [bootstrap_request(), required_request("s")] {
            assert_eq!(header(&request, "x-humanymous-token"), None);
            assert_eq!(header(&request, "x-humanymous-counter"), None);
            assert_eq!(header(&request, "cookie"), None);
            assert_eq!(
                header(&request, "x-humanymous-protocol").as_deref(),
                Some(PROTOCOL)
            );
            assert_eq!(
                header(&request, "host").as_deref(),
                fixture()["scope"]["authority"].as_str()
            );
        }
        assert!(required_request("s").starts_with(b"POST /requires-collector "));
        assert!(bootstrap_request().starts_with(b"POST /_humanymous/browser/session "));
    }

    #[test]
    fn missing_capture_reports_every_row_unobserved() {
        let value = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run("127.0.0.1:9".parse().unwrap(), &Receipts::default()));
        let rows = value["rows"].as_array().unwrap();
        assert_eq!(rows.len(), CONTROLS.len());
        assert!(
            rows.iter()
                .all(|row| row["failure"].is_string() && row["http"].is_null())
        );
        assert_eq!(value["captured_collect_requests"], 0);
    }

    /// Component substitute, not Docker/browser evidence: the actual runtime
    /// host and Gate-side proxy in-process, a native signer standing in for the
    /// browser transport, then the controls against the still-serving runtime.
    #[test]
    fn controls_against_the_actual_runtime_host_fail_closed() {
        use crate::browser_proxy::{ProxyConfig, serve};
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        use humanymous_core::integrity::request::{
            browser::{Binding, canonical},
            compute_token, seed,
        };
        use humanymous_runtime::gateway::service::{ServiceHost, ShutdownRequest};
        use tokio::{net::TcpListener, sync::watch};

        let installation = crate::installation::Installation::new(false).unwrap();
        let fixture = fixture();
        let mut config = installation.config.clone();
        config["scope"] = fixture["scope"].clone();
        // The lab image runs as UID 65532; a host test owns its own directory.
        config["owner"]["uid"] = json!(std::os::unix::fs::MetadataExt::uid(
            &std::fs::metadata(&installation.directory).unwrap()
        ));
        config["browser"] = json!({"origin":fixture["browser_origin"]});
        let route = |path: &Value, collector: bool| json!({"method":"POST","path":path,"rit_profile":"browser_auth_only","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":collector,"rit":true}});
        config["routes"] = json!([
            route(&fixture["routes"]["effect"], false),
            route(&fixture["routes"]["required_collector"], true)
        ]);
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (value, effects) = executor.block_on(async move {
            let host = ServiceHost::bind(serde_json::from_value(config).unwrap())
                .await
                .unwrap();
            let runtime = host.readiness().listen;
            let (shutdown, controller) = watch::channel(None);
            let serving = tokio::spawn(host.run(controller));
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let gate = listener.local_addr().unwrap();
            let receipts = Receipts::default();
            let (stop, stopped) = watch::channel(false);
            let proxy = tokio::spawn(serve(
                listener,
                ProxyConfig {
                    listen: gate,
                    upstream: runtime,
                    browser: false,
                },
                stopped,
                receipts.clone(),
            ));
            let boot = send(gate, &bootstrap_request()).await.unwrap();
            assert_eq!(boot.status, 200);
            let issued: Value = serde_json::from_slice(&boot.body).unwrap();
            let id = issued["session"].as_str().unwrap().to_owned();
            let key: [u8; 32] = URL_SAFE_NO_PAD
                .decode(issued["key"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap();
            // Today's Rust snapshot shape: incomplete guard provenance.
            let body = serde_json::to_vec(&json!({"signals":[],"guard":{"reported":false},
                "engineVersion":"rust-wasm-0.1.0"}))
            .unwrap();
            let scope = &fixture["scope"];
            let target = fixture["routes"]["collect"].as_str().unwrap();
            let now = humanymous_runtime::authority::now_ms().unwrap();
            let (deadline, bucket) = (now + 4000, now / 10_000);
            let text = canonical(&Binding {
                installation: scope["installation"].as_str().unwrap(),
                tenant: scope["tenant"].as_str().unwrap(),
                origin_identity: scope["origin"].as_str().unwrap(),
                revision: scope["revision"].as_str().unwrap(),
                authority: scope["authority"].as_str().unwrap(),
                session: &id,
                method: "POST",
                target,
                counter: 1,
                bucket,
                deadline,
                content_type: "application/json",
                application_authorization: "",
                body: &body,
            })
            .unwrap();
            let token = compute_token(&seed(&key, 0), &text, 1, bucket);
            let mut collect = format!(
                "POST {target} HTTP/1.1\r\nHost: {}\r\nOrigin: {}\r\nContent-Type: application/json\r\nSec-Fetch-Site: same-origin\r\nSec-Fetch-Mode: same-origin\r\nSec-Fetch-Dest: empty\r\nX-Humanymous-Protocol: {PROTOCOL}\r\nX-Humanymous-Session: {id}\r\nX-Humanymous-Counter: 1\r\nX-Humanymous-Bucket: {bucket}\r\nX-Humanymous-Deadline: {deadline}\r\nX-Humanymous-Token: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                scope["authority"].as_str().unwrap(),
                fixture["browser_origin"].as_str().unwrap(),
                body.len()
            )
            .into_bytes();
            collect.extend_from_slice(&body);
            let collected = send(gate, &collect).await.unwrap();
            assert_eq!((collected.status, collected.ack), (200, true));
            let stripped = send(runtime, &required_request(&id)).await.unwrap();
            assert_eq!(
                (stripped.status, error_code(&stripped.body).as_deref(), stripped.ack),
                (422, Some("required_capability"), false)
            );
            let value = run(runtime, &receipts).await;
            stop.send(true).unwrap();
            proxy.await.unwrap().unwrap();
            shutdown.send(Some(ShutdownRequest::new(2000).unwrap())).unwrap();
            serving.await.unwrap().unwrap();
            // Only the one consumed collect reached the durable audit.
            let records = humanymous_runtime::audit::storage::Wal::open(
                &installation.directory,
                Default::default(),
            )
            .unwrap()
            .records()
            .unwrap();
            let events: Vec<_> = records
                .iter()
                .map(|record| (record.event_type.clone(), record.verdict.clone()))
                .collect();
            assert_eq!(
                events,
                [("runtime.collect".to_owned(), "COLLECTED".to_owned())]
            );
            // The actual runtime's audit request_id attributes that record to
            // the one epoch the Gate-side proxy opened at the bootstrap.
            let (epochs, unattributed) = crate::browser_product::audit_epochs(&receipts, &records);
            assert_eq!(unattributed, 0);
            assert_eq!(epochs.len(), 1);
            assert_eq!(epochs[0]["audit_seqs"], json!([records[0].seq]));
            assert_eq!(epochs[0]["sessions"], 1);
            assert_eq!(epochs[0]["collector_reports"], json!([0]));
            (value, installation.origin.effects())
        });
        assert_eq!(effects, 0);
        assert_eq!(value["captured_collect_requests"], 1);
        assert_eq!(value["epoch"], "stripped");
        assert_eq!(value["fresh_session_distinct"], true);
        let observed: Vec<_> = value["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["id"].clone(),
                    row["http"].clone(),
                    row["error"].clone(),
                    row["ack"].clone(),
                )
            })
            .collect();
        assert_eq!(
            observed,
            [
                (json!(CONTROLS[0]), json!(200), Value::Null, json!(false)),
                (
                    json!(CONTROLS[1]),
                    json!(422),
                    json!("required_capability"),
                    json!(false)
                ),
                (
                    json!(CONTROLS[2]),
                    json!(409),
                    json!("counter_replay_or_gap"),
                    json!(false)
                ),
                (
                    json!(CONTROLS[3]),
                    json!(403),
                    json!("request_integrity"),
                    json!(false)
                ),
                (
                    json!(CONTROLS[4]),
                    json!(422),
                    json!("required_capability"),
                    json!(false)
                ),
            ]
        );
    }
}
