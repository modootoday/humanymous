//! Owned recording origin on the backend network. Every accepted TCP
//! connection is logged with its peer address before any byte is read, so a
//! premature or misrouted contact is counted too. The runner reads this log.
use super::{ORIGIN_PORT, emit};
use serde_json::json;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    time::Duration,
};

const MAX_REQUEST: usize = 1 << 20;

pub fn run() -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, ORIGIN_PORT))?;
    emit(&json!({"role":"oa_origin_ready","port":ORIGIN_PORT}))?;
    let mut contacts = 0_u64;
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        contacts += 1;
        let peer = stream
            .peer_addr()
            .map(|address| address.ip().to_string())
            .unwrap_or_default();
        emit(&json!({"role":"oa_origin_contact","n":contacts,"peer":peer}))?;
        let _ = answer(&mut stream);
    }
    Ok(())
}

/// Length of a complete HTTP/1.1 request in `bytes`, if one is present.
fn complete(bytes: &[u8]) -> Option<usize> {
    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let head = std::str::from_utf8(&bytes[..end]).ok()?;
    let length = head
        .split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(Some(0), |(_, value)| value.trim().parse().ok())?;
    (bytes.len() >= end + length).then_some(end + length)
}

fn answer(stream: &mut TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut part = [0; 4096];
    while complete(&bytes).is_none() {
        let n = stream.read(&mut part)?;
        if n == 0 || bytes.len() + n > MAX_REQUEST {
            return Ok(());
        }
        bytes.extend_from_slice(&part[..n]);
    }
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\norigin-ok")
}

#[cfg(test)]
mod tests {
    use super::complete;

    #[test]
    fn request_completion_waits_for_the_declared_body() {
        assert_eq!(complete(b"POST / HTTP/1.1\r\nHost: a\r\n\r\n"), Some(28));
        assert_eq!(
            complete(b"POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\neff"),
            None
        );
        assert_eq!(
            complete(b"POST / HTTP/1.1\r\ncontent-length: 6\r\n\r\neffect"),
            Some(44)
        );
        assert_eq!(complete(b"POST / HTTP/1.1\r\n"), None);
        assert_eq!(
            complete(b"POST / HTTP/1.1\r\nContent-Length: x\r\n\r\n"),
            None
        );
    }
}
