//! Bounded test-client framing; complete messages do not require TCP closure.
use std::io::{self, Read};
fn invalid() -> io::Error {
    io::Error::other("invalid fixture HTTP response")
}
fn parsed(bytes: &[u8]) -> io::Result<Option<(usize, Vec<u8>)>> {
    let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") else {
        if bytes.len() > 65536 {
            return Err(invalid());
        }
        return Ok(None);
    };
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| invalid())?;
    let mut length = None;
    let mut chunked = false;
    for line in head.lines().skip(1) {
        let (name, value) = line
            .trim_end_matches('\r')
            .split_once(':')
            .ok_or_else(invalid)?;
        match name.to_ascii_lowercase().as_str() {
            "content-length" => {
                if length.is_some() {
                    return Err(invalid());
                }
                length = Some(value.trim().parse::<usize>().map_err(|_| invalid())?);
            }
            "transfer-encoding" => {
                if value.trim() != "chunked" {
                    return Err(invalid());
                }
                chunked = true;
            }
            _ => {}
        }
    }
    let start = end + 4;
    if let Some(length) = length {
        if chunked || length > 2_097_152 {
            return Err(invalid());
        }
        if bytes.len() < start + length {
            return Ok(None);
        }
        return Ok(Some((
            start + length,
            bytes[start..start + length].to_vec(),
        )));
    }
    if head.starts_with("HTTP/1.1 204") || head.starts_with("HTTP/1.1 304") {
        return Ok(Some((start, Vec::new())));
    }
    if !chunked {
        return Err(invalid());
    }
    let mut at = start;
    let mut body = Vec::new();
    loop {
        let Some(n) = bytes[at..].windows(2).position(|v| v == b"\r\n") else {
            return Ok(None);
        };
        let size = std::str::from_utf8(&bytes[at..at + n])
            .map_err(|_| invalid())?
            .split(';')
            .next()
            .ok_or_else(invalid)?;
        let size = usize::from_str_radix(size, 16).map_err(|_| invalid())?;
        at += n + 2;
        if size == 0 {
            if bytes.get(at..at + 2) == Some(b"\r\n") {
                return Ok(Some((at + 2, body)));
            }
            return Err(invalid());
        }
        if size > 2_097_152 - body.len() {
            return Err(invalid());
        }
        if bytes.len() < at + size + 2 {
            return Ok(None);
        }
        if bytes.get(at + size..at + size + 2) != Some(b"\r\n") {
            return Err(invalid());
        }
        body.extend_from_slice(&bytes[at..at + size]);
        at += size + 2;
    }
}
pub fn receive(stream: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if let Some((end, _)) = parsed(&bytes)? {
            bytes.truncate(end);
            return Ok(bytes);
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(io::Error::other("incomplete fixture HTTP response"));
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > 2_162_688 {
            return Err(invalid());
        }
    }
}
pub fn body(wire: &[u8]) -> Vec<u8> {
    parsed(wire)
        .expect("valid response")
        .expect("complete response")
        .1
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_without_connection_close() {
        let wire = b"HTTP/1.1 503 Error\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(receive(&mut &wire[..]).unwrap(), wire);
        assert_eq!(
            body(b"HTTP/1.1 201 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n"),
            b"abc"
        );
    }
    #[test]
    fn rejects_incomplete_or_conflicting_framing() {
        for wire in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nx".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n"
                .as_slice(),
        ] {
            assert!(receive(&mut &wire[..]).is_err());
        }
    }
}
