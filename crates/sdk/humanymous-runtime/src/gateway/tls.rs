//! Joined blocking TLS ingress. This owner alone does not qualify a route.
mod h2;

pub use h2::CapturedH2Request;

use crate::capture::tls::{Cancellation, Connection, PendingHandshake};
use crate::gateway::http::{self, Request};
use rustls::ServerConfig;
use std::{
    io,
    net::TcpStream,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    thread,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, ReadBuf},
    sync::oneshot,
};

fn alpn_supported(protocol: &[u8]) -> bool {
    protocol == b"http/1.1" || protocol == b"h2"
}

pub(in crate::gateway) fn validate_configuration(
    config: &crate::gateway::configuration::Configuration,
    tls: &ServerConfig,
) -> io::Result<()> {
    if config.browser.is_some()
        || config.routes.iter().any(|route| {
            route.response_profile != crate::gateway::configuration::ResponseProfile::Buffered
        })
        || tls
            .alpn_protocols
            .iter()
            .any(|protocol| !alpn_supported(protocol))
    {
        return Err(io::Error::other(
            "TLS requires native buffered routes and HTTP/1.1 or H2 ALPN",
        ));
    }
    Ok(())
}

/// R13 H1 or R14 H2 complete request retained with its live TLS owner.
pub enum CapturedTls {
    /// HTTP/1.1 negotiated consumer.
    H1(CapturedRequest),
    /// HTTP/2 stream-1 serving consumer.
    H2(CapturedH2Request),
}

impl CapturedTls {
    /// Accept one TLS connection and dispatch to the R13 or R14 consumer.
    pub async fn receive(
        socket: TcpStream,
        config: Arc<ServerConfig>,
        budget: Duration,
    ) -> io::Result<Self> {
        // Peek negotiation via a short handshake would consume the socket; instead
        // prefer advertised order: if server offers only h2, take H2 path; if only
        // http/1.1, H1; if both, handshake first through H1 receiver's shared
        // PendingHandshake by dispatching after accept inside each path.
        // Dual-offer: use H2 receiver when h2 is listed first or alone, else H1.
        let offers_h2 = config.alpn_protocols.iter().any(|p| p.as_slice() == b"h2");
        let offers_h1 = config
            .alpn_protocols
            .iter()
            .any(|p| p.as_slice() == b"http/1.1");
        match (offers_h2, offers_h1) {
            (true, false) => Ok(Self::H2(
                CapturedH2Request::receive(socket, config, budget).await?,
            )),
            (false, true) => Ok(Self::H1(
                CapturedRequest::receive(socket, config, budget).await?,
            )),
            (true, true) => receive_dual(socket, config, budget).await,
            (false, false) => Err(io::Error::other("TLS ALPN unsupported")),
        }
    }
}

async fn receive_dual(
    socket: TcpStream,
    config: Arc<ServerConfig>,
    budget: Duration,
) -> io::Result<CapturedTls> {
    // Handshake once, then branch on negotiated ALPN without reparsing H1 as H2.
    let pending = PendingHandshake::new(socket, config, budget)?;
    let cancellation = pending.cancellation()?;
    let runtime = tokio::runtime::Handle::current();
    joined(cancellation, move || {
        let mut connection = pending.accept()?;
        match connection.negotiated_alpn() {
            Some(b"h2") => Ok(CapturedTls::H2(h2::serve_accepted(connection)?)),
            None | Some(b"http/1.1") => {
                let request = runtime
                    .block_on(http::read(&mut Plaintext(&mut connection)))
                    .map_err(|_| io::Error::other("TLS complete request framing failed"))?;
                connection.remaining()?;
                Ok(CapturedTls::H1(CapturedRequest {
                    request,
                    connection,
                }))
            }
            _ => Err(io::Error::other("TLS application protocol unsupported")),
        }
    })
    .await
}

/// A complete bounded request and its original successfully handshaken owner.
/// No constructor accepts caller-supplied observations or plaintext frames.
pub struct CapturedRequest {
    request: Request,
    connection: Connection,
}
impl CapturedRequest {
    /// Receive on one worker using the same strict parser as plaintext ingress.
    /// Dropping this future revokes the socket and joins its worker synchronously.
    /// The external service supervisor still owns an ultimate process deadline.
    pub async fn receive(
        socket: TcpStream,
        config: Arc<ServerConfig>,
        budget: Duration,
    ) -> io::Result<Self> {
        let pending = PendingHandshake::new(socket, config, budget)?;
        let cancellation = pending.cancellation()?;
        let runtime = tokio::runtime::Handle::current();
        joined(cancellation, move || {
            let mut connection = pending.accept()?;
            if connection
                .negotiated_alpn()
                .is_some_and(|value| value != b"http/1.1")
            {
                return Err(io::Error::other("TLS application protocol unsupported"));
            }
            let request = runtime
                .block_on(http::read(&mut Plaintext(&mut connection)))
                .map_err(|_| io::Error::other("TLS complete request framing failed"))?;
            connection.remaining()?;
            Ok(Self {
                request,
                connection,
            })
        })
        .await
    }

    /// Exact parsed request from this connection, never an independent claim.
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Borrow current handshake evidence while its original owner remains live.
    /// Lifetime availability alone does not prove remote liveness after audit.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub(in crate::gateway) fn parts(&mut self) -> (&Request, &mut Connection) {
        (&self.request, &mut self.connection)
    }

    /// Write one already bounded complete HTTP response and consume this epoch.
    /// Cancellation joins the writer; there is no retry or second response path.
    pub async fn respond(mut self, bytes: Vec<u8>) -> io::Result<()> {
        if bytes.len() > http::MAX_ORIGIN_FRAME {
            return Err(io::Error::other("TLS complete response budget"));
        }
        let cancellation = self.connection.cancellation()?;
        joined(cancellation, move || {
            let bytes = zeroize::Zeroizing::new(bytes);
            self.connection.write_plaintext(&bytes)
        })
        .await
    }
}

// This adapter is polled exclusively inside the owned blocking worker. It must
// never be attached to a Tokio reactor task: reads intentionally block there.
struct Plaintext<'a>(&'a mut Connection);
impl AsyncRead for Plaintext<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let count = match self
            .get_mut()
            .0
            .read_plaintext(output.initialize_unfilled())
        {
            Ok(count) => count,
            Err(error) => return Poll::Ready(Err(error)),
        };
        output.advance(count);
        Poll::Ready(Ok(()))
    }
}

struct Worker {
    cancellation: Cancellation,
    thread: Option<thread::JoinHandle<()>>,
}
impl Worker {
    fn finish(mut self) -> io::Result<()> {
        self.thread
            .take()
            .ok_or_else(|| io::Error::other("TLS worker ownership missing"))?
            .join()
            .map_err(|_| io::Error::other("TLS worker panicked"))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.cancellation.cancel();
            let _ = thread.join();
        }
    }
}
async fn joined<T: Send + 'static>(
    cancellation: Cancellation,
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    let (sender, receiver) = oneshot::channel();
    let thread = thread::Builder::new()
        .name("runtime-tls".into())
        .spawn(move || {
            let _ = sender.send(work());
        })?;
    let worker = Worker {
        cancellation,
        thread: Some(thread),
    };
    let result = receiver
        .await
        .map_err(|_| io::Error::other("TLS worker unavailable"));
    worker.finish()?;
    result?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::configuration::{BrowserPolicy, ResponseProfile};

    #[test]
    fn configuration_refuses_profiles_without_a_tls_h1_consumer() {
        let mut config = crate::gateway::tests::config();
        let mut tls = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(rustls::server::ResolvesServerCertUsingSni::new()));
        config.routes[0].required.raw_tls = true;
        assert!(validate_configuration(&config, &tls).is_ok());
        tls.alpn_protocols = vec![b"h2".to_vec()];
        assert!(validate_configuration(&config, &tls).is_ok());
        tls.alpn_protocols = vec![b"http/1.1".to_vec(), b"h2".to_vec()];
        assert!(validate_configuration(&config, &tls).is_ok());
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        config.routes[0].response_profile = ResponseProfile::NativeStream;
        assert!(validate_configuration(&config, &tls).is_err());
        config.routes[0].response_profile = ResponseProfile::Buffered;
        config.browser = Some(BrowserPolicy {
            origin: "http://owned.test".into(),
        });
        assert!(validate_configuration(&config, &tls).is_err());
        config.browser = None;
        tls.alpn_protocols = vec![b"http/1.0".to_vec()];
        assert!(validate_configuration(&config, &tls).is_err());
    }
}
