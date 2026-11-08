//! Desktop-lab TLS listen+client measure for Unlock B W2 Cap.
//!
//! Fail-closed: `original_h2_measured` is true only after a live loopback TLS/H2
//! serve with `required.original_h2=true` clears the observation gate (not 422
//! `required_capability`). Never claims `pass_product_clean_room` or
//! `desktop_as_hosted`. Detection freeze — does not edit R14 freeze pins.
#![cfg(feature = "hosted-pilot")]

use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::{
    application::{Application, Decision},
    configuration::Configuration,
    http::Request,
    service::ShutdownRequest,
    session::canonical,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::integrity::request::{compute_token, seed};
use httlib_hpack::{Decoder, Encoder};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream as NativeStream},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;

const BUDGET: Duration = Duration::from_secs(5);
const STREAM: u32 = 1;
const FLAG_END_STREAM: u8 = 0x1;
const FLAG_END_HEADERS: u8 = 0x4;
const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const RECEIPT_SCHEMA: &str = "humanymous.lab-h2-product-binding-measure-receipt/1";

#[derive(Default)]
struct MemoryAudit;

impl DurableAudit for MemoryAudit {
    fn append(&mut self, _record: Record) -> std::io::Result<()> {
        Ok(())
    }
}

/// Cap oneshot entry: print one JSON receipt line and exit 0/1.
pub fn run() -> i32 {
    match measure() {
        Ok(receipt) => {
            let measured = receipt
                .get("original_h2_measured")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            println!("{}", serde_json::to_string(&receipt).unwrap_or_else(|_| {
                json!({
                    "schema": RECEIPT_SCHEMA,
                    "original_h2_measured": false,
                    "product": 0,
                    "pass_product_clean_room": 0,
                    "desktop_as_hosted": false,
                    "error": "receipt_serialize"
                })
                .to_string()
            }));
            if measured { 0 } else { 1 }
        }
        Err(error) => {
            println!(
                "{}",
                json!({
                    "schema": RECEIPT_SCHEMA,
                    "original_h2_measured": false,
                    "product": 0,
                    "pass_product_clean_room": 0,
                    "desktop_as_hosted": false,
                    "venue": "desktop_lab_loopback",
                    "error": error,
                    "note": "fail-closed measure; never invent pass_product_clean_room or desktop_as_hosted"
                })
            );
            1
        }
    }
}

fn measure() -> Result<Value, String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("runtime: {error}"))?;
    runtime.block_on(measure_async())
}

async fn measure_async() -> Result<Value, String> {
    let origin = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("origin bind: {error}"))?;
    let origin_addr = origin.local_addr().map_err(|e| e.to_string())?;
    let mut control_config = configuration(origin_addr);
    control_config.routes[0].required.original_h2 = true;

    let mut control_app =
        Application::new(control_config, Default::default(), Box::new(MemoryAudit))
            .map_err(|error| format!("application: {}", error.code))?;
    let now = crate::authority::now_ms().map_err(|error| format!("clock: {}", error.code))?;
    let control_issued = issue(&mut control_app, now)?;
    let mut control_cfg = configuration(origin_addr);
    control_cfg.routes[0].required.original_h2 = true;

    // Control: plaintext decide must stay 422 while original_h2 is required.
    let control = protected(&control_cfg, &control_issued, 1, now);
    let control_status = match control_app.decide(&control, || Ok(now)) {
        Err(rejection) => rejection.status,
        Ok(_) => {
            return Err("plaintext decide cleared original_h2 without TLS observation".into());
        }
    };
    if control_status != 422 {
        return Err(format!(
            "plaintext control expected 422, got {control_status}"
        ));
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("listen: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;
    let (server_tls, client_tls) = tls_configs()?;
    let (stop, stopping) = tokio::sync::watch::channel(None);
    let mut server_config = configuration(origin_addr);
    server_config.routes[0].required.original_h2 = true;
    let mut server_app =
        Application::new(server_config, Default::default(), Box::new(MemoryAudit))
            .map_err(|error| format!("server application: {}", error.code))?;
    let issued = issue(&mut server_app, now)?;
    let mut serve_cfg = configuration(origin_addr);
    serve_cfg.routes[0].required.original_h2 = true;
    let request = protected(&serve_cfg, &issued, 1, now);
    let server = tokio::spawn(async move {
        server_app
            .serve_tls_until(listener, server_tls, stopping)
            .await
    });
    let wire = encode_h2_request(&request)?;
    let response = exchange_h2(address, client_tls, wire).await?;
    let status = decode_h2_status(&response)?;
    let _ = stop.send(Some(
        ShutdownRequest::new(200).map_err(|error| format!("shutdown: {error}"))?,
    ));
    let _ = tokio::time::timeout(BUDGET, server).await;

    let measured = status != 422;
    Ok(json!({
        "schema": RECEIPT_SCHEMA,
        "kind": "w2-h2-product-binding-measure",
        "original_h2_measured": measured,
        "required_original_h2": true,
        "plaintext_control_status": control_status,
        "tls_h2_status": status,
        "venue": "desktop_lab_loopback",
        "desktop_as_hosted": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "network": "loopback_network_mode_none",
        "note": if measured {
            "Desktop lab TLS listen+client measure cleared original_h2 observation gate; not hosted product; N06 readiness/owner join still Root"
        } else {
            "Desktop lab TLS measure ran but original_h2 gate still 422; keep original_h2_measured=false"
        }
    }))
}

fn configuration(origin: SocketAddr) -> Configuration {
    serde_json::from_value(json!({
        "listen":"127.0.0.1:0",
        "origin": origin.to_string(),
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":"unused","node_id":"h2-product-binding-lab","keys":{"directory":"unused-keys","passphrase":"lab-fixture"},"allow_new":true,"trusted_head":null},
        "owner": if cfg!(windows) {
            json!({"kind":"windows","sid":"S-1-5-21-1","pid":1})
        } else {
            json!({"kind":"unix","uid":0})
        },
        "session_capacity":16,
        "session_ttl_ms":60_000,
        "timeout_ms":5000,
        "rate_per_second":100
    }))
    .expect("lab measure configuration")
}

fn tls_configs() -> Result<(Arc<ServerConfig>, Arc<ClientConfig>), String> {
    let certified = rcgen::generate_simple_self_signed(vec!["owned.test".into()])
        .map_err(|error| format!("rcgen: {error}"))?;
    let certificate = certified.cert.der().clone();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let mut server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())
        .map_err(|error| format!("server cert: {error}"))?;
    server.alpn_protocols = vec![b"h2".to_vec()];
    let mut roots = RootCertStore::empty();
    roots
        .add(certificate)
        .map_err(|error| format!("root: {error}"))?;
    let mut client = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"h2".to_vec()];
    Ok((Arc::new(server), Arc::new(client)))
}

fn request(method: &str, target: &str, body: Vec<u8>) -> Request {
    Request {
        method: method.into(),
        target: target.into(),
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

fn sign(config: &Configuration, req: &mut Request, id: &str, key: &str, n: u64, now: u64) {
    let deadline = now + 900;
    req.headers.retain(|(name, _)| {
        !name.starts_with("x-humanymous-") && !name.eq_ignore_ascii_case("authorization")
    });
    req.headers
        .push(("Authorization".into(), format!("Bearer {id}")));
    req.headers
        .push(("x-humanymous-counter".into(), n.to_string()));
    req.headers
        .push(("x-humanymous-bucket".into(), (now / 10_000).to_string()));
    req.headers
        .push(("x-humanymous-deadline".into(), deadline.to_string()));
    let text = canonical(config, req, id, deadline);
    let key = URL_SAFE_NO_PAD.decode(key).expect("session key");
    let token = compute_token(&seed(&key, n - 1), &text, n, now / 10_000);
    req.headers.push(("x-humanymous-token".into(), token));
}

fn issue(app: &mut Application, now: u64) -> Result<Value, String> {
    match app
        .decide(&request("POST", "/_humanymous/session", vec![]), || Ok(now))
        .map_err(|error| format!("issue: {}", error.code))?
    {
        Decision::Local(200, bytes) => {
            serde_json::from_slice(&bytes).map_err(|error| format!("issue json: {error}"))
        }
        Decision::Local(status, _) => Err(format!("issue status {status}")),
        Decision::Origin { .. } => Err("issue unexpected origin".into()),
    }
}

fn protected(cfg: &Configuration, issued: &Value, n: u64, now: u64) -> Request {
    let mut req = request("POST", "/effect", b"{}".to_vec());
    sign(
        cfg,
        &mut req,
        issued["session"].as_str().expect("session"),
        issued["key"].as_str().expect("key"),
        n,
        now,
    );
    req
}

fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + payload.len());
    let len = payload.len() as u32;
    out.push((len >> 16) as u8);
    out.push((len >> 8) as u8);
    out.push(len as u8);
    out.push(kind);
    out.push(flags);
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn encode_h2_request(req: &Request) -> Result<Vec<u8>, String> {
    let mut encoder = Encoder::default();
    let flags = Encoder::NEVER_INDEXED;
    let mut block = Vec::new();
    let authority = req
        .header("host")
        .ok_or_else(|| "host missing".to_string())?;
    for (name, value) in [
        (":method", req.method.as_str()),
        (":scheme", "https"),
        (":authority", authority),
        (":path", req.target.as_str()),
    ] {
        encoder
            .encode(
                (name.as_bytes().to_vec(), value.as_bytes().to_vec(), flags),
                &mut block,
            )
            .map_err(|error| format!("hpack: {error:?}"))?;
    }
    for (name, value) in &req.headers {
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        encoder
            .encode(
                (
                    name.to_ascii_lowercase().into_bytes(),
                    value.as_bytes().to_vec(),
                    flags,
                ),
                &mut block,
            )
            .map_err(|error| format!("hpack header: {error:?}"))?;
    }
    if !req.body.is_empty() {
        encoder
            .encode(
                (
                    b"content-length".to_vec(),
                    req.body.len().to_string().into_bytes(),
                    flags,
                ),
                &mut block,
            )
            .map_err(|error| format!("hpack cl: {error:?}"))?;
    }
    let mut wire = PREFACE.to_vec();
    wire.extend(frame(4, 0, 0, &[0, 1, 0, 0, 0, 0]));
    if req.body.is_empty() {
        wire.extend(frame(1, FLAG_END_HEADERS | FLAG_END_STREAM, STREAM, &block));
    } else {
        wire.extend(frame(1, FLAG_END_HEADERS, STREAM, &block));
        wire.extend(frame(0, FLAG_END_STREAM, STREAM, &req.body));
    }
    Ok(wire)
}

async fn exchange_h2(
    address: SocketAddr,
    config: Arc<ClientConfig>,
    bytes: Vec<u8>,
) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        let socket = NativeStream::connect_timeout(&address, BUDGET)
            .map_err(|error| format!("connect: {error}"))?;
        socket
            .set_read_timeout(Some(BUDGET))
            .map_err(|error| format!("read timeout: {error}"))?;
        socket
            .set_write_timeout(Some(BUDGET))
            .map_err(|error| format!("write timeout: {error}"))?;
        let mut socket = rustls::StreamOwned::new(
            ClientConnection::new(
                config,
                rustls::pki_types::ServerName::try_from("owned.test")
                    .map_err(|error| format!("server name: {error}"))?,
            )
            .map_err(|error| format!("client hello: {error}"))?,
            socket,
        );
        socket
            .write_all(&bytes)
            .map_err(|error| format!("write: {error}"))?;
        socket.flush().map_err(|error| format!("flush: {error}"))?;
        if socket.conn.alpn_protocol() != Some(b"h2".as_slice()) {
            return Err(format!(
                "negotiated alpn {:?}",
                socket.conn.alpn_protocol()
            ));
        }
        let mut response = Vec::new();
        let mut scratch = [0u8; 16_384];
        loop {
            match socket.read(&mut scratch) {
                Ok(0) => break,
                Ok(count) => {
                    response.extend_from_slice(&scratch[..count]);
                    if response.len() > 65_536 {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => break,
                Err(error) => return Err(format!("read: {error}")),
            }
        }
        if response.is_empty() {
            return Err("empty TLS/H2 response".into());
        }
        Ok(response)
    })
    .await
    .map_err(|error| format!("join: {error}"))?
}

fn decode_h2_status(frames: &[u8]) -> Result<u16, String> {
    let mut offset = 0;
    while offset + 9 <= frames.len() {
        let len = ((frames[offset] as usize) << 16)
            | ((frames[offset + 1] as usize) << 8)
            | frames[offset + 2] as usize;
        let kind = frames[offset + 3];
        let start = offset + 9;
        let end = start
            .checked_add(len)
            .filter(|value| *value <= frames.len())
            .ok_or_else(|| "truncated H2 frame".to_string())?;
        if kind == 1 {
            let mut buf = frames[start..end].to_vec();
            let mut decoder = Decoder::default();
            let mut decoded = Vec::new();
            decoder
                .decode(&mut buf, &mut decoded)
                .map_err(|error| format!("hpack decode: {error:?}"))?;
            for (name, value, _) in decoded {
                if name == b":status" {
                    let text = std::str::from_utf8(&value)
                        .map_err(|error| format!("status utf8: {error}"))?;
                    return text
                        .parse()
                        .map_err(|error| format!("status parse: {error}"));
                }
            }
        }
        offset = end;
    }
    Err("H2 :status missing".into())
}
