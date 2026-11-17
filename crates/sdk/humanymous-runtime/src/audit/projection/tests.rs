//! Ports of legacy internal/audit/sink_adapters_test.go (oracle 981d25c)
//! against in-process RESP and HTTP servers.
use super::Projection;
use super::clickhouse::{ClickHouseSettings, ClickHouseSink, batch_body, valid_table_name};
use super::redis::RedisStreamSink;
use crate::audit::record::Record;
use crate::gateway::coordinator::redis::{ClientSettings, Reply, encode, read_reply};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn record(seq: u64) -> Record {
    Record {
        seq,
        node_id: "node-1".into(),
        event_type: "gate.decision".into(),
        verdict: "deny".into(),
        ..Record::default()
    }
}

fn eventually(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "condition not reached in time");
        thread::yield_now();
    }
}

#[test]
fn resp_command_encoding() {
    assert_eq!(
        encode(&[b"XADD", b"s", b"1-0"]),
        b"*3\r\n$4\r\nXADD\r\n$1\r\ns\r\n$3\r\n1-0\r\n"
    );
}

#[test]
fn redis_stream_sink_xadds_each_record_with_its_sequence_id() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        while let Ok(Reply::Array(items)) = read_reply(&mut reader, 0) {
            let args: Vec<Vec<u8>> = items
                .into_iter()
                .map(|item| match item {
                    Reply::Bulk(bytes) => bytes,
                    _ => Vec::new(),
                })
                .collect();
            let id = args[5].clone();
            log.lock().unwrap().push(args);
            writer
                .write_all(
                    &[
                        b"$".as_slice(),
                        id.len().to_string().as_bytes(),
                        b"\r\n",
                        &id,
                        b"\r\n",
                    ]
                    .concat(),
                )
                .unwrap();
        }
    });
    let sink =
        RedisStreamSink::new(address, "audit:node-1", 100, 16, ClientSettings::default()).unwrap();
    sink.append(&record(1));
    sink.append(&record(2));
    eventually(|| seen.lock().unwrap().len() == 2);
    sink.close();
    let seen = seen.lock().unwrap();
    for (index, args) in seen.iter().enumerate() {
        assert_eq!(
            args[..6],
            [
                b"XADD".to_vec(),
                b"audit:node-1".to_vec(),
                b"MAXLEN".to_vec(),
                b"~".to_vec(),
                b"100".to_vec(),
                format!("{}-0", index + 1).into_bytes(),
            ]
        );
        assert_eq!(args[6], b"record");
        let shipped: Record = serde_json::from_slice(&args[7]).unwrap();
        assert_eq!(shipped, record(index as u64 + 1));
    }
    assert_eq!(sink.dropped(), 0);
}

#[test]
fn redis_stream_sink_drops_and_counts_when_full_or_unreachable() {
    // A server that accepts and never answers: the shipper stalls on the first
    // record, so the one-slot backlog fills and later records are dropped.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (release, hold) = mpsc::channel::<()>();
    thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        let _ = hold.recv();
    });
    let sink = RedisStreamSink::new(
        address,
        "x",
        10,
        1,
        ClientSettings {
            timeout: Duration::from_millis(300),
            ..ClientSettings::default()
        },
    )
    .unwrap();
    for seq in 1..=5 {
        sink.append(&record(seq));
    }
    assert!(sink.dropped() >= 3, "dropped {}", sink.dropped());
    // The stalled record times out and is counted as a projection gap too.
    sink.close();
    assert_eq!(sink.dropped(), 5);
    let _ = release.send(());
    // After close nothing is accepted.
    sink.append(&record(6));
    assert_eq!(sink.dropped(), 6);
    assert!(RedisStreamSink::new("127.0.0.1:1", "", 10, 1, ClientSettings::default()).is_err());
}

/// One captured HTTP request.
#[derive(Debug, Default, Clone)]
struct Captured {
    target: String,
    authorization: Option<String>,
    content_type: Option<String>,
    body: String,
}

/// HTTP server answering every request with `status`, capturing each one.
fn http_server(status: u16) -> (String, Arc<Mutex<Vec<Captured>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let captured = Arc::new(Mutex::new(Vec::new()));
    let log = captured.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut request = Captured {
                target: line.split(' ').nth(1).unwrap_or_default().to_owned(),
                ..Captured::default()
            };
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (name, value) = header.split_once(':').unwrap();
                let value = value.trim().to_owned();
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.parse().unwrap(),
                    "authorization" => request.authorization = Some(value),
                    "content-type" => request.content_type = Some(value),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            request.body = String::from_utf8(body).unwrap();
            log.lock().unwrap().push(request);
            let reply = if status < 300 { "" } else { "table missing" };
            let mut writer = stream;
            let _ = write!(
                writer,
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (address, captured)
}

#[test]
fn clickhouse_sink_inserts_a_json_each_row_batch() {
    let (base, captured) = http_server(200);
    let sink = ClickHouseSink::new(ClickHouseSettings {
        max_wait: Duration::from_secs(60),
        user: Some("gate".into()),
        password: Some(Zeroizing::new("s3cret".into())),
        ..ClickHouseSettings::new(format!("{base}/"))
    })
    .unwrap();
    sink.append(&record(1));
    sink.append(&record(2));
    sink.flush().unwrap();
    let captured = captured.lock().unwrap().clone();
    assert_eq!(captured.len(), 1);
    let request = &captured[0];
    assert_eq!(
        request.target,
        "/?query=INSERT+INTO+audit_log+FORMAT+JSONEachRow&async_insert=1&wait_for_async_insert=1&input_format_skip_unknown_fields=1"
    );
    assert!(!request.target.contains("s3cret"));
    assert_eq!(
        request.authorization.as_deref(),
        Some("Basic Z2F0ZTpzM2NyZXQ=")
    );
    assert_eq!(
        request.content_type.as_deref(),
        Some("application/x-ndjson")
    );
    let rows: Vec<Record> = request
        .body
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows, [record(1), record(2)]);
    assert_eq!(request.body.as_bytes(), batch_body(&[record(1), record(2)]));
    assert_eq!(sink.dropped(), 0);
    assert!(
        !format!(
            "{:?}",
            ClickHouseSettings {
                password: Some(Zeroizing::new("s3cret".into())),
                ..ClickHouseSettings::new(base)
            }
        )
        .contains("s3cret")
    );
}

#[test]
fn clickhouse_sink_batches_on_row_count_and_on_close() {
    let (base, captured) = http_server(200);
    let sink = ClickHouseSink::new(ClickHouseSettings {
        max_rows: 2,
        max_wait: Duration::from_secs(60),
        ..ClickHouseSettings::new(base)
    })
    .unwrap();
    sink.append(&record(1));
    sink.append(&record(2));
    eventually(|| captured.lock().unwrap().len() == 1);
    sink.append(&record(3));
    sink.close();
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[1].body.lines().count(), 1);
}

#[test]
fn clickhouse_failures_and_overflow_are_counted_as_drops() {
    let (base, _) = http_server(500);
    let sink = ClickHouseSink::new(ClickHouseSettings {
        max_wait: Duration::from_secs(60),
        capacity: 2,
        ..ClickHouseSettings::new(base)
    })
    .unwrap();
    for seq in 1..=3 {
        sink.append(&record(seq));
    }
    assert_eq!(sink.dropped(), 1);
    let error = sink.flush().unwrap_err().to_string();
    assert!(
        error.contains("500") && error.contains("table missing"),
        "{error}"
    );
    assert_eq!(sink.dropped(), 3);
    // An unreachable server loses the batch the same way.
    let closed = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let down = ClickHouseSink::new(ClickHouseSettings {
        max_wait: Duration::from_secs(60),
        ..ClickHouseSettings::new(format!("http://{closed}"))
    })
    .unwrap();
    down.append(&record(1));
    assert!(down.flush().is_err());
    assert_eq!(down.dropped(), 1);
}

#[test]
fn clickhouse_table_name_validation() {
    for bad in ["audit_log; DROP TABLE x", "", "1table", "a-b", "a b", "t`"] {
        assert!(!valid_table_name(bad), "{bad}");
        assert!(
            ClickHouseSink::new(ClickHouseSettings {
                table: bad.into(),
                ..ClickHouseSettings::new("http://127.0.0.1:1")
            })
            .is_err()
        );
    }
    for good in ["audit_log", "_t", "Audit2"] {
        assert!(valid_table_name(good), "{good}");
    }
    assert!(ClickHouseSink::new(ClickHouseSettings::new("ftp://host")).is_err());
}
