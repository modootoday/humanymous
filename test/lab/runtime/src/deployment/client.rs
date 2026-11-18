//! Ingress-only client for the documented example routes. It reaches the
//! runtime only through the edge, first proves the runtime's and the origin
//! relay's loopback ports and the origin itself are unreachable from here,
//! then for every documented route sends an unsigned request (refused) and a
//! signed one in a fresh session (forwarded), and replays the last one.
//! Origin contacts are counted independently by the runner.
use super::{CLIENT_SCHEMA, EDGE_PORT, ORIGIN_PORT, emit, read_line};
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

fn unreachable(address: SocketAddr) -> (bool, String) {
    match TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
        Ok(_) => (false, "connected".into()),
        Err(error) => (true, format!("{:?}", error.kind())),
    }
}

fn row(id: String, passed: bool, detail: Value) -> Value {
    json!({"id":id,"status":if passed {"passed"} else {"failed"},"detail":detail})
}

/// A request for one documented route, addressed to the documented authority.
fn routed(config: &Configuration, method: &str, path: &str, body: Vec<u8>) -> Request {
    let mut built = request(path, body);
    built.method = method.into();
    for (name, value) in &mut built.headers {
        if name.eq_ignore_ascii_case("host") {
            *value = config.scope.authority.clone();
        }
    }
    built
}

/// Expected row identifiers for a configuration, in execution order.
pub fn expected_rows(config: &Configuration) -> Vec<String> {
    let mut ids = vec![
        "runtime-loopback-unreachable".to_owned(),
        "origin-direct-unreachable".to_owned(),
    ];
    for route in &config.routes {
        ids.push(format!("unsigned-refused {} {}", route.method, route.path));
        ids.push(format!("signed-forwarded {} {}", route.method, route.path));
    }
    ids.push("replay-refused".into());
    ids
}

pub fn execute(config: &Configuration, edge: IpAddr, origin: IpAddr) -> Vec<Value> {
    let front = SocketAddr::new(edge, EDGE_PORT);
    let mut rows = Vec::new();
    let runtime = unreachable(SocketAddr::new(edge, config.listen.port()));
    let relay = unreachable(SocketAddr::new(edge, config.origin.port()));
    rows.push(row(
        "runtime-loopback-unreachable".into(),
        runtime.0 && relay.0,
        json!({"runtime":runtime.1,"origin_relay":relay.1}),
    ));
    let (closed, kind) = unreachable(SocketAddr::new(origin, ORIGIN_PORT));
    rows.push(row(
        "origin-direct-unreachable".into(),
        closed,
        json!({"observed":kind}),
    ));
    let mut last = None;
    for route in &config.routes {
        let label = format!("{} {}", route.method, route.path);
        let body = if route.method == "GET" {
            Vec::new()
        } else {
            b"{\"lab\":\"deployment\"}".to_vec()
        };
        let unsigned = exchange(
            front,
            &wire(&routed(config, &route.method, &route.path, body.clone())),
        );
        rows.push(row(
            format!("unsigned-refused {label}"),
            unsigned.status == Some(401),
            json!({"status":unsigned.status}),
        ));
        // A fresh session per route keeps every forwarded request on the same
        // issue/collect/sign sequence as the origin/audit stage.
        let issued = exchange(
            front,
            &wire(&routed(config, "POST", "/_humanymous/session", vec![])),
        );
        let session: Value = serde_json::from_slice(&issued.body).unwrap_or(Value::Null);
        let detail = if issued.status == Some(200)
            && session["session"].is_string()
            && session["key"].is_string()
        {
            let mut collect = routed(config, "POST", "/_humanymous/collect", report(&session));
            sign(config, &mut collect, &session, 1);
            let collected = exchange(front, &wire(&collect));
            let mut effect = routed(config, &route.method, &route.path, body);
            sign(config, &mut effect, &session, 2);
            let bytes = wire(&effect);
            let forwarded = exchange(front, &bytes);
            last = Some(bytes);
            json!({"session_status":issued.status,"collect_status":collected.status,
                "status":forwarded.status,"origin_body":forwarded.body == b"origin-ok"})
        } else {
            json!({"session_status":issued.status})
        };
        rows.push(row(
            format!("signed-forwarded {label}"),
            detail["collect_status"] == 200
                && detail["status"] == 200
                && detail["origin_body"] == true,
            detail,
        ));
    }
    let replay = last.map(|bytes| exchange(front, &bytes).status);
    rows.push(row(
        "replay-refused".into(),
        replay == Some(Some(403)),
        json!({"status":replay.flatten()}),
    ));
    rows
}

/// One control line: `{"edge":IP,"origin":IP,"phase":NAME,"configuration":DOC}`.
/// The configuration is the documented example with its custody fields
/// replaced by placeholders: signing binds only the scope and routes.
pub fn run() {
    let report = (|| -> Result<Value> {
        let control = read_line(&mut BufReader::new(io::stdin()), 65_536)?;
        let edge: IpAddr = control["edge"].as_str().ok_or("edge address")?.parse()?;
        let origin: IpAddr = control["origin"]
            .as_str()
            .ok_or("origin address")?
            .parse()?;
        let config: Configuration = serde_json::from_value(control["configuration"].clone())?;
        let rows = execute(&config, edge, origin);
        let expected = expected_rows(&config);
        let exact = rows.len() == expected.len()
            && rows.iter().zip(&expected).all(|(row, id)| row["id"] == **id);
        let passed = exact && rows.iter().all(|row| row["status"] == "passed");
        Ok(
            json!({"schema":CLIENT_SCHEMA,"phase":control["phase"],"status":if passed {"passed"} else {"failed"},
            "cases":rows,"expected_origin_effects":config.routes.len(),"full_product_qualified":false}),
        )
    })()
    .unwrap_or_else(|error| {
        json!({"schema":CLIENT_SCHEMA,"status":"failed","cases":[],"error":error.to_string(),
            "full_product_qualified":false})
    });
    let passed = report["status"] == "passed";
    emit(&report).expect("client report output");
    if !passed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_follow_the_documented_routes_and_authority() {
        let config: Configuration = serde_json::from_value(json!({"listen":"127.0.0.1:8444","origin":"127.0.0.1:8080",
          "scope":{"installation":"example-installation","tenant":"t","origin":"o","revision":"r1","authority":"example.com"},
          "routes":[{"method":"POST","path":"/login","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}},
                    {"method":"GET","path":"/account","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
          "audit":{"directory":"/a","node_id":"n","keys":{"directory":"/k","passphrase":"p"},"allow_new":true,"trusted_head":null},
          "owner":{"kind":"unix","uid":65532},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000}))
        .unwrap();
        let ids = expected_rows(&config);
        assert_eq!(ids.len(), 2 + 2 * 2 + 1);
        assert_eq!(ids[3], "signed-forwarded POST /login");
        assert_eq!(ids[4], "unsigned-refused GET /account");
        let built = routed(&config, "GET", "/account", vec![]);
        assert_eq!(built.method, "GET");
        assert!(
            built
                .headers
                .iter()
                .any(|(n, v)| n == "Host" && v == "example.com")
        );
    }
}
