//! PROXY protocol v2 accepting listener for a gate behind an L4 balancer.
//!
//! Port of legacy internal/gate proxyproto.go `WrapProxyProto`,
//! `proxyListener.Accept` and `proxyConn` at baseline 981d25c, checked against
//! the `accept` section of `tests/fixtures/network/proxyproto-go-golden.json`.
//! Header decoding and the trust decision are `network::proxy`; this module
//! owns only the socket loop:
//! - a peer outside the allowlist is returned untouched, nothing is read and
//!   its bytes stay ordinary payload (anti-spoof);
//! - a trusted balancer must send one complete v2 header within the read
//!   timeout (Go: 5 s); the declared TCP source replaces the socket peer, and
//!   LOCAL or a non-TCP family keeps the socket peer;
//! - a trusted balancer that sends a malformed, truncated or late header has
//!   its connection closed, and accepting continues with the next one.
//!
//! As in Go, the header is read inside `accept`, so one slow trusted balancer
//! delays later accepts by at most the read timeout. Only the bytes of the
//! header are consumed; the TLS ClientHello that follows stays on the stream.
//! The returned [`Peer`] is only for address-keyed state; it never becomes TLS
//! or original-client provenance (network-capture N01). The public TLS
//! listener (`application::tls`, plan B26) applies the same trust gate with
//! [`read_header`] inside its per-connection receive task instead of
//! wrapping the listener, so a slow balancer holds one receive permit, not
//! the accept loop.
use crate::network::proxy::{self, Allowlist, FIXED_LEN, Header, Peer, Refusal};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

/// Go `WrapProxyProto` header read timeout.
pub const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// A TCP listener that recovers the client address from a trusted balancer's
/// PROXY v2 header.
#[derive(Debug)]
pub struct ProxyListener {
    listener: TcpListener,
    allowlist: Allowlist,
    timeout: Duration,
}

impl ProxyListener {
    /// Wrap `listener` with the Go read timeout. An allowlist is never empty,
    /// so a listener without trusted balancers is simply not wrapped.
    pub fn new(listener: TcpListener, allowlist: Allowlist) -> Self {
        Self {
            listener,
            allowlist,
            timeout: READ_TIMEOUT,
        }
    }

    /// Replace the header read timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Header read timeout in force.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Bound local address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Accept the next usable connection and the peer to key it by. A
    /// listener error is returned at once, as Go did.
    pub async fn accept(&self) -> io::Result<(TcpStream, Peer)> {
        loop {
            let (mut stream, socket_peer) = self.listener.accept().await?;
            if !self.allowlist.trusts_peer(socket_peer) {
                return Ok((stream, Peer::Socket(socket_peer)));
            }
            match tokio::time::timeout(self.timeout, read_header(&mut stream)).await {
                Ok(Ok(Header::Source(source))) => return Ok((stream, Peer::Declared(source))),
                Ok(Ok(Header::Local | Header::Unspecified)) => {
                    return Ok((stream, Peer::Balancer(socket_peer)));
                }
                // Malformed, truncated or late header from a trusted balancer:
                // close it and keep serving.
                Ok(Err(_)) | Err(_) => drop(stream),
            }
        }
    }
}

/// Read exactly one PROXY v2 header (and nothing after it) from a trusted
/// balancer's stream. Shared with the public TLS listener (plan B26), which
/// reads it in its per-connection receive task.
pub(in crate::gateway) async fn read_header(stream: &mut TcpStream) -> Result<Header, Refusal> {
    let mut fixed = [0u8; FIXED_LEN];
    stream
        .read_exact(&mut fixed)
        .await
        .map_err(|_| Refusal::Truncated)?;
    let mut block = vec![0u8; proxy::block_len(&fixed)?];
    stream
        .read_exact(&mut block)
        .await
        .map_err(|_| Refusal::Truncated)?;
    proxy::decode(&fixed, &block)
}

#[cfg(test)]
mod tests;
