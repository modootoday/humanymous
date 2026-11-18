//! Operator IP reputation in the serving path: only the datacenter bit reaches
//! the frozen engine, shadow categories never change a verdict, and the client
//! address comes from the trusted edge's PROXY v2 header, never from HTTP.
use super::*;
use crate::network::intel::{Classification, IntelReader};
use std::net::IpAddr;

/// Fixed classification regardless of address; records what it was asked.
struct Fixed(Classification, Mutex<Vec<Option<IpAddr>>>);
impl IntelReader for Fixed {
    fn classify(&self, client: Option<IpAddr>, _: u64) -> Classification {
        self.1.lock().unwrap().push(client);
        self.0
    }
}

fn verdict_with(classification: Option<Classification>) -> (String, Vec<Record>) {
    let cfg = config();
    let audit = Audit::default();
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    if let Some(classification) = classification {
        app.install_intel(Arc::new(Fixed(classification, Mutex::new(Vec::new()))));
    }
    let issued = issue(&mut app, 10_000);
    collected(&mut app, &cfg, &issued, 10_000);
    let req = protected(&cfg, &issued, 2, 10_000);
    let verdict = match app.decide(&req, || Ok(10_000)).unwrap() {
        Decision::Origin { .. } => "ALLOW".to_owned(),
        Decision::Local(_, body) => serde_json::from_slice::<Value>(&body).unwrap()["verdict"]
            .as_str()
            .unwrap()
            .to_owned(),
    };
    let records = audit.records.lock().unwrap().clone();
    (verdict, records)
}

#[test]
fn datacenter_classification_challenges_and_shadow_categories_change_nothing() {
    let (baseline, _) = verdict_with(None);
    assert_eq!(baseline, "ALLOW");
    let (shadow, _) = verdict_with(Some(Classification {
        datacenter: false,
        tor_exit: true,
        blocklist: true,
    }));
    assert_eq!(
        shadow, baseline,
        "shadow-only categories must not change a verdict"
    );
    let (datacenter, records) = verdict_with(Some(Classification {
        datacenter: true,
        ..Default::default()
    }));
    assert_eq!(datacenter, "CHALLENGE");
    let decision = records
        .iter()
        .rfind(|r| r.event_type == "runtime.decision")
        .unwrap();
    assert_eq!(decision.verdict, "CHALLENGE");
    // The existing frozen hard rule, not a new one.
    assert_eq!(decision.triggered_rules, Some(vec!["HR-11".to_owned()]));
}

#[test]
fn ip_reputation_configuration_is_optional_and_reported() {
    let mut cfg = config();
    assert!(cfg.readiness()["ip_reputation"].is_null());
    cfg.ip_reputation = Some(
        serde_json::from_value(
            json!({"snapshot":"/var/lib/humanymous/ip-intel.json","client_address":"proxy_v2"}),
        )
        .unwrap(),
    );
    assert!(cfg.validate().is_ok());
    let readiness = cfg.readiness();
    assert_eq!(readiness["ip_reputation"]["client_address"], "proxy_v2");
    assert_eq!(
        readiness["ip_reputation"]["scoring_categories"],
        json!(["datacenter"])
    );
    cfg.ip_reputation.as_mut().unwrap().categories.clear();
    assert!(cfg.validate().is_err());
}

fn proxy_v2(source: [u8; 4]) -> Vec<u8> {
    let mut header = crate::network::proxy::SIGNATURE.to_vec();
    header.extend_from_slice(&[0x21, 0x11, 0x00, 0x0c]);
    header.extend_from_slice(&source);
    header.extend_from_slice(&[127, 0, 0, 1, 0x30, 0x39, 0x01, 0xbb]);
    header
}

#[tokio::test]
async fn proxy_v2_source_reaches_the_reader_and_missing_header_drops_connection() {
    let mut cfg = config();
    cfg.listen = "127.0.0.1:0".parse().unwrap();
    cfg.ip_reputation = Some(
        serde_json::from_value(
            json!({"snapshot":"/nonexistent/ip-intel.json","client_address":"proxy_v2"}),
        )
        .unwrap(),
    );
    let reader = Arc::new(Fixed(Classification::default(), Mutex::new(Vec::new())));
    let server = TcpListener::bind(cfg.listen).await.unwrap();
    let address = server.local_addr().unwrap();
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    app.install_intel(reader.clone());
    let server_task = tokio::spawn(app.serve(server));

    // Session issuance is local and never classified; the header is still required.
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(&proxy_v2([192, 0, 2, 44])).await.unwrap();
    socket
        .write_all(&encode(&request("POST", "/_humanymous/session", vec![])))
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert!(
        bytes.starts_with(b"HTTP/1.1 200"),
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let issued = json_body(&bytes);

    let now = crate::authority::now_ms().unwrap();
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(&proxy_v2([192, 0, 2, 44])).await.unwrap();
    socket
        .write_all(&encode(&protected(&config(), &issued, 1, now)))
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(
        reader.1.lock().unwrap().as_slice(),
        [Some("192.0.2.44".parse().unwrap())]
    );

    // Without the PROXY v2 header the edge contract is violated: dropped, unclassified.
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(&encode(&request("POST", "/_humanymous/session", vec![])))
        .await
        .unwrap();
    let mut bytes = Vec::new();
    let _ = socket.read_to_end(&mut bytes).await;
    assert!(bytes.is_empty());
    assert_eq!(reader.1.lock().unwrap().len(), 1);
    server_task.abort();
}

fn proxy_v2_v6(source: [u8; 16]) -> Vec<u8> {
    let mut header = crate::network::proxy::SIGNATURE.to_vec();
    header.extend_from_slice(&[0x21, 0x21, 0x00, 0x24]);
    header.extend_from_slice(&source);
    header.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
    header.extend_from_slice(&[0x30, 0x39, 0x01, 0xbb]);
    header
}

/// LOCAL (`0x20`) or PROXY with UNSPEC family (`0x21`, `0x00`), no addresses.
fn proxy_v2_unattributed(command: u8) -> Vec<u8> {
    let mut header = crate::network::proxy::SIGNATURE.to_vec();
    header.extend_from_slice(&[command, 0x00, 0x00, 0x00]);
    header
}

fn edge_config(header_timeout_ms: u64, timeout_ms: u64) -> Configuration {
    let mut cfg = config();
    cfg.listen = "127.0.0.1:0".parse().unwrap();
    cfg.timeout_ms = timeout_ms;
    cfg.ip_reputation = Some(
        serde_json::from_value(json!({"snapshot":"/nonexistent/ip-intel.json",
            "client_address":"proxy_v2","proxy_header_timeout_ms":header_timeout_ms}))
        .unwrap(),
    );
    cfg
}

/// Serve `cfg` with a recording reader; returns the address, the reader and
/// the server task.
async fn edge_server(
    cfg: Configuration,
) -> (
    std::net::SocketAddr,
    Arc<Fixed>,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let reader = Arc::new(Fixed(Classification::default(), Mutex::new(Vec::new())));
    let server = TcpListener::bind(cfg.listen).await.unwrap();
    let address = server.local_addr().unwrap();
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    app.install_intel(reader.clone());
    (address, reader, tokio::spawn(app.serve(server)))
}

/// One connection: raw prefix bytes, then a request or end of stream.
async fn edge_exchange(
    address: std::net::SocketAddr,
    prefix: &[u8],
    request: Option<&Request>,
) -> Vec<u8> {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(prefix).await.unwrap();
    match request {
        Some(request) => socket.write_all(&encode(request)).await.unwrap(),
        // A truncated header ends the stream instead of waiting for a deadline.
        None => socket.shutdown().await.unwrap(),
    }
    let mut bytes = Vec::new();
    let _ = socket.read_to_end(&mut bytes).await;
    bytes
}

#[tokio::test]
async fn malformed_or_truncated_proxy_v2_headers_drop_only_that_connection() {
    let (address, reader, server_task) = edge_server(edge_config(1_000, 1_000)).await;
    let session = request("POST", "/_humanymous/session", vec![]);
    let signature = crate::network::proxy::SIGNATURE;
    let mut wrong_signature = signature.to_vec();
    wrong_signature[0] = 0;
    wrong_signature.extend_from_slice(&[0x21, 0x11, 0x00, 0x00]);
    let mut wrong_version = signature.to_vec();
    wrong_version.extend_from_slice(&[0x11, 0x11, 0x00, 0x0c]);
    let mut short_address = signature.to_vec();
    short_address.extend_from_slice(&[0x21, 0x11, 0x00, 0x04, 192, 0, 2, 1]);
    let mut truncated_block = proxy_v2([192, 0, 2, 44]);
    truncated_block.truncate(20);
    for (name, prefix, with_request) in [
        ("wrong signature", wrong_signature, true),
        ("wrong version", wrong_version, true),
        ("short address block", short_address, true),
        ("truncated fixed part", signature[..8].to_vec(), false),
        ("truncated address block", truncated_block, false),
    ] {
        let bytes = edge_exchange(address, &prefix, with_request.then_some(&session)).await;
        assert!(
            bytes.is_empty(),
            "{name}: {}",
            String::from_utf8_lossy(&bytes)
        );
    }
    // The listener keeps serving a well-formed edge connection afterwards.
    let bytes = edge_exchange(address, &proxy_v2([192, 0, 2, 44]), Some(&session)).await;
    assert!(bytes.starts_with(b"HTTP/1.1 200"));
    assert!(
        reader.1.lock().unwrap().is_empty(),
        "nothing was classified"
    );
    server_task.abort();
}

#[tokio::test]
async fn local_unspec_and_ipv6_headers_reach_the_reader_as_declared() {
    let cfg = edge_config(1_000, 1_000);
    let (address, reader, server_task) = edge_server(cfg).await;
    let session = request("POST", "/_humanymous/session", vec![]);
    let v6: std::net::Ipv6Addr = "2001:db8::44".parse().unwrap();
    let headers = [
        proxy_v2_unattributed(0x20),
        proxy_v2_unattributed(0x21),
        proxy_v2_v6(v6.octets()),
    ];
    for header in &headers {
        // A fresh session per header, so each protected request is its first.
        let issued = json_body(&edge_exchange(address, header, Some(&session)).await);
        let now = crate::authority::now_ms().unwrap();
        let bytes = edge_exchange(
            address,
            header,
            Some(&protected(&config(), &issued, 1, now)),
        )
        .await;
        assert!(!bytes.is_empty());
    }
    assert_eq!(
        reader.1.lock().unwrap().as_slice(),
        [None, None, Some(IpAddr::V6(v6))]
    );
    server_task.abort();
}

#[tokio::test]
async fn silent_edge_is_bounded_by_the_header_deadline() {
    // A long request deadline, a short header deadline: a silent peer cannot
    // hold the serialized accept loop for the request deadline.
    let (address, _reader, server_task) = edge_server(edge_config(100, 30_000)).await;
    let silent = TcpStream::connect(address).await.unwrap();
    let started = std::time::Instant::now();
    let session = request("POST", "/_humanymous/session", vec![]);
    let bytes = edge_exchange(address, &proxy_v2([192, 0, 2, 44]), Some(&session)).await;
    assert!(bytes.starts_with(b"HTTP/1.1 200"));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "served after {:?}",
        started.elapsed()
    );
    let mut silent = silent;
    let mut dropped = Vec::new();
    let _ = silent.read_to_end(&mut dropped).await;
    assert!(dropped.is_empty());
    server_task.abort();
}

#[tokio::test]
async fn shutdown_is_observed_while_waiting_for_the_header() {
    let cfg = edge_config(30_000, 30_000);
    let server = TcpListener::bind(cfg.listen).await.unwrap();
    let address = server.local_addr().unwrap();
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    let (stop, shutdown) = tokio::sync::watch::channel(None);
    let server_task = tokio::spawn(async move { app.serve_until(server, shutdown).await });
    let _silent = TcpStream::connect(address).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let started = std::time::Instant::now();
    stop.send(Some(
        crate::gateway::service::ShutdownRequest::new(1_000).unwrap(),
    ))
    .unwrap();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
        .await
        .expect("shutdown must not wait for the edge header deadline")
        .unwrap();
    assert!(outcome.is_ok());
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[tokio::test]
async fn snapshot_store_classifies_proxy_v2_clients_end_to_end() {
    use crate::network::intel::{
        IntelCategory,
        prefix_set::Prefix,
        snapshot::{self, Origin, Snapshot, SourceRecord, SourceStatus},
        store::IntelStore,
    };
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin.local_addr().unwrap();
    let effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = effects.clone();
    let origin_task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = origin.accept().await.unwrap();
            let _ = crate::gateway::http::read(&mut socket).await.unwrap();
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await
                .unwrap();
        }
    });
    let now_s = crate::authority::now_ms().unwrap() / 1000;
    let path = std::env::temp_dir().join(format!(
        "humanymous-gateway-ip-intel-{}.json",
        std::process::id()
    ));
    let bytes = snapshot::encode(&Snapshot::new(
        now_s,
        vec![SourceRecord {
            name: "test-datacenter".into(),
            category: IntelCategory::Datacenter,
            licence: "synthetic test data".into(),
            origin: Origin::File { read_at: now_s },
            status: SourceStatus::Ok,
            detail: String::new(),
            prefixes: vec![Prefix::parse_canonical("192.0.2.0/24").unwrap()],
        }],
    ))
    .unwrap();
    std::fs::write(&path, bytes).unwrap();

    let edge = || {
        let mut cfg = super::configuration(origin_addr);
        cfg.ip_reputation = Some(
            serde_json::from_value(json!({"snapshot":path,"client_address":"proxy_v2"})).unwrap(),
        );
        cfg
    };
    let cfg = edge();
    let policy = cfg.ip_reputation.as_ref().unwrap().store_policy();
    let store = Arc::new(IntelStore::open(policy, now_s));
    assert!(store.status(now_s).datacenter_scoring_active);
    let server = TcpListener::bind(cfg.listen).await.unwrap();
    let address = server.local_addr().unwrap();
    let mut app = Application::new(edge(), Default::default(), Box::new(Audit::default())).unwrap();
    app.install_intel(store.clone());
    let server_task = tokio::spawn(app.serve(server));

    let residential = proxy_v2([198, 51, 100, 1]);
    let session = request("POST", "/_humanymous/session", vec![]);
    let issued = json_body(&edge_exchange(address, &residential, Some(&session)).await);
    let now = crate::authority::now_ms().unwrap();
    let mut collect = request(
        "POST",
        "/_humanymous/collect",
        super::client_report(issued["session"].as_str().unwrap()),
    );
    super::sign(
        &cfg,
        &mut collect,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        now,
    );
    assert!(
        edge_exchange(address, &residential, Some(&collect))
            .await
            .starts_with(b"HTTP/1.1 200")
    );
    // Not in the snapshot: the request reaches the origin.
    let allowed = edge_exchange(
        address,
        &residential,
        Some(&protected(&cfg, &issued, 2, now)),
    )
    .await;
    assert!(
        allowed.starts_with(b"HTTP/1.1 200"),
        "{}",
        String::from_utf8_lossy(&allowed)
    );
    assert_eq!(effects.load(std::sync::atomic::Ordering::SeqCst), 1);
    // Same session from a datacenter range: the frozen HR-11 challenges it.
    let datacenter = proxy_v2([192, 0, 2, 44]);
    let challenged = edge_exchange(
        address,
        &datacenter,
        Some(&protected(&cfg, &issued, 3, now)),
    )
    .await;
    assert_eq!(json_body(&challenged)["verdict"], "CHALLENGE");
    assert_eq!(effects.load(std::sync::atomic::Ordering::SeqCst), 1);
    let status = store.status(now_s);
    assert_eq!(status.lookups.datacenter_inputs, 1);
    assert_eq!(status.lookups.lookups, 2);
    server_task.abort();
    origin_task.abort();
    let _ = std::fs::remove_file(&path);
}
