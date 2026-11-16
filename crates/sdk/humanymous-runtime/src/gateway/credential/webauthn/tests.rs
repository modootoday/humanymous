//! Ports of legacy internal/gate/webauthn_test.go (baseline 981d25c) plus
//! the fail-closed boundaries this kernel adds.
use super::*;
use rcgen::{KeyPair, PKCS_ECDSA_P256_SHA256, PKCS_ECDSA_P384_SHA384, PublicKeyData, SigningKey};
use serde_json::json;

pub(in crate::gateway::credential) const CRED: &str = "cred-abc";
const CLIENT_DATA: &str =
    r#"{"type":"webauthn.get","challenge":"Y2g","origin":"https://example.com"}"#;

pub(in crate::gateway::credential) fn key() -> KeyPair {
    KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap()
}
/// Go `makeAuthData`: zero rpIdHash, flags UP|UV, big-endian counter.
fn auth_data(counter: u32, flags: u8) -> Vec<u8> {
    let mut data = vec![0; 37];
    data[32] = flags;
    data[33..37].copy_from_slice(&counter.to_be_bytes());
    data
}
fn assertion_with(key: &KeyPair, id: &str, data: &[u8], client: &str) -> String {
    let mut signed = data.to_vec();
    signed.extend_from_slice(&Sha256::digest(client.as_bytes()));
    let signature = key.sign(&signed).unwrap();
    let body = json!({
        "credentialId": id,
        "authenticatorData": STANDARD.encode(data),
        "clientDataJSON": STANDARD.encode(client),
        "signature": STANDARD.encode(signature),
    });
    STANDARD.encode(body.to_string())
}
/// Go `signAssertion`.
pub(in crate::gateway::credential) fn sign_assertion(
    key: &KeyPair,
    id: &str,
    counter: u32,
) -> String {
    assertion_with(key, id, &auth_data(counter, 0x05), CLIENT_DATA)
}
/// Go `webauthnDir`.
pub(in crate::gateway::credential) fn registry(id: &str, key: &KeyPair) -> Registry {
    let spki = URL_SAFE_NO_PAD.encode(key.subject_public_key_info());
    Registry::parse(&format!("{id} {spki}")).unwrap()
}
fn verified() -> Verdict {
    Verdict::Verified {
        credential_id: CRED.into(),
    }
}
fn rejected(reason: Reason) -> Verdict {
    Verdict::Invalid {
        reason,
        credential_id: Some(CRED.into()),
    }
}

#[test]
fn webauthn_verified() {
    let key = key();
    let mut r = registry(CRED, &key);
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 1))), verified());
}

#[test]
fn webauthn_replay_rejected() {
    let key = key();
    let mut r = registry(CRED, &key);
    let assertion = sign_assertion(&key, CRED, 5);
    assert_eq!(r.verify(Some(&assertion)), verified());
    assert_eq!(r.verify(Some(&assertion)), rejected(Reason::Replay));
}

#[test]
fn webauthn_unknown_credential() {
    let key = key();
    let mut r = registry(CRED, &self::key());
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, CRED, 1))),
        rejected(Reason::BadSignature)
    );
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, "cred-other", 1))),
        invalid(Reason::UnknownCredential)
    );
}

#[test]
fn webauthn_none() {
    let key = key();
    let mut r = registry(CRED, &key);
    assert_eq!(r.verify(None), Verdict::Absent);
}

#[test]
fn webauthn_origin_binding() {
    let key = key();
    let mut r = registry(CRED, &key);
    r.set_binding("https://example.com", "");
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 1))), verified());
    let mut r2 = registry(CRED, &key);
    r2.set_binding("https://victim.example", "");
    assert_eq!(
        r2.verify(Some(&sign_assertion(&key, CRED, 2))),
        rejected(Reason::Origin)
    );
}

#[test]
fn rp_id_binding_and_counter_policy() {
    let key = key();
    let mut r = registry(CRED, &key);
    r.set_binding("", "example.com");
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, CRED, 1))),
        rejected(Reason::RelyingParty)
    );
    let mut bound = auth_data(2, 0x05);
    bound[..32].copy_from_slice(&Sha256::digest(b"example.com"));
    assert_eq!(
        r.verify(Some(&assertion_with(&key, CRED, &bound, CLIENT_DATA))),
        verified()
    );
    let mut r = registry(CRED, &key);
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, CRED, 0))),
        rejected(Reason::Replay)
    );
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 7))), verified());
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, CRED, 6))),
        rejected(Reason::Replay)
    );
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 8))), verified());
}

#[test]
fn client_data_and_flags_are_enforced() {
    let key = key();
    let mut r = registry(CRED, &key);
    let create = r#"{"type":"webauthn.create","origin":"https://example.com"}"#;
    assert_eq!(
        r.verify(Some(&assertion_with(
            &key,
            CRED,
            &auth_data(1, 0x05),
            create
        ))),
        rejected(Reason::ClientDataType)
    );
    assert_eq!(
        r.verify(Some(&assertion_with(
            &key,
            CRED,
            &auth_data(1, 0x04),
            CLIENT_DATA
        ))),
        rejected(Reason::UserPresence)
    );
    assert_eq!(
        r.verify(Some(&assertion_with(
            &key,
            CRED,
            &auth_data(1, 0x05)[..36],
            CLIENT_DATA
        ))),
        rejected(Reason::Malformed)
    );
    assert_eq!(
        r.verify(Some(&assertion_with(
            &key,
            CRED,
            &auth_data(1, 0x05),
            "not json"
        ))),
        rejected(Reason::Malformed)
    );
}

#[test]
fn malformed_headers_are_invalid_not_absent() {
    let key = key();
    let mut r = registry(CRED, &key);
    let duplicate = STANDARD.encode(format!(
        r#"{{"credentialId":"{CRED}","credentialId":"{CRED}","authenticatorData":"","clientDataJSON":"","signature":""}}"#
    ));
    let folded = STANDARD.encode(
        r#"{"CREDENTIALID":"cred-abc","authenticatorData":"","clientDataJSON":"","signature":""}"#,
    );
    for header in [
        "".to_owned(),
        "!!!".to_owned(),
        STANDARD.encode("[]"),
        duplicate,
        folded,
        "A".repeat(MAX_HEADER + 1),
    ] {
        assert_eq!(
            r.verify(Some(&header)),
            invalid(Reason::Malformed),
            "{header}"
        );
    }
}

#[test]
fn absent_registry_refuses_instead_of_bypassing() {
    let key = key();
    let assertion = sign_assertion(&key, CRED, 1);
    assert_eq!(
        verify(None, Some(&assertion)),
        invalid(Reason::NotConfigured)
    );
    assert_eq!(verify(None, None), Verdict::Absent);
    let mut r = registry(CRED, &key);
    assert_eq!(verify(Some(&mut r), Some(&assertion)), verified());
}

#[test]
fn registry_configuration_is_strict() {
    let key = key();
    let spki = key.subject_public_key_info();
    let pem = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        STANDARD.encode(&spki)
    );
    let mut from_pem = Registry::parse(&format!("{CRED} {}", STANDARD.encode(pem))).unwrap();
    assert_eq!(
        from_pem.verify(Some(&sign_assertion(&key, CRED, 1))),
        verified()
    );
    let std_b64 = Registry::parse(&format!("{CRED} {}", STANDARD.encode(&spki)));
    assert!(std_b64.is_ok());
    assert_eq!(Registry::parse("").err(), Some(ConfigError::Empty));
    assert_eq!(Registry::parse(CRED).err(), Some(ConfigError::Syntax));
    assert_eq!(
        Registry::parse(&format!("{CRED} AAAA")).err(),
        Some(ConfigError::Key)
    );
    let p384 = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384).unwrap();
    assert_eq!(
        Registry::parse(&format!(
            "{CRED} {}",
            URL_SAFE_NO_PAD.encode(p384.subject_public_key_info())
        ))
        .err(),
        Some(ConfigError::Key)
    );
    let line = format!("{CRED} {}", URL_SAFE_NO_PAD.encode(&spki));
    assert_eq!(
        Registry::parse(&format!("{line}\n{line}")).err(),
        Some(ConfigError::Duplicate)
    );
}

/// Wargame R219 (legacy wargame_axis_cred_r201_test.go): the counter must
/// strictly increase. Each assertion is freshly signed (ECDSA is randomized),
/// so an equal counter is refused by the counter rule, not by byte replay.
#[test]
fn wargame_r219_counter_must_strictly_increase() {
    let key = key();
    let mut r = registry(CRED, &key);
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 10))), verified());
    let same = sign_assertion(&key, CRED, 10);
    assert_eq!(r.verify(Some(&same)), rejected(Reason::Replay));
    assert_eq!(
        r.verify(Some(&sign_assertion(&key, CRED, 9))),
        rejected(Reason::Replay)
    );
    assert_eq!(r.verify(Some(&sign_assertion(&key, CRED, 11))), verified());
}
