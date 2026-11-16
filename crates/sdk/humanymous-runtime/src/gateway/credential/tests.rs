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

/// Cross-kernel compositions from legacy wargame_axis_cred_r201_test.go
/// (R235, R236, R240). The kernels share no state; these pin that one plane's
/// success never changes another plane's refusal.
mod compositions {
    use super::super::privacy_pass::{self, tests as pat};
    use super::super::web_bot_auth::{self, tests as wba};
    use super::super::webauthn::{self, tests as wan};
    use crate::network::proxy::{Allowlist, Refusal};

    fn pat_verified(verdict: &privacy_pass::Verdict) -> bool {
        matches!(verdict, privacy_pass::Verdict::Verified { .. })
    }
    fn webauthn_verified() -> webauthn::Verdict {
        webauthn::Verdict::Verified {
            credential_id: wan::CRED.into(),
        }
    }

    #[test]
    fn wargame_r235_pat_does_not_launder_wba_forgery() {
        let mut pv = pat::verifier(pat::ISSUER_A_SPKI);
        assert!(pat_verified(&pv.verify(Some(&pat::mint()), pat::NOW)));
        let dir = wba::directory(wba::KID, &wba::signer(1));
        let target = wba::request("example.com", "GET", "/data");
        let forged = wba::sign_wba(&target, wba::KID, &wba::signer(2), wba::NOW, "n1");
        assert_eq!(
            wba::check(target, &forged, &dir, &mut wba::nonces()),
            web_bot_auth::Verdict::Forged {
                keyid: wba::KID.into()
            }
        );
    }

    #[test]
    fn wargame_r236_webauthn_does_not_bypass_proxy_trust_config() {
        assert_eq!(Allowlist::parse("0.0.0.0/0"), Err(Refusal::TooBroad));
        let key = wan::key();
        let mut r = wan::registry(wan::CRED, &key);
        assert_eq!(
            r.verify(Some(&wan::sign_assertion(&key, wan::CRED, 7))),
            webauthn_verified()
        );
        assert_eq!(Allowlist::parse("0.0.0.0/0"), Err(Refusal::TooBroad));
    }

    /// R240 axis-close lock: PAT double spend, PROXY world trust, zero
    /// WebAuthn counter and authority-only Web Bot Auth coverage.
    #[test]
    fn wargame_r240_axis_close_regression_lock() {
        let mut pv = pat::verifier(pat::ISSUER_A_SPKI);
        let token = pat::mint();
        assert!(pat_verified(&pv.verify(Some(&token), pat::NOW)));
        assert!(matches!(
            pv.verify(Some(&token), pat::NOW),
            privacy_pass::Verdict::Invalid {
                reason: privacy_pass::Reason::DoubleSpend,
                ..
            }
        ));
        assert_eq!(Allowlist::parse("0.0.0.0/0"), Err(Refusal::TooBroad));
        let key = wan::key();
        let mut r = wan::registry(wan::CRED, &key);
        assert!(matches!(
            r.verify(Some(&wan::sign_assertion(&key, wan::CRED, 0))),
            webauthn::Verdict::Invalid {
                reason: webauthn::Reason::Replay,
                ..
            }
        ));
        let signer = wba::signer(1);
        let dir = wba::directory(wba::KID, &signer);
        let target = wba::request("example.com", "GET", "/admin");
        let inner = format!(
            r#"("@authority");created={};keyid="{}";alg="ed25519";tag="web-bot-auth""#,
            wba::NOW,
            wba::KID
        );
        let headers = wba::sign(&inner, &["@authority"], &target, &signer);
        assert_eq!(
            wba::check(target, &headers, &dir, &mut wba::nonces()),
            web_bot_auth::Verdict::Neutral {
                reason: web_bot_auth::Reason::Coverage,
                keyid: None
            }
        );
    }
}
