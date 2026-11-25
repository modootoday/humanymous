//! Owned recording origin: every contact is recorded before parsing, and each
//! parsed request is echoed back exactly as the origin observed it.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

const LIMIT: usize = 70_000;
type Records = Arc<Mutex<Vec<Value>>>;

fn head_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Parse one origin-form request; `None` keeps the contact as malformed.
fn parse(bytes: &[u8]) -> Option<(Value, usize)> {
    let end = head_end(bytes)?;
    let head = std::str::from_utf8(&bytes[..end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut start = lines.next()?.split(' ');
    let (method, target, version) = (start.next()?, start.next()?, start.next()?);
    let mut headers = Vec::new();
    let mut length = 0;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let value = value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().ok()?;
        }
        headers.push(json!([name, value]));
    }
    let record = json!({"method":method,"target":target,"version":version,"headers":headers});
    Some((record, end + 4 + length))
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut part = [0; 4096];
    loop {
        if let Some((_, total)) = parse(&bytes)
            && bytes.len() >= total
        {
            break;
        }
        match stream.read(&mut part) {
            Ok(0) | Err(_) => break,
            Ok(n) => bytes.extend_from_slice(&part[..n]),
        }
        if bytes.len() > LIMIT {
            break;
        }
    }
    bytes
}

fn respond(stream: &mut TcpStream, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn application(stream: &mut TcpStream, records: &Records) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let bytes = read_request(stream);
    let mut list = records.lock().expect("origin record lock");
    let sequence = list.len() + 1;
    let record = match parse(&bytes) {
        Some((mut record, total)) if bytes.len() == total => {
            let body = &bytes[head_end(&bytes).unwrap_or(0) + 4..total];
            record["sequence"] = json!(sequence);
            record["body_length"] = json!(body.len());
            record["body_sha256"] = json!(hex::encode(Sha256::digest(body)));
            record
        }
        _ => json!({"sequence":sequence,"malformed":true,"bytes":bytes.len()}),
    };
    list.push(record.clone());
    drop(list);
    respond(
        stream,
        &serde_json::to_vec(&json!({"origin_record":record})).unwrap_or_default(),
    );
}

/// Serve the application origin and a separate lab observation endpoint.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    serve(super::OBSERVE)
}

/// Profile `tls`: the same origin, with only the read-only observation
/// endpoint bound beside the public Gate so a driver in its own namespace can
/// read the records. The application origin stays loopback-only.
pub fn run_public_observe() -> Result<(), Box<dyn std::error::Error>> {
    serve(super::PUBLIC_OBSERVE_LISTEN)
}

fn serve(observe: &str) -> Result<(), Box<dyn std::error::Error>> {
    let records: Records = Arc::new(Mutex::new(Vec::new()));
    let application_listener = TcpListener::bind(super::ORIGIN)?;
    let observe_listener = TcpListener::bind(observe)?;
    let observed = records.clone();
    let observer = std::thread::spawn(move || {
        for stream in observe_listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = read_request(&mut stream);
            let snapshot = observed.lock().expect("origin record lock").clone();
            respond(
                &mut stream,
                &serde_json::to_vec(&json!({"records":snapshot})).unwrap_or_default(),
            );
        }
    });
    eprintln!("product-e2e origin listening on {}", super::ORIGIN);
    for stream in application_listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        application(&mut stream, &records);
    }
    observer.join().map_err(|_| "observer thread")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_keeps_order_casing_and_body_bounds() {
        let frame =
            b"POST /effect HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 2\r\nX-Lab: a\r\n\r\nok";
        let (record, total) = super::parse(frame).unwrap();
        assert_eq!(total, frame.len());
        assert_eq!(record["headers"][2], serde_json::json!(["X-Lab", "a"]));
        assert!(super::parse(b"GET / HTTP/1.1\r\nbad\r\n\r\n").is_none());
    }
}
