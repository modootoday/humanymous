//! Bounded TLS/H1 consumer; all request authority stays in Application.
use super::*;
use crate::{service::ShutdownRequest, tls::CapturedRequest};
use rustls::ServerConfig;
use std::{io, sync::Arc};
use tokio::sync::watch;

impl Application {
    pub(crate) fn validate_tls_h1(&self, config: &ServerConfig) -> io::Result<()> {
        crate::tls::validate_configuration(&self.config, config)
    }

    /// Same serialized application owner with an explicitly injected TLS server.
    /// No plaintext fallback, H2, early data, or native streaming inference.
    pub async fn serve_tls_until(
        &mut self,
        listener: TcpListener,
        tls: Arc<ServerConfig>,
        mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        self.validate_tls_h1(&tls)?;
        if !listener.local_addr()?.ip().is_loopback() {
            return Err(io::Error::other("loopback only"));
        }
        let mut admission = Admission::new(self.config.rate_per_second);
        let mut commands = self.take_settings_commands();
        let mut admission_rate = self.config.rate_per_second;
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
                accepted = listener.accept() => accepted?,
            };
            if admission_rate != self.config.rate_per_second {
                admission_rate = self.config.rate_per_second;
                admission = Admission::new(admission_rate);
            }
            if !peer.ip().is_loopback() || !admission.admit() {
                continue;
            }
            let budget = Duration::from_millis(self.config.timeout_ms);
            let until = Instant::now() + budget;
            let connection_shutdown = shutdown.clone();
            let pending = timeout_at(until, async {
                let captured =
                    CapturedRequest::receive(stream.into_std()?, tls.clone(), budget).await?;
                self.captured_connection(captured, until, &connection_shutdown)
                    .await
            });
            tokio::pin!(pending);
            let stopping = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                    let request = *shutdown.borrow();
                    if let Some(request) = request {
                        tokio::time::timeout(request.drain_timeout(), &mut pending).await
                            .map_err(|_| io::Error::other("TLS shutdown drain incomplete"))?
                            .map_err(|_| io::Error::other("TLS request deadline incomplete"))??;
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

    async fn captured_connection(
        &mut self,
        mut captured: CapturedRequest,
        until: Instant,
        shutdown: &watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        let mut ack = None;
        let response = self
            .captured_response(&mut captured, until, shutdown, &mut ack)
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

    async fn captured_response(
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
            self.decide_observed(request, humanymous_authority::now_ms, ack, Some(connection))?;
        connection
            .check_h1_idle()
            .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
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
            Decision::Origin { deadline } => {
                self.check_deadline(deadline, humanymous_authority::now_ms())?;
                let remaining = deadline
                    .checked_sub(humanymous_authority::now_ms()?)
                    .filter(|value| *value > 0)
                    .ok_or(Rejection::new(503, "deadline"))?;
                let until = until.min(Instant::now() + Duration::from_millis(remaining));
                timeout_at(until, async {
                    if shutdown.borrow().is_some() {
                        return Err(Rejection::new(503, "service_stopping"));
                    }
                    let mut upstream = TcpStream::connect(self.config.origin)
                        .await
                        .map_err(|_| Rejection::new(503, "origin_connect"))?;
                    // Recheck after both synchronous durable audit and async connect.
                    connection
                        .check_h1_idle()
                        .map_err(|_| Rejection::new(400, "tls_client_closed"))?;
                    self.check_deadline(deadline, humanymous_authority::now_ms())?;
                    if shutdown.borrow().is_some() || Instant::now() >= until {
                        return Err(Rejection::new(503, "service_stopping"));
                    }
                    upstream
                        .write_all(&http::origin_request(request, &self.config.scope.authority))
                        .await
                        .map_err(|_| Rejection::new(503, "origin_write"))?;
                    let mut bytes = Vec::new();
                    upstream
                        .take((http::MAX_ORIGIN_FRAME + 1) as u64)
                        .read_to_end(&mut bytes)
                        .await
                        .map_err(|_| Rejection::new(503, "origin_read"))?;
                    http::Response::parse(&bytes, request.method == "HEAD")
                })
                .await
                .map_err(|_| Rejection::new(503, "origin_timeout"))??
            }
        };
        let headers = ack
            .as_ref()
            .map(|value| value.headers(response.status))
            .transpose()?
            .unwrap_or_default();
        response.encode(&headers)
    }
}
