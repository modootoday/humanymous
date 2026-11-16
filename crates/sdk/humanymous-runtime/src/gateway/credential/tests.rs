//! Shared parsing and primitive-selection boundaries for the verifier kernels.
use super::*;

#[test]
fn line_parser_skips_comments_and_refuses_bad_shapes() {
    assert_eq!(parse_lines("# c\n\n  a  b  \n"), Ok(vec![("a", "b")]));
    assert_eq!(parse_lines(""), Err(ConfigError::Empty));
    assert_eq!(parse_lines("a"), Err(ConfigError::Syntax));
    assert_eq!(parse_lines("a b c"), Err(ConfigError::Syntax));
    let long = format!("{} b", "x".repeat(MAX_ID + 1));
    assert_eq!(parse_lines(&long), Err(ConfigError::Syntax));
    let many = "a b\n".repeat(MAX_ENTRIES + 1);
    assert_eq!(parse_lines(&many), Err(ConfigError::Capacity));
    let huge = "#".repeat(MAX_CONFIG + 1);
    assert_eq!(parse_lines(&huge), Err(ConfigError::Capacity));
}

#[test]
fn loose_base64_accepts_padded_and_unpadded_only() {
    assert_eq!(decode_b64_loose("YQ=="), Some(b"a".to_vec()));
    assert_eq!(decode_b64_loose("YQ"), Some(b"a".to_vec()));
    assert_eq!(
        decode_b64_loose("YR=="),
        None,
        "non-canonical trailing bits"
    );
    assert_eq!(decode_b64_loose("Y_=="), None, "url alphabet");
}

#[test]
fn der_reader_is_strict() {
    assert_eq!(
        Der(&[0x02, 0x01, 0x05]).positive_integer(),
        Some(&[5u8][..])
    );
    assert_eq!(
        Der(&[0x02, 0x02, 0x00, 0x80]).positive_integer(),
        Some(&[0x80u8][..])
    );
    assert_eq!(Der(&[0x02, 0x02, 0x00, 0x7f]).positive_integer(), None);
    assert_eq!(Der(&[0x02, 0x01, 0x80]).positive_integer(), None);
    assert_eq!(Der(&[0x02, 0x00]).positive_integer(), None);
    assert_eq!(
        Der(&[0x04, 0x81, 0x01, 0x00]).read(0x04),
        None,
        "non-minimal length"
    );
    assert_eq!(Der(&[0x04, 0x02, 0x00]).read(0x04), None, "truncated");
    assert_eq!(Der(&[0x04, 0x01, 0x00]).read(0x05), None, "tag");
    let mut long = vec![0x04, 0x81, 0x80];
    long.extend_from_slice(&[0; 0x80]);
    assert_eq!(Der(&long).read(0x04).map(<[u8]>::len), Some(0x80));
    assert_eq!(spki_parts(&[0x30, 0x00]), None);
}

#[test]
fn pem_reader_accepts_only_public_key_blocks() {
    let block = "-----BEGIN PUBLIC KEY-----\nYQ==\n-----END PUBLIC KEY-----\n";
    assert_eq!(pem_public_keys(block), Ok(vec![b"a".to_vec()]));
    assert_eq!(pem_public_keys("\n"), Err(ConfigError::Empty));
    let private = "-----BEGIN PRIVATE KEY-----\nYQ==\n-----END PRIVATE KEY-----\n";
    assert_eq!(pem_public_keys(private), Err(ConfigError::Syntax));
}

#[test]
fn ring_algorithms_exist_and_unknown_pairs_fail_closed() {
    use rustls::pki_types::alg_id;
    assert!(ring_algorithm(SignatureScheme::RSA_PSS_SHA384, alg_id::RSA_ENCRYPTION).is_some());
    assert!(ring_algorithm(SignatureScheme::ECDSA_NISTP256_SHA256, alg_id::ECDSA_P256).is_some());
    assert!(ring_algorithm(SignatureScheme::RSA_PSS_SHA384, alg_id::ECDSA_P256).is_none());
    assert!(!ring_verify(
        SignatureScheme::ECDSA_NISTP256_SHA256,
        alg_id::ECDSA_P256,
        &[4; 65],
        b"message",
        &[0; 70],
    ));
}
