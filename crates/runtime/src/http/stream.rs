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
    use std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll},
    };
    use tokio::io::ReadBuf;
    struct CountedInput {
        bytes: Vec<u8>,
        position: usize,
        count: Arc<AtomicUsize>,
    }
    impl AsyncRead for CountedInput {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let this = self.get_mut();
            let count = buffer.remaining().min(this.bytes.len() - this.position);
            buffer.put_slice(&this.bytes[this.position..this.position + count]);
            this.position += count;
            this.count.fetch_add(count, Ordering::SeqCst);
            Poll::Ready(Ok(()))
        }
    }
    struct LimitedOutput {
        bytes: Vec<u8>,
        limit: usize,
        fail: bool,
    }
    impl AsyncWrite for LimitedOutput {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            if this.bytes.len() == this.limit {
                return if this.fail {
                    Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()))
                } else {
                    Poll::Pending
                };
            }
            let count = bytes.len().min(this.limit - this.bytes.len());
            this.bytes.extend_from_slice(&bytes[..count]);
            Poll::Ready(Ok(count))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    /// In-memory flow-control model; actual socket abort/drain remains hosted.
    #[tokio::test]
    async fn downstream_backpressure_or_abort_does_not_buffer_the_whole_origin() {
        for fail in [false, true] {
            let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {MAX_STREAM_BODY}\r\n\r\n");
            let mut bytes = head.as_bytes().to_vec();
            bytes.resize(bytes.len() + MAX_STREAM_BODY, b'x');
            let count = Arc::new(AtomicUsize::new(0));
            let mut input = CountedInput {
                bytes,
                position: 0,
                count: count.clone(),
            };
            let mut output = LimitedOutput {
                bytes: Vec::new(),
                limit: head.len() + 64,
                fail,
            };
            let mut started = false;
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(10),
                forward(&mut input, &mut output, false, &mut started),
            )
            .await;
            if fail {
                assert!(result.unwrap().is_err());
            } else {
                assert!(result.is_err());
            }
            assert!(started);
            assert!(count.load(Ordering::SeqCst) <= 32768);
            assert!(!output.bytes.ends_with(b"0\r\n\r\n"));
        }
    }
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
