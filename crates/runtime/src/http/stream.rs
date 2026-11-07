//! Bounded native streaming with downstream truncation detection and backpressure.
use super::{MAX_HEADER, Response, origin_response::chunk_size};
use crate::Rejection;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
pub(super) const MAX_STREAM_BODY: usize = 1_048_576;
#[derive(Clone, Copy)]
pub(super) enum Framing {
    Empty,
    Length(u64),
    Chunked,
    Eof,
}

pub(crate) async fn forward(
    upstream: &mut (impl AsyncRead + Unpin),
    downstream: &mut (impl AsyncWrite + Unpin),
    head: bool,
    started: &mut bool,
) -> Result<(), Rejection> {
    let mut input = BufReader::with_capacity(8192, upstream);
    let mut bytes = Vec::new();
    loop {
        if bytes.len() >= MAX_HEADER {
            return Err(Rejection::new(503, "origin_header_budget"));
        }
        bytes.push(
            input
                .read_u8()
                .await
                .map_err(|_| Rejection::new(503, "origin_truncated_head"))?,
        );
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let (bytes, framing) = Response::stream_head(&bytes, head)?;
    *started = true;
    write(downstream, &bytes).await?;
    let mut total = 0usize;
    match framing {
        Framing::Empty => {}
        Framing::Length(length) => {
            copy_exact(&mut input, downstream, length, false, &mut total).await?
        }
        Framing::Chunked => {
            for chunk in 0..=65_536 {
                let mut line = Vec::new();
                loop {
                    if line.len() >= 18 {
                        return Err(Rejection::new(503, "origin_chunk_framing"));
                    }
                    line.push(
                        input
                            .read_u8()
                            .await
                            .map_err(|_| Rejection::new(503, "origin_chunk_truncated"))?,
                    );
                    if line.ends_with(b"\r\n") {
                        break;
                    }
                }
                let size = chunk_size(&line[..line.len() - 2])?;
                if size == 0 {
                    crlf(&mut input).await?;
                    eof(&mut input).await?;
                    write(downstream, b"0\r\n\r\n").await?;
                    return downstream
                        .shutdown()
                        .await
                        .map_err(|_| Rejection::new(503, "response_write"));
                }
                if chunk == 65_536 {
                    return Err(Rejection::new(503, "origin_chunk_count"));
                }
                copy_exact(&mut input, downstream, size as u64, true, &mut total).await?;
                crlf(&mut input).await?;
            }
        }
        Framing::Eof => {
            let mut buffer = [0u8; 8192];
            loop {
                let count = input
                    .read(&mut buffer)
                    .await
                    .map_err(|_| Rejection::new(503, "origin_read"))?;
                if count == 0 {
                    break;
                }
                budget(&mut total, count)?;
                chunk(downstream, &buffer[..count]).await?;
            }
            write(downstream, b"0\r\n\r\n").await?;
        }
    }
    if matches!(framing, Framing::Length(_) | Framing::Empty) {
        eof(&mut input).await?;
    }
    downstream
        .shutdown()
        .await
        .map_err(|_| Rejection::new(503, "response_write"))
}
async fn eof(input: &mut (impl AsyncRead + Unpin)) -> Result<(), Rejection> {
    let mut byte = [0];
    if input
        .read(&mut byte)
        .await
        .map_err(|_| Rejection::new(503, "origin_read"))?
        != 0
    {
        return Err(Rejection::new(503, "origin_trailing_frame"));
    }
    Ok(())
}
async fn copy_exact(
    input: &mut (impl AsyncRead + Unpin),
    output: &mut (impl AsyncWrite + Unpin),
    mut remaining: u64,
    encoded: bool,
    total: &mut usize,
) -> Result<(), Rejection> {
    if remaining > MAX_STREAM_BODY as u64 || remaining as usize > MAX_STREAM_BODY - *total {
        return Err(Rejection::new(503, "origin_stream_budget"));
    }
    let mut buffer = [0u8; 8192];
    while remaining > 0 {
        let count = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| Rejection::new(503, "origin_stream_budget"))?;
        input
            .read_exact(&mut buffer[..count])
            .await
            .map_err(|_| Rejection::new(503, "origin_stream_truncated"))?;
        budget(total, count)?;
        if encoded {
            chunk(output, &buffer[..count]).await?;
        } else {
            write(output, &buffer[..count]).await?;
        }
        remaining -= count as u64;
    }
    Ok(())
}
fn budget(total: &mut usize, count: usize) -> Result<(), Rejection> {
    *total = total
        .checked_add(count)
        .filter(|value| *value <= MAX_STREAM_BODY)
        .ok_or(Rejection::new(503, "origin_stream_budget"))?;
    Ok(())
}
async fn crlf(input: &mut (impl AsyncRead + Unpin)) -> Result<(), Rejection> {
    let mut bytes = [0; 2];
    input
        .read_exact(&mut bytes)
        .await
        .map_err(|_| Rejection::new(503, "origin_chunk_truncated"))?;
    if bytes != *b"\r\n" {
        return Err(Rejection::new(503, "origin_chunk_framing"));
    }
    Ok(())
}
async fn write(output: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> Result<(), Rejection> {
    output
        .write_all(bytes)
        .await
        .map_err(|_| Rejection::new(503, "response_write"))
}
async fn chunk(output: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> Result<(), Rejection> {
    write(output, format!("{:x}\r\n", bytes.len()).as_bytes()).await?;
    write(output, bytes).await?;
    write(output, b"\r\n").await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn complete_chunk_stream_reframes_and_strips_forged_gateway_fields() {
        let mut input=&b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/event-stream\r\nX-Humanymous-Ack: forged\r\n\r\n2\r\nhe\r\n3\r\nllo\r\n0\r\n\r\n"[..];
        let mut output = Vec::new();
        let mut started = false;
        forward(&mut input, &mut output, false, &mut started)
            .await
            .unwrap();
        assert!(started);
        let text = std::str::from_utf8(&output).unwrap();
        assert!(text.contains("Transfer-Encoding: chunked\r\n"));
        assert!(!text.contains("X-Humanymous-Ack"));
        assert!(!text.contains("Content-Length:"));
        assert!(output.ends_with(b"2\r\nhe\r\n3\r\nllo\r\n0\r\n\r\n"));
    }
    #[tokio::test]
    async fn malformed_or_truncated_stream_never_emits_a_success_terminator() {
        for body in [
            "5\r\nhe",
            "1\r\nxXX0\r\n\r\n",
            "1;x=y\r\nx\r\n0\r\n\r\n",
            "0\r\nX-Trailer: x\r\n\r\n",
            "0\r\n\r\ntrailing",
        ] {
            let frame = format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{body}");
            let mut input = frame.as_bytes();
            let mut output = Vec::new();
            let mut started = false;
            assert!(
                forward(&mut input, &mut output, false, &mut started)
                    .await
                    .is_err()
            );
            assert!(started);
            assert!(!output.ends_with(b"0\r\n\r\n"));
            assert!(!output.windows(12).any(|bytes| bytes == b"HTTP/1.1 503"));
        }
    }
    #[tokio::test]
    async fn known_oversized_body_is_refused_before_downstream_head() {
        let frame = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            MAX_STREAM_BODY + 1
        );
        let mut input = frame.as_bytes();
        let mut output = Vec::new();
        let mut started = false;
        assert!(
            forward(&mut input, &mut output, false, &mut started)
                .await
                .is_err()
        );
        assert!(!started);
        assert!(output.is_empty());
        let mut total = MAX_STREAM_BODY;
        assert!(budget(&mut total, 1).is_err());
    }
}
