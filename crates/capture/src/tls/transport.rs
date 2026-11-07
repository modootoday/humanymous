use super::{CaptureError, ClientHelloCapture};
use humanymous_network::tls::ClientHello;
use rustls::{HandshakeKind, ServerConfig, ServerConnection};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const HANDSHAKE_BYTES: usize = 262_144;

/// Revocation-only supervisor handle for one connection. No raw transport or
/// observation constructor is exposed. Dropping this handle alone does not
/// cancel; the owner always shuts down its socket when dropped.
pub struct Cancellation {
    stream: TcpStream,
    cancelled: Arc<AtomicBool>,
}
impl Cancellation {
    /// Immediately revoke evidence and wake blocked socket I/O. A shutdown
    /// error does not restore evidence availability or permit another epoch.
    pub fn cancel(&self) -> io::Result<()> {
        if self.cancelled.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        match self.stream.shutdown(Shutdown::Both) {
            Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
            result => result,
        }
    }
}

struct Ingress {
    stream: TcpStream,
    deadline: Instant,
    bytes: usize,
    capture: Option<ClientHelloCapture>,
    cancelled: Arc<AtomicBool>,
}
impl Ingress {
    fn remaining(&self) -> io::Result<Duration> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "capture cancelled",
            ));
        }
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|value| !value.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "capture deadline"))
    }

    fn cancellation(&self) -> io::Result<Cancellation> {
        self.remaining()?;
        Ok(Cancellation {
            stream: self.stream.try_clone()?,
            cancelled: Arc::clone(&self.cancelled),
        })
    }
}
impl Read for Ingress {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        let count = self.stream.read(buf)?;
        self.remaining()?;
        if let Some(capture) = &mut self.capture {
            self.bytes = self
                .bytes
                .checked_add(count)
                .filter(|value| *value <= HANDSHAKE_BYTES)
                .ok_or_else(|| io::Error::other("TLS handshake byte budget"))?;
            capture.feed(&buf[..count]).map_err(io::Error::other)?;
        }
        Ok(count)
    }
}
impl Write for Ingress {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        let count = self.stream.write(buf)?;
        self.remaining()?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.remaining()?;
        self.stream.flush()
    }
}

impl Drop for Ingress {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

/// Owns an accepted socket before any TLS I/O. Obtain its revocation handle
/// before moving it into a supervised blocking worker, then always join that
/// worker after cancellation. Dropping this value closes the same socket.
pub struct PendingHandshake {
    tls: ServerConnection,
    ingress: Ingress,
}

impl PendingHandshake {
    /// Consume an ingress socket, establish one absolute lifetime and prepare
    /// rustls without performing TLS reads/writes. The socket is set to blocking
    /// mode so the bounded read/write timeouts apply to actual worker I/O.
    pub fn new(stream: TcpStream, config: Arc<ServerConfig>, budget: Duration) -> io::Result<Self> {
        if budget.is_zero() || budget > Duration::from_secs(30) {
            let _ = stream.shutdown(Shutdown::Both);
            return Err(io::Error::other("invalid capture lifetime"));
        }
        let ingress = Ingress {
            stream,
            deadline: Instant::now() + budget,
            bytes: 0,
            capture: Some(ClientHelloCapture::default()),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        ingress.stream.set_nonblocking(false)?;
        let mut config = (*config).clone();
        config.max_early_data_size = 0;
        let mut tls = ServerConnection::new(Arc::new(config)).map_err(io::Error::other)?;
        tls.set_buffer_limit(Some(65_536));
        ingress.remaining()?;
        Ok(Self { tls, ingress })
    }

    /// Obtain revocation custody before dispatching blocking handshake work.
    /// The handle remains bound to this epoch after successful acceptance.
    pub fn cancellation(&self) -> io::Result<Cancellation> {
        self.ingress.cancellation()
    }

    /// Remaining time in the original pre-queue lifetime. Acceptance never
    /// resets it. Cancellation and elapsed time return terminal errors.
    pub fn remaining(&self) -> io::Result<Duration> {
        self.ingress.remaining()
    }

    /// Complete the handshake on this exact socket. Cancellation, timeout or
    /// any error consumes and closes the pending owner; there is no retry path.
    pub fn accept(mut self) -> io::Result<Connection> {
        self.ingress.remaining()?;
        while self.tls.is_handshaking() {
            self.ingress.remaining()?;
            self.tls.complete_io(&mut self.ingress)?;
        }
        self.ingress.remaining()?;
        if self.tls.handshake_kind() == Some(HandshakeKind::FullWithHelloRetryRequest) {
            return Err(io::Error::other("HelloRetryRequest capture unsupported"));
        }
        let hello = self
            .ingress
            .capture
            .take()
            .ok_or_else(|| io::Error::other("capture absent"))?
            .finish()
            .map_err(io::Error::other)?;
        let h2 = (self.tls.alpn_protocol() == Some(b"h2".as_slice()))
            .then(crate::http2::Capture::default);
        self.ingress.remaining()?;
        Ok(Connection {
            tls: self.tls,
            ingress: self.ingress,
            hello,
            h2,
            failed: false,
        })
    }
}

/// Owns one actual TCP/TLS epoch. Cannot be deserialized, cloned or constructed
/// from an observation. The configured deployment still owns hop authenticity.
pub struct Connection {
    tls: ServerConnection,
    ingress: Ingress,
    hello: ClientHello,
    h2: Option<crate::http2::Capture>,
    failed: bool,
}
impl Connection {
    /// Complete an actual bounded TLS handshake on the consumed socket. The
    /// deadline covers this owner's entire lifetime, not each I/O independently.
    /// No early data or HelloRetryRequest qualifies this initial profile.
    pub fn accept(
        stream: TcpStream,
        config: Arc<ServerConfig>,
        budget: Duration,
    ) -> io::Result<Self> {
        PendingHandshake::new(stream, config, budget)?.accept()
    }

    /// Recheck the original epoch after audit and before an origin effect.
    /// This is a liveness snapshot, not a reusable authorization ticket.
    pub fn remaining(&self) -> io::Result<Duration> {
        if self.failed {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "capture unavailable",
            ));
        }
        self.ingress.remaining()
    }

    /// Borrow the complete hello from this successfully handshaken owner. Any
    /// observed terminal I/O failure or elapsed lifetime removes availability.
    pub fn client_hello(&self) -> Option<&ClientHello> {
        self.remaining().ok().map(|_| &self.hello)
    }

    /// Actual negotiated protocol, distinct from ClientHello's offered ALPN.
    pub fn negotiated_alpn(&self) -> Option<&[u8]> {
        self.client_hello()?;
        self.tls.alpn_protocol()
    }

    /// Borrow a stream's actual decrypted ordered header evidence from this
    /// owner. A pure `http2::Capture` cannot be attached to an existing owner.
    pub fn h2_request(&self, stream: u32) -> Option<&crate::http2::Request> {
        self.client_hello()?;
        self.h2.as_ref()?.request(stream)
    }

    /// Existing frozen connection frame-monitor observation, if H2 is active.
    /// Remains available after failure for audit, never as an allow capability.
    pub fn h2_abuse(&self) -> Option<&'static str> {
        self.h2.as_ref()?.abuse()
    }

    /// Mint a revocation-only handle for a supervised joined worker. It grants
    /// no ability to inject capture bytes or access application plaintext.
    pub fn cancellation(&self) -> io::Result<Cancellation> {
        if self.client_hello().is_none() {
            return Err(io::Error::other("capture unavailable"));
        }
        self.ingress.cancellation()
    }

    /// Read decrypted ingress before framework normalization. This method is
    /// intentionally not `Read`: consumers must preserve the connection owner
    /// and its observations instead of extracting an unqualified TLS socket.
    pub fn read_plaintext(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.ensure_live()?;
        let result = rustls::Stream::new(&mut self.tls, &mut self.ingress).read(buf);
        if let Err(error) = self.ingress.remaining() {
            self.failed = true;
            return Err(error);
        }
        if let Ok(count) = result
            && let Some(h2) = &mut self.h2
            && let Err(error) = h2.feed(&buf[..count])
        {
            self.failed = true;
            return Err(io::Error::other(error));
        }
        if result.is_err() || matches!(result, Ok(0)) && !buf.is_empty() {
            self.failed = true;
        }
        result
    }

    /// Write complete bounded response bytes, propagating TLS flush failures.
    pub fn write_plaintext(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.ensure_live()?;
        let mut stream = rustls::Stream::new(&mut self.tls, &mut self.ingress);
        let result = stream.write_all(bytes).and_then(|()| stream.flush());
        if let Err(error) = self.ingress.remaining() {
            self.failed = true;
            return Err(error);
        }
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn ensure_live(&mut self) -> io::Result<()> {
        if self.client_hello().is_none() {
            self.failed = true;
            return Err(io::Error::other(CaptureError::Truncated));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_connection_and_revocation_can_move_to_supervised_workers() {
        fn assert_send<T: Send>() {}
        assert_send::<PendingHandshake>();
        assert_send::<Connection>();
        assert_send::<Cancellation>();
        fn assert_sync<T: Sync>() {}
        assert_sync::<Cancellation>();
    }
}
