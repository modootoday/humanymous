//! Bounded TLS/H1 and R14 H2 consumers; all request authority stays in Application.
use super::*;
use crate::gateway::{
    configuration::exposure,
    service::ShutdownRequest,
    tls::{CapturedH2Request, CapturedRequest, CapturedTls},
};
use rustls::ServerConfig;
use std::{io, sync::Arc};
use tokio::sync::watch;

impl Application {
    pub(in crate::gateway) fn validate_tls_h1(&self, config: &ServerConfig) -> io::Result<()> {
        crate::gateway::tls::validate_configuration(&self.config, config)
    }

    /// Same serialized application owner with an explicitly injected TLS server.
    /// No plaintext fallback, early data, or native streaming inference.
    pub async fn serve_tls_until(
        &mut self,
        listener: TcpListener,
        tls: Arc<ServerConfig>,
        mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        self.validate_tls_h1(&tls)?;
        // Owner decision B01 #1: only an explicitly public configuration (a
        // validated non-loopback listen with TLS) may serve a non-loopback
        // listener and its non-loopback peers.
        let public = self.config.public_listen;
        if !exposure::bound_permitted(listener.local_addr()?, public) {
            return Err(io::Error::other("loopback only"));
        }
        if public {
            return self.serve_public_tls(listener, tls, shutdown).await;
        }
        let mut admission = Admission::new(self.config.rate_per_second);
        let mut commands = self.take_settings_commands();
        let mut admission_rate = self.config.rate_per_second;
        let mut sweep = maintenance::ticker(self.sweep_interval);
        loop {
            if shutdown.borrow().is_some() {
                return Ok(());
            }
            let (stream, peer) = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                    continue;
                }
                command = settings::next_command(&mut commands) => {
                    self.apply_settings_command(command, &shutdown)?;
                    continue;
                }
                _ = sweep.tick() => {
                    self.maintenance_tick();
                    continue;
                }
                accepted = listener.accept() => accepted?,
            };
            if admission_rate != self.config.rate_per_second {
                admission_rate = self.config.rate_per_second;
                admission = Admission::new(admission_rate);
            }
            if !exposure::peer_admitted(peer, public) || !admission.admit() {
                continue;
            }
            let budget = Duration::from_millis(self.config.timeout_ms);
            let until = Instant::now() + budget;
            let mut stream = stream;
            self.client = None;
            match edge_client(self.edge_policy(), &mut stream, until, &mut shutdown).await? {
                Edge::Client(client) => self.client = client,
                // The trusted edge contract was violated; drop this connection only.
                Edge::Drop => continue,
                // No TLS bytes were read yet, so there is nothing to drain.
                Edge::Stop => return Ok(()),
            }
            let connection_shutdown = shutdown.clone();
            let pending = timeout_at(until, async {
                let captured =
                    CapturedTls::receive(stream.into_std()?, tls.clone(), budget).await?;
                self.captured_tls_connection(captured, until, &connection_shutdown)
                    .await
            });
            tokio::pin!(pending);
            let stopping = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                    let request = *shutdown.borrow();
                    if let Some(request) = request {
                        // Drain and deadline expiry stay failures. A drained connection's
                        // own refusal is that connection's outcome, as in plaintext serving.
                        let _connection_outcome = tokio::time::timeout(request.drain_timeout(), &mut pending).await
                            .map_err(|_| io::Error::other("TLS shutdown drain incomplete"))?
                            .map_err(|_| io::Error::other("TLS request deadline incomplete"))?;
                        true
                    } else { let _ = pending.await; false }
                }
                _ = &mut pending => false,
            };
            if stopping {
                return Ok(());
            }
        }
    }

    /// Public listener (owner decision B01 #1): each admitted peer's TLS
    /// handshake and complete request read run in their own task, at most
    /// `public_receive.max_receiving` at once and each within the receive
    /// deadline, so a silent or slow peer holds one permit, not the listener.
    /// A peer beyond the bound is closed at once (fail closed, never queued).
    /// Received requests are decided and answered one at a time by this
    /// serialized owner, exactly as on the loopback listener; a received
    /// request keeps its permit until answered, which bounds the queue.
    /// Client address (plan B26): a peer outside `trusted_proxies` is its
    /// socket address and nothing is read from it; a listed balancer must
    /// send one PROXY v2 header first, inside the receive task and deadline
    /// (at most Go's 5 s), and its declared TCP source is the client. LOCAL,
    /// UNSPEC or a non-TCP family leave the request unattributed (Go kept the
    /// balancer address); a malformed, truncated or late header closes that
    /// connection only. The address keys bans, credential fan-out and the IP
    /// reputation lookup; it is never TLS or original-client provenance.
    async fn serve_public_tls(
        &mut self,
        listener: TcpListener,
        tls: Arc<ServerConfig>,
        mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        let trusted = self.config.trusted_proxy_allowlist().map(Arc::new);
        if self.edge_policy().is_some() && trusted.is_none() {
            return Err(io::Error::other("public PROXY v2 needs trusted_proxies"));
        }
        let bounds = self.config.public_receive;
        let permits = Arc::new(tokio::sync::Semaphore::new(bounds.max_receiving));
        let (received_sender, mut received) =
            tokio::sync::mpsc::channel::<Received>(bounds.max_receiving);
        let mut receiving = tokio::task::JoinSet::new();
        let mut admission = Admission::new(self.config.rate_per_second);
        let mut commands = self.take_settings_commands();
        let mut admission_rate = self.config.rate_per_second;
        let mut sweep = maintenance::ticker(self.sweep_interval);
        self.client = None;
        loop {
            while receiving.try_join_next().is_some() {}
            if shutdown.borrow().is_some() {
                return Ok(());
            }
            let next = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                    continue;
                }
                command = settings::next_command(&mut commands) => {
                    self.apply_settings_command(command, &shutdown)?;
                    continue;
                }
                _ = sweep.tick() => {
                    self.maintenance_tick();
                    continue;
                }
                Some(next) = received.recv() => next,
                accepted = crate::gateway::accept::accept(&listener) => {
                    let (stream, peer) = accepted?;
                    if admission_rate != self.config.rate_per_second {
                        admission_rate = self.config.rate_per_second;
                        admission = Admission::new(admission_rate);
                    }
                    if !exposure::peer_admitted(peer, true) || !admission.admit() {
                        continue;
                    }
                    // Over the bound the stream is dropped, closing it.
                    let Ok(permit) = permits.clone().try_acquire_owned() else {
                        continue;
                    };
                    let budget = Duration::from_millis(self.config.timeout_ms);
                    let until = Instant::now() + budget;
                    let deadline = Instant::now() + bounds.deadline(self.config.timeout_ms);
                    let (tls, sender) = (tls.clone(), received_sender.clone());
                    let trusted = trusted.clone();
                    receiving.spawn(async move {
                        let mut stream = stream;
                        let Some(client) = public_client(trusted.as_deref(), peer, &mut stream, deadline).await else {
                            return;
                        };
                        // The capture lifetime stays the connection budget, so
                        // a queued request keeps its original deadline.
                        let captured = timeout_at(deadline, async {
                            CapturedTls::receive(stream.into_std()?, tls, budget).await
                        })
                        .await;
                        if let Ok(Ok(captured)) = captured {
                            let _ = sender.send(Received { captured, until, client, _permit: permit }).await;
                        }
                    });
                    continue;
                }
            };
            if self.serve_received(next, &mut shutdown).await? {
                return Ok(());
            }
        }
    }

    /// Decide and answer one received public request under the shutdown
    /// drain rules of the loopback listener; true when the service stops.
    async fn serve_received(
        &mut self,
        received: Received,
        shutdown: &mut watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<bool> {
        let Received {
            captured,
            until,
            client,
            _permit,
        } = received;
        self.client = client;
        let connection_shutdown = shutdown.clone();
        let pending = timeout_at(
            until,
            self.captured_tls_connection(captured, until, &connection_shutdown),
        );
        tokio::pin!(pending);
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                let request = *shutdown.borrow();
                if let Some(request) = request {
                    let _connection_outcome = tokio::time::timeout(request.drain_timeout(), &mut pending).await
                        .map_err(|_| io::Error::other("TLS shutdown drain incomplete"))?
                        .map_err(|_| io::Error::other("TLS request deadline incomplete"))?;
                    Ok(true)
                } else {
                    let _ = pending.await;
                    Ok(false)
                }
            }
            _ = &mut pending => Ok(false),
        }
    }

    async fn captured_tls_connection(
        &mut self,
        captured: CapturedTls,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        match captured {
            CapturedTls::H1(captured) => {
                self.captured_h1_connection(captured, until, shutdown).await
            }
            CapturedTls::H2(captured) => {
                self.captured_h2_connection(captured, until, shutdown).await
            }
        }
    }

    async fn captured_h1_connection(
        &mut self,
        mut captured: CapturedRequest,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        let mut ack = None;
        let response = self
            .captured_h1_response(&mut captured, until, shutdown, &mut ack)
            .await;
        let bytes = match response {
            Ok(bytes) => bytes,
            Err(error) => {
                let headers = ack
                    .as_ref()
                    .map(|value| value.headers(error.status))
                    .transpose()
                    .map_err(|_| io::Error::other("TLS response integrity failed"))?
                    .unwrap_or_default();
                let body = serde_json::to_vec(&serde_json::json!({"error": error.code}))?;
                let mut bytes = Vec::new();
                http::write_headers(&mut bytes, error.status, &body, &headers).await?;
                bytes
            }
        };
        captured.respond(bytes).await
    }

    async fn captured_h2_connection(
        &mut self,
        mut captured: CapturedH2Request,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        let mut ack = None;
        let response = self
            .captured_h2_response(&mut captured, until, shutdown, &mut ack)
            .await;
        let bytes = match response {
            Ok(bytes) => bytes,
            Err(error) => {
                let headers = ack
                    .as_ref()
                    .map(|value| value.headers(error.status))
                    .transpose()
                    .map_err(|_| io::Error::other("TLS response integrity failed"))?
                    .unwrap_or_default();
                let body = serde_json::to_vec(&serde_json::json!({"error": error.code}))?;
                let mut bytes = Vec::new();
                http::write_headers(&mut bytes, error.status, &body, &headers).await?;
                bytes
            }
        };
        captured.respond(bytes).await
    }

    async fn captured_h1_response(
        &mut self,
        captured: &mut CapturedRequest,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
        ack: &mut Option<Ack>,
    ) -> Result<Vec<u8>, Rejection> {
        let (request, connection) = captured.parts();
        connection
            .check_h1_idle()
            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
        let decision =
            self.decide_observed(request, crate::authority::now_ms, ack, Some(connection))?;
        connection
            .check_h1_idle()
            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
        self.encode_decision(EncodeArgs {
            decision,
            request,
            connection,
            until,
            shutdown,
            ack,
            h1_idle: true,
        })
        .await
    }

    async fn captured_h2_response(
        &mut self,
        captured: &mut CapturedH2Request,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
        ack: &mut Option<Ack>,
    ) -> Result<Vec<u8>, Rejection> {
        captured
            .check_stream()
            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
        let decision = {
            let (request, connection, _) = captured.parts();
            self.decide_observed(request, crate::authority::now_ms, ack, Some(connection))?
        };
        captured
            .check_stream()
            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
        let (request, connection, _) = captured.parts();
        self.encode_decision(EncodeArgs {
            decision,
            request,
            connection,
            until,
            shutdown,
            ack,
            h1_idle: false,
        })
        .await
    }

    async fn encode_decision(&mut self, args: EncodeArgs<'_>) -> Result<Vec<u8>, Rejection> {
        let EncodeArgs {
            decision,
            request,
            connection,
            until,
            shutdown,
            ack,
            h1_idle,
        } = args;
        let response = match decision {
            Decision::Local(status, body) => {
                let body = zeroize::Zeroizing::new(body);
                let headers = ack
                    .as_ref()
                    .map(|value| value.headers(status))
                    .transpose()?
                    .unwrap_or_default();
                let mut bytes = Vec::new();
                http::write_headers(&mut bytes, status, &body, &headers)
                    .await
                    .map_err(|_| Rejection::new(503, "serialization"))?;
                return Ok(bytes);
            }
            Decision::Gate(gate) => {
                let head = request.method == "HEAD";
                let fields = gate.with_ack(ack.as_ref())?;
                return http::text_frame(gate.status, gate.body, &fields, head)
                    .map_err(|_| Rejection::new(503, "serialization"));
            }
            Decision::Probe(status, body) => {
                return Ok(http::probe_frame(status, body, request.method == "HEAD"));
            }
            Decision::Origin { deadline, upstream } => {
                let origin_auth = self.origin_auth()?;
                self.check_deadline(deadline, crate::authority::now_ms())?;
                let remaining = deadline
                    .checked_sub(crate::authority::now_ms()?)
                    .filter(|value| *value > 0)
                    .ok_or(Rejection::new(503, "deadline"))?;
                let until = until.min(Instant::now() + Duration::from_millis(remaining));
                let outcome = timeout_at(until, async {
                    if shutdown.borrow().is_some() {
                        return Err(Rejection::new(503, "service_stopping").into());
                    }
                    let mut origin_stream = TcpStream::connect(self.config.origin)
                        .await
                        .map_err(|error| OriginError::io("origin_connect", &error))?;
                    if h1_idle {
                        connection
                            .check_h1_idle()
                            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
                    } else {
                        connection
                            .remaining()
                            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
                    }
                    self.check_deadline(deadline, crate::authority::now_ms())?;
                    if shutdown.borrow().is_some() || Instant::now() >= until {
                        return Err(Rejection::new(503, "service_stopping").into());
                    }
                    origin_stream
                        .write_all(&http::origin_request(
                            request,
                            &self.config.scope.authority,
                            origin_auth.as_deref(),
                        ))
                        .await
                        .map_err(|error| OriginError::io("origin_write", &error))?;
                    let mut bytes = Vec::new();
                    origin_stream
                        .take((http::MAX_ORIGIN_FRAME + 1) as u64)
                        .read_to_end(&mut bytes)
                        .await
                        .map_err(|error| OriginError::io("origin_read", &error))?;
                    Ok::<_, OriginError>(http::Response::parse(&bytes, request.method == "HEAD")?)
                })
                .await
                .unwrap_or_else(|_| Err(Rejection::new(503, "origin_timeout").into()));
                // No TLS response byte precedes this encoded frame.
                outcome.map_err(|error| self.upstream_failure(&upstream, error, false))?
            }
        };
        let mut response = response;
        // TLS listener: HSTS only when configured (owner decision C10).
        response.secure(self.config.edge.hsts);
        let headers = ack
            .as_ref()
            .map(|value| value.headers(response.status))
            .transpose()?
            .unwrap_or_default();
        response.encode(&headers)
    }
}

/// One public request whose handshake and read completed in its own task.
struct Received {
    captured: CapturedTls,
    until: Instant,
    /// Client address of the connection (plan B26), `None` when unattributed.
    client: Option<std::net::IpAddr>,
    /// Held until the request is answered, bounding queued requests.
    _permit: tokio::sync::OwnedSemaphorePermit,
}

struct EncodeArgs<'a> {
    decision: Decision,
    request: &'a Request,
    connection: &'a mut crate::capture::tls::Connection,
    until: Instant,
    shutdown: &'a watch::Receiver<Option<ShutdownRequest>>,
    ack: &'a mut Option<Ack>,
    h1_idle: bool,
}

/// Plan B26 trust gate for one public connection. `None` means the connection
/// is closed (a listed balancer sent a malformed, truncated or late header);
/// `Some(None)` is served unattributed.
async fn public_client(
    trusted: Option<&crate::network::proxy::Allowlist>,
    peer: std::net::SocketAddr,
    stream: &mut TcpStream,
    deadline: Instant,
) -> Option<Option<std::net::IpAddr>> {
    use crate::network::proxy::Header;
    if !trusted.is_some_and(|allowlist| allowlist.trusts_peer(peer)) {
        return Some(Some(peer.ip()));
    }
    let deadline = deadline.min(Instant::now() + crate::gateway::proxy_listener::READ_TIMEOUT);
    match timeout_at(
        deadline,
        crate::gateway::proxy_listener::read_header(stream),
    )
    .await
    {
        Ok(Ok(Header::Source(source))) => Some(Some(source.ip())),
        Ok(Ok(Header::Local | Header::Unspecified)) => Some(None),
        Ok(Err(_)) | Err(_) => None,
    }
}
