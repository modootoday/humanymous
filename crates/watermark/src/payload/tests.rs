//! Source-only acceptance cases; execution is separately allocated by root.

use super::*;

fn payload() -> Payload {
    Payload {
        tag_id: 0x0102_0304,
        session_id: "sess-abc".into(),
        asset_id: "/res/logo.png".into(),
        bucket: 20_300,
    }
}

#[test]
fn legacy_tag_matches_explicit_preimage_and_big_endian_carrier() {
    let key_bytes = b"master-secret-key";
    let key = WatermarkKey::new(key_bytes.to_vec()).unwrap();
    let carrier = key.carrier(&payload()).unwrap();
    // Independent literal preimage protects delimiters and bucket endianness;
    // the expected tag uses the established crypto library, not local hashing.
    let mut mac = Hmac::<Sha256>::new_from_slice(key_bytes).unwrap();
    mac.update(b"sess-abc\0/res/logo.png\0\0\0\0\0\0\0\x4f\x4c");
    let expected = mac.finalize().into_bytes();
    assert_eq!(&carrier[..4], &[1, 2, 3, 4]);
    assert_eq!(&carrier[4..], &expected[..TAG_LEN]);
    let parsed = split_carrier(&carrier).unwrap();
    assert_eq!(parsed.tag_id, 0x0102_0304);
    assert!(!parsed.ignored_suffix);
    assert!(key.verify_legacy(&payload(), parsed.tag).unwrap());
}

#[test]
fn rfc4231_case_one_confirms_selected_hmac_primitive_and_truncation() {
    // Primary test vector: https://www.rfc-editor.org/rfc/rfc4231#section-4.2
    let mut mac = Hmac::<Sha256>::new_from_slice(&[0x0b; 20]).unwrap();
    mac.update(b"Hi There");
    let digest = mac.finalize().into_bytes();
    assert_eq!(
        &digest[..TAG_LEN],
        &[0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8]
    );
}

#[test]
fn wrong_key_modified_context_and_truncated_tag_do_not_verify() {
    let key = WatermarkKey::new(b"master-secret-key".to_vec()).unwrap();
    let wrong = WatermarkKey::new(b"other-key".to_vec()).unwrap();
    let original = payload();
    let bytes = key.carrier(&original).unwrap();
    let tag = &bytes[4..];
    assert!(!wrong.verify_legacy(&original, tag).unwrap());
    let mut changed = original.clone();
    changed.session_id.push('x');
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    changed = original.clone();
    changed.asset_id.push('x');
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    changed = original.clone();
    changed.bucket += 1;
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    assert!(!key.verify_legacy(&original, &tag[..9]).unwrap());
    let mut corrupted = tag.to_vec();
    corrupted[0] ^= 1;
    assert!(!key.verify_legacy(&original, &corrupted).unwrap());
}

#[test]
fn v1_id_is_intentionally_not_mac_bound_and_suffix_tolerance_is_explicit() {
    let key = WatermarkKey::new(b"master-secret-key".to_vec()).unwrap();
    let mut other = payload();
    let mut carrier = key.carrier(&other).unwrap().to_vec();
    other.tag_id += 1;
    assert!(key.verify_legacy(&other, &carrier[4..]).unwrap());
    carrier.push(0xaa);
    assert_eq!(split_carrier(&carrier), Err(Error::InvalidCarrier));
    assert!(split_legacy_carrier(&carrier).unwrap().ignored_suffix);
    for size in 0..CARRIER_LEN {
        assert_eq!(
            split_legacy_carrier(&carrier[..size]),
            Err(Error::InvalidCarrier)
        );
    }
}

#[test]
fn identifier_limits_nul_ambiguity_and_zero_id_are_rejected_for_issuance() {
    let key = WatermarkKey::new(vec![1; 32]).unwrap();
    let mut candidate = payload();
    candidate.session_id = "x".repeat(MAX_IDENTIFIER_BYTES);
    assert!(key.carrier(&candidate).is_ok());
    candidate.session_id.push('x');
    assert_eq!(key.carrier(&candidate), Err(Error::InvalidPayload));
    candidate = payload();
    candidate.asset_id = "x".repeat(MAX_IDENTIFIER_BYTES + 1);
    assert_eq!(key.carrier(&candidate), Err(Error::InvalidPayload));
    candidate = payload();
    candidate.tag_id = 0;
    assert_eq!(key.carrier(&candidate), Err(Error::InvalidPayload));
    candidate = payload();
    candidate.session_id = "a\0b".into();
    candidate.asset_id = "c".into();
    assert_eq!(key.carrier(&candidate), Err(Error::InvalidPayload));
    // Preserve the legacy preimage equivalence while refusing to issue it.
    let tag = key.tag(&candidate).unwrap();
    candidate.session_id = "a".into();
    candidate.asset_id = "b\0c".into();
    assert!(key.verify_legacy(&candidate, &tag).unwrap());
    assert_eq!(key.carrier(&candidate), Err(Error::InvalidPayload));
}

#[test]
fn key_bounds_are_explicit_and_rejected_material_is_owned_for_zeroization() {
    assert!(matches!(WatermarkKey::new(vec![]), Err(Error::InvalidKey)));
    assert!(WatermarkKey::new(vec![1; MAX_KEY_BYTES]).is_ok());
    assert!(matches!(
        WatermarkKey::new(vec![1; MAX_KEY_BYTES + 1]),
        Err(Error::InvalidKey)
    ));
}
