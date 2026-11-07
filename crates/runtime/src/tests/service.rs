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
        // Resolve platform aliases such as /tmp before shared privacy checks.
        let path = std::fs::canonicalize(path).unwrap();
        for name in ["keys", "audit", "settings", "witness"] {
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

/// A real durable signer in the same test process; not deployment independence.
struct SharedWitness(Arc<Mutex<humanymous_audit::checkpoint::DurableWitness>>);
impl humanymous_audit::checkpoint::CheckpointWitness for SharedWitness {
    fn public_key(&self) -> [u8; 32] {
        self.0.lock().unwrap().public_key()
    }
    fn last_size(&self) -> std::io::Result<u64> {
        humanymous_audit::checkpoint::CheckpointWitness::last_size(&*self.0.lock().unwrap())
    }
    fn validate_local_head(
        &self,
        head: &humanymous_audit::checkpoint::Checkpoint,
    ) -> std::io::Result<()> {
        humanymous_audit::checkpoint::CheckpointWitness::validate_local_head(
            &*self.0.lock().unwrap(),
            head,
        )
    }
    fn counter_sign(
        &mut self,
        head: &humanymous_audit::checkpoint::Checkpoint,
        proof: &[humanymous_audit::merkle::Hash],
    ) -> std::io::Result<humanymous_audit::checkpoint::WitnessReceipt> {
        humanymous_audit::checkpoint::CheckpointWitness::counter_sign(
            &mut *self.0.lock().unwrap(),
            head,
            proof,
        )
    }
}

/// Hosted-only actual snapshot/WAL/key locks and signer, with synthetic keys.
#[test]
fn hosted_only_witnessed_settings_commit_invalidates_sessions_and_restores() {
    use crate::{
        settings::{SettingsBootstrap, SettingsPolicy, SettingsStore},
        startup::{ExternalWitness, open_audit_with_witness},
    };
    use humanymous_audit::{
        checkpoint::{DurableWitness, Witness, WitnessBootstrap},
        keys::custody::Durability,
        log::WitnessDurability,
    };
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.audit.require_external_witness = true;
    // Shared primitive derives only the public identity of the known writer
    // fixture seed. This temporary holder never co-signs or grants an effect.
    let writer = Witness::from_seed(&[7; 32], [0; 32], "public-fixture-only".into()).public_key();
    let witness = DurableWitness::open(
        &fixture.0.join("witness"),
        Owner::CurrentUser,
        &[8; 32],
        writer,
        config.audit.node_id.clone(),
        WitnessBootstrap::CreateNew,
    )
    .unwrap();
    let public = witness.public_key();
    let witness = Arc::new(Mutex::new(witness));
    let injected = ExternalWitness::new(
        Box::new(SharedWitness(witness.clone())),
        public,
        WitnessDurability::FileAndDirectory,
    )
    .unwrap();
    let mut audit = open_audit_with_witness(&mut config, injected).unwrap();
    let mut store = SettingsStore::open(
        SettingsPolicy {
            directory: fixture.0.join("settings"),
            bootstrap: SettingsBootstrap::CreateNew,
            durability: Durability::FileAndDirectory,
            recover_native_pending: false,
        },
        Owner::CurrentUser,
        &config,
        &mut audit,
    )
    .unwrap();
    let audit = Arc::new(Mutex::new(audit));
    let binding = fixture.configuration();
    let mut app = Application::new(
        config,
        Default::default(),
        Box::new(crate::service::SharedAudit(audit.clone())),
    )
    .unwrap();
    let issued = issue(&mut app, 10000);
    let mut desired = store.snapshot().settings();
    desired.session_capacity = 32;
    let prepared = store.snapshot().prepare(&binding, 0, desired).unwrap();
    let committed = store
        .commit(&binding, prepared, &mut audit.lock().unwrap(), || Ok(()))
        .unwrap();
    app.apply_committed_settings(committed).unwrap();
    assert_eq!(app.readiness()["operational"]["session_capacity"], 32);
    let old = protected(&binding, &issued, 1, 10000);
    assert_eq!(app.decide(&old, || Ok(10000)).err().unwrap().status, 401);
    let pin = witness
        .lock()
        .unwrap()
        .last_checkpoint()
        .unwrap()
        .unwrap()
        .clone();
    drop(store);
    drop(app);
    drop(audit);
    let mut restarted_config = fixture.configuration();
    restarted_config.audit.require_external_witness = true;
    restarted_config.audit.allow_new = false;
    restarted_config.audit.trusted_head = Some(pin);
    let injected = ExternalWitness::new(
        Box::new(SharedWitness(witness.clone())),
        public,
        WitnessDurability::FileAndDirectory,
    )
    .unwrap();
    let mut audit = open_audit_with_witness(&mut restarted_config, injected).unwrap();
    let store = SettingsStore::open(
        SettingsPolicy {
            directory: fixture.0.join("settings"),
            bootstrap: SettingsBootstrap::Existing,
            durability: Durability::FileAndDirectory,
            recover_native_pending: false,
        },
        Owner::CurrentUser,
        &restarted_config,
        &mut audit,
    )
    .unwrap();
    store.restore_configuration(&mut restarted_config).unwrap();
    assert_eq!(restarted_config.session_capacity, 32);
    assert_eq!(store.snapshot().revision(), 1);
    assert_eq!(audit.sequence(), 2);
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
