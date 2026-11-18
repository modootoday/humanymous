use super::*;
#[test]
fn suppression_resume_and_exact_boundary_match_source_regression() {
    let sec = 1_000_000_000;
    let mut registry = NodeRegistry::default();
    for node in ["proxy-1", "proxy-2"] {
        registry.expect(node.into(), sec, 0).unwrap();
        registry.record_receipt(node, sec).unwrap();
    }
    assert!(registry.missing(sec).unwrap().is_empty());
    assert!(registry.missing(3 * sec).unwrap().is_empty());
    registry.record_receipt("proxy-1", 3 * sec).unwrap();
    assert_eq!(
        registry.missing(3 * sec + sec / 2).unwrap(),
        vec!["proxy-2"]
    );
    registry.record_receipt("proxy-2", 4 * sec).unwrap();
    assert!(registry.missing(4 * sec + sec / 2).unwrap().is_empty());
    assert_eq!(
        registry.nodes(4 * sec + sec / 2).unwrap()[0].last_seen_ms,
        1500
    );
}
#[test]
fn invalid_and_regressing_inputs_cannot_refresh_liveness() {
    let mut registry = NodeRegistry::default();
    assert_eq!(
        registry.expect("".into(), 1, 0),
        Err(RegistryError::InvalidRegistration)
    );
    assert_eq!(
        registry.expect("n".into(), i128::MAX, 0),
        Err(RegistryError::InvalidRegistration)
    );
    registry.expect("n".into(), 10, 100).unwrap();
    assert_eq!(
        registry.expect("n".into(), 10, 1000),
        Err(RegistryError::AlreadyRegistered)
    );
    assert_eq!(
        registry.record_receipt("n", 99),
        Err(RegistryError::ClockRegressed)
    );
    assert_eq!(
        registry.record_receipt("unknown", 100),
        Err(RegistryError::UnknownNode)
    );
    assert_eq!(registry.nodes(99), Err(RegistryError::ClockRegressed));
    assert_eq!(registry.missing(121).unwrap(), vec!["n"]);
}
/// Go `NodeRegistry.Expect(nodeID, interval, pub, now)` kept each node's
/// Ed25519 key beside its cadence (`NodeInfo.PubKey`, nil allowed). The key
/// must authenticate that node's signed receipt only, never change the grace
/// window, and never appear in the console snapshot.
#[test]
fn expected_node_keeps_its_ed25519_key_beside_the_cadence() {
    use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
    let sec = 1_000_000_000;
    let signer = SigningKey::from_bytes(&[3; 32]);
    let other = SigningKey::from_bytes(&[4; 32]);
    let key = signer.verifying_key().to_bytes();
    let mut registry = NodeRegistry::default();
    registry
        .expect_with_key("proxy-1".into(), sec, Some(key), 0)
        .unwrap();
    registry.expect("proxy-2".into(), sec, 0).unwrap();
    assert_eq!(registry.public_key("proxy-1"), Ok(Some(key)));
    assert_eq!(registry.public_key("proxy-2"), Ok(None));
    assert_eq!(
        registry.public_key("proxy-3"),
        Err(RegistryError::UnknownNode)
    );
    let heartbeat = b"proxy-1 heartbeat";
    let pinned =
        VerifyingKey::from_bytes(&registry.public_key("proxy-1").unwrap().unwrap()).unwrap();
    assert!(pinned.verify(heartbeat, &signer.sign(heartbeat)).is_ok());
    assert!(pinned.verify(heartbeat, &other.sign(heartbeat)).is_err());
    assert_eq!(
        registry.expect_with_key("proxy-1".into(), sec, Some([0; 32]), 0),
        Err(RegistryError::AlreadyRegistered)
    );
    assert_eq!(registry.public_key("proxy-1"), Ok(Some(key)));
    assert_eq!(
        registry.expect_with_key(String::new(), sec, Some(key), 0),
        Err(RegistryError::InvalidRegistration)
    );
    registry.record_receipt("proxy-1", 3 * sec).unwrap();
    assert_eq!(
        registry.missing(3 * sec + sec / 2).unwrap(),
        vec!["proxy-2"]
    );
    let snapshot = serde_json::to_string(&registry.nodes(3 * sec).unwrap()).unwrap();
    assert!(!snapshot.contains(&hex::encode(key)));
    assert_eq!(
        snapshot,
        r#"[{"node":"proxy-1","lastSeenMs":0,"missing":false},{"node":"proxy-2","lastSeenMs":3000,"missing":true}]"#
    );
}
