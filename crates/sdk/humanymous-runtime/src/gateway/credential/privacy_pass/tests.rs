//! Ports of legacy internal/gate/privacypass_test.go (baseline 981d25c) plus
//! the fail-closed boundaries this kernel adds.
//!
//! The issuer keys are throwaway RSA-2048 test fixtures generated for these
//! tests only; they protect nothing. The mock issuer PSS-signs the token
//! input directly, because the unblinded RSABSSA authenticator is a plain
//! RSASSA-PSS-SHA384 signature (as in the Go test).
use super::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rcgen::PublicKeyData;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs1KeyDer};

/// Issuer A private key, PKCS#1 DER (test fixture).
const ISSUER_A_KEY: &str = concat!(
    "MIIEowIBAAKCAQEAygRNNevNuJl1InL7cX/r4cGCoe7uGCITmcPR9fno/+Q666KGPXh1Yyxx1pfs",
    "xkhvsuUDg961XmqajQKJa0PNnZIDthOb8NPCjbVFmmOjKo27NJwTlQy6AlfLrHP54eARWVai+8Rd",
    "2jGYujgHsjppDE2gKc5R2xcT7VDkzyq21UZzXMOkuPan3pDR3Gk9aKUA6lhdLL0oZmyBFz3qDEKM",
    "RHAbmgTKajiu1S8mXjdsbigPMXSqEa2azVMIkk9AwgYRDroSfAZM3Ng2mF8DULY1rQ6UaEtyiBK4",
    "M5P5WYr4BQSYdAmQ9jdfzTk/NIsbg/nIgkMJ0QlYUSh8/79F0nA4owIDAQABAoIBACBVK5W86GpV",
    "EVTSqgt36bpyZ3ngd7YKxNqq5ztjD2zeoU7ebnV/MUE5YfzycusnY81HmdZCJQLNYBq/3wZOeKXG",
    "ssEHsWvhkKAan7RpTGT+qzRCjgsghCg6LLcswWqVRgw88WKyApgYsQo49oO1lTjqtv7fQ+Wjc2/b",
    "ryC4hEEXt7yRhA5xcwkycEROJ5M6jN1Fp2h9OuqhlauqD9GKEyGqs2eEAap2T0Z44Hg6e+cC/Yib",
    "YvrKMBKDXhvM1w2b3OwEpZZ1tnGIHWpw1hZz7qlW/dlW3ZdUX951T3xHHVFreaSufarsUk8flnA8",
    "V6svchTSmaCmPdbj2MBrT3E0ZnECgYEA8dGPkcNmWqpw5JiP4fGU4+4bdqMLzxDjYvHz+Ih1Q/A0",
    "R92ZR5Ml16lYhxdxaYMf8ANE4IW8fsm+L6ZRikuWO5B1lodIHSqlfUmbb7qIWNsxG7eMRqTtNHPx",
    "aVeiE3FmcdbzyBSGAOhyuKCctP8os2hVyYiUouj+Xx0yFfT0GpMCgYEA1d0xmAKhAxx0mL2a433P",
    "713PISqBZr6eyX/F4dsY2yRuJIXubvEXVFsKSY0pcmnec4eaw/DuKemWF9lVdd+A0/iDGHY59MxR",
    "xQ3FvWU6ZHAime10sqQYGT+UbLOrW83HUHZVQKDaqgyzzwo3g/Ox8BEJ2zcYGINOx4idB6nxY7EC",
    "gYBANrDUOTol21rNKUIT8q3h9OmwEE3jd42aSbvKhJn3J4MKjmrUmduTsDGE9hG0vhAuEQAqSvkq",
    "BfKN/rFqcvX67Tp8dYIN1LRLV8+B1Nu+FL6uqIgEfmlK43r4yT+NwDA/DKCBwglvlkpYgrX6TkOF",
    "mLAnKzClLg/WPA9/yyNDJwKBgCDxTIVcyk5/YJpyglxOcewuKAy0zm/0JuNk8idDJ4YCC5FGrB6c",
    "tnpA98mym9LK6gAjwDw0bYs5Nux/hsF8Zwy/t/nZpD3IB0RyRaZR5He6KipJwHMQAcJJ3+QKnPes",
    "L+p9/83PuT7+a+gVa83AiQUwqrMAvo5wE3961K1siQlRAoGBALmCboi0dHHrXT+/jzpgLk7XpsBQ",
    "YaCApAy1fWc6IR/UYI2JmKeP9yRSAjW3ViLskMGXNkykw+eJTkskZW3R6bmXNId+Xo2LkzWyaDGT",
    "Y/3+GHwXiIaSMUZbmP2dWEIQRN1sjbXHI+XqvmizlWHj0eZp4jZDaMVUhO2cqw1qcX2f",
);
/// Issuer A SubjectPublicKeyInfo DER (rsaEncryption).
pub(in crate::gateway) const ISSUER_A_SPKI: &str = concat!(
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAygRNNevNuJl1InL7cX/r4cGCoe7uGCIT",
    "mcPR9fno/+Q666KGPXh1Yyxx1pfsxkhvsuUDg961XmqajQKJa0PNnZIDthOb8NPCjbVFmmOjKo27",
    "NJwTlQy6AlfLrHP54eARWVai+8Rd2jGYujgHsjppDE2gKc5R2xcT7VDkzyq21UZzXMOkuPan3pDR",
    "3Gk9aKUA6lhdLL0oZmyBFz3qDEKMRHAbmgTKajiu1S8mXjdsbigPMXSqEa2azVMIkk9AwgYRDroS",
    "fAZM3Ng2mF8DULY1rQ6UaEtyiBK4M5P5WYr4BQSYdAmQ9jdfzTk/NIsbg/nIgkMJ0QlYUSh8/79F",
    "0nA4owIDAQAB",
);
/// Issuer B SubjectPublicKeyInfo DER, an issuer the tokens are not from.
const ISSUER_B_SPKI: &str = concat!(
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtbVgVsMjwsGUQKgZZuQta090XPdZNI5h",
    "iVXevhoq7y7dw0SaPf/tVHMS6sTl7vtR0y5cL4QrfLtB1mvePQluzYGBUZYJu4k/a4buj+0OH9nP",
    "4awmUQ34xjT15PwAlqbBUBeQNXTEE1lVFi1Ff1XeP9jIOAxVB/2ZAjf/SpNjY0qHLW5QwZ9aBSno",
    "aFqywP34iaFaYTkSwhVG95U7/aF7dsTxpOZu2t/FvmW/9GI32pyvi4UiLjy5o//xefF+FUUKHw3t",
    "hnYLd2/2Ee4Nq+bLZ4nrsf+egkMcMleYcAWAAceuoZe2CrJF7uuUtF9Cp47rT0N2xxKmVzUu6Yx2",
    "kxdnqwIDAQAB",
);
pub(in crate::gateway) const NOW: Duration = Duration::from_secs(1_700_000_000);
const PREFIX_HEADER: &str = "PrivateToken token=\"";

/// Go `pubPEM`.
fn pem(spki_b64: &str) -> String {
    let lines: Vec<&str> = spki_b64
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        lines.join("\n")
    )
}
fn key_id(spki_b64: &str) -> [u8; 32] {
    Sha256::digest(STANDARD.decode(spki_b64).unwrap()).into()
}
fn pss_sign(message: &[u8], scheme: SignatureScheme) -> Vec<u8> {
    let der = STANDARD.decode(ISSUER_A_KEY).unwrap();
    let key = rustls::crypto::ring::sign::any_supported_type(&PrivateKeyDer::Pkcs1(
        PrivatePkcs1KeyDer::from(der),
    ))
    .unwrap();
    key.choose_scheme(&[scheme]).unwrap().sign(message).unwrap()
}
/// Raw token bytes; Go `mintPAT` with an explicit nonce and challenge digest.
fn mint_raw(nonce: u8, challenge: [u8; 32], scheme: SignatureScheme) -> Vec<u8> {
    let mut raw = TOKEN_TYPE.to_be_bytes().to_vec();
    raw.extend_from_slice(&[nonce; 32]);
    raw.extend_from_slice(&challenge);
    raw.extend_from_slice(&key_id(ISSUER_A_SPKI));
    let signature = pss_sign(&raw, scheme);
    raw.extend_from_slice(&signature);
    raw
}
fn header(raw: &[u8]) -> String {
    format!("{PREFIX_HEADER}{}\"", STANDARD.encode(raw))
}
/// Go `mintPAT`: zero nonce and challenge digest.
pub(in crate::gateway) fn mint() -> String {
    header(&mint_raw(0, [0; 32], SignatureScheme::RSA_PSS_SHA384))
}
pub(in crate::gateway) fn verifier(spki_b64: &str) -> Verifier {
    Verifier::from_pem(&pem(spki_b64)).unwrap()
}
fn issuer_a() -> String {
    hex::encode(key_id(ISSUER_A_SPKI))
}
fn rejected(reason: Reason) -> Verdict {
    Verdict::Invalid {
        reason,
        key_id: Some(issuer_a()),
    }
}

#[test]
fn pat_verified() {
    let mut v = verifier(ISSUER_A_SPKI);
    assert_eq!(
        v.verify(Some(&mint()), NOW),
        Verdict::Verified { key_id: issuer_a() }
    );
}

#[test]
fn pat_tampered() {
    let mut v = verifier(ISSUER_A_SPKI);
    let mut raw = mint_raw(0, [0; 32], SignatureScheme::RSA_PSS_SHA384);
    *raw.last_mut().unwrap() ^= 0xff;
    assert_eq!(
        v.verify(Some(&header(&raw)), NOW),
        rejected(Reason::BadSignature)
    );
}

#[test]
fn pat_unknown_issuer() {
    let mut v = verifier(ISSUER_B_SPKI);
    assert_eq!(v.verify(Some(&mint()), NOW), invalid(Reason::UnknownIssuer));
}

#[test]
fn pat_none() {
    let mut v = verifier(ISSUER_A_SPKI);
    assert_eq!(v.verify(None, NOW), Verdict::Absent);
}

#[test]
fn pat_double_spend() {
    let mut v = verifier(ISSUER_A_SPKI);
    let token = mint();
    assert!(matches!(
        v.verify(Some(&token), NOW),
        Verdict::Verified { .. }
    ));
    assert_eq!(v.verify(Some(&token), NOW), rejected(Reason::DoubleSpend));
}

#[test]
fn key_id_matches_go_spki_hash_and_header_forms() {
    let raw = mint_raw(1, [0; 32], SignatureScheme::RSA_PSS_SHA384);
    assert_eq!(&raw[66..98], &key_id(ISSUER_A_SPKI));
    let mut v = verifier(ISSUER_A_SPKI);
    let bare = STANDARD.encode(&raw);
    assert!(matches!(
        v.verify(Some(&bare), NOW),
        Verdict::Verified { .. }
    ));
    let unpadded = mint_raw(2, [0; 32], SignatureScheme::RSA_PSS_SHA384);
    let trimmed = STANDARD.encode(&unpadded).trim_end_matches('=').to_owned();
    let with_params = format!("PrivateToken token={trimmed}, extra=1");
    assert!(matches!(
        v.verify(Some(&with_params), NOW),
        Verdict::Verified { .. }
    ));
}

#[test]
fn spent_nonce_expires_after_ttl() {
    let mut v = verifier(ISSUER_A_SPKI);
    let token = mint();
    assert!(matches!(
        v.verify(Some(&token), NOW),
        Verdict::Verified { .. }
    ));
    let later = NOW + SPENT_TTL + Duration::from_secs(1);
    assert!(matches!(
        v.verify(Some(&token), later),
        Verdict::Verified { .. }
    ));
}

#[test]
fn rfc_salt_length_and_shape_are_enforced() {
    let mut v = verifier(ISSUER_A_SPKI);
    let other_hash = mint_raw(3, [0; 32], SignatureScheme::RSA_PSS_SHA256);
    assert_eq!(
        v.verify(Some(&header(&other_hash)), NOW),
        rejected(Reason::BadSignature)
    );
    let mut short = mint_raw(4, [0; 32], SignatureScheme::RSA_PSS_SHA384);
    short.pop();
    assert_eq!(
        v.verify(Some(&header(&short)), NOW),
        rejected(Reason::BadSignature)
    );
    let mut wrong_type = mint_raw(5, [0; 32], SignatureScheme::RSA_PSS_SHA384);
    wrong_type[1] = 0x01;
    assert_eq!(
        v.verify(Some(&header(&wrong_type)), NOW),
        invalid(Reason::TokenType)
    );
    for malformed in [
        "PrivateToken".to_owned(),
        "PrivateToken realm=x".to_owned(),
        format!("{PREFIX_HEADER}!!!\""),
        header(&[0; PREFIX]),
        format!("{PREFIX_HEADER}{}\"", "A".repeat(MAX_HEADER)),
        "   ".to_owned(),
    ] {
        assert_eq!(v.verify(Some(&malformed), NOW), invalid(Reason::Malformed));
    }
}

#[test]
fn challenge_binding_when_configured() {
    let mut v = verifier(ISSUER_A_SPKI);
    let accepted = Sha256::digest(b"issued challenge").into();
    v.accept_challenges(&[accepted]).unwrap();
    let bound = mint_raw(6, accepted, SignatureScheme::RSA_PSS_SHA384);
    assert!(matches!(
        v.verify(Some(&header(&bound)), NOW),
        Verdict::Verified { .. }
    ));
    let foreign = mint_raw(7, [9; 32], SignatureScheme::RSA_PSS_SHA384);
    assert_eq!(
        v.verify(Some(&header(&foreign)), NOW),
        rejected(Reason::Challenge)
    );
    assert_eq!(v.accept_challenges(&[]), Err(ConfigError::Empty));
}

#[test]
fn issuer_configuration_is_strict() {
    assert_eq!(Verifier::from_pem("").err(), Some(ConfigError::Empty));
    let both = format!("{}\n{}", pem(ISSUER_A_SPKI), pem(ISSUER_B_SPKI));
    assert!(Verifier::from_pem(&both).is_ok());
    let twice = format!("{}{}", pem(ISSUER_A_SPKI), pem(ISSUER_A_SPKI));
    assert_eq!(
        Verifier::from_pem(&twice).err(),
        Some(ConfigError::Duplicate)
    );
    let prose = format!("issuer keys\n{}", pem(ISSUER_A_SPKI));
    assert_eq!(Verifier::from_pem(&prose).err(), Some(ConfigError::Syntax));
    let unterminated = "-----BEGIN PUBLIC KEY-----\nAAAA\n";
    assert_eq!(
        Verifier::from_pem(unterminated).err(),
        Some(ConfigError::Syntax)
    );
    let garbage = "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----\n";
    assert_eq!(Verifier::from_pem(garbage).err(), Some(ConfigError::Key));
    // A P-256 key is not an RSA issuer.
    let ec = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
    let ec_pem = pem(&STANDARD.encode(ec.subject_public_key_info()));
    assert_eq!(Verifier::from_pem(&ec_pem).err(), Some(ConfigError::Key));
}

/// Independent oracle: OpenSSL 3 `dgst -sha384 -sign` with PSS over the token
/// input `0x0002 || 0x0b*32 || 0x00*32 || token_key_id(issuer A)`, with salt
/// lengths 48 and 0. Go's `PSSSaltLengthAuto` accepted both.
#[test]
fn openssl_vectors_pin_rfc_salt_length() {
    const SALT_48: &str = concat!(
        "S7zBBMNix90GtrCbFPTSmrO8SjRxkyh1mgWji0nSJn0OVg7yIZ+9nbhg51qJ4S23CrMsFED0aeSK",
        "wGCKi3iQ6PMlPsBTYBrfZGb0X7fCmguZbxo9aG3VVIsYuv/H6RyCTtLsEvpYie246Ixe8pC9EWw8",
        "BWSYjZF4axHn/tHiVTYFREYd0ft+FwtBi7BauVc2rRAmw/dD5k3n7S7GddAM5jiCGXeaqyIeLVLC",
        "KMTX/+Ub2SxCjzNCzb31AHdGLo1gxEBG22QMVws0/Ko6XdXdiOZdsSGq0KoaU3mWUGHY31UTlT/S",
        "JlV2icj/L6ITeXW1mRsQ7LcOtJTDrT4hLojj3g==",
    );
    const SALT_0: &str = concat!(
        "cuXJ79dRJHuEk1yc42hKW6ze+HZh6jITPu578pa4U9ssF9DvCfGZjvhzKWTuNtbcsIN9f4G+0q+q",
        "AbBx7JYSoEqth3jt5EuxIHkSpTHcFx4n4Evf3i+ouEMC/VvxCzHBl/Tx+bfNQjkHkiTP7Acb2atJ",
        "yHIeJHzjjKuMfo/6Gt19TlaTVqj/AoGkdJq2Z0T1zmiwDL2Ui41mWGnwgv5U1ahv9nWeAJe7MDYS",
        "+LKHeo3wPrWn+6eBLD7VI/ysrJQxYhsPlyAiOAwvst5/U5kxUdNVj62HxoO6HCZXf7Qbrf7Y2XFs",
        "kLQwybX1DPtiTtHNl+sFtZEjmdoVHwOF86wDyA==",
    );
    let token = |signature: &str| {
        let mut raw = TOKEN_TYPE.to_be_bytes().to_vec();
        raw.extend_from_slice(&[0x0b; 32]);
        raw.extend_from_slice(&[0; 32]);
        raw.extend_from_slice(&key_id(ISSUER_A_SPKI));
        raw.extend_from_slice(&STANDARD.decode(signature).unwrap());
        header(&raw)
    };
    let mut v = verifier(ISSUER_A_SPKI);
    assert_eq!(
        v.verify(Some(&token(SALT_0)), NOW),
        rejected(Reason::BadSignature)
    );
    assert_eq!(
        v.verify(Some(&token(SALT_48)), NOW),
        Verdict::Verified { key_id: issuer_a() }
    );
}

/// Wargame R205 (legacy wargame_axis_cred_r201_test.go): a token of another
/// type that is correctly PSS-signed by a configured issuer is still refused
/// by the type gate, before any signature check or spend.
#[test]
fn wargame_r205_correctly_signed_wrong_token_type_is_invalid() {
    let mut v = verifier(ISSUER_A_SPKI);
    let mut raw = 0x0001u16.to_be_bytes().to_vec();
    raw.extend_from_slice(&[0; 64]);
    raw.extend_from_slice(&key_id(ISSUER_A_SPKI));
    let signature = pss_sign(&raw, SignatureScheme::RSA_PSS_SHA384);
    raw.extend_from_slice(&signature);
    assert_eq!(
        v.verify(Some(&header(&raw)), NOW),
        invalid(Reason::TokenType)
    );
}

/// Wargame R238: junk parameters after a quoted token. Go accepted either a
/// verified or an invalid result. Rust difference (fail closed): the closing
/// quote stays attached once the value is cut at `;`, so the token is
/// `Invalid(Malformed)`, and that refusal spends no nonce.
#[test]
fn wargame_r238_junk_after_quoted_token_is_malformed_and_spends_nothing() {
    let mut v = verifier(ISSUER_A_SPKI);
    let token = mint();
    let junked = format!("{token}; evil=1");
    assert_eq!(v.verify(Some(&junked), NOW), invalid(Reason::Malformed));
    assert_eq!(
        v.verify(Some(&token), NOW),
        Verdict::Verified { key_id: issuer_a() }
    );
}
