//! The ClickHouse projection against a real self-hosted ClickHouse server
//! (for example the `clickhouse/clickhouse-server` image), reached at
//! `HUMANYMOUS_TEST_CLICKHOUSE` (`http://host:port`) with the credential in
//! `HUMANYMOUS_TEST_CLICKHOUSE_USER` and `HUMANYMOUS_TEST_CLICKHOUSE_PASSWORD`.
//! It needs that server, and an unset variable fails the test instead of
//! passing it. No CI job provides a server, so the target is `test = false`
//! (left out of `cargo test` and the native runner's workspace harnesses) and
//! the test is also `#[ignore]`. Run it by hand:
//! `cargo test -p humanymous-runtime --test audit-clickhouse-live -- --ignored`.
use humanymous_runtime::audit::{
    projection::{
        RecordSink,
        clickhouse::{ClickHouseOptions, ClickHouseSink},
    },
    record::{Actor, Record, Signal},
};
use serde_json::Value;
use std::{
    env,
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

struct Server {
    endpoint: String,
    user: String,
    password: String,
}

fn server() -> Server {
    let variable = |name: &str| env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    Server {
        endpoint: variable("HUMANYMOUS_TEST_CLICKHOUSE"),
        user: variable("HUMANYMOUS_TEST_CLICKHOUSE_USER"),
        password: variable("HUMANYMOUS_TEST_CLICKHOUSE_PASSWORD"),
    }
}

/// Run one SQL statement through the HTTP interface and return the body.
fn query(server: &Server, sql: &str) -> String {
    use base64::Engine as _;
    let authority = server.endpoint.strip_prefix("http://").unwrap();
    let mut stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let credential = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", server.user, server.password));
    write!(
        stream,
        "POST / HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Basic {credential}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{sql}",
        sql.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    assert!(head.starts_with("HTTP/1.1 200"), "{sql}: {response}");
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(body)
    } else {
        body.to_owned()
    }
}

fn dechunk(mut body: &str) -> String {
    let mut out = String::new();
    loop {
        let (size, rest) = body.split_once("\r\n").unwrap();
        let size = usize::from_str_radix(size.trim(), 16).unwrap();
        if size == 0 {
            return out;
        }
        out.push_str(&rest[..size]);
        body = &rest[size + 2..];
    }
}

fn record(seq: u64) -> Record {
    Record {
        event_id: format!("0190f1a2-0000-7000-8000-{seq:012x}"),
        seq,
        node_id: "node-live".into(),
        ts: "2026-11-19T00:00:03.123456789Z".into(),
        event_type: "enforcement.deny".into(),
        event_version: "1.0.0".into(),
        actor: Actor {
            kind: "subject".into(),
            id_pseudonym: "psn-\u{e9}\u{2028}".into(),
        },
        tenant_id: "t\u{f6}nant".into(),
        host: "<shop>&co \"quoted\"\n".into(),
        risk_score: -7,
        triggered_rules: Some(vec!["HR-07".into(), "<b>".into()]),
        top_signals: Some(vec![
            Signal {
                id: "a".into(),
                verdict: "bot".into(),
                conf: 1.0,
            },
            Signal {
                id: "b".into(),
                verdict: "human".into(),
                conf: 1e-7,
            },
            Signal {
                id: "c".into(),
                verdict: "bot".into(),
                conf: 1e21,
            },
        ]),
        latency_us: u32::MAX,
        key_id: "k1".into(),
        record_hash: format!("{seq:064x}"),
        ..Default::default()
    }
}

/// Real server: the table declares a subset of the record columns (the rest
/// are skipped by `input_format_skip_unknown_fields=1`), the sink inserts a
/// row-triggered batch, a flushed batch and a batch flushed on close with
/// Basic authentication, and every stored value reads back exactly, including
/// the HTML-escaped and line-separator text and Go-spelled floats. A wrong
/// password is refused with the server's status and its rows are counted.
#[test]
#[ignore = "needs a live ClickHouse server; set HUMANYMOUS_TEST_CLICKHOUSE"]
fn live_clickhouse_stores_every_projected_row() {
    let server = server();
    query(&server, "DROP TABLE IF EXISTS humanymous_audit_parity");
    query(
        &server,
        "CREATE TABLE humanymous_audit_parity (node_id String, seq UInt64, event_id String, ts String, \
         event_type String, actor Tuple(kind String, id_pseudonym String), tenant_id String, host String, \
         risk_score Int64, triggered_rules Array(String), \
         top_signals Array(Tuple(id String, verdict String, conf Float64)), latency_us UInt32, \
         key_id String, record_hash String) ENGINE = MergeTree ORDER BY (node_id, seq)",
    );
    let sink = ClickHouseSink::new(ClickHouseOptions {
        endpoint: server.endpoint.clone(),
        table: "humanymous_audit_parity".into(),
        max_rows: 3,
        max_wait: Duration::from_secs(3600),
        backlog: 100,
    })
    .unwrap();
    sink.set_auth(&server.user, &server.password);
    for seq in 1..=4 {
        sink.append_record(&record(seq)).unwrap();
    }
    sink.flush().unwrap();
    for seq in 5..=6 {
        sink.append_record(&record(seq)).unwrap();
    }
    sink.close().unwrap();
    assert_eq!(sink.dropped(), 0);

    let stored = query(
        &server,
        "SELECT * FROM humanymous_audit_parity ORDER BY seq FORMAT JSONEachRow \
         SETTINGS output_format_json_named_tuples_as_objects = 1, output_format_json_quote_64bit_integers = 0",
    );
    let rows: Vec<Value> = stored
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 6, "{stored}");
    for (row, seq) in rows.iter().zip(1..) {
        let expected = record(seq);
        assert_eq!(row["seq"], seq);
        assert_eq!(row["node_id"], expected.node_id);
        assert_eq!(row["event_id"], expected.event_id);
        assert_eq!(row["ts"], expected.ts);
        assert_eq!(row["actor"]["id_pseudonym"], expected.actor.id_pseudonym);
        assert_eq!(row["tenant_id"], expected.tenant_id);
        assert_eq!(row["host"], expected.host);
        assert_eq!(row["risk_score"], -7);
        assert_eq!(row["triggered_rules"][1], "<b>");
        let confs: Vec<f64> = row["top_signals"]
            .as_array()
            .unwrap()
            .iter()
            .map(|signal| signal["conf"].as_f64().unwrap())
            .collect();
        assert_eq!(confs, [1.0, 1e-7, 1e21]);
        assert_eq!(row["latency_us"], u32::MAX);
        assert_eq!(row["record_hash"], expected.record_hash);
    }

    let refused = ClickHouseSink::new(ClickHouseOptions {
        endpoint: server.endpoint.clone(),
        table: "humanymous_audit_parity".into(),
        max_rows: 100,
        max_wait: Duration::from_secs(3600),
        backlog: 100,
    })
    .unwrap();
    refused.set_auth(&server.user, "wrong-password");
    refused.append_record(&record(7)).unwrap();
    let error = refused.flush().unwrap_err().to_string();
    assert!(error.starts_with("clickhouse insert status "), "{error}");
    assert_eq!(refused.dropped(), 1);
    refused.close().unwrap();
    query(&server, "DROP TABLE humanymous_audit_parity");
}
