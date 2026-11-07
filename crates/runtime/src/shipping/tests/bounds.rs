use super::*;

#[test]
fn every_scope_token_enforces_exact_ascii_length_and_alphabet() {
    for key in ["installation", "tenant", "origin", "revision"] {
        let mut v = document();
        v["scope"][key] = json!("a".repeat(128));
        assert!(parse(&v).is_ok());
        v["scope"][key] = json!("AZaz09._:-");
        assert!(parse(&v).is_ok());
        for bad in [
            String::new(),
            "a".repeat(129),
            "a b".into(),
            "a/b".into(),
            "é".into(),
            "a\n".into(),
        ] {
            v["scope"][key] = json!(bad);
            invalid(&encoded(&v));
        }
    }
}

#[test]
fn authority_is_exact_bounded_ascii_data_without_address_normalization() {
    let mut v = document();
    v["scope"]["application_authority"] = json!("a".repeat(1024));
    assert_eq!(
        parse(&v).unwrap().scope().application_authority().len(),
        1024
    );
    for allowed in ["Example.TEST:443", "[::1]:8443", "!"] {
        v["scope"]["application_authority"] = json!(allowed);
        assert_eq!(parse(&v).unwrap().scope().application_authority(), allowed);
    }
    for bad in [
        String::new(),
        "a".repeat(1025),
        "é".into(),
        "a b".into(),
        "a\u{7f}".into(),
    ] {
        v["scope"]["application_authority"] = json!(bad);
        invalid(&encoded(&v));
    }
    for bad in ['@', '/', '#', '?', '\\', ','] {
        v["scope"]["application_authority"] = json!(format!("a{bad}b"));
        invalid(&encoded(&v));
    }
}

#[test]
fn epoch_requires_exact_lower_hex_and_nonzero_but_proves_no_freshness() {
    let mut v = document();
    for epoch in ["01".repeat(16), "ff".repeat(16)] {
        v["binding"]["installation_epoch"] = json!(epoch);
        assert!(parse(&v).is_ok());
    }
    for bad in [
        "00".repeat(16),
        "AA".repeat(16),
        "gg".repeat(16),
        "1".repeat(31),
        "1".repeat(33),
        "é".repeat(16),
    ] {
        v["binding"]["installation_epoch"] = json!(bad);
        invalid(&encoded(&v));
    }
}

#[test]
fn both_digest_fields_are_exact_lower_hex_external_document_assertions() {
    for key in ["base_plan_sha256", "executable_sha256"] {
        let mut v = document();
        for good in ["00".repeat(32), "ff".repeat(32)] {
            v["binding"][key] = json!(good);
            assert!(parse(&v).is_ok());
        }
        for bad in [
            "A".repeat(64),
            "g".repeat(64),
            "a".repeat(63),
            "a".repeat(65),
            "a ".repeat(32),
        ] {
            v["binding"][key] = json!(bad);
            invalid(&encoded(&v));
        }
    }
}

#[test]
fn generation_requires_positive_safe_integer_without_coercion_or_overflow() {
    let mut v = document();
    for good in [1_u64, 9_007_199_254_740_991] {
        v["binding"]["generation"] = json!(good);
        assert_eq!(parse(&v).unwrap().binding().generation(), good);
    }
    let original = String::from_utf8(encoded(&document())).unwrap();
    for bad in [
        "0",
        "-0",
        "-1",
        "1.0",
        "1e0",
        "9007199254740992",
        "18446744073709551616",
        "1e999",
        "\"1\"",
        "null",
        "false",
        "[]",
        "{}",
    ] {
        invalid(
            original
                .replace("\"generation\":1", &format!("\"generation\":{bad}"))
                .as_bytes(),
        );
    }
}
