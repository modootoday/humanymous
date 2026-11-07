//! Native TCP with the real shared client state. These are fixture controls,
//! not ordinary-browser, human, assistive-technology or whole-product evidence.
use super::*;
use crate::configuration::{BrowserPolicy, RitProfile};
use humanymous_authority::protocol::Response;
use humanymous_collector::session::{
    Request as ClientRequest, Scope as ClientScope, Session as ClientSession,
};
use humanymous_integrity::request::browser::PROTOCOL;

fn configured(origin: std::net::SocketAddr) -> Configuration {
    let mut config = configuration(origin);
    config.browser = Some(BrowserPolicy {
        origin: "http://owned.test".into(),
    });
    config.routes[0].rit_profile = RitProfile::BrowserAuthOnly;
    config.timeout_ms = 5000;
    config
}
fn scope() -> ClientScope {
    ClientScope {
        installation: "install".into(),
        tenant: "tenant".into(),
        origin: "origin".into(),
        revision: "r1".into(),
        authority: "owned.test".into(),
    }
}
fn browser_request(method: &str, target: &str, body: Vec<u8>) -> Request {
    let mut req = request(method, target, body);
    req.headers.extend([
        ("Origin".into(), "http://owned.test".into()),
        ("Sec-Fetch-Mode".into(), "same-origin".into()),
        ("Sec-Fetch-Dest".into(), "empty".into()),
    ]);
    if target != "/_humanymous/browser/session" {
        req.headers
            .push(("X-Humanymous-Protocol".into(), PROTOCOL.into()));
    }
    req
}
fn signed(client: &mut ClientSession, req: &mut Request, now: u64) -> u64 {
    let signed = client
        .begin(
            &ClientRequest {
                method: &req.method,
                target: &req.target,
                content_type: req.header("content-type").unwrap_or(""),
                authorization: req.header("authorization").unwrap_or(""),
                body: &req.body,
            },
            now,
        )
        .unwrap();
    req.headers.extend([
        ("X-Humanymous-Session".into(), signed.session().into()),
        ("X-Humanymous-Counter".into(), signed.counter.to_string()),
        ("X-Humanymous-Bucket".into(), signed.bucket.to_string()),
        ("X-Humanymous-Deadline".into(), signed.deadline.to_string()),
        ("X-Humanymous-Token".into(), signed.token().into()),
    ]);
    signed.counter
}
fn header<'a>(response: &'a Response, name: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}
fn complete(client: &mut ClientSession, response: &Response) {
    client
        .complete(
            header(response, "x-humanymous-protocol").unwrap(),
            header(response, "x-humanymous-counter")
                .unwrap()
                .parse()
                .unwrap(),
            response.status,
            header(response, "x-humanymous-ack").unwrap(),
            humanymous_authority::now_ms().unwrap(),
        )
        .unwrap();
}
async fn call(address: std::net::SocketAddr, req: &Request) -> Response {
    let mut stream = TcpStream::connect(address).await.unwrap();
    stream.write_all(&encode(req)).await.unwrap();
    humanymous_authority::protocol::read_response(&mut stream)
        .await
        .unwrap()
}

/// Hosted-only native sockets/shared client, not browser or durable-audit proof.
#[tokio::test]
async fn origin_failure_acknowledges_consumed_counter_without_claiming_zero_effects() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = configured(origin.local_addr().unwrap());
    let effects = Arc::new(AtomicUsize::new(0));
    let origin_effects = effects.clone();
    let origin_task = tokio::spawn(async move {
        let (mut socket, _) = origin.accept().await.unwrap();
        let request = crate::http::read(&mut socket).await.unwrap();
        assert_eq!(request.body, b"original bytes");
        origin_effects.fetch_add(1, Ordering::SeqCst);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nx")
            .await
            .unwrap();
        socket.shutdown().await.unwrap();
    });
    let audit = Audit::default();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Application::new(config, Default::default(), Box::new(audit.clone())).unwrap();
    let server = tokio::spawn(app.serve(listener));
    let bootstrap = call(
        address,
        &browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec()),
    )
    .await;
    assert_eq!(bootstrap.status, 200);
    let mut client = ClientSession::from_json(
        scope(),
        &bootstrap.body,
        humanymous_authority::now_ms().unwrap(),
        5000,
    )
    .unwrap();
    let mut collect = browser_request("POST", "/_humanymous/browser/collect", client_report(""));
    signed(
        &mut client,
        &mut collect,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &collect).await;
    assert_eq!(response.status, 200);
    complete(&mut client, &response);
    let mut protected = browser_request("POST", "/effect", b"original bytes".to_vec());
    signed(
        &mut client,
        &mut protected,
        humanymous_authority::now_ms().unwrap(),
    );
    let failed = call(address, &protected).await;
    assert_eq!(failed.status, 503);
    complete(&mut client, &failed);
    assert!(!client.is_closed());
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    assert_eq!(audit.records.lock().unwrap().len(), 2);
    assert_eq!(call(address, &protected).await.status, 409);
    let mut collect = browser_request("POST", "/_humanymous/browser/collect", client_report(""));
    signed(
        &mut client,
        &mut collect,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &collect).await;
    assert_eq!(response.status, 200);
    complete(&mut client, &response);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    origin_task.await.unwrap();
    server.abort();
    let _ = server.await;
}

/// Hosted-only socket boundary: a resource429 still carries the consumed ACK.
#[tokio::test]
async fn authenticated_session_quota_consumes_counter_and_preserves_ack_progression() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = configured(origin.local_addr().unwrap());
    config.session_rate_per_second = 1;
    let audit = Audit::default();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Application::new(config, Default::default(), Box::new(audit.clone())).unwrap();
    let server = tokio::spawn(app.serve(listener));
    let response = call(
        address,
        &browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec()),
    )
    .await;
    assert_eq!(response.status, 200);
    let mut client = ClientSession::from_json(
        scope(),
        &response.body,
        humanymous_authority::now_ms().unwrap(),
        5000,
    )
    .unwrap();
    let mut collect = browser_request("POST", "/_humanymous/browser/collect", client_report(""));
    signed(
        &mut client,
        &mut collect,
        humanymous_authority::now_ms().unwrap(),
    );
    // Invalid proof cannot debit another session's quota or consume its counter.
    let mut invalid = Request {
        method: collect.method.clone(),
        target: collect.target.clone(),
        headers: collect.headers.clone(),
        body: collect.body.clone(),
    };
    invalid
        .headers
        .iter_mut()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-humanymous-token"))
        .unwrap()
        .1 = "invalid".into();
    let rejected = call(address, &invalid).await;
    assert_eq!(rejected.status, 403);
    assert!(header(&rejected, "x-humanymous-ack").is_none());
    let response = call(address, &collect).await;
    assert_eq!(response.status, 200);
    complete(&mut client, &response);
    let mut protected = browser_request("POST", "/effect", b"original bytes".to_vec());
    signed(
        &mut client,
        &mut protected,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &protected).await;
    assert_eq!(response.status, 429);
    complete(&mut client, &response);
    assert!(!client.is_closed());
    assert_eq!(call(address, &protected).await.status, 409);
    assert_eq!(audit.records.lock().unwrap().len(), 1);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), origin.accept())
            .await
            .is_err()
    );
    server.abort();
    let _ = server.await;
}

#[test]
fn explicit_profile_origin_cookie_and_report_binding_negatives() {
    let cfg = configured("127.0.0.1:32123".parse().unwrap());
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    let bootstrap = browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec());
    let Decision::Local(200, body) = app.decide(&bootstrap, || Ok(10_000)).unwrap() else {
        panic!("bootstrap");
    };
    let mut client = ClientSession::from_json(scope(), &body, 10_000, 1000).unwrap();
    let mut collect = browser_request(
        "POST",
        "/_humanymous/browser/collect",
        client_report("wrong-session"),
    );
    signed(&mut client, &mut collect, 10_000);
    assert_eq!(
        app.decide(&collect, || Ok(10_000)).err().unwrap().code,
        "report_session"
    );
    // A rejected post-consumption report never restores its consumed counter.
    assert_eq!(
        app.decide(&collect, || Ok(10_000)).err().unwrap().status,
        409
    );
    for mutation in 0..4 {
        let mut req = browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec());
        match mutation {
            0 => req
                .headers
                .push(("Cookie".into(), "path-scoped=credential".into())),
            1 => {
                req.headers
                    .iter_mut()
                    .find(|(n, _)| n == "Origin")
                    .unwrap()
                    .1 = "null".into()
            }
            2 => {
                req.headers
                    .iter_mut()
                    .find(|(n, _)| n == "Sec-Fetch-Mode")
                    .unwrap()
                    .1 = "cors".into()
            }
            _ => req
                .headers
                .push(("X-Humanymous-Protocol".into(), "unknown/0.1".into())),
        }
        assert!(
            app.decide(&req, || Ok(10_000)).is_err(),
            "mutation {mutation}"
        );
    }
    let mut req = browser_request("GET", "/effect", vec![]);
    req.headers
        .retain(|(n, _)| !n.eq_ignore_ascii_case("origin"));
    assert!(
        crate::browser::validate(&configured("127.0.0.1:32123".parse().unwrap()), &req).is_ok()
    );
}

#[tokio::test]
async fn real_shared_client_consumption_ack_and_origin_fidelity() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_address = origin.local_addr().unwrap();
    let effects = Arc::new(AtomicUsize::new(0));
    let observed = effects.clone();
    let audit = Audit::default();
    let observed_audit = audit.clone();
    let origin_task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = origin.accept().await.unwrap();
            let req = crate::http::read(&mut socket).await.unwrap();
            assert_eq!(
                req.header("authorization"),
                Some("Bearer application-credential")
            );
            assert!(req.header("x-humanymous-session").is_none());
            assert!(req.header("x-humanymous-token").is_none());
            assert!(
                observed_audit
                    .records
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|record| record.verdict == "ALLOW")
            );
            observed.fetch_add(1, Ordering::SeqCst);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nX-Humanymous-Ack: forged-origin-value\r\n\r\n{\"ok\":true}").await.unwrap();
        }
    });
    let cfg = configured(origin_address);
    let listener = TcpListener::bind(cfg.listen).await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Application::new(cfg, Default::default(), Box::new(audit)).unwrap();
    let server = tokio::spawn(app.serve(listener));
    let response = call(
        address,
        &browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec()),
    )
    .await;
    assert_eq!(response.status, 200);
    assert!(header(&response, "x-humanymous-ack").is_none());
    assert_eq!(
        serde_json::from_slice::<Value>(&response.body)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        10
    );
    let mut client = ClientSession::from_json(
        scope(),
        &response.body,
        humanymous_authority::now_ms().unwrap(),
        1000,
    )
    .unwrap();
    let mut req = browser_request("POST", "/effect?exact=1", b"original bytes".to_vec());
    req.headers.push((
        "Authorization".into(),
        "Bearer application-credential".into(),
    ));
    signed(
        &mut client,
        &mut req,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &req).await;
    assert_eq!(response.status, 403);
    assert_eq!(
        serde_json::from_slice::<Value>(&response.body).unwrap()["verdict"],
        "CHALLENGE"
    );
    complete(&mut client, &response);
    assert_eq!(effects.load(Ordering::SeqCst), 0);

    // Empty report sessionId is server-bound; no capability getter is needed.
    let mut req = browser_request("POST", "/_humanymous/browser/collect", client_report(""));
    signed(
        &mut client,
        &mut req,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &req).await;
    assert_eq!(response.status, 200);
    complete(&mut client, &response);
    let mut req = browser_request("POST", "/effect?exact=1", b"original bytes".to_vec());
    req.headers.push((
        "Authorization".into(),
        "Bearer application-credential".into(),
    ));
    signed(
        &mut client,
        &mut req,
        humanymous_authority::now_ms().unwrap(),
    );
    let response = call(address, &req).await;
    assert_eq!(response.status, 200);
    complete(&mut client, &response);
    assert_eq!(response.body, b"{\"ok\":true}");
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    assert_eq!(
        response
            .headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case("set-cookie"))
            .count(),
        2
    );
    assert_eq!(
        response
            .headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case("x-humanymous-ack"))
            .count(),
        1
    );
    assert_ne!(
        header(&response, "x-humanymous-ack"),
        Some("forged-origin-value")
    );
    let replay = call(address, &req).await;
    assert_eq!(replay.status, 409);
    assert!(header(&replay, "x-humanymous-ack").is_none());
    assert_eq!(effects.load(Ordering::SeqCst), 1);

    let mut req = browser_request("POST", "/effect", b"original bytes".to_vec());
    let counter = signed(
        &mut client,
        &mut req,
        humanymous_authority::now_ms().unwrap(),
    );
    req.headers
        .push(("Cookie".into(), "scoped=unsupported".into()));
    let response = call(address, &req).await;
    assert_eq!(response.status, 422);
    assert!(header(&response, "x-humanymous-ack").is_none());
    assert!(
        client
            .complete(
                "",
                counter,
                response.status,
                "",
                humanymous_authority::now_ms().unwrap()
            )
            .is_err()
    );
    assert!(client.is_closed());
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    server.abort();
    origin_task.abort();
}
