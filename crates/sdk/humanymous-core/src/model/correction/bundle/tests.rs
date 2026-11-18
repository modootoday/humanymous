use super::*;
use crate::behavior::features::FEATURE_DIM;
use crate::model::inference::MlpBundle;
use std::sync::Arc;

/// Legacy `writeBundle`: a shape-valid two-unit bundle, returned as bytes.
fn bundle_bytes(version: &str, schema: Option<&str>) -> Vec<u8> {
    let bundle = MlpBundle {
        version: version.into(),
        schema_hash: schema.map_or_else(schema_hash, str::to_owned),
        feature_dim: FEATURE_DIM as i64,
        hidden: 2,
        mean: vec![0.0; FEATURE_DIM],
        std: vec![1.0; FEATURE_DIM],
        w1: vec![vec![0.0; FEATURE_DIM]; 2],
        b1: vec![0.0; 2],
        w2: vec![0.0; 2],
        b2: 0.0,
        cal_a: 1.0,
        cal_b: 0.0,
    };
    serde_json::to_vec(&bundle).unwrap()
}

#[test]
fn stage_accepts_a_valid_bundle() {
    let manager = BundleManager::unsigned_lab();
    let bytes = bundle_bytes("v1", None);
    let identity = manager.stage(&bytes, None).unwrap();
    assert_eq!(manager.staged(), Some(identity.clone()));
    assert_eq!(identity.version, "v1");
    assert_eq!(identity.digest, digest(&bytes));
    assert_eq!(identity.digest.len(), 64);
}

#[test]
fn stage_rejects_a_stale_schema() {
    let manager = BundleManager::unsigned_lab();
    assert_eq!(
        manager
            .stage(&bundle_bytes("stale", Some("deadbeefcafe")), None)
            .unwrap_err(),
        CorrectionError::StaleSchema
    );
    assert_eq!(manager.staged(), None);
}

#[test]
fn stage_enforces_the_pinned_digest() {
    let manager = BundleManager::unsigned_lab();
    let bytes = bundle_bytes("v1", None);
    let good = digest(&bytes);
    manager.stage(&bytes, Some(&good)).unwrap();
    assert_eq!(
        manager.stage(&bytes, Some("0000")).unwrap_err(),
        CorrectionError::Digest
    );
    assert_eq!(
        manager.stage(&bytes, Some("")).unwrap_err(),
        CorrectionError::Digest
    );
    assert_eq!(
        manager
            .stage(&bytes, Some(&good.to_uppercase()))
            .unwrap_err(),
        CorrectionError::Digest
    );
}

#[test]
fn signed_policy_fails_closed_without_ed25519_support() {
    for key in [
        &b""[..],
        b"-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----\n",
    ] {
        assert_eq!(
            BundleManager::with_public_key(key).unwrap_err(),
            CorrectionError::SignatureUnsupported
        );
    }
}

/// Test verifier: a "signature" is the SHA-256 of the exact bytes.
struct DigestVerifier;
impl BundleVerifier for DigestVerifier {
    fn verify(&self, bundle_bytes: &[u8], signature: &[u8]) -> bool {
        signature == Sha256::digest(bundle_bytes).as_slice()
    }
}

#[test]
fn signed_policy_requires_a_verified_signature_in_legacy_order() {
    let manager = BundleManager::with_verifier(Box::new(DigestVerifier));
    assert!(manager.signed());
    let bytes = bundle_bytes("signed", None);
    let good = Sha256::digest(&bytes).to_vec();
    // stage without a signature uses the signed policy and is refused.
    for signature in [None, Some(&b""[..]), Some(&b"not-a-sig"[..])] {
        assert_eq!(
            manager.stage_signed(&bytes, None, signature).unwrap_err(),
            CorrectionError::Signature
        );
    }
    assert_eq!(
        manager.stage(&bytes, None).unwrap_err(),
        CorrectionError::Signature
    );
    assert_eq!(manager.staged(), None);
    // The digest pin is checked before the signature, the schema after it.
    assert_eq!(
        manager
            .stage_signed(&bytes, Some("0000"), Some(&good))
            .unwrap_err(),
        CorrectionError::Digest
    );
    let stale = bundle_bytes("stale", Some("deadbeefcafe"));
    assert_eq!(
        manager.stage_signed(&stale, None, None).unwrap_err(),
        CorrectionError::Signature
    );
    assert_eq!(
        manager
            .stage_signed(&stale, None, Some(&Sha256::digest(&stale)))
            .unwrap_err(),
        CorrectionError::StaleSchema
    );
    let identity = manager.stage_signed(&bytes, None, Some(&good)).unwrap();
    assert_eq!(identity.version, "signed");
    assert_eq!(manager.staged(), Some(identity));
}

#[test]
fn unsigned_policy_ignores_a_supplied_signature() {
    let manager = BundleManager::unsigned_lab();
    assert!(!manager.signed());
    let bytes = bundle_bytes("v1", None);
    manager
        .stage_signed(&bytes, None, Some(b"not-a-sig"))
        .unwrap();
    assert_eq!(manager.staged().unwrap().version, "v1");
}

#[test]
fn promote_and_rollback_move_lab_identities_only() {
    let manager = BundleManager::unsigned_lab();
    let v1 = manager.stage(&bundle_bytes("v1", None), None).unwrap();
    assert_eq!(manager.promote().unwrap(), v1);
    assert_eq!(manager.active(), Some(v1.clone()));
    let v2 = manager.stage(&bundle_bytes("v2", None), None).unwrap();
    manager.promote().unwrap();
    assert_eq!(manager.active().unwrap().version, "v2");
    assert_eq!(manager.staged(), None);
    assert_ne!(v1.digest, v2.digest);
    assert_eq!(manager.rollback().unwrap(), Some(v1.clone()));
    assert_eq!(manager.active(), Some(v1));
    assert_eq!(manager.rollback().unwrap(), None);
    assert_eq!(manager.active(), None);
    assert_eq!(manager.rollback().unwrap_err(), CorrectionError::NoPrevious);
}

#[test]
fn promote_requires_a_staged_candidate() {
    let manager = BundleManager::unsigned_lab();
    assert_eq!(manager.promote().unwrap_err(), CorrectionError::NoStaged);
    manager.stage(&bundle_bytes("v1", None), None).unwrap();
    manager.promote().unwrap();
    assert_eq!(manager.promote().unwrap_err(), CorrectionError::NoStaged);
}

#[test]
fn stage_rejects_empty_oversized_and_malformed_buffers() {
    let manager = BundleManager::unsigned_lab();
    assert_eq!(
        manager.stage(&[], None).unwrap_err(),
        CorrectionError::Limit
    );
    assert_eq!(
        manager
            .stage(&vec![b' '; MAX_BUNDLE_BYTES + 1], None)
            .unwrap_err(),
        CorrectionError::Limit
    );
    assert_eq!(
        manager.stage(b"{not json", None).unwrap_err(),
        CorrectionError::InvalidBundle
    );
    let mut wrong_shape: MlpBundle = serde_json::from_slice(&bundle_bytes("v1", None)).unwrap();
    wrong_shape.hidden = 3;
    assert_eq!(
        manager
            .stage(&serde_json::to_vec(&wrong_shape).unwrap(), None)
            .unwrap_err(),
        CorrectionError::InvalidBundle
    );
    assert_eq!(manager.staged(), None);
}

#[test]
fn concurrent_lifecycle_transitions_keep_identities_consistent() {
    let manager = Arc::new(BundleManager::unsigned_lab());
    let versions: Vec<String> = (0..8).map(|index| format!("v{index}")).collect();
    let workers: Vec<_> = versions
        .iter()
        .cloned()
        .map(|version| {
            let manager = manager.clone();
            std::thread::spawn(move || {
                let bytes = bundle_bytes(&version, None);
                for _ in 0..50 {
                    manager.stage(&bytes, None).unwrap();
                    let _ = manager.promote();
                    let _ = manager.rollback();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    for identity in [manager.active(), manager.staged()].into_iter().flatten() {
        assert!(versions.contains(&identity.version));
        assert_eq!(
            identity.digest,
            digest(&bundle_bytes(&identity.version, None))
        );
    }
}
