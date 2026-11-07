//! Hosted-only actual Unix IPC/listener/custody boundaries, not pure local units.
use super::*;
use crate::{
    management::{ManagementPolicy, ManagementScope},
    service::{ExitStage, ServiceHost, StopBudget},
};
use humanymous_audit::keys::{
    KeyMaterial,
    custody::{Custody, Owner},
};
use humanymous_authority::{configuration::Peer, credentials::Credential};
use std::{
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::PathBuf,
};

struct PrivateFixture(PathBuf);
impl PrivateFixture {
    fn new() -> Self {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).unwrap();
        let path = PathBuf::from("/tmp")
            .join(format!("humanymous-runtime-service-{}", hex::encode(nonce)));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        for name in ["keys", "audit"] {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(path.join(name))
                .unwrap();
        }
        let mut custody = Custody::acquire(&path.join("keys"), Owner::CurrentUser).unwrap();
        custody
            .load_or_create(b"unit-fixture", || {
                KeyMaterial::new([7; 32], vec![9; 32], None, None)
            })
            .unwrap();
        Self(path)
    }
    fn configuration(&self) -> Configuration {
        let mut config = config();
        config.audit.directory = self.0.join("audit");
        config.audit.keys.directory = self.0.join("keys");
        config.owner = Peer::Unix {
            uid: std::fs::metadata(&self.0).unwrap().uid(),
        };
        config
    }
}
impl Drop for PrivateFixture {
    fn drop(&mut self) {
        // This is the freshly created exact random fixture root, never supplied
        // by a request or obtained by traversing another directory.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
async fn status(endpoint: &str, authorization: &str, body: &[u8]) -> u16 {
    management_call(endpoint, authorization, body, "/management/v0.1/status")
        .await
        .status
}
async fn management_call(
    endpoint: &str,
    authorization: &str,
    body: &[u8],
    target: &str,
) -> humanymous_authority::protocol::Response {
    let mut stream = tokio::net::UnixStream::connect(endpoint).await.unwrap();
    let head = format!(
        "POST {target} HTTP/1.1\r\nHost: humanymous.management.local\r\nContent-Type: application/json\r\nAuthorization: {authorization}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    humanymous_authority::protocol::read_response(&mut stream)
        .await
        .unwrap()
}
#[tokio::test]
async fn unix_service_management_drains_before_custody_release() {
    let fixture = PrivateFixture::new();
    let config = fixture.configuration();
    let body = serde_json::to_vec(&ManagementScope::from_scope(&config.scope)).unwrap();
    let endpoint = fixture
        .0
        .join("management.sock")
        .to_str()
        .unwrap()
        .to_owned();
    let uid = std::fs::metadata(&fixture.0).unwrap().uid();
    let budget = StopBudget::new(1000, 1000, 5000).unwrap();
    let host = ServiceHost::bind_with_management(
        config,
        None,
        ManagementPolicy {
            endpoint: endpoint.clone(),
            peer: Peer::Unix { uid },
            stop_budget: budget,
            credential: Credential::generate().unwrap(),
        },
    )
    .await
    .unwrap();
    let address = host.readiness().listen;
    let secret = format!(
        "Bearer {}",
        host.management_credential_for_private_provisioning()
            .unwrap()
    );
    assert!(
        !serde_json::to_string(host.readiness())
            .unwrap()
            .contains(secret.strip_prefix("Bearer ").unwrap())
    );
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_err());
    let (sender, receiver) = tokio::sync::watch::channel(None);
    let running = tokio::spawn(host.run(receiver));
    assert_eq!(status(&endpoint, "Bearer wrong", &body).await, 401);
    assert_eq!(status(&endpoint, &secret, &body).await, 200);
    let settings = management_call(&endpoint, &secret, &body, "/management/v0.1/settings").await;
    assert_eq!(settings.status, 200);
    let settings: Value = serde_json::from_slice(&settings.body).unwrap();
    assert_eq!(settings["writable"], false);
    assert_eq!(settings["settings"]["session_capacity"], 16);
    let mut wrong: Value = serde_json::from_slice(&body).unwrap();
    wrong["tenant"] = json!("different");
    assert_eq!(
        status(&endpoint, &secret, &serde_json::to_vec(&wrong).unwrap()).await,
        403
    );
    sender.send(Some(budget.request())).unwrap();
    let report = tokio::time::timeout(std::time::Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(report.stopped && report.drained && report.audit_flushed);
    assert!(!std::path::Path::new(&endpoint).exists());
    assert!(TcpStream::connect(address).await.is_err());
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());
}
#[tokio::test]
async fn unix_closed_shutdown_controller_cannot_report_stopped() {
    let fixture = PrivateFixture::new();
    let host = ServiceHost::bind(fixture.configuration()).await.unwrap();
    let (sender, receiver) = tokio::sync::watch::channel(None);
    drop(sender);
    let failure = host.run_report(receiver).await.err().unwrap();
    assert_eq!(failure.stage, ExitStage::Drain);
    assert!(!failure.drained && failure.audit_flushed);
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());
}
