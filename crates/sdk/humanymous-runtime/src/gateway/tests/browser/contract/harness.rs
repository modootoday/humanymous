//! Owned bounded TCP fixture; no replacement Fetch implementation or production API.
use super::*;
use crate::gateway::service::ShutdownRequest;
use std::time::Duration;

pub(super) fn check(value: bool, message: &'static str) -> Result<(), &'static str> {
    if value { Ok(()) } else { Err(message) }
}
pub(super) struct Harness {
    fixture: Value,
    address: std::net::SocketAddr,
    origin: TcpListener,
    stop: tokio::sync::watch::Sender<Option<ShutdownRequest>>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
    pub(super) audit: Audit,
}
impl Harness {
    pub(super) async fn start() -> Self {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/gateway/collector-pilot.json"
        ))
        .unwrap();
        let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut config = configured(origin.local_addr().unwrap());
        config.scope = serde_json::from_value(fixture["scope"].clone()).unwrap();
        config.browser.as_mut().unwrap().origin =
            fixture["browser_origin"].as_str().unwrap().into();
        config.routes = serde_json::from_value(json!([
            {"method":"POST","path":fixture["routes"]["effect"],"rit_profile":"browser_auth_only","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}},
            {"method":"POST","path":fixture["routes"]["required_collector"],"rit_profile":"browser_auth_only","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":true,"rit":true}}
        ])).unwrap();
        let audit = Audit::default();
        let mut app =
            Application::new(config, Default::default(), Box::new(audit.clone())).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopping) = tokio::sync::watch::channel(None);
        let task = tokio::spawn(async move { app.serve_until(listener, stopping).await });
        Self {
            fixture,
            address,
            origin,
            stop,
            task,
            audit,
        }
    }
    pub(super) fn request(&self, route: &str, body: Vec<u8>) -> Request {
        let mut request = browser_request(
            "POST",
            self.fixture["routes"][route].as_str().unwrap(),
            body,
        );
        for (name, value) in &mut request.headers {
            if name.eq_ignore_ascii_case("host") {
                *value = self.fixture["scope"]["authority"].as_str().unwrap().into();
            }
            if name.eq_ignore_ascii_case("origin") {
                *value = self.fixture["browser_origin"].as_str().unwrap().into();
            }
            if matches!(route, "effect" | "required_collector")
                && name.eq_ignore_ascii_case("content-type")
            {
                *value = "application/octet-stream".into();
            }
        }
        if matches!(route, "effect" | "required_collector") {
            request.headers.push((
                "Authorization".into(),
                "Bearer owned-browser-fixture".into(),
            ));
        }
        request
    }
    pub(super) async fn call(
        &self,
        request: &Request,
        header_only: bool,
    ) -> Result<Response, &'static str> {
        tokio::time::timeout(Duration::from_secs(4), async {
            let mut stream = TcpStream::connect(self.address)
                .await
                .map_err(|_| "connect failed")?;
            let mut bytes = encode(request);
            if header_only {
                let end = bytes
                    .windows(4)
                    .position(|part| part == b"\r\n\r\n")
                    .ok_or("request header boundary")?;
                bytes.truncate(end + 4);
            }
            stream.write_all(&bytes).await.map_err(|_| "write failed")?;
            crate::authority::protocol::read_response(&mut stream)
                .await
                .map_err(|_| "response failed")
        })
        .await
        .map_err(|_| "HTTP deadline")?
    }
    pub(super) async fn bootstrap(&self) -> Result<ClientSession, &'static str> {
        let response = self
            .call(&self.request("session", b"{}".to_vec()), false)
            .await?;
        check(response.status == 200, "bootstrap status")?;
        check(
            header(&response, "x-humanymous-ack").is_none(),
            "bootstrap ACK forbidden",
        )?;
        let body: Value = serde_json::from_slice(&response.body).map_err(|_| "bootstrap JSON")?;
        let mut actual: Vec<_> = body
            .as_object()
            .ok_or("bootstrap object")?
            .keys()
            .map(String::as_str)
            .collect();
        actual.sort_unstable();
        check(
            actual
                == [
                    "authority",
                    "counter",
                    "expires",
                    "installation",
                    "key",
                    "origin",
                    "protocol",
                    "revision",
                    "session",
                    "tenant",
                ],
            "bootstrap exact10 fields",
        )?;
        ClientSession::from_json(
            serde_json::from_value(self.fixture["scope"].clone()).map_err(|_| "fixture scope")?,
            &response.body,
            crate::authority::now_ms().map_err(|_| "clock")?,
            5000,
        )
        .map_err(|_| "strict shared bootstrap")
    }
    pub(super) fn complete(
        &self,
        client: &mut ClientSession,
        response: &Response,
    ) -> Result<(), &'static str> {
        client
            .complete(
                header(response, "x-humanymous-protocol").ok_or("missing protocol")?,
                header(response, "x-humanymous-counter")
                    .ok_or("missing counter")?
                    .parse()
                    .map_err(|_| "counter format")?,
                response.status,
                header(response, "x-humanymous-ack").ok_or("missing ACK")?,
                crate::authority::now_ms().map_err(|_| "clock")?,
            )
            .map_err(|_| "consumed ACK failed")
    }
    pub(super) fn refuse_without_ack(
        &self,
        client: &mut ClientSession,
        counter: u64,
        response: &Response,
    ) -> Result<(), &'static str> {
        check(
            header(response, "x-humanymous-ack").is_none(),
            "preconsume refusal carried ACK",
        )?;
        check(
            header(response, "x-humanymous-counter").is_none(),
            "preconsume refusal claimed counter",
        )?;
        check(
            client
                .complete(
                    PROTOCOL,
                    counter,
                    response.status,
                    "",
                    crate::authority::now_ms().map_err(|_| "clock")?,
                )
                .is_err(),
            "missing ACK accepted",
        )?;
        check(client.is_closed(), "ambiguous epoch remained open")
    }
    pub(super) async fn finish(self) -> Result<(), &'static str> {
        self.stop
            .send(Some(ShutdownRequest::new(100).map_err(|_| "stop budget")?))
            .map_err(|_| "stop unavailable")?;
        tokio::time::timeout(Duration::from_secs(4), self.task)
            .await
            .map_err(|_| "server join deadline")?
            .map_err(|_| "server panic")?
            .map_err(|_| "server drain")?;
        check(
            tokio::time::timeout(Duration::from_millis(30), self.origin.accept())
                .await
                .is_err(),
            "unexpected origin connection",
        )
    }
}
