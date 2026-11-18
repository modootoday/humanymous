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
