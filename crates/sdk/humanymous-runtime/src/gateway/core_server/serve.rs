//! Loopback-only Core serving (the listener half of legacy cmd/server main.go,
//! tlsconn.go `captureListener` and maxconns_test.go at baseline 981d25c).
//!
//! The accept loop takes a [`ConnectionCap`] permit before each accept, then
//! serves exactly one request per connection on a blocking worker: plaintext
//! sockets are peeked for wire header order ([`peek_tcp`]); TLS sockets are
//! owned by the capture transport, which records the complete ClientHello
//! and supplies the decrypted plaintext that is peeked the same way. The
//! evidence stays with the [`Connection`] (owner decision 2026-11-16: no
//! address-keyed registry).
//!
//! Deliberate differences from Go: only explicit loopback addresses can be
//! bound (runtime-application SoT; a public Core profile is plan B01, owned by
//! Root), HTTP/2 is not served (an `h2` ALPN closes the connection), and
//! there is no keep-alive.
use super::http::{Refused, Request, read_request, write_response};
use super::routes::date_now;
use super::{Connection, CoreServer};
use crate::capture::tls::Connection as TlsConnection;
use crate::gateway::connection_cap::ConnectionCap;
use crate::network::http::{peek_owned, peek_tcp};
use rustls::ServerConfig;
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

/// Bytes drained after a response so unread request bytes do not reset it.
const DRAIN_LIMIT: usize = 256 * 1024;
/// Absolute time allowed for that drain.
const DRAIN_TIME: Duration = Duration::from_millis(500);

/// Listener options.
#[derive(Clone)]
pub struct Options {
    /// TLS server configuration; `None` serves plaintext.
    pub tls: Option<Arc<ServerConfig>>,
    /// Whole-connection I/O budget: one absolute deadline over the TLS
    /// handshake (when TLS), the request read and the response write. A peer
    /// that trickles bytes cannot extend it.
    pub budget: Duration,
    /// Concurrent connection cap.
    pub cap: ConnectionCap,
}

/// Bind an explicit loopback address; any other address is refused.
pub async fn bind(address: SocketAddr) -> io::Result<TcpListener> {
    if !address.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Core binds loopback only",
        ));
    }
    TcpListener::bind(address).await
}

/// Accept and serve until `shutdown` resolves or the cap is closed.
pub async fn serve(
    server: Arc<CoreServer>,
    listener: TcpListener,
    options: Options,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    if !listener.local_addr()?.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Core serves loopback only",
        ));
    }
    tokio::pin!(shutdown);
    loop {
        let permit = tokio::select! {
            () = &mut shutdown => return Ok(()),
            permit = options.cap.acquire() => match permit {
                Ok(permit) => permit,
                Err(_) => return Ok(()),
            },
        };
        let (stream, _) = tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(_) => continue,
            },
        };
        let Ok(stream) = stream.into_std() else {
            continue;
        };
        let server = Arc::clone(&server);
        let options = options.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _ = serve_connection(&server, stream, &options);
        });
    }
}

fn refused(status: Refused) -> super::Response {
    let reply = crate::gateway::resource::Response::text(
        status.0,
        Vec::new(),
        super::http::reason(status.0),
    );
    super::routes::finish(reply, Vec::new())
}

/// Serve one request on an accepted socket.
pub fn serve_connection(
    server: &CoreServer,
    stream: TcpStream,
    options: &Options,
) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    let remote = stream.peer_addr()?.to_string();
    match &options.tls {
        None => {
            let deadline = Bounded::deadline(options.budget)?;
            stream.set_read_timeout(Some(options.budget))?;
            stream.set_write_timeout(Some(options.budget))?;
            let peek = peek_tcp(&stream);
            let connection = Connection::plaintext(peek.wire_order());
            let mut reader = peek.into_replay(Bounded {
                stream: &stream,
                deadline,
            });
            let request = read_request(&mut reader, &remote, |head| server.body_limit(head));
            let mut writer = Bounded {
                stream: &stream,
                deadline,
            };
            respond(server, request, &connection, &mut writer)?;
            drain(&stream);
            Ok(())
        }
        Some(config) => {
            let mut tls = TlsConnection::accept(stream, Arc::clone(config), options.budget)?;
            if tls
                .negotiated_alpn()
                .is_some_and(|alpn| alpn != b"http/1.1")
            {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "HTTP/2 is not served by the Core",
                ));
            }
            let peek = peek_owned(|buffer| tls.read_plaintext(buffer));
            let wire = peek.wire_order();
            let request = {
                let mut reader = peek.into_replay(Plaintext(&mut tls));
                read_request(&mut reader, &remote, |head| server.body_limit(head))
            };
            let connection = Connection::tls(tls.client_hello().cloned(), wire, tls.h2_abuse());
            let mut writer = TlsWriter(&mut tls);
            respond(server, request, &connection, &mut writer)
        }
    }
}

fn respond(
    server: &CoreServer,
    request: Result<Request, Refused>,
    connection: &Connection,
    writer: &mut impl Write,
) -> io::Result<()> {
    let (response, head) = match request {
        Ok(request) => (
            server.handle(&request, connection),
            request.method == "HEAD",
        ),
        Err(status) => (refused(status), false),
    };
    if response.status == 0 {
        // A handler panicked with observability off: drop the connection.
        return Ok(());
    }
    write_response(writer, &response, head, &date_now(server))
}

fn drain(stream: &TcpStream) {
    let _ = stream.shutdown(Shutdown::Write);
    let Ok(deadline) = Bounded::deadline(DRAIN_TIME) else {
        return;
    };
    let mut reader = Bounded { stream, deadline };
    let mut left = DRAIN_LIMIT;
    let mut buffer = [0u8; 4096];
    while left > 0 {
        match reader.read(&mut buffer[..left.min(4096)]) {
            Ok(0) | Err(_) => break,
            Ok(n) => left -= n,
        }
    }
}

/// A plaintext socket under one absolute deadline, as the TLS capture
/// transport bounds its whole lifetime: each read or write gets only the
/// time left, so a slow peer cannot reset the budget byte by byte.
struct Bounded<'a> {
    stream: &'a TcpStream,
    deadline: Instant,
}

impl Bounded<'_> {
    fn deadline(budget: Duration) -> io::Result<Instant> {
        Instant::now()
            .checked_add(budget)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "budget out of range"))
    }

    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection budget"))
    }
}

impl Read for Bounded<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        let mut stream = self.stream;
        stream.read(buffer)
    }
}

impl Write for Bounded<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        let mut stream = self.stream;
        stream.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut stream = self.stream;
        stream.flush()
    }
}

struct Plaintext<'a>(&'a mut TlsConnection);

impl Read for Plaintext<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read_plaintext(buffer)
    }
}

struct TlsWriter<'a>(&'a mut TlsConnection);

impl Write for TlsWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write_plaintext(bytes)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
