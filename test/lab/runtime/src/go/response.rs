//! Project already-validated shared runtime response framing into observations.
use std::io;
pub struct Response<'a> {
    pub status: u16,
    pub headers: Vec<(&'a str, &'a str)>,
    pub entity: &'a [u8],
}
pub fn observe(bytes: &[u8], head: bool) -> io::Result<Response<'_>> {
    humanymous_runtime::gateway::http::response(bytes, head)
        .map_err(|_| io::Error::other("shared response framing rejected"))?;
    let end = bytes
        .windows(4)
        .position(|p| p == b"\r\n\r\n")
        .ok_or_else(|| io::Error::other("validated response boundary"))?;
    let header = std::str::from_utf8(&bytes[..end]).map_err(io::Error::other)?;
    let mut lines = header.split("\r\n");
    let status = lines
        .next()
        .and_then(|s| s.split(' ').nth(1))
        .ok_or_else(|| io::Error::other("validated response status"))?
        .parse()
        .map_err(io::Error::other)?;
    let mut headers = Vec::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::other("validated response header"))?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(io::Error::other(
                "Go pilot requires buffered nonchunked outer framing",
            ));
        }
        headers.push((name, value.trim()));
    }
    Ok(Response {
        status,
        headers,
        entity: &bytes[end + 4..],
    })
}
