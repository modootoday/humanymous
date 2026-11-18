//! Ports of legacy cmd/server/ml_setup_test.go and
//! internal/mlcorrect/bundle_test.go `TestStage_SignatureRequiredWhenKeyConfigured`
//! (baseline 981d25c), plus the fail-closed differences documented on the module.
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use humanymous_core::behavior::features::{FEATURE_DIM, schema_hash};
use humanymous_core::model::correction::digest;
use humanymous_core::model::inference::{MlpBundle, Runtime};
use std::path::Path;
use std::sync::Arc;

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-model-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Legacy `writeValidBundle`: shape-valid two-unit bundle for the running schema.
fn valid_bundle(version: &str) -> Vec<u8> {
    serde_json::to_vec(&MlpBundle {
        version: version.into(),
        schema_hash: schema_hash(),
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
    })
    .unwrap()
}

/// Legacy `x509.MarshalPKIXPublicKey` + `pem.EncodeToMemory("PUBLIC KEY")`.
fn pkix_pem(public: &[u8; 32]) -> Vec<u8> {
    let mut der = ED25519_SPKI_PREFIX.to_vec();
    der.extend_from_slice(public);
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        STANDARD.encode(der)
    )
    .into_bytes()
}

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn flags(bundle: &Path) -> Flags {
    Flags {
        bundle: Some(bundle.to_path_buf()),
        ..Flags::default()
    }
}

/// The admitted model serves a prediction once installed; a refusal leaves the
/// seam abstaining (legacy `mlserve.Score(zero vector).Abstain`).
fn serving_abstains(admission: &Admission) -> bool {
    let runtime = Runtime::default();
    if let Some(admitted) = admission.admitted() {
        runtime.set(Some(Arc::new(admitted.model.clone())));
    }
    runtime.score(&[0.0; FEATURE_DIM]).abstain
}

/// Go TestSetupBehavioralML_AdmitsUnsignedWhenNoPubkey.
#[test]
fn admits_unsigned_bundle_when_no_public_key() {
    let dir = TempDir::new("unsigned");
    let bytes = valid_bundle("v1");
    let path = dir.write("v1.json", &bytes);
    let admission = admit(&flags(&path));
    let admitted = admission.admitted().expect("schema-valid bundle admitted");
    assert_eq!(admitted.identity.version, "v1");
    assert_eq!(admitted.identity.digest, digest(&bytes));
    assert_eq!(admitted.bundles.active(), Some(admitted.identity.clone()));
    assert!(!admitted.signed);
    assert!(admitted.schema_pin_only);
    assert!(!serving_abstains(&admission));
}

/// Go TestSetupBehavioralML_RefusesSignatureRequiredButMissing.
#[test]
fn refuses_bundle_when_signature_required_but_missing() {
    let dir = TempDir::new("missing-sig");
    let path = dir.write("v1.json", &valid_bundle("v1"));
    let key = dir.write(
        "pub.pem",
        &pkix_pem(&signing_key(3).verifying_key().to_bytes()),
    );
    let admission = admit(&Flags {
        public_key: Some(key),
        ..flags(&path)
    });
    assert!(matches!(
        admission,
        Admission::Refused(Refusal::Stage(CorrectionError::Signature))
    ));
    assert!(serving_abstains(&admission));
}

/// Go TestSetupBehavioralML_AdmitsProperlySignedBundle.
#[test]
fn admits_properly_signed_bundle() {
    let dir = TempDir::new("signed");
    let bytes = valid_bundle("signed-v1");
    let path = dir.write("signed-v1.json", &bytes);
    let signer = signing_key(5);
    let key = dir.write("pub.pem", &pkix_pem(&signer.verifying_key().to_bytes()));
    let signature = dir.write("sig.bin", &signer.sign(&bytes).to_bytes());
    let admission = admit(&Flags {
        public_key: Some(key),
        signature: Some(signature),
        ..flags(&path)
    });
    let admitted = admission.admitted().expect("signed bundle admitted");
    assert_eq!(admitted.identity.version, "signed-v1");
    assert_eq!(admitted.bundles.active().unwrap().version, "signed-v1");
    assert!(admitted.signed);
    assert!(!admitted.schema_pin_only);
    assert!(!serving_abstains(&admission));
}

/// Go internal/mlcorrect TestStage_SignatureRequiredWhenKeyConfigured, with a
/// real PKIX PEM Ed25519 key through the core signed policy.
#[test]
fn stage_requires_signature_when_key_configured() {
    let signer = signing_key(9);
    let verifier =
        Ed25519Verifier::from_pkix_pem(&pkix_pem(&signer.verifying_key().to_bytes())).unwrap();
    let manager = BundleManager::with_verifier(Box::new(verifier));
    let bytes = valid_bundle("signed");
    assert_eq!(
        manager.stage_signed(&bytes, None, None).unwrap_err(),
        CorrectionError::Signature
    );
    assert_eq!(
        manager
            .stage_signed(&bytes, None, Some(b"not-a-sig"))
            .unwrap_err(),
        CorrectionError::Signature
    );
    let signature = signer.sign(&bytes).to_bytes();
    let identity = manager
        .stage_signed(&bytes, None, Some(&signature))
        .unwrap();
    assert_eq!(identity.version, "signed");
    // A signature by another key, or over other bytes, is refused.
    let other = signing_key(10).sign(&bytes).to_bytes();
    assert_eq!(
        manager
            .stage_signed(&bytes, None, Some(&other))
            .unwrap_err(),
        CorrectionError::Signature
    );
    assert_eq!(
        manager
            .stage_signed(&valid_bundle("other"), None, Some(&signature))
            .unwrap_err(),
        CorrectionError::Signature
    );
}

#[test]
fn digest_pin_and_flag_default_follow_legacy() {
    let dir = TempDir::new("digest");
    let bytes = valid_bundle("v1");
    let path = dir.write("v1.json", &bytes);
    let pinned = admit(&Flags {
        digest: digest(&bytes),
        ..flags(&path)
    });
    let admitted = pinned.admitted().unwrap();
    assert!(!admitted.schema_pin_only);
    assert!(matches!(
        admit(&Flags {
            digest: "00".repeat(32),
            ..flags(&path)
        }),
        Admission::Refused(Refusal::Stage(CorrectionError::Digest))
    ));
    // Empty flag value is the legacy unset default, not an empty pin.
    assert!(admit(&flags(&path)).admitted().is_some());
}

#[test]
fn disabled_without_bundle_and_shadow_is_ignored() {
    assert!(matches!(
        admit(&Flags::default()),
        Admission::Disabled {
            shadow_ignored: false
        }
    ));
    assert!(matches!(
        admit(&Flags {
            shadow_bundle: Some(PathBuf::from("shadow.json")),
            ..Flags::default()
        }),
        Admission::Disabled {
            shadow_ignored: true
        }
    ));
}

#[test]
fn unreadable_inputs_and_stale_schema_are_refused() {
    let dir = TempDir::new("refusals");
    let path = dir.write("v1.json", &valid_bundle("v1"));
    let missing = dir.0.join("absent");
    assert!(matches!(
        admit(&flags(&missing)),
        Admission::Refused(Refusal::BundleRead)
    ));
    let empty = dir.write("empty.json", b"");
    assert!(matches!(
        admit(&flags(&empty)),
        Admission::Refused(Refusal::BundleRead)
    ));
    assert!(matches!(
        admit(&Flags {
            public_key: Some(missing.clone()),
            ..flags(&path)
        }),
        Admission::Refused(Refusal::PublicKeyRead)
    ));
    // Legacy reads the signature file even without a key; a read failure disables.
    assert!(matches!(
        admit(&Flags {
            signature: Some(missing),
            ..flags(&path)
        }),
        Admission::Refused(Refusal::SignatureRead)
    ));
    let mut stale: MlpBundle = serde_json::from_slice(&valid_bundle("stale")).unwrap();
    stale.schema_hash = "deadbeefcafe".into();
    let stale = dir.write("stale.json", &serde_json::to_vec(&stale).unwrap());
    assert!(matches!(
        admit(&flags(&stale)),
        Admission::Refused(Refusal::Stage(CorrectionError::StaleSchema))
    ));
    // Without a key a supplied signature is ignored, as in legacy.
    let junk = dir.write("sig.bin", b"junk");
    assert!(
        admit(&Flags {
            signature: Some(junk),
            ..flags(&path)
        })
        .admitted()
        .is_some()
    );
}

/// Deliberate difference: legacy treated an empty key file as no key and
/// admitted the bundle unsigned while logging signed=true.
#[test]
fn empty_public_key_file_is_refused_not_unsigned() {
    let dir = TempDir::new("empty-key");
    let path = dir.write("v1.json", &valid_bundle("v1"));
    let key = dir.write("pub.pem", b"");
    assert!(matches!(
        admit(&Flags {
            public_key: Some(key),
            ..flags(&path)
        }),
        Admission::Refused(Refusal::PublicKeyRead)
    ));
}

#[test]
fn public_key_parsing_matches_pem_and_pkix_rules() {
    let public = signing_key(1).verifying_key().to_bytes();
    let good = pkix_pem(&public);
    assert!(Ed25519Verifier::from_pkix_pem(&good).is_ok());
    // Leading text is skipped and the block type is not checked (pem.Decode).
    let mut prefixed = b"comment line\n".to_vec();
    prefixed.extend_from_slice(
        &String::from_utf8(good.clone())
            .unwrap()
            .replace("PUBLIC KEY", "KEY")
            .into_bytes(),
    );
    assert!(Ed25519Verifier::from_pkix_pem(&prefixed).is_ok());
    let cases: [(&[u8], KeyError); 5] = [
        (b"", KeyError::Pem),
        (
            b"-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----\n",
            KeyError::Der,
        ),
        (b"-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEA\n", KeyError::Pem),
        (
            b"-----BEGIN PUBLIC KEY-----\nProc-Type: 4,ENCRYPTED\n\nAAAA\n-----END PUBLIC KEY-----\n",
            KeyError::Pem,
        ),
        (
            b"-----BEGIN PUBLIC KEY-----\n!!!!\n-----END PUBLIC KEY-----\n",
            KeyError::Pem,
        ),
    ];
    for (bad, expected) in cases {
        assert_eq!(Ed25519Verifier::from_pkix_pem(bad).unwrap_err(), expected);
    }
    // A P-256 SubjectPublicKeyInfo (id-ecPublicKey) is well formed but not Ed25519.
    let mut p256 = vec![
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00, 0x04,
    ];
    p256.extend_from_slice(&[0x11; 64]);
    let pem = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        STANDARD.encode(&p256)
    );
    assert_eq!(
        Ed25519Verifier::from_pkix_pem(pem.as_bytes()).unwrap_err(),
        KeyError::NotEd25519
    );
    // Ed25519 OID with a truncated key is malformed, not another algorithm.
    let mut short = ED25519_SPKI_PREFIX.to_vec();
    short[1] = 0x29;
    short[10] = 0x20;
    short.extend_from_slice(&[0x22; 31]);
    let pem = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        STANDARD.encode(&short)
    );
    assert_eq!(
        Ed25519Verifier::from_pkix_pem(pem.as_bytes()).unwrap_err(),
        KeyError::Der
    );
}

/// Deliberate difference: `verify_strict` refuses a small-order public key
/// that legacy `ed25519.Verify` would accept for a crafted signature.
#[test]
fn small_order_public_key_never_verifies() {
    let mut identity = [0_u8; 32];
    identity[0] = 1;
    let verifier = Ed25519Verifier::from_pkix_pem(&pkix_pem(&identity)).unwrap();
    // R = identity point, S = 0 satisfies the cofactorless equation for A = identity.
    let mut signature = [0_u8; 64];
    signature[0] = 1;
    assert!(!verifier.verify(b"bundle", &signature));
}
