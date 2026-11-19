//! Go `internal/audit/sink_adapters_test.go` (oracle 981d25c) against
//! in-process Redis and ClickHouse stand-ins, plus the deliberate differences.
use super::{
    RecordSink,
    clickhouse::{ClickHouseOptions, ClickHouseSink, valid_table_name},
    redis_stream::{RedisStreamOptions, RedisStreamSink, resp_command},
};
use crate::audit::record::{Actor, Record, Signal};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

mod parity;
#[cfg(feature = "audit-tls")]
mod tls;

/// Go `sampleRecord` from `audit_test.go`.
fn sample_record(seq: u64) -> Record {
    Record {
        seq,
        event_type: "enforcement.deny".into(),
        actor: Actor {
            kind: "system".into(),
            ..Default::default()
        },
        tenant_id: "t1".into(),
        session_pseudonym: "psn-abc".into(),
        host: "app.example".into(),
        route_class: "html".into(),
        verdict: "deny".into(),
        risk_score: 83,
        triggered_rules: Some(vec!["HR-23".into(), "HR-2".into()]),
        top_signals: Some(vec![
            Signal {
                id: "l5.h2dos.rapid_reset".into(),
                verdict: "BOT".into(),
                conf: 1.0,
            },
            Signal {
                id: "x.ua_vs_ja4".into(),
                verdict: "BOT".into(),
                conf: 0.9,
            },
        ]),
        enforcement_action: "block".into(),
        enforcement_mode: "enforce".into(),
        key_id: "k1".into(),
        ..Default::default()
    }
}

fn eventually(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

/// Read one RESP array of bulk strings, as the Go stand-in did.
fn read_command(reader: &mut impl BufRead) -> Option<Vec<Vec<u8>>> {
    let mut header = String::new();
    if reader.read_line(&mut header).ok()? == 0 {
        return None;
    }
    let count: usize = header.trim_end().strip_prefix('*')?.parse().ok()?;
    let mut args = Vec::with_capacity(count);
    for _ in 0..count {
        let mut length = String::new();
        reader.read_line(&mut length).ok()?;
        let length: usize = length.trim_end().strip_prefix('$')?.parse().ok()?;
        let mut value = vec![0; length + 2];
        reader.read_exact(&mut value).ok()?;
        value.truncate(length);
        args.push(value);
    }
    Some(args)
}

/// In-process Redis that answers every command on one connection with
/// `reply(command)` and reports each command it read.
fn redis_stand_in(reply: fn(&[Vec<u8>]) -> Vec<u8>) -> (String, mpsc::Receiver<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut writer = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        while let Some(command) = read_command(&mut reader) {
            let answer = reply(&command);
            if sender.send(command).is_err() || writer.write_all(&answer).is_err() {
                return;
            }
        }
    });
    (address, receiver)
}

fn redis_options(address: String, backlog: usize) -> RedisStreamOptions {
    RedisStreamOptions {
        address,
        stream: "audit:n1".into(),
        max_len: 1000,
        backlog,
    }
}

/// Go `TestRespCommandEncoding`.
#[test]
fn resp_command_encoding() {
    assert_eq!(
        resp_command(&[b"XADD", b"s", b"1-0"]),
        b"*3\r\n$4\r\nXADD\r\n$1\r\ns\r\n$3\r\n1-0\r\n"
    );
}

/// Go `TestRedisStreamSinkXADD`: the shipper connects and issues an XADD with
/// the stream, MAXLEN, the seq-derived id and the record field. The stand-in
/// answers with the bulk id real Redis returns; the second record on the same
/// connection proves the whole reply was consumed (Go read one line only).
#[test]
fn redis_stream_sink_xadd() {
    let (address, commands) = redis_stand_in(|command| {
        let id = &command[6];
        let mut reply = format!("${}\r\n", id.len()).into_bytes();
        reply.extend_from_slice(id);
        reply.extend_from_slice(b"\r\n");
        reply
    });
    let sink = RedisStreamSink::new(redis_options(address, 16)).unwrap();
    for seq in [7, 8] {
        sink.append_record(&sample_record(seq)).unwrap();
    }
    for seq in [7u64, 8] {
        let command = commands
            .recv_timeout(Duration::from_secs(3))
            .expect("redis shipper did not send XADD");
        let text: Vec<&[u8]> = command.iter().map(Vec::as_slice).collect();
        let id = format!("{seq}-0");
        assert_eq!(
            text[..7],
            [
                &b"XADD"[..],
                b"audit:n1",
                b"MAXLEN",
                b"~",
                b"1000",
                id.as_bytes(),
                b"record"
            ]
        );
        let shipped: Record = serde_json::from_slice(&command[7]).unwrap();
        assert_eq!(shipped, sample_record(seq));
    }
    sink.flush().unwrap();
    assert_eq!(sink.dropped(), 0);
    sink.close().unwrap();
}

/// Go `TestRedisStreamSinkDropsWhenFull`: a full backlog drops and counts
/// instead of blocking the seal.
#[test]
fn redis_stream_sink_drops_when_full() {
    let (sink, _undrained) = RedisStreamSink::unstarted(1);
    for seq in 0..3 {
        sink.append_record(&sample_record(seq)).unwrap();
    }
    assert_eq!(sink.dropped(), 2);
    sink.close().unwrap();
    sink.append_record(&sample_record(3)).unwrap();
    assert_eq!(sink.dropped(), 3, "an append after close is dropped");
}

/// Deliberate difference: an unreachable Redis or an error reply is counted
/// as a drop (Go lost those records without counting them), and the append
/// itself still succeeds so the seal path never fails.
#[test]
fn redis_outage_and_error_replies_are_counted() {
    let refused = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().to_string()
    };
    let down = RedisStreamSink::new(redis_options(refused, 4)).unwrap();
    down.append_record(&sample_record(1)).unwrap();
    eventually("the dial failure drop", || down.dropped() == 1);
    down.close().unwrap();

    let (address, commands) = redis_stand_in(|_| b"-ERR stream rejected\r\n".to_vec());
    let rejected = RedisStreamSink::new(redis_options(address, 4)).unwrap();
    rejected.append_record(&sample_record(1)).unwrap();
    rejected.append_record(&sample_record(2)).unwrap();
    for _ in 0..2 {
        commands.recv_timeout(Duration::from_secs(3)).unwrap();
    }
    eventually("both error replies counted on one connection", || {
        rejected.dropped() == 2
    });
}

struct Captured {
    request_line: String,
    headers: Vec<String>,
    body: String,
}

/// In-process ClickHouse HTTP interface answering every insert with `status`.
fn clickhouse_stand_in(status: &'static str) -> (String, mpsc::Receiver<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let Some(captured) = read_request(&stream) else {
                continue;
            };
            let message = if status.starts_with('2') {
                ""
            } else {
                "Code: 60. DB::Exception: Table default.audit_log does not exist"
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
                message.len()
            );
            let _ = (&stream).write_all(response.as_bytes());
            if sender.send(captured).is_err() {
                return;
            }
        }
    });
    (format!("http://{address}"), receiver)
}

fn read_request(stream: &TcpStream) -> Option<Captured> {
    let mut reader = BufReader::new(stream);
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
    Some(Captured {
        request_line: request_line.trim_end().to_owned(),
        headers,
        body: String::from_utf8(body).ok()?,
    })
}

fn clickhouse_options(endpoint: String, max_rows: usize, max_wait: Duration) -> ClickHouseOptions {
    ClickHouseOptions {
        endpoint,
        table: "audit_log".into(),
        max_rows,
        max_wait,
        backlog: 1000,
    }
}

/// Go `TestCHSinkInsert`: the sink POSTs an `INSERT ... FORMAT JSONEachRow`
/// batch with the records as the body.
#[test]
fn clickhouse_sink_insert() {
    let (endpoint, requests) = clickhouse_stand_in("200 OK");
    let sink =
        ClickHouseSink::new(clickhouse_options(endpoint, 100, Duration::from_millis(50))).unwrap();
    sink.append_record(&sample_record(42)).unwrap();
    sink.flush().expect("flush");
    let captured = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("clickhouse sink did not POST an insert");
    assert!(
        captured
            .request_line
            .starts_with("POST /?query=INSERT+INTO+audit_log+FORMAT+JSONEachRow&async_insert=1&wait_for_async_insert=1&input_format_skip_unknown_fields=1 HTTP/1.1"),
        "{}",
        captured.request_line
    );
    assert!(
        captured
            .headers
            .contains(&"Content-Type: application/x-ndjson".to_owned())
    );
    assert!(
        !captured
            .headers
            .iter()
            .any(|h| h.starts_with("Authorization"))
    );
    assert!(captured.body.contains("\"seq\":42"), "{}", captured.body);
    let lines: Vec<&str> = captured.body.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(captured.body.ends_with('\n'));
    let shipped: Record = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(shipped, sample_record(42));
    assert_eq!(sink.dropped(), 0);
    sink.close().unwrap();
}

/// Go `TestCHTableNameValidation`: an unsafe table name is rejected and the
/// sink falls back to the safe default.
#[test]
fn clickhouse_table_name_validation() {
    assert!(!valid_table_name("audit_log; DROP TABLE x"));
    assert!(valid_table_name("audit_log"));
    assert!(valid_table_name("_Audit2"));
    assert!(!valid_table_name(""));
    assert!(!valid_table_name("2audit"));
    let sink = ClickHouseSink::new(ClickHouseOptions {
        table: "audit_log; DROP TABLE x".into(),
        ..clickhouse_options("http://127.0.0.1:9".into(), 1, Duration::from_secs(60))
    })
    .unwrap();
    assert!(
        sink.insert_target()
            .starts_with("/?query=INSERT+INTO+audit_log+FORMAT+JSONEachRow&")
    );
}

/// Go `SetAuth` and the batch trigger: the credential travels only as Basic
/// authentication, and reaching `max_rows` ships without an explicit flush.
#[test]
fn clickhouse_basic_auth_and_row_trigger() {
    let (endpoint, requests) = clickhouse_stand_in("200 OK");
    let sink =
        ClickHouseSink::new(clickhouse_options(endpoint, 2, Duration::from_secs(60))).unwrap();
    sink.set_auth("auditor", "s3cret");
    assert!(!sink.insert_target().contains("s3cret"));
    sink.append_record(&sample_record(1)).unwrap();
    sink.append_record(&sample_record(2)).unwrap();
    let captured = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("max_rows did not trigger a batch");
    assert!(
        captured
            .headers
            .contains(&"Authorization: Basic YXVkaXRvcjpzM2NyZXQ=".to_owned())
    );
    assert!(!captured.request_line.contains("s3cret"));
    let seqs: Vec<u64> = captured
        .body
        .lines()
        .map(|line| serde_json::from_str::<Record>(line).unwrap().seq)
        .collect();
    assert_eq!(seqs, [1, 2]);
    sink.set_auth("", "");
    sink.append_record(&sample_record(3)).unwrap();
    sink.flush().unwrap();
    let captured = requests.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(
        !captured
            .headers
            .iter()
            .any(|h| h.starts_with("Authorization"))
    );
}

/// Go `CHSink.Close` flushed before stopping: a partial batch that neither
/// reached `max_rows` nor waited `max_wait` is still inserted on close, after
/// a full batch already went out on the row trigger. Carried over from the
/// superseded PR #71 projection tests.
#[test]
fn clickhouse_close_flushes_the_partial_batch() {
    let (endpoint, requests) = clickhouse_stand_in("200 OK");
    let sink =
        ClickHouseSink::new(clickhouse_options(endpoint, 2, Duration::from_secs(60))).unwrap();
    sink.append_record(&sample_record(1)).unwrap();
    sink.append_record(&sample_record(2)).unwrap();
    let full = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("max_rows did not trigger a batch");
    assert_eq!(full.body.lines().count(), 2);
    sink.append_record(&sample_record(3)).unwrap();
    assert!(
        requests.recv_timeout(Duration::from_millis(200)).is_err(),
        "a partial batch waits for max_wait"
    );
    sink.close().unwrap();
    let partial = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("close did not flush the partial batch");
    let seqs: Vec<u64> = partial
        .body
        .lines()
        .map(|line| serde_json::from_str::<Record>(line).unwrap().seq)
        .collect();
    assert_eq!(seqs, [3]);
    assert_eq!(sink.dropped(), 0);
}

/// Go counted a failed or refused insert as dropped rows and returned the
/// status; a full backlog drops too, and the append never fails. Rust also
/// drops an append after close.
#[test]
fn clickhouse_failures_drop_and_count() {
    let (endpoint, requests) = clickhouse_stand_in("404 Not Found");
    let sink = ClickHouseSink::new(ClickHouseOptions {
        backlog: 2,
        ..clickhouse_options(endpoint, 100, Duration::from_secs(60))
    })
    .unwrap();
    for seq in 1..=3 {
        sink.append_record(&sample_record(seq)).unwrap();
    }
    assert_eq!(sink.dropped(), 1, "the third row overflows the backlog");
    let error = sink.flush().unwrap_err().to_string();
    assert!(
        error.starts_with("clickhouse insert status 404: Code: 60."),
        "{error}"
    );
    requests.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(sink.dropped(), 3, "the refused batch is counted");

    let refused = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let down =
        ClickHouseSink::new(clickhouse_options(refused, 100, Duration::from_secs(60))).unwrap();
    down.append_record(&sample_record(1)).unwrap();
    assert!(down.flush().is_err());
    assert_eq!(down.dropped(), 1);
    down.close().unwrap();
    down.append_record(&sample_record(2)).unwrap();
    assert_eq!(down.dropped(), 2, "an append after close is dropped");
}

/// Deliberate difference: `new` accepts only plain `http://` endpoints
/// without a query or embedded credentials; `https://` needs `with_tls` and
/// explicit trust roots (see `tls.rs`).
#[test]
fn clickhouse_endpoint_is_refused_unless_plain_http() {
    for endpoint in [
        "https://ch:8443",
        "ch:8123",
        "http://user:pass@ch:8123",
        "http://ch:8123/?query=SELECT+1",
        "http://",
        "http://ch 8123",
    ] {
        assert!(
            ClickHouseSink::new(clickhouse_options(
                endpoint.into(),
                1,
                Duration::from_secs(60)
            ))
            .is_err(),
            "{endpoint}"
        );
    }
    let sink = ClickHouseSink::new(clickhouse_options(
        "http://ch/base".into(),
        1,
        Duration::from_secs(60),
    ))
    .unwrap();
    assert!(sink.insert_target().starts_with("/base/?query="));
}

/// Deliberate difference: a reply that is not RESP, or a connection the server
/// closes before replying, is a transport failure. The record is counted as
/// dropped and the connection is discarded, so the next record redials.
#[test]
fn redis_transport_failures_are_counted_and_redial() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (sender, accepted) = mpsc::channel();
    thread::spawn(move || {
        for (index, stream) in listener.incoming().take(2).enumerate() {
            let Ok(stream) = stream else { return };
            let mut reader = BufReader::new(&stream);
            let Some(command) = read_command(&mut reader) else {
                return;
            };
            if index == 0 {
                let _ = (&stream).write_all(b"?not resp\r\n");
            }
            // The second connection closes without any reply.
            drop(reader);
            drop(stream);
            if sender.send(command).is_err() {
                return;
            }
        }
    });
    let sink = RedisStreamSink::new(redis_options(address, 4)).unwrap();
    sink.append_record(&sample_record(1)).unwrap();
    accepted.recv_timeout(Duration::from_secs(3)).unwrap();
    eventually("the garbage reply drop", || sink.dropped() == 1);
    sink.append_record(&sample_record(2)).unwrap();
    let redialled = accepted
        .recv_timeout(Duration::from_secs(3))
        .expect("the shipper did not redial after a transport failure");
    assert_eq!(redialled[5], b"2-0");
    eventually("the closed connection drop", || sink.dropped() == 2);
    sink.close().unwrap();
}

/// Deliberate difference: records still queued when the sink closes are
/// counted as dropped (Go lost them silently). The record already in flight
/// completes.
#[test]
fn redis_records_queued_at_close_are_counted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (seen, commands) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(&stream);
        while let Some(command) = read_command(&mut reader) {
            if seen.send(command).is_err() || released.recv().is_err() {
                return;
            }
            if (&stream).write_all(b"+OK\r\n").is_err() {
                return;
            }
        }
    });
    let sink = RedisStreamSink::new(redis_options(address, 8)).unwrap();
    sink.append_record(&sample_record(1)).unwrap();
    let in_flight = commands.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(in_flight[5], b"1-0");
    for seq in 2..=4 {
        sink.append_record(&sample_record(seq)).unwrap();
    }
    sink.close().unwrap();
    release.send(()).unwrap();
    eventually("the three queued records counted", || sink.dropped() == 3);
    thread::sleep(Duration::from_millis(100));
    assert_eq!(sink.dropped(), 3, "the in-flight record was delivered");
    assert!(
        commands.try_recv().is_err(),
        "nothing queued is shipped after close"
    );
}

/// Deliberate difference: only a 2xx status is a delivered insert. Go took
/// any status below 300, so an informational 1xx line counted as success;
/// Rust refuses it like a redirect or a server error, reports the status and
/// counts the batch as dropped.
#[test]
fn clickhouse_only_2xx_is_success() {
    for (status, code) in [
        ("103 Early Hints", 103),
        ("302 Found", 302),
        ("500 Internal Server Error", 500),
    ] {
        let (endpoint, requests) = clickhouse_stand_in(status);
        let sink = ClickHouseSink::new(clickhouse_options(endpoint, 100, Duration::from_secs(60)))
            .unwrap();
        sink.append_record(&sample_record(1)).unwrap();
        sink.append_record(&sample_record(2)).unwrap();
        let error = sink.flush().unwrap_err().to_string();
        assert!(
            error.starts_with(&format!("clickhouse insert status {code}:")),
            "{status}: {error}"
        );
        requests.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(sink.dropped(), 2, "{status}: the batch is counted");
        sink.close().unwrap();
    }
}

/// Each insert has one total deadline (Go `http.Client.Timeout`). A server
/// that accepts the request and then trickles its response headers one byte
/// at a time cannot keep `flush` waiting past it: the insert fails and its
/// rows count as dropped.
#[test]
fn clickhouse_trickled_response_misses_the_insert_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        if read_request(&stream).is_none() {
            return;
        }
        let mut writer = &stream;
        if writer.write_all(b"HTTP/1.1 200 OK\r\nX-Pad: ").is_err() {
            return;
        }
        let stop = Instant::now() + Duration::from_secs(10);
        while Instant::now() < stop && writer.write_all(b"a").is_ok() {
            thread::sleep(Duration::from_millis(20));
        }
    });
    let deadline = Duration::from_millis(300);
    let sink = ClickHouseSink::with_request_timeout(
        clickhouse_options(endpoint, 100, Duration::from_secs(60)),
        deadline,
    )
    .unwrap();
    sink.append_record(&sample_record(1)).unwrap();
    let started = Instant::now();
    let result = sink.flush();
    let elapsed = started.elapsed();
    assert!(
        result.is_err(),
        "a trickled response must not count as an insert"
    );
    assert!(elapsed >= deadline, "{elapsed:?}");
    assert!(
        elapsed < deadline + Duration::from_secs(1),
        "flush overran its deadline: {elapsed:?}"
    );
    assert_eq!(sink.dropped(), 1);
}
