//! ClickHouse projection against outputs captured from the Go oracle at
//! 981d25c (`tests/fixtures/audit/clickhouse-go-golden.json`; its
//! `provenance` records the command and the uncommitted capture program).
use super::super::{
    RecordSink,
    clickhouse::{ClickHouseOptions, ClickHouseSink, batch_body},
};
use crate::audit::record::{Actor, Record};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Deserialize)]
struct Golden {
    insert_targets: Vec<Target>,
    bodies: Vec<Body>,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Target {
    base: String,
    table: String,
    insert_url: String,
}

#[derive(Deserialize)]
struct Body {
    name: String,
    records: Vec<Record>,
    nan_conf_at: Option<Vec<usize>>,
    body: String,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    requests: Option<Vec<GoRequest>>,
    errors: Vec<String>,
    dropped: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
struct GoRequest {
    method: String,
    request_uri: String,
    content_type: String,
    content_length: usize,
    authorization: String,
    body: String,
}

fn golden() -> Golden {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/audit/clickhouse-go-golden.json"
    ))
    .unwrap()
}

fn options(
    endpoint: &str,
    table: &str,
    max_rows: usize,
    max_wait: Duration,
    backlog: usize,
) -> ClickHouseOptions {
    ClickHouseOptions {
        endpoint: endpoint.into(),
        table: table.into(),
        max_rows,
        max_wait,
        backlog,
    }
}

/// Go capture `rows(n, from)`.
fn rows(count: u64, from: u64) -> Vec<Record> {
    (from..from + count)
        .map(|seq| Record {
            event_id: "e".into(),
            seq,
            node_id: "n".into(),
            ts: "2026-11-19T00:00:00Z".into(),
            event_type: "enforcement.allow".into(),
            actor: Actor {
                kind: "subject".into(),
                ..Default::default()
            },
            triggered_rules: Some(Vec::new()),
            top_signals: Some(Vec::new()),
            key_id: "k".into(),
            ..Default::default()
        })
        .collect()
}

/// Go `NewCHSink(base, table, ..)` built `base + "/?query=..."`; the Rust
/// sink keeps the path and query, and the scheme and authority stay as
/// written.
#[test]
fn insert_targets_match_go() {
    let targets = golden().insert_targets;
    assert_eq!(targets.len(), 10);
    for case in targets {
        let sink = ClickHouseSink::new(options(
            &case.base,
            &case.table,
            1,
            Duration::from_secs(3600),
            1,
        ))
        .unwrap();
        let rest = case.base.strip_prefix("http://").unwrap();
        let authority = &rest[..rest.find('/').unwrap_or(rest.len())];
        assert_eq!(
            format!("http://{authority}{}", sink.insert_target()),
            case.insert_url,
            "{} {}",
            case.base,
            case.table
        );
        sink.close().unwrap();
    }
}

/// Go `chBatchBody` bytes: field order, HTML and line-separator escaping,
/// control characters, null slices, optional objects, Go float spelling, and
/// records Go could not encode left out of the body.
#[test]
fn batch_bodies_match_go_bytes() {
    let bodies = golden().bodies;
    assert_eq!(bodies.len(), 6);
    for case in bodies {
        let mut records = case.records;
        let unencodable = case.nan_conf_at.unwrap_or_default();
        for &index in &unencodable {
            for signal in records[index].top_signals.iter_mut().flatten() {
                signal.conf = match signal.id.as_str() {
                    "nan" => f64::NAN,
                    "inf" => f64::INFINITY,
                    _ => signal.conf,
                };
            }
        }
        let (body, encoded) = batch_body(&records);
        assert_eq!(String::from_utf8(body).unwrap(), case.body, "{}", case.name);
        assert_eq!(encoded, records.len() - unencodable.len(), "{}", case.name);
    }
}

struct Seen {
    request_line: String,
    headers: Vec<String>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> String {
        self.headers
            .iter()
            .find_map(|line| {
                let (key, value) = line.split_once(": ")?;
                key.eq_ignore_ascii_case(name).then(|| value.to_owned())
            })
            .unwrap_or_default()
    }

    /// The fields the Go capture recorded. `User-Agent` and
    /// `Accept-Encoding` are left out: Go's client added them, this sink does
    /// not (documented difference).
    fn as_go(&self) -> GoRequest {
        let mut parts = self.request_line.split(' ');
        GoRequest {
            method: parts.next().unwrap().to_owned(),
            request_uri: parts.next().unwrap().to_owned(),
            content_type: self.header("Content-Type"),
            content_length: self.header("Content-Length").parse().unwrap(),
            authorization: self.header("Authorization"),
            body: String::from_utf8(self.body.clone()).unwrap(),
        }
    }
}

pub(super) fn read_http_request(
    reader: &mut impl BufRead,
) -> Option<(String, Vec<String>, Vec<u8>)> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        headers.push(line);
    }
    let length: usize = headers
        .iter()
        .find_map(|h| h.strip_prefix("Content-Length: "))?
        .parse()
        .ok()?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some((request_line.trim_end().to_owned(), headers, body))
}

/// In-process ClickHouse HTTP interface answering every insert with
/// `status` and `message`, as the Go capture's httptest handler did.
fn stand_in(status: &'static str, message: String) -> (String, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let Some((request_line, headers, body)) =
                read_http_request(&mut BufReader::new(&stream))
            else {
                continue;
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
                message.len()
            );
            let _ = (&stream).write_all(response.as_bytes());
            let seen = Seen {
                request_line,
                headers,
                body,
            };
            if sender.send(seen).is_err() {
                return;
            }
        }
    });
    (format!("http://{address}"), receiver)
}

fn outcome(result: std::io::Result<()>) -> String {
    result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

fn drain(requests: &mpsc::Receiver<Seen>) -> Vec<GoRequest> {
    let mut seen = Vec::new();
    while let Ok(request) = requests.recv_timeout(Duration::from_millis(300)) {
        seen.push(request.as_go());
    }
    seen
}

/// Replay one Go capture scenario; returns the requests, the per-call errors
/// and the dropped count.
fn replay(name: &str) -> (Vec<GoRequest>, Vec<String>, u64) {
    let hour = Duration::from_secs(3600);
    let ok = || stand_in("200 OK", String::new());
    match name {
        "backlog_drop_flush_close" => {
            let (endpoint, requests) = ok();
            let sink = ClickHouseSink::new(options(&endpoint, "audit_log", 100, hour, 3)).unwrap();
            for record in rows(5, 1) {
                sink.append_record(&record).unwrap();
            }
            let errors = vec![
                outcome(sink.flush()),
                outcome(sink.flush()),
                outcome(sink.close()),
                outcome(sink.close()),
            ];
            (drain(&requests), errors, sink.dropped())
        }
        "basic_auth_set_and_cleared" => {
            let (endpoint, requests) = ok();
            let sink = ClickHouseSink::new(options(&endpoint, "events", 100, hour, 10)).unwrap();
            sink.set_auth("user", "p@ss:w\u{f6}rd");
            sink.append_record(&rows(1, 1)[0]).unwrap();
            let first = outcome(sink.flush());
            sink.set_auth("", "ignored");
            sink.append_record(&rows(1, 2)[0]).unwrap();
            let errors = vec![first, outcome(sink.flush()), outcome(sink.close())];
            (drain(&requests), errors, sink.dropped())
        }
        "status_500_drops_batch" => {
            let message = format!(
                "Code: 60. DB::Exception: Table default.audit_log does not exist. {}",
                "x".repeat(600)
            );
            let (endpoint, requests) = stand_in("500 Internal Server Error", message);
            let sink = ClickHouseSink::new(options(&endpoint, "audit_log", 100, hour, 10)).unwrap();
            for record in rows(2, 1) {
                sink.append_record(&record).unwrap();
            }
            let errors = vec![outcome(sink.flush()), outcome(sink.close())];
            (drain(&requests), errors, sink.dropped())
        }
        "status_400_short_body" => {
            let (endpoint, requests) = stand_in("400 Bad Request", "bad\n".into());
            let sink = ClickHouseSink::new(options(&endpoint, "audit_log", 100, hour, 10)).unwrap();
            sink.append_record(&rows(1, 1)[0]).unwrap();
            let errors = vec![outcome(sink.flush()), outcome(sink.close())];
            (drain(&requests), errors, sink.dropped())
        }
        "row_trigger_then_close" => {
            let (endpoint, requests) = ok();
            let sink = ClickHouseSink::new(options(&endpoint, "audit_log", 2, hour, 10)).unwrap();
            for record in rows(2, 1) {
                sink.append_record(&record).unwrap();
            }
            let first = requests
                .recv_timeout(Duration::from_secs(5))
                .expect("max_rows did not trigger an insert");
            sink.append_record(&rows(1, 3)[0]).unwrap();
            let errors = vec![outcome(sink.close())];
            sink.append_record(&rows(1, 4)[0]).unwrap();
            let mut seen = vec![first.as_go()];
            seen.extend(drain(&requests));
            (seen, errors, sink.dropped())
        }
        "timer_flush" => {
            let (endpoint, requests) = ok();
            let sink = ClickHouseSink::new(options(
                &endpoint,
                "audit_log",
                100,
                Duration::from_millis(50),
                10,
            ))
            .unwrap();
            sink.append_record(&rows(1, 1)[0]).unwrap();
            let first = requests
                .recv_timeout(Duration::from_secs(5))
                .expect("max_wait did not trigger an insert");
            let errors = vec![outcome(sink.close())];
            let mut seen = vec![first.as_go()];
            seen.extend(drain(&requests));
            (seen, errors, sink.dropped())
        }
        "server_down" => {
            let endpoint = {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                format!("http://{}", listener.local_addr().unwrap())
            };
            let sink = ClickHouseSink::new(options(&endpoint, "audit_log", 100, hour, 10)).unwrap();
            for record in rows(3, 1) {
                sink.append_record(&record).unwrap();
            }
            let error = sink.flush().err().map(|_| "transport error".to_owned());
            sink.close().unwrap();
            (Vec::new(), vec![error.unwrap_or_default()], sink.dropped())
        }
        other => panic!("unknown scenario {other}"),
    }
}

/// Go request bytes, error texts and dropped counts for each captured
/// scenario: backlog overflow, Basic authentication set and cleared, a 500
/// with a quoted body cut at 512 bytes, a 400 whose body keeps its newline,
/// the row trigger then close, the timer and a server that is down. The one
/// expected difference: an append after close counts as dropped here.
#[test]
fn request_scenarios_match_go() {
    let scenarios = golden().scenarios;
    assert_eq!(scenarios.len(), 7);
    for case in scenarios {
        let (requests, errors, dropped) = replay(&case.name);
        assert_eq!(requests, case.requests.unwrap_or_default(), "{}", case.name);
        assert_eq!(errors, case.errors, "{}", case.name);
        let difference = u64::from(case.name == "row_trigger_then_close");
        assert_eq!(dropped, case.dropped + difference, "{}", case.name);
    }
}
