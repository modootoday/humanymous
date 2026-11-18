//! Ingress-only client. It first tries the origin and the Gate's loopback
//! ports directly, then drives real HTTP/1.1 bytes through the Gate ingress.
//! It reports what it observed on the wire; origin contacts are counted
//! independently by the runner from the origin's own log.
use super::{CLIENT_ROWS, CLIENT_SCHEMA, EFFECT, FRONTEND_PORT, ORIGIN_PORT, RUNTIME_PORT};
use crate::client::{report, request, sign, wire};
use humanymous_runtime::gateway::{configuration::Configuration, http::Request};
use serde_json::{Value, json};
use std::{
    io::{self, BufReader, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Reply {
    status: Option<u16>,
    body: Vec<u8>,
}

fn exchange(address: SocketAddr, bytes: &[u8]) -> Reply {
    let result = (|| -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(bytes)?;
        let mut out = Vec::new();
        stream.take(1 << 20).read_to_end(&mut out)?;
        Ok(out)
    })();
    let bytes = result.unwrap_or_default();
    let end = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(bytes.len(), |end| end + 4);
    Reply {
        status: std::str::from_utf8(bytes.get(9..12).unwrap_or_default())
            .ok()
            .and_then(|code| code.parse().ok()),
        body: bytes.get(end..).unwrap_or_default().to_vec(),
    }
}

/// A direct TCP connect attempt; `Ok` would mean the address is reachable.
fn unreachable(address: SocketAddr) -> (bool, String) {
    match TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
        Ok(_) => (false, "connected".into()),
        Err(error) => (true, format!("{:?}", error.kind())),
    }
}

fn row(id: &str, passed: bool, detail: Value) -> Value {
    json!({"id":id,"status":if passed {"passed"} else {"failed"},"detail":detail})
}

/// Signing configuration: the Gate's scope and routes, placeholder custody.
fn configuration() -> Result<Configuration> {
    Ok(serde_json::from_value(super::document(
        "/unused",
        "/unused",
        "unused",
        false,
        Value::Null,
        65532,
    ))?)
}

/// Execute every row against `gate` (ingress address) and `origin` (backend
/// address, which must be unreachable from here).
pub fn execute(gate: IpAddr, origin: IpAddr) -> Result<Vec<Value>> {
    let config = configuration()?;
    let front = SocketAddr::new(gate, FRONTEND_PORT);
    let mut rows = Vec::new();

    let (closed, kind) = unreachable(SocketAddr::new(origin, ORIGIN_PORT));
    rows.push(row(
        "origin-direct-unreachable",
        closed,
        json!({"target":format!("{origin}:{ORIGIN_PORT}"),"observed":kind}),
    ));
    let runtime = unreachable(SocketAddr::new(gate, RUNTIME_PORT));
    let backend = unreachable(SocketAddr::new(gate, ORIGIN_PORT));
    rows.push(row(
        "gate-loopback-ports-unreachable",
        runtime.0 && backend.0,
        json!({"runtime":runtime.1,"backend_relay":backend.1}),
    ));

    let issued = exchange(front, &wire(&request("/_humanymous/session", vec![])));
    let session: Value = serde_json::from_slice(&issued.body).unwrap_or(Value::Null);
    let has_session =
        issued.status == Some(200) && session["session"].is_string() && session["key"].is_string();
    rows.push(row(
        "session-issued-via-gate",
        has_session,
        json!({"status":issued.status}),
    ));

    let unsigned = exchange(front, &wire(&request(EFFECT, b"effect".to_vec())));
    rows.push(row(
        "unsigned-refused",
        unsigned.status == Some(401),
        json!({"status":unsigned.status}),
    ));

    let absolute = request(
        &format!("http://{origin}:{ORIGIN_PORT}{EFFECT}"),
        b"effect".to_vec(),
    );
    let reply = exchange(front, &wire(&absolute));
    rows.push(row(
        "absolute-form-refused",
        reply.status.is_some_and(|code| (400..500).contains(&code)),
        json!({"status":reply.status}),
    ));

    let (forwarded, replayed) = if has_session {
        let mut collect = request("/_humanymous/collect", report(&session));
        sign(&config, &mut collect, &session, 1);
        let collected = exchange(front, &wire(&collect));
        let mut effect: Request = request(EFFECT, b"effect".to_vec());
        sign(&config, &mut effect, &session, 2);
        let bytes = wire(&effect);
        let first = exchange(front, &bytes);
        let second = exchange(front, &bytes);
        (
            (
                collected.status == Some(200)
                    && first.status == Some(200)
                    && first.body == b"origin-ok",
                json!({"collect_status":collected.status,"status":first.status,"origin_body":first.body == b"origin-ok"}),
            ),
            (second.status == Some(403), json!({"status":second.status})),
        )
    } else {
        ((false, json!("no session")), (false, json!("no session")))
    };
    rows.push(row("signed-effect-forwarded", forwarded.0, forwarded.1));
    rows.push(row("replay-refused", replayed.0, replayed.1));
    Ok(rows)
}

/// One control line on stdin: `{"gate":IP,"origin":IP,"phase":NAME}`.
pub fn run() {
    let report = (|| -> Result<Value> {
        let control = super::read_line(&mut BufReader::new(io::stdin()), 4096)?;
        let gate: IpAddr = control["gate"].as_str().ok_or("gate address")?.parse()?;
        let origin: IpAddr = control["origin"]
            .as_str()
            .ok_or("origin address")?
            .parse()?;
        let phase = control["phase"].as_str().ok_or("phase")?.to_owned();
        let rows = execute(gate, origin)?;
        let exact = rows.len() == CLIENT_ROWS.len()
            && rows.iter().zip(CLIENT_ROWS).all(|(row, id)| row["id"] == *id);
        let passed = exact && rows.iter().all(|row| row["status"] == "passed");
        Ok(
            json!({"schema":CLIENT_SCHEMA,"phase":phase,"status":if passed {"passed"} else {"failed"},
            "cases":rows,"expected_effects":1,"full_product_qualified":false}),
        )
    })()
    .unwrap_or_else(|error| {
        json!({"schema":CLIENT_SCHEMA,"status":"failed","cases":[],"error":error.to_string(),
            "full_product_qualified":false})
    });
    let passed = report["status"] == "passed";
    super::emit(&report).expect("client report output");
    if !passed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_configuration_matches_the_gate_document() {
        let config = configuration().unwrap();
        let gate: Configuration = serde_json::from_value(super::super::document(
            "/a",
            "/k",
            "secret",
            true,
            Value::Null,
            65532,
        ))
        .unwrap();
        assert_eq!(config.scope.authority, gate.scope.authority);
        assert_eq!(config.scope.installation, gate.scope.installation);
        assert_eq!(config.routes.len(), gate.routes.len());
        assert_eq!(config.routes[0].path, gate.routes[0].path);
    }

    #[test]
    fn unreachable_reports_a_closed_port_and_a_connected_one() {
        let open = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(!unreachable(open.local_addr().unwrap()).0);
        let address = open.local_addr().unwrap();
        drop(open);
        assert!(unreachable(address).0);
    }
}
