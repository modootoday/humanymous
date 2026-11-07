//! Bounded, Rust-authored in-container network controls. No product impersonation.
use crate::docker::Result;
mod dns;
#[path = "../../dns_wire.rs"]
mod dns_wire;
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};

pub fn owned_ipv4(text: &str) -> Result<Ipv4Addr> {
    let address: Ipv4Addr = text.parse()?;
    if !address.is_private() {
        return Err("only inspected RFC1918 Docker targets are allowed".into());
    }
    Ok(address)
}
fn reachable(address: SocketAddr) -> bool {
    TcpStream::connect_timeout(&address, Duration::from_millis(800)).is_ok()
}
fn control_request(stream: &mut TcpStream) -> std::io::Result<bool> {
    let mut bytes = Vec::new();
    while bytes.len() < 4096 {
        let mut buffer = [0; 512];
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Ok(false);
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            return Ok(bytes.starts_with(b"GET / HTTP/1.1\r\n"));
        }
    }
    Ok(false)
}
pub fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("dns-server") => dns::serve(),
        Some("dns-positive") => dns::positive(),
        Some("dns-snapshot") => dns::snapshot(),
        Some("serve") => serve(args.get(1).ok_or("fixture label required")?),
        Some("health") => health("127.0.0.1"),
        Some("control") => health("origin"),
        Some("probe") if args.len() == 4 => probe(
            owned_ipv4(&args[1])?,
            owned_ipv4(&args[2])?,
            owned_ipv4(&args[3])?,
        ),
        _ => Err("fixture role/owned target arguments invalid".into()),
    }
}
fn serve(label: &str) -> Result<()> {
    if !matches!(label, "origin" | "outside-canary") {
        return Err("unknown owned fixture".into());
    }
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        if !control_request(&mut stream).unwrap_or(false) {
            continue;
        }
        let body = format!("{{\"fixture\":\"{label}\"}}");
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes());
    }
    Ok(())
}
fn health(host: &str) -> Result<()> {
    // host is a literal role selected by the caller; no URL/redirect option.
    let address = (host, 8080)
        .to_socket_addrs()?
        .next()
        .ok_or("fixture DNS unavailable")?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: owned.invalid\r\nConnection: close\r\n\r\n")?;
    let mut response = Vec::new();
    stream.take(4096).read_to_end(&mut response)?;
    if !response.starts_with(b"HTTP/1.1 200 ") {
        return Err("fixture positive control refused".into());
    }
    println!("{}", json!({"status":"passed","fixture":host}));
    Ok(())
}
fn probe(origin: Ipv4Addr, outside: Ipv4Addr, gateway: Ipv4Addr) -> Result<()> {
    let mut cases = Vec::new();
    for (id, address, port) in [
        ("no-origin-bypass-direct-ip", origin, 8080),
        ("no-outside-bridge-direct-ip", outside, 8080),
        ("no-host-docker-api", gateway, 2375),
        ("no-host-docker-tls", gateway, 2376),
        ("no-metadata-http", Ipv4Addr::new(169, 254, 169, 254), 80),
    ] {
        cases.push(json!({"id":id,"status":if reachable(SocketAddr::new(IpAddr::V4(address),port)){"failed"}else{"passed"},"detail":{"address":address.to_string(),"port":port}}));
    }
    for host in ["origin", "outside-canary"] {
        cases.push(json!({"id":format!("no-private-dns-{host}"),"status":if (host,8080).to_socket_addrs().is_err(){"passed"}else{"failed"},"detail":"paired direct-IP denial"}));
    }
    let status = fs::read_to_string("/proc/self/status")?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap_or("")
            .trim()
    };
    for (id, passed) in [
        (
            "nonroot-client",
            field("Uid:").split_whitespace().next() == Some("65532"),
        ),
        ("capabilities-empty", field("CapEff:") == "0000000000000000"),
        ("no-new-privileges", field("NoNewPrivs:") == "1"),
        (
            "no-container-api-mount",
            !std::path::Path::new("/var/run/docker.sock").exists(),
        ),
        (
            "no-default-ipv4-route",
            !fs::read_to_string("/proc/net/route")?
                .lines()
                .skip(1)
                .any(|line| line.split_whitespace().nth(1) == Some("00000000")),
        ),
    ] {
        cases.push(json!({"id":id,"status":if passed{"passed"}else{"failed"},"detail":"observed inside isolated client"}));
    }
    let root_write = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("/rootfs-write-probe");
    let readonly = root_write
        .as_ref()
        .err()
        .is_some_and(|error| error.raw_os_error() == Some(30));
    cases.push(json!({"id":"read-only-rootfs","status":if readonly{"passed"}else{"failed"},"detail":"requires EROFS, not generic permission failure"}));
    let passed = cases.iter().all(|case| case["status"] == "passed");
    println!(
        "{}",
        json!({"schema":"humanymous.topology-controls/2","evidence_type":"topology-control","cases":cases})
    );
    if !passed {
        return Err("one or more topology controls failed".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_arbitrary_target_input() {
        for input in [
            "1.1.1.1",
            "127.0.0.1",
            "169.254.169.254",
            "192.0.2.1",
            "0.0.0.0",
            "::1",
            "https://example.com",
        ] {
            assert!(owned_ipv4(input).is_err());
        }
        assert!(owned_ipv4("172.20.0.2").is_ok());
    }
}
