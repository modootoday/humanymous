//! Owned HTTP pilot forwarding. No TLS/raw browser-header provenance claim.
use humanymous_runtime::gateway::http;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, watch},
    task::JoinSet,
};

pub fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../crates/collector/pilot/fixture.json"
    ))
    .expect("owned browser fixture schema")
}
fn authority() -> String {
    fixture()["scope"]["authority"]
        .as_str()
        .expect("fixture authority")
        .into()
}
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Browser asset response ceiling (headers + body).
///
/// Measured Cap=1 attempt-3 image artifact
/// `/usr/local/lib/collector-product-tests.wasm` = 40_915_031 bytes. The prior
/// 32 MiB ceiling returned `proxy response bound` → HTTP 503 on
/// `wasm-bindgen-test_bg.wasm`, so the selected runtime_pilot test never ran
/// and `browser-actual-witness-exchange` stayed at bootstrap transactions=1.
const ASSET_RESPONSE_MAX: u64 = 64 * 1024 * 1024;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    pub listen: SocketAddr,
    pub upstream: SocketAddr,
    /// Browser F accepts absolute-form and serves assets from its own S.
    pub browser: bool,
}
#[derive(Clone, Default)]
pub struct Receipts(Arc<Mutex<BTreeMap<String, u64>>>);
impl Receipts {
    fn hit(&self, key: &str) {
        if let Ok(mut counts) = self.0.lock() {
            *counts.entry(key.into()).or_default() += 1;
        }
    }
    pub fn snapshot(&self) -> BTreeMap<String, u64> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

fn destination(method: &str, target: &str, browser: bool) -> Result<(String, bool)> {
    let fixture = fixture();
    let authority = authority();
    let target = if browser {
        target
            .strip_prefix(&format!("http://{authority}"))
            .filter(|t| t.starts_with('/') && !t.starts_with("//"))
            .ok_or("foreign proxy authority")?
    } else {
        target
    };
    let path = target.split('?').next().ok_or("missing target")?;
    if fixture["routes"]
        .as_object()
        .ok_or("fixture routes")?
        .values()
        .any(|value| value.as_str() == Some(path))
    {
        if method != "POST" || target != path {
            return Err("API method/query refused".into());
        }
        Ok((target.into(), true))
    } else if browser && matches!(method, "GET" | "HEAD") && !path.starts_with("/_humanymous/") {
        Ok((target.into(), false))
    } else {
        Err("unknown API path".into())
    }
}

async fn request(socket: &mut TcpStream, browser: bool) -> Result<(http::Request, bool)> {
    let mut bytes = Vec::new();
    let end = loop {
        if let Some(end) = bytes.windows(4).position(|p| p == b"\r\n\r\n") {
            break end;
        }
        if bytes.len() >= 16_384 {
            return Err("proxy header bound".into());
        }
        let mut chunk = [0; 1024];
        let n = socket.read(&mut chunk).await?;
        if n == 0 {
            return Err("truncated proxy header".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
    };
    if end + 4 > 16_384 {
        return Err("proxy header bound".into());
    }
    let text = std::str::from_utf8(&bytes[..end])?;
    let mut lines = text.split("\r\n");
    let first = lines.next().ok_or("missing proxy start")?;
    let parts: Vec<_> = first.split(' ').collect();
    if parts.len() != 3 || parts[2] != "HTTP/1.1" {
        return Err("proxy start".into());
    }
    let (target, api) = destination(parts[0], parts[1], browser)?;
    let mut head = format!("{} {target} HTTP/1.1\r\n", parts[0]);
    let mut seen = std::collections::HashSet::new();
    let mut length = false;
    let mut host = false;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("proxy header")?;
        let lower = name.to_ascii_lowercase();
        if !seen.insert(lower.clone()) {
            return Err("duplicate proxy header".into());
        }
        if lower == "host" {
            if value.trim() != authority() {
                return Err("proxy Host mismatch".into());
            }
            host = true;
        }
        if lower == "content-length" {
            length = true;
        }
        if matches!(
            lower.as_str(),
            "transfer-encoding" | "upgrade" | "te" | "trailer" | "expect"
        ) {
            return Err("proxy framing refused".into());
        }
        if lower == "connection" {
            if !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "close" | "keep-alive"
            ) {
                return Err("proxy connection tokens".into());
            }
            continue;
        }
        if matches!(
            lower.as_str(),
            "proxy-connection" | "proxy-authorization" | "proxy-authenticate" | "keep-alive"
        ) {
            continue;
        }
        head.push_str(line);
        head.push_str("\r\n");
    }
    if !host {
        return Err("proxy Host missing".into());
    }
    // GET assets have no signed entity. Production browser POST framing is not repaired.
    if !length && !api {
        head.push_str("Content-Length: 0\r\n");
    }
    head.push_str("Connection: close\r\n\r\n");
    let mut prefix = head.into_bytes();
    prefix.extend_from_slice(&bytes[end + 4..]);
    let mut input = std::io::Cursor::new(prefix).chain(socket);
    let frame = http::read(&mut input)
        .await
        .map_err(|_| "proxy runtime framing")?;
    Ok((frame, api))
}

fn encode(request: &http::Request) -> Vec<u8> {
    let mut bytes = format!("{} {} HTTP/1.1\r\n", request.method, request.target).into_bytes();
    for (name, value) in &request.headers {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(b": ");
        bytes.extend_from_slice(value.as_bytes());
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    bytes
}

async fn exchange(socket: &mut TcpStream, config: &ProxyConfig, receipts: &Receipts) -> Result<()> {
    let (frame, api) = request(socket, config.browser).await?;
    if api {
        record_wire(receipts, frame.header("origin"), frame.header("referer"))?;
    }
    let upstream = if api {
        config.upstream
    } else {
        fixture()["runner_listen"]
            .as_str()
            .ok_or("fixture listen")?
            .parse()?
    };
    receipts.hit(if api { &frame.target } else { "assets" });
    let mut remote = TcpStream::connect(upstream).await?;
    remote.write_all(&encode(&frame)).await?;
    // No retry, redirect following, status rewriting or synthesized ACK.
    let max = if api { 131_072 } else { ASSET_RESPONSE_MAX };
    let mut reply = Vec::new();
    (&mut remote).take(max + 1).read_to_end(&mut reply).await?;
    if reply.len() as u64 > max {
        return Err("proxy response bound".into());
    }
    let end = reply
        .windows(4)
        .position(|p| p == b"\r\n\r\n")
        .ok_or("response headers")?;
    let header = std::str::from_utf8(&reply[..end])?;
    let status: u16 = header
        .split("\r\n")
        .next()
        .and_then(|l| l.split(' ').nth(1))
        .ok_or("response status")?
        .parse()?;
    if (300..400).contains(&status) {
        return Err("proxy redirect refused".into());
    }
    if api {
        http::response(&reply, false).map_err(|_| "proxy response framing")?;
    }
    if api {
        let ack = header
            .split("\r\n")
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .any(|(name, _)| name.eq_ignore_ascii_case("x-humanymous-ack"));
        receipts.hit(&format!("response:{}:{status}:ack_{ack}", frame.target));
    }
    socket.write_all(&reply).await?;
    socket.shutdown().await?;
    receipts.hit("completed");
    Ok(())
}

fn record_wire(receipts: &Receipts, origin: Option<&str>, referer: Option<&str>) -> Result<()> {
    let fixture = fixture();
    let expected = fixture["browser_origin"]
        .as_str()
        .ok_or("owned browser origin absent")?;
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../../browser-wire-contract.json"))?;
    let mut keys = vec![
        if origin == Some(expected) {
            "origin_matched"
        } else {
            "origin_other"
        },
        if referer.is_none() {
            "referer_absent"
        } else {
            "referer_present"
        },
    ];
    match origin {
        None => keys.push("origin_absent"),
        Some("null") => keys.push("origin_null"),
        _ => {}
    }
    if let Some(referer) = referer {
        keys.push(if referer == format!("{expected}/") {
            "referer_root_matched"
        } else {
            "referer_other"
        });
    }
    let mut counts = receipts
        .0
        .lock()
        .map_err(|_| "wire counter lock poisoned")?;
    for key in [
        "origin_matched",
        "origin_other",
        "origin_absent",
        "origin_null",
        "referer_absent",
        "referer_present",
        "referer_root_matched",
        "referer_other",
    ] {
        let name = contract[key].as_str().ok_or("owned wire counter absent")?;
        counts.entry(name.into()).or_default();
    }
    for key in keys {
        let name = contract[key].as_str().ok_or("owned wire counter absent")?;
        let count = counts.entry(name.into()).or_default();
        *count = count.checked_add(1).ok_or("wire counter overflow")?;
    }
    Ok(())
}

pub async fn serve(
    listener: TcpListener,
    config: ProxyConfig,
    mut stop: watch::Receiver<bool>,
    receipts: Receipts,
) -> io::Result<()> {
    if listener.local_addr()? != config.listen
        || config.browser && !config.listen.ip().is_loopback()
        || !config.browser && !config.upstream.ip().is_loopback()
    {
        return Err(io::Error::other("proxy listener/upstream ownership"));
    }
    let limit = Arc::new(Semaphore::new(16));
    let mut tasks = JoinSet::new();
    // Match browser-product.Dockerfile WASM_BINDGEN_TEST_TIMEOUT so a full
    // ASSET_RESPONSE_MAX debug-wasm transfer can finish under Cap=1 Desktop.
    let exchange_timeout = Duration::from_secs(90);
    loop {
        tokio::select! {
            changed=stop.changed()=>{if changed.is_err() || *stop.borrow(){break;}}
            joined=tasks.join_next(),if !tasks.is_empty()=>{if joined.is_some_and(|r|r.is_err()){return Err(io::Error::other("proxy task failed"));}}
            accepted=listener.accept()=>{
                let (mut socket,_)=accepted?;
                let Ok(permit)=limit.clone().try_acquire_owned() else {receipts.hit("capacity_refused");continue;};
                let config=config.clone();let receipts=receipts.clone();
                tasks.spawn(async move {
                    let _permit=permit;
                    if !matches!(tokio::time::timeout(exchange_timeout,exchange(&mut socket,&config,&receipts)).await,Ok(Ok(()))) {
                        receipts.hit("refused_or_failed");
                        let _=http::write(&mut socket,503,b"{}").await;
                    }
                });
            }
        }
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asset_response_max_covers_measured_debug_collector_wasm() {
        // Attempt-3 Desktop image: humanymous/browser-product:hm-rust-lab-f87186e0695e4771
        // Runtime operand keeps this from folding into clippy::assertions_on_constants
        // (attempt-4 browser_gate_build -D warnings failure on a const-only assert!).
        let measured_attempt3_wasm_bytes = std::hint::black_box(40_915_031u64);
        let required = measured_attempt3_wasm_bytes.saturating_add(65_536);
        assert!(
            ASSET_RESPONSE_MAX > required,
            "ASSET_RESPONSE_MAX ({ASSET_RESPONSE_MAX}) must exceed measured debug wasm + 64KiB margin ({required})"
        );
    }
    #[test]
    fn origin_classes_initialize_zeroes_and_never_retain_foreign_values() {
        for (origin, expected) in [
            (None, "wire_origin_absent"),
            (Some("null"), "wire_origin_null"),
            (Some("http://127.0.0.1:31111"), "wire_origin_matched"),
            (Some(""), "wire_origin_other"),
            (Some("NULL"), "wire_origin_other"),
            (Some("PRIVATE_VALUE"), "wire_origin_other"),
        ] {
            let receipts = Receipts::default();
            record_wire(&receipts, origin, None).unwrap();
            let counts = receipts.snapshot();
            for key in [
                "wire_origin_matched",
                "wire_origin_other",
                "wire_origin_absent",
                "wire_origin_null",
            ] {
                assert!(counts.contains_key(key));
            }
            assert_eq!(counts[expected], 1);
            assert_eq!(
                counts["wire_origin_matched"] + counts["wire_origin_other"],
                1
            );
            assert_eq!(counts["wire_origin_absent"], u64::from(origin.is_none()));
            assert_eq!(
                counts["wire_origin_null"],
                u64::from(origin == Some("null"))
            );
            assert!(!serde_json::to_string(&counts).unwrap().contains("PRIVATE"));
        }
    }
    #[test]
    fn wire_observations_count_only_fixed_classes_and_treat_empty_referer_as_present() {
        let receipts = Receipts::default();
        record_wire(&receipts, Some("http://127.0.0.1:31111"), None).unwrap();
        record_wire(&receipts, Some("null"), Some("PRIVATE_URL")).unwrap();
        record_wire(&receipts, None, Some("")).unwrap();
        assert_eq!(
            serde_json::to_value(receipts.snapshot()).unwrap(),
            serde_json::json!({
                "wire_origin_matched":1,"wire_origin_other":2,"wire_origin_absent":1,
                "wire_origin_null":1,"wire_referer_absent":1,"wire_referer_present":2,
                "wire_referer_root_matched":0,"wire_referer_other":2
            })
        );
    }
    #[test]
    fn only_exact_owned_root_is_classified_as_private() {
        for value in [
            "http://127.0.0.1:31111/",
            "http://127.0.0.1:31111/private",
            "http://127.0.0.1:31111/?private",
            "http://127.0.0.1:31111/#private",
            "http://user@127.0.0.1:31111/",
            "http://foreign/",
            "",
            " http://127.0.0.1:31111/",
        ] {
            let receipts = Receipts::default();
            record_wire(&receipts, None, Some(value)).unwrap();
            let counts = receipts.snapshot();
            let matched = u64::from(value == "http://127.0.0.1:31111/");
            assert_eq!(counts["wire_referer_root_matched"], matched);
            assert_eq!(counts["wire_referer_other"], 1 - matched);
            assert_eq!(counts["wire_referer_present"], 1);
        }
    }
    #[test]
    fn owned_authority_and_api_dispatch_cannot_be_retargeted() {
        assert_eq!(
            destination("POST", "http://127.0.0.1:31111/effect", true).unwrap(),
            ("/effect".into(), true)
        );
        for target in [
            "http://127.0.0.1:31111.evil/effect",
            "http://u@127.0.0.1:31111/effect",
            "http://127.0.0.1:31112/effect",
            "http://169.254.169.254/effect",
            "/effect",
        ] {
            assert!(destination("POST", target, true).is_err());
        }
        assert!(destination("CONNECT", "http://127.0.0.1:31111/effect", true).is_err());
        assert!(destination("POST", "/effect?alias", false).is_err());
        assert!(destination("GET", "/", false).is_err());
    }
}
