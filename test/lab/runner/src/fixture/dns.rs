//! Owned DNS egress canary. It never resolves/forwards a name outside the lab.
use super::dns_wire::{query, question as name, reply};
use crate::docker::Result;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    time::Duration,
};

pub(super) fn serve() -> Result<()> {
    // Failure to bind UID65532/caps0 is a failed gate; never add privileges.
    let udp = UdpSocket::bind("0.0.0.0:53")?;
    udp.set_nonblocking(true)?;
    let tcp = TcpListener::bind("0.0.0.0:8080")?;
    tcp.set_nonblocking(true)?;
    let mut received = 0u64;
    let mut invalid = 0u64;
    let mut counts = BTreeMap::<String, u64>::new();
    loop {
        let mut packet = [0; 513];
        match udp.recv_from(&mut packet) {
            Ok((length, peer)) => {
                received = received.checked_add(1).ok_or("DNS count exhausted")?;
                match name(&packet[..length]) {
                    Ok(value) => {
                        let hash = format!("{:x}", Sha256::digest(value.as_bytes()));
                        if counts.len() >= 64 && !counts.contains_key(&hash) {
                            return Err("DNS canary inventory bound".into());
                        }
                        *counts.entry(hash).or_default() += 1;
                        udp.send_to(&reply(&packet[..length], 3)?, peer)?;
                    }
                    Err(_) => {
                        invalid = invalid
                            .checked_add(1)
                            .ok_or("DNS invalid count exhausted")?;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        match tcp.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(1)))?;
                stream.set_write_timeout(Some(Duration::from_secs(1)))?;
                if super::control_request(&mut stream)? {
                    let body = serde_json::to_vec(
                        &json!({"schema":"humanymous.owned-dns-canary/1","fixture":"dns-canary","received":received,"invalid":invalid,"question_hash_counts":counts}),
                    )?;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream.write_all(response.as_bytes())?;
                    stream.write_all(&body)?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
pub(super) fn positive() -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    let packet = query("positive-control.invalid")?;
    socket.send_to(&packet, "127.0.0.1:53")?;
    let mut reply = [0; 512];
    let (length, peer) = socket.recv_from(&mut reply)?;
    if peer != "127.0.0.1:53".parse::<std::net::SocketAddr>()?
        || reply[..length] != super::dns_wire::reply(&packet, 3)?
    {
        return Err("DNS positive response mismatch".into());
    }
    println!(
        "{}",
        json!({"status":"passed","control":"owned-dns-positive"})
    );
    Ok(())
}
pub(super) fn snapshot() -> Result<()> {
    let mut stream =
        std::net::TcpStream::connect_timeout(&"127.0.0.1:8080".parse()?, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: owned.invalid\r\nConnection: close\r\n\r\n")?;
    let mut bytes = Vec::new();
    stream.take(65537).read_to_end(&mut bytes)?;
    let end = bytes
        .windows(4)
        .position(|p| p == b"\r\n\r\n")
        .ok_or("DNS snapshot framing")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes[end + 4..])?;
    if !bytes.starts_with(b"HTTP/1.1 200 ") || value["schema"] != "humanymous.owned-dns-canary/1" {
        return Err("DNS snapshot refused".into());
    }
    println!("{value}");
    Ok(())
}
