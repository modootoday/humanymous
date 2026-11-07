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
    let max = if api { 131_072 } else { 32 * 1024 * 1024 };
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
    for key in [
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
    ] {
        receipts.hit(contract[key].as_str().ok_or("owned wire counter absent")?);
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
                    if !matches!(tokio::time::timeout(Duration::from_secs(15),exchange(&mut socket,&config,&receipts)).await,Ok(Ok(()))) {
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
    fn wire_observations_count_only_fixed_classes_and_treat_empty_referer_as_present() {
        let receipts = Receipts::default();
        record_wire(&receipts, Some("http://127.0.0.1:31111"), None).unwrap();
        record_wire(&receipts, Some("null"), Some("PRIVATE_URL")).unwrap();
        record_wire(&receipts, None, Some("")).unwrap();
        assert_eq!(
            serde_json::to_value(receipts.snapshot()).unwrap(),
            serde_json::json!({
                "wire_origin_matched":1,"wire_origin_other":2,"wire_referer_absent":1,"wire_referer_present":2
            })
        );
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
