mod fixture;
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

fn check(
    records: &[Vec<u8>; 3],
    e: &ExpectedAuthorizationData,
    now: u64,
) -> Result<SignatureCheckedAuthorization> {
    verify_bundle(parse_bundle(&records[0], &records[1], &records[2])?, e, now)
}

#[test]
fn signed_data_is_bound_but_cannot_mint_native_preparation() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    let checked = check(&records, &e, 2000).unwrap();
    assert_eq!(checked.grant_id(), &e.attempt.grant_id);
    assert_eq!(checked.not_after_ms(), 5000);
    assert_eq!(checked.startup_cap_ms(), 1000);
    assert_eq!(checked.scope().installation, "i");
    e.scope.installation = "changed-after-verification".into();
    assert_eq!(checked.scope().installation, "i");
    assert!(matches!(
        crate::authority::startup::observer::native_preparation(),
        Err(crate::authority::startup::observer::NativePreparationUnavailable::Unsupported)
    ));
}

#[test]
fn independent_maximum_wire_lengths_and_closed_hello_fit_2768() {
    let mut e = fixture::expected(true);
    let records = fixture::records(&mut e);
    assert_eq!(records.each_ref().map(Vec::len), [467, 784, 275]);
    assert_eq!(
        records.each_ref().map(|r| URL_SAFE_NO_PAD.encode(r).len()),
        [623, 1046, 367]
    );
    assert_eq!(&records[0][..12], b"HMBGD002\x01\x89\x00\x02");
    assert_eq!(&records[1][..12], b"HMBGP002\x02\xc6\x00\x02");
    assert_eq!(&records[2][..12], b"HMBGC002\x00\xc9\x00\x02");
    assert_eq!(binding::scope(&e.scope), fixture::scope(&e));
    assert_eq!(binding::role(&e.service), fixture::role(&e.service));
    let wire = hello::fixture_hello(&records, &e);
    assert_eq!(wire.len(), 2768);
    verify_hello(parse_hello(&wire).unwrap(), &e, 2000).unwrap();
}

#[test]
fn complete_artifact_and_signature_mutations_all_refuse() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for index in 0..3 {
        for byte in 0..records[index].len() {
            let mut changed = records.clone();
            changed[index][byte] ^= 1;
            assert!(
                check(&changed, &e, 2000).is_err(),
                "record{index} byte{byte}"
            );
        }
    }
}

#[test]
fn every_truncation_trailing_byte_oversize_and_declared_length_refuses() {
    let mut e = fixture::expected(true);
    let records = fixture::records(&mut e);
    for index in 0..3 {
        for length in 0..records[index].len() {
            let mut changed = records.clone();
            changed[index].truncate(length);
            assert!(check(&changed, &e, 2000).is_err());
        }
        let mut changed = records.clone();
        changed[index].push(0);
        assert!(check(&changed, &e, 2000).is_err());
        let mut changed = records.clone();
        changed[index][8..10].copy_from_slice(&[255, 255]);
        assert!(check(&changed, &e, 2000).is_err());
    }
    assert!(matches!(
        parse_hello(&vec![b' '; 3073]),
        Err(AuthorizationError::Size)
    ));
}

#[test]
fn expected_data_field_mutations_do_not_silently_drop_any_binding() {
    type Mutation = fn(&mut ExpectedAuthorizationData);
    let mutations: &[Mutation] = &[
        |e| e.installer.public_key[0] ^= 1,
        |e| e.installer.sid[7] ^= 1,
        |e| e.installer.installation_epoch[0] ^= 1,
        |e| e.installer.delegation_commitment[0] ^= 1,
        |e| e.installer.delegation_id[0] ^= 1,
        |e| e.installer.not_before_ms += 1,
        |e| e.installer.not_after_ms -= 1,
        |e| e.publisher.public_key[0] ^= 1,
        |e| e.publisher.sid[7] ^= 1,
        |e| e.publisher.name.push('x'),
        |e| e.publisher.artifact[0] ^= 1,
        |e| e.publisher.pid += 1,
        |e| e.publisher.creation += 1,
        |e| e.publisher.epoch[0] ^= 1,
        |e| e.service.name.push('x'),
        |e| e.service.sid[7] ^= 1,
        |e| e.service.pid += 1,
        |e| e.service.creation += 1,
        |e| e.service.epoch[0] ^= 1,
        |e| e.service.base_plan[0] ^= 1,
        |e| e.service.artifact[0] ^= 1,
        |e| e.observer.name.push('x'),
        |e| e.observer.sid[7] ^= 1,
        |e| e.observer.pid += 1,
        |e| e.observer.creation += 1,
        |e| e.observer.epoch[0] ^= 1,
        |e| e.observer.base_plan[0] ^= 1,
        |e| e.observer.artifact[0] ^= 1,
        |e| e.scope.installation.push('x'),
        |e| e.scope.tenant.push('x'),
        |e| e.scope.origin.push('x'),
        |e| e.scope.revision.push('x'),
        |e| e.scope.application_authority.push('x'),
        |e| e.attempt.grant_id[0] ^= 1,
        |e| e.attempt.credential_epoch[0] ^= 1,
        |e| e.attempt.startup_cap_ms += 1,
        |e| e.attempt.earliest_issued_ms = 1001,
        |e| e.attempt.latest_not_after_ms = 4999,
    ];
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = e.clone();
        mutate(&mut changed);
        assert!(
            check(&records, &changed, 2000).is_err(),
            "expected field{index}"
        );
    }
}

#[test]
fn role_alias_key_reuse_and_invalid_context_are_rejected_before_hashing() {
    type Mutation = fn(&mut ExpectedAuthorizationData);
    let mutations: &[Mutation] = &[
        |e| e.publisher.public_key = e.installer.public_key,
        |e| e.publisher.sid = e.installer.sid.clone(),
        |e| e.service.sid = e.publisher.sid.clone(),
        |e| e.observer.sid = e.service.sid.clone(),
        |e| e.service.pid = e.publisher.pid,
        |e| e.observer.pid = e.service.pid,
        |e| e.service.creation = 0,
        |e| e.observer.role = AudienceRole::Service,
        |e| e.service.name = e.publisher.name.clone(),
        |e| e.publisher.name = "humanymous-lab-A".into(),
        |e| e.publisher.name = "humanymous-lab-".into(),
        |e| e.publisher.name = format!("humanymous-lab-{}", "a".repeat(49)),
        |e| e.publisher.sid[0] = 2,
        |e| e.publisher.sid[1] = 15,
        |e| e.scope.origin = "x".repeat(129),
        |e| e.scope.application_authority = "example test".into(),
        |e| e.scope.application_authority = "x".repeat(1025),
        |e| e.attempt.startup_cap_ms = 0,
        |e| e.installer.installation_epoch = [0; 16],
        |e| e.publisher.epoch = [0; 16],
        |e| e.service.epoch = [0; 16],
        |e| e.observer.epoch = [0; 16],
        |e| e.attempt.credential_epoch = [0; 16],
        |e| e.attempt.grant_id = [0; 32],
        |e| e.installer.delegation_id = [0; 32],
    ];
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for mutate in mutations {
        let mut changed = e.clone();
        mutate(&mut changed);
        assert!(matches!(
            check(&records, &changed, 2000),
            Err(AuthorizationError::Context)
        ));
    }
    // I!=S/O is an unaccepted extra policy, not a hidden mandatory codec gate.
    let mut e = fixture::expected(false);
    e.installer.sid = e.service.sid.clone();
    let records = fixture::records(&mut e);
    check(&records, &e, 2000).unwrap();
}

#[test]
fn issuer_domain_cross_grant_and_noncanonical_signatures_are_not_interchangeable() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for (index, kind, seed) in [
        (0, "prepared", 1),
        (0, "delegation", 2),
        (1, "commit", 2),
        (2, "prepared", 2),
    ] {
        let mut changed = records.clone();
        let length = changed[index].len() - 64;
        changed[index] = fixture::wrap(
            changed[index][..8].try_into().unwrap(),
            kind,
            &changed[index][10..length],
            seed,
        );
        assert!(check(&changed, &e, 2000).is_err());
    }
    let mut other = e.clone();
    other.attempt.grant_id = [55; 32];
    let other_records = fixture::records(&mut other);
    let mut changed = records.clone();
    changed[2] = other_records[2].clone();
    assert!(check(&changed, &e, 2000).is_err());
    for index in 0..3 {
        let mut changed = records.clone();
        let end = changed[index].len();
        changed[index][end - 1] = 255;
        assert!(check(&changed, &e, 2000).is_err());
    }
}

#[test]
fn low_order_key_and_identity_signature_forgeries_are_strictly_rejected() {
    let mut identity = [0; 32];
    identity[0] = 1;
    for issuer in [true, false] {
        let mut e = fixture::expected(false);
        if issuer {
            e.installer.public_key = identity;
        } else {
            e.publisher.public_key = identity;
        }
        let mut records = fixture::records(&mut e);
        let index = if issuer { 0 } else { 1 };
        let offset = records[index].len() - 64;
        // R is the identity point and S=0. A permissive equation-only verifier
        // under an identity public key can accept arbitrary messages; strict
        // validation must reject before treating this as signature-checked DATA.
        records[index][offset..].fill(0);
        records[index][offset] = 1;
        if issuer {
            e.installer.delegation_commitment = fixture::digest("delegation-record", &records[0]);
        }
        assert!(matches!(
            check(&records, &e, 2000),
            Err(AuthorizationError::Signature)
        ));
    }
}

#[test]
fn checked_lifetimes_expiry_equality_and_cap_cannot_renew() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for now in [0, 1000, 5000, 5001, u64::MAX] {
        assert!(matches!(
            check(&records, &e, now),
            Err(AuthorizationError::Lifetime)
        ));
    }
    check(&records, &e, 1001).unwrap();
    check(&records, &e, 4999).unwrap();
    for (issued, commit, expiry, cap) in [
        (1000, 999, 5000, 1000),
        (1000, 5000, 5000, 1000),
        (5001, 5001, 5000, 1000),
        (1000, 1001, 5000, 999),
        (u64::MAX, 1, 5000, 1000),
        (1000, 1001, u64::MAX, 1000),
    ] {
        let mut e = fixture::expected(false);
        let records = fixture::timed(&mut e, issued, commit, expiry, cap);
        assert!(check(&records, &e, 2000).is_err());
    }
    let mut e = fixture::expected(false);
    e.attempt.startup_cap_ms = 4001;
    let records = fixture::records(&mut e);
    assert!(matches!(
        check(&records, &e, 2000),
        Err(AuthorizationError::Lifetime)
    ));
}

#[test]
fn legacy_versions_mixed_magic_and_zero_identifiers_fail_closed() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    for index in 0..3 {
        let mut changed = records.clone();
        changed[index][7] = b'1';
        assert!(check(&changed, &e, 2000).is_err());
        let mut changed = records.clone();
        changed[index][11] = 1;
        assert!(check(&changed, &e, 2000).is_err());
    }
    let mut e = fixture::expected(false);
    e.attempt.grant_id = [0; 32];
    let records = fixture::records(&mut e);
    assert!(parse_bundle(&records[0], &records[1], &records[2]).is_err());
}

#[test]
fn canonical_hello_rejects_duplicate_unknown_whitespace_escape_and_wrong_versions() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    let bytes = hello::fixture_hello(&records, &e);
    let text = String::from_utf8(bytes.clone()).unwrap();
    for changed in [
        format!(" {text}"),
        format!("{text}\n"),
        text.replace("\"phase\":", "\"phase\":\"hello\",\"phase\":"),
        text.replace("\"phase\":", "\"unknown\":1,\"phase\":"),
        text.replace("\"hello\"", "\"he\\u006clo\""),
        text.replace("/0.3", "/0.2"),
        text.replace("authorization/2", "signed-grant/1"),
    ] {
        assert!(parse_hello(changed.as_bytes()).is_err());
    }
    let parsed = parse_hello(&bytes).unwrap();
    assert_eq!(parsed.nonce(), &[0x11; 16]);
    verify_hello(parsed, &e, 2000).unwrap();
}

#[test]
fn hello_repeated_bindings_and_nonce_shapes_are_not_selected_from_response() {
    let mut e = fixture::expected(false);
    let records = fixture::records(&mut e);
    let bytes = hello::fixture_hello(&records, &e);
    for field in [
        "bootstrap_scope",
        "grant_id",
        "service_epoch",
        "observer_epoch",
        "credential_epoch",
        "sender_pin",
        "receiver_pin",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let s = value[field].as_str().unwrap();
        value[field] = format!("{}{}", if s.starts_with('f') { "e" } else { "f" }, &s[1..]).into();
        // Preserve the exact canonical field order through the declared DTO.
        let data = hello::canonical_fixture(value);
        assert!(verify_hello(parse_hello(&data).unwrap(), &e, 2000).is_err());
    }
    let text = String::from_utf8(bytes).unwrap();
    for nonce in ["00".repeat(16), "11".repeat(32), "AA".repeat(16)] {
        assert!(parse_hello(text.replace(&"11".repeat(16), &nonce).as_bytes()).is_err());
    }
}

#[test]
fn base64url_unused_bits_padding_and_alternate_alphabets_refuse() {
    let mut e = fixture::expected(true);
    let records = fixture::records(&mut e);
    let bytes = hello::fixture_hello(&records, &e);
    let text = String::from_utf8(bytes).unwrap();
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    for (index, bits) in [(0, 2), (1, 4), (2, 2)] {
        let encoded = URL_SAFE_NO_PAD.encode(&records[index]);
        let mut changed = encoded.as_bytes().to_vec();
        let last = changed.len() - 1;
        let position = ALPHABET.iter().position(|b| *b == changed[last]).unwrap();
        for low in 1..(1 << bits) {
            changed[last] = ALPHABET[position | low];
            let invalid = String::from_utf8(changed.clone()).unwrap();
            assert!(parse_hello(text.replace(&encoded, &invalid).as_bytes()).is_err());
        }
        for invalid in [
            format!("{encoded}="),
            format!(" {encoded}"),
            format!("+{}", &encoded[1..]),
        ] {
            assert!(parse_hello(text.replace(&encoded, &invalid).as_bytes()).is_err());
        }
    }
}

#[test]
fn variable_record_lengths_cover_all_permitted_base64_remainders() {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut seen = [[false; 3]; 3];
    for p_extra in 0..3 {
        for s_extra in 0..3 {
            let mut e = fixture::expected(false);
            e.publisher.name.push_str(&"p".repeat(p_extra));
            e.service.name.push_str(&"s".repeat(s_extra));
            let records = fixture::records(&mut e);
            let wire = hello::fixture_hello(&records, &e);
            verify_hello(parse_hello(&wire).unwrap(), &e, 2000).unwrap();
            let text = String::from_utf8(wire).unwrap();
            for (index, record) in records.iter().enumerate() {
                let remainder = record.len() % 3;
                seen[index][remainder] = true;
                let unused = match remainder {
                    0 => continue, // Every bit is data; the positive case above must pass.
                    1 => 4,
                    2 => 2,
                    _ => unreachable!(),
                };
                let encoded = URL_SAFE_NO_PAD.encode(record);
                let mut changed = encoded.as_bytes().to_vec();
                let last = changed.len() - 1;
                let position = ALPHABET.iter().position(|b| *b == changed[last]).unwrap();
                for low in 1..(1 << unused) {
                    changed[last] = ALPHABET[position | low];
                    assert!(
                        parse_hello(
                            text.replace(&encoded, std::str::from_utf8(&changed).unwrap())
                                .as_bytes()
                        )
                        .is_err()
                    );
                }
            }
        }
    }
    assert_eq!(seen[0], [true; 3]);
    assert_eq!(seen[1], [true; 3]);
    // Commit's exact201byte body plus74byte envelope is always275 (remainder2).
    assert_eq!(seen[2], [false, false, true]);
}

#[test]
fn public_fixture_domain_vectors_have_stable_artifact_bytes() {
    let profiles = [
        (
            false,
            include_str!("../../../../tests/fixtures/authority/authorization-v2-minimal.json"),
            [
                "5362954b2b6cdf6efcf647aa13880a5757a7a885aa14e2bcff52cb70b81c2e40",
                "a6b3068d420acc71b465c41e9741a22965b648fc12e76dfb12bcdbf6539810a5",
                "d7e4966334b7cef1355f0370c3bee2a3159898650b6bdee74d9c6ed7d787ecbc",
            ],
        ),
        (
            true,
            include_str!("../../../../tests/fixtures/authority/authorization-v2-maximum.json"),
            [
                "77852fe1a8b9b0bac4129483314cda682ef70689b1414df416bd05b9d455c456",
                "b6e6a1f06cda49da379d11b82861d2b8699a06fbf891eb5b0bf403825a900f6c",
                "9ef2eab3d021c9e9d2f7d717eb9157d626018f3f8b3ad845eeaf7d0a29028161",
            ],
        ),
    ];
    for (max, saved, hashes) in profiles {
        let mut e = fixture::expected(max);
        let records = fixture::records(&mut e);
        let vector: serde_json::Value = serde_json::from_str(saved).unwrap();
        assert_eq!(fixture::public_vector(&e, &records), vector);
        for (index, record) in records.iter().enumerate() {
            assert_eq!(
                hex::encode(fixture::digest("golden-fixture", record)),
                hashes[index]
            );
            let field = ["delegation", "prepared", "commit"][index];
            assert_eq!(
                *record,
                URL_SAFE_NO_PAD
                    .decode(vector[field].as_str().unwrap())
                    .unwrap()
            );
        }
        // These are fixed regression bytes/signatures/commitments captured from
        // an independently written test encoder, NOT an external crypto oracle
        // or native issuer/provenance/freshness evidence.
        check(&records, &e, 2000).unwrap();
    }
}
