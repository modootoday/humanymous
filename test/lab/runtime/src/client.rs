use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_integrity::request::{compute_token, seed};
use humanymous_runtime::{configuration::Configuration, http::Request, session::canonical};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn request(path: &str, body: Vec<u8>) -> Request {
    Request {
        method: "POST".into(),
        target: path.into(),
        headers: vec![
            ("Host".into(), "owned.test".into()),
            ("Content-Length".into(), body.len().to_string()),
            ("Content-Type".into(), "application/json".into()),
            ("sec-ch-ua".into(), "Chromium".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 Chrome/126.0 Safari/537.36".into(),
            ),
            ("Accept".into(), "*/*".into()),
            ("Sec-Fetch-Site".into(), "same-origin".into()),
            ("Accept-Encoding".into(), "gzip, deflate, br, zstd".into()),
        ],
        body,
    }
}
pub fn sign(config: &Configuration, req: &mut Request, issued: &Value, counter: u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let deadline = now + 4000;
    let id = issued["session"].as_str().unwrap();
    req.headers.extend([
        ("Authorization".into(), format!("Bearer {id}")),
        ("x-humanymous-counter".into(), counter.to_string()),
        ("x-humanymous-bucket".into(), (now / 10_000).to_string()),
        ("x-humanymous-deadline".into(), deadline.to_string()),
    ]);
    let key = URL_SAFE_NO_PAD
        .decode(issued["key"].as_str().unwrap())
        .unwrap();
    let text = canonical(config, req, id, deadline);
    req.headers.push((
        "x-humanymous-token".into(),
        compute_token(&seed(&key, counter - 1), &text, counter, now / 10_000),
    ));
}
pub fn wire(req: &Request) -> Vec<u8> {
    let mut bytes = format!("{} {} HTTP/1.1\r\n", req.method, req.target).into_bytes();
    for (name, value) in &req.headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&req.body);
    bytes
}
pub fn exchange(address: SocketAddr, bytes: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(bytes).unwrap();
    let mut result = Vec::new();
    stream.read_to_end(&mut result).unwrap();
    result
}
pub fn expect(address: SocketAddr, req: &Request, status: u16) -> Vec<u8> {
    let result = exchange(address, &wire(req));
    assert!(
        result.starts_with(format!("HTTP/1.1 {status} ").as_bytes()),
        "unexpected HTTP status"
    );
    result
}
pub fn issue(address: SocketAddr) -> Value {
    let bytes = expect(address, &request("/_humanymous/session", vec![]), 200);
    let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap() + 4;
    serde_json::from_slice(&bytes[end..]).unwrap()
}
pub fn report(issued: &Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"sessionId":issued["session"],
        "userAgent":"Mozilla/5.0 Chrome/126.0 Safari/537.36","environment":{"probed":true},
        "behavior":{},"uaClientHints":{"brands":[{"brand":"Chromium","version":"126"}],"platform":"Windows"},
        "signals":[{"id":"l1.navigator.webdriver","value":false,"verdict":"OK","confidence":1,"collected":"wasm"}]})).unwrap()
}
