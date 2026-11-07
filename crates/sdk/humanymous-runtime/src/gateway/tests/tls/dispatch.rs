//! Frozen policy outcomes and actual origin effects for explicit fixture identities.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chrome_claim_over_actual_rustls_is_audited_and_never_reaches_origin() {
    dispatch(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn truthful_native_tls_profile_preserves_policy_and_origin_agreement() {
    dispatch(true).await;
}

async fn dispatch(native_identity: bool) {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = configuration(origin.local_addr().unwrap());
    config.routes[0].required.raw_tls = true;
    let mut application_config = configuration(config.origin);
    application_config.routes[0].required.raw_tls = true;
    let audit = Audit::default();
    let mut app = Application::new(
        application_config,
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let now = crate::authority::now_ms().unwrap();
    let issued = issue(&mut app, now);
    let mut report: Value =
        serde_json::from_slice(&client_report(issued["session"].as_str().unwrap())).unwrap();
    if native_identity {
        // Explicit synthetic native identity, not a browser or human claim.
        report["userAgent"] = json!("HumanymousRuntimeFixture/0.1 Rustls");
        report.as_object_mut().unwrap().remove("uaClientHints");
    }
    let mut collect = request(
        "POST",
        "/_humanymous/collect",
        serde_json::to_vec(&report).unwrap(),
    );
    identify(&mut collect, native_identity);
    sign(
        &config,
        &mut collect,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        now,
    );
    assert!(matches!(
        app.decide(&collect, || Ok(now)).unwrap(),
        Decision::Local(200, _)
    ));
    let mut request = protected(&config, &issued, 2, now);
    identify(&mut request, native_identity);
    sign(
        &config,
        &mut request,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        2,
        now,
    );
    let observed = audit.clone();
    let upstream = tokio::spawn(async move {
        let Ok(accepted) = tokio::time::timeout(BUDGET, origin.accept()).await else {
            return false;
        };
        let (mut socket, _) = accepted.unwrap();
        let request = crate::gateway::http::read(&mut socket).await.unwrap();
        assert_eq!(request.body, b"original bytes");
        assert!(
            observed
                .records
                .lock()
                .unwrap()
                .iter()
                .any(|record| record.event_type == "runtime.decision" && record.verdict == "ALLOW")
        );
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .await
            .unwrap();
        true
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (server_tls, client_tls) = configurations();
    let (stop, stopping) = tokio::sync::watch::channel(None);
    let server =
        tokio::spawn(async move { app.serve_tls_until(listener, server_tls, stopping).await });
    let response = exchange_tls(address, client_tls, encode(&request)).await;
    stop.send(Some(ShutdownRequest::new(100).unwrap())).unwrap();
    tokio::time::timeout(BUDGET, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let origin_effect = upstream.await.unwrap();
    // All owned workers are joined before inspecting response/audit diagnostics.
    let records = audit.records.lock().unwrap();
    assert_eq!(records.len(), 2);
    let decision = &records[1];
    assert_eq!(decision.event_type, "runtime.decision");
    let status = String::from_utf8_lossy(response.split(|byte| *byte == b'\n').next().unwrap());
    eprintln!(
        "TLS fixture native={native_identity} status={} verdict={} risk={} origin_effect={origin_effect}",
        status.trim(),
        decision.verdict,
        decision.risk_score
    );
    if !native_identity {
        assert!(
            response.starts_with(b"HTTP/1.1 403"),
            "unexpected TLS policy response: {status}"
        );
        assert_ne!(decision.verdict, "ALLOW");
        assert!(!origin_effect);
    } else if decision.verdict == "ALLOW" {
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(response.ends_with(b"ok"));
        assert!(origin_effect);
    } else {
        assert!(response.starts_with(b"HTTP/1.1 403"));
        assert!(!origin_effect);
        assert!(matches!(decision.verdict.as_str(), "DENY" | "CHALLENGE"));
        // A passing policy agreement check does not qualify an ALLOW-to-origin
        // TLS pilot. Evidence must retain this actual refusal and open gate.
    }
    // NativeFull consumes RIT but has no BrowserAuthOnly response ACK contract.
    // Do not fabricate an ACK or relabel this fixture as a browser exchange.
    assert!(
        !String::from_utf8_lossy(&response)
            .to_ascii_lowercase()
            .contains("x-humanymous-ack:")
    );
}

fn identify(request: &mut Request, native: bool) {
    if native {
        request.headers.retain(|(name, _)| {
            !name.eq_ignore_ascii_case("sec-ch-ua") && !name.eq_ignore_ascii_case("sec-fetch-site")
        });
        let (_, user_agent) = request
            .headers
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
            .unwrap();
        *user_agent = "HumanymousRuntimeFixture/0.1 Rustls".into();
    }
}
