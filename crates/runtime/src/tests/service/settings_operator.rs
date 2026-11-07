//! Real IPC/CAS/witness/session/restart integration, executed only by allocation.
use super::*;
use crate::{
    management::OperatorPolicy,
    settings::{SettingsBootstrap, SettingsPolicy},
    startup::ExternalWitness,
};
use humanymous_audit::{
    checkpoint::{DurableWitness, Witness, WitnessBootstrap},
    keys::custody::Durability,
    log::WitnessDurability,
};

#[tokio::test]
async fn native_operator_commit_is_witnessed_cas_and_invalidates_sessions() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.audit.require_external_witness = true;
    let scope = ManagementScope::from_scope(&config.scope);
    let mut desired = config.operational();
    desired.session_capacity = 32;
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
    // Same-process actual signer/storage is a component fixture, not independent deployment.
    let witness = Arc::new(Mutex::new(witness));
    let injected = ExternalWitness::new(
        Box::new(SharedWitness(witness.clone())),
        public,
        WitnessDurability::FileAndDirectory,
    )
    .unwrap();
    let endpoint = fixture.0.join("operator.sock").to_str().unwrap().to_owned();
    let budget = StopBudget::new(1000, 1000, 5000).unwrap();
    let host = ServiceHost::bind_with_operator(
        config,
        injected,
        ManagementPolicy {
            endpoint: endpoint.clone(),
            peer: Peer::Unix {
                uid: std::fs::metadata(&fixture.0).unwrap().uid(),
            },
            stop_budget: budget,
            credential: Credential::generate().unwrap(),
        },
        OperatorPolicy {
            credential: Credential::generate().unwrap(),
        },
        SettingsPolicy {
            directory: fixture.0.join("settings"),
            bootstrap: SettingsBootstrap::CreateNew,
            durability: Durability::FileAndDirectory,
            recover_native_pending: false,
        },
        None,
        None,
    )
    .await
    .unwrap();
    let address = host.readiness().listen;
    let observer = format!(
        "Bearer {}",
        host.management_credential_for_private_provisioning()
            .unwrap()
    );
    let operator = format!(
        "Bearer {}",
        host.operator_credential_for_private_provisioning().unwrap()
    );
    let manifest = serde_json::to_string(host.readiness()).unwrap();
    assert!(!manifest.contains(observer.strip_prefix("Bearer ").unwrap()));
    assert!(!manifest.contains(operator.strip_prefix("Bearer ").unwrap()));
    assert_eq!(host.operational_revision(), Some(0));
    assert_eq!(host.readiness().profile["administration_writable"], true);
    let (stop, receiver) = tokio::sync::watch::channel(None);
    let running = tokio::spawn(host.run(receiver));
    let issued =
        json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let body = serde_json::to_vec(&json!({"scope":scope,"expected_revision":0,"settings":desired}))
        .unwrap();
    let target = "/management/v0.1/settings/commit";
    assert_eq!(
        management_call(&endpoint, &observer, &body, target)
            .await
            .status,
        401
    );
    let committed = management_call(&endpoint, &operator, &body, target).await;
    assert_eq!(committed.status, 200);
    let committed: Value = serde_json::from_slice(&committed.body).unwrap();
    assert_eq!(committed["revision"], 1);
    assert_eq!(committed["settings"]["session_capacity"], 32);
    assert_eq!(
        management_call(&endpoint, &operator, &body, target)
            .await
            .status,
        409
    );
    let old = protected(
        &fixture.configuration(),
        &issued,
        1,
        humanymous_authority::now_ms().unwrap(),
    );
    assert!(exchange(address, &old).await.starts_with(b"HTTP/1.1 401"));
    for _ in 0..32 {
        assert!(
            exchange(address, &request("POST", "/_humanymous/session", vec![]))
                .await
                .starts_with(b"HTTP/1.1 200")
        );
    }
    assert!(
        exchange(address, &request("POST", "/_humanymous/session", vec![]))
            .await
            .starts_with(b"HTTP/1.1 429")
    );
    let scope_bytes = serde_json::to_vec(&scope).unwrap();
    assert_eq!(
        management_call(
            &endpoint,
            &operator,
            &scope_bytes,
            "/management/v0.1/settings"
        )
        .await
        .status,
        401
    );
    let observed = management_call(
        &endpoint,
        &observer,
        &scope_bytes,
        "/management/v0.1/settings",
    )
    .await;
    assert_eq!(observed.status, 200);
    let observed: Value = serde_json::from_slice(&observed.body).unwrap();
    assert_eq!(observed["settings"]["session_capacity"], 32);
    assert_eq!(observed["revision"], 1);
    assert_eq!(observed["writable"], false);
    assert_eq!(observed["operator_available"], true);
    stop.send(Some(budget.request())).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(result.stopped && result.drained && result.audit_flushed);
    let pin = witness
        .lock()
        .unwrap()
        .last_checkpoint()
        .unwrap()
        .unwrap()
        .clone();
    assert_eq!(pin.tree_size, 2);
    let mut config = fixture.configuration();
    config.audit.require_external_witness = true;
    config.audit.allow_new = false;
    config.audit.trusted_head = Some(pin);
    let injected = ExternalWitness::new(
        Box::new(SharedWitness(witness)),
        public,
        WitnessDurability::FileAndDirectory,
    )
    .unwrap();
    let restarted = ServiceHost::bind_with_settings(
        config,
        injected,
        None,
        SettingsPolicy {
            directory: fixture.0.join("settings"),
            bootstrap: SettingsBootstrap::Existing,
            durability: Durability::FileAndDirectory,
            recover_native_pending: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(restarted.operational_revision(), Some(1));
    assert_eq!(
        restarted.readiness().profile["operational"]["session_capacity"],
        32
    );
    assert_eq!(
        restarted.readiness().profile["administration_writable"],
        false
    );
}
