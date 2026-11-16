//! Ports of legacy internal/gate/webbotauth_test.go (baseline 981d25c) plus
//! the fail-closed boundaries this kernel adds.
use super::*;
use crate::gateway::token::MAX_NONCES;
use ed25519_dalek::{Signer, SigningKey};

pub(in crate::gateway::credential) const KID: &str = "test-agent-key-1";
pub(in crate::gateway::credential) const NOW: u64 = 1_700_000_000;
const COMPONENTS: [&str; 3] = ["@authority", "@method", "@path"];

pub(in crate::gateway::credential) fn now() -> Duration {
    Duration::from_secs(NOW)
}
pub(in crate::gateway::credential) fn nonces() -> Nonces {
    Nonces::new(Duration::from_secs(3600), MAX_NONCES)
}
pub(in crate::gateway::credential) fn signer(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}
pub(in crate::gateway::credential) fn directory(kid: &str, key: &SigningKey) -> KeyDirectory {
    let public = URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes());
    KeyDirectory::parse(&format!("{kid} {public}")).unwrap()
}
pub(in crate::gateway::credential) fn request<'a>(
    authority: &'a str,
    method: &'a str,
    path: &'a str,
) -> Request<'a> {
    Request {
        authority,
        method,
        path,
        signature_input: None,
        signature: None,
    }
}
/// Sign `inner` over the request line with the verifier's own base builder.
pub(in crate::gateway::credential) fn sign(
    inner: &str,
    components: &[&str],
    target: &Request<'_>,
    key: &SigningKey,
) -> (String, String) {
    let base = signature_base(target, components, inner);
    let signature = STANDARD.encode(key.sign(base.as_bytes()).to_bytes());
    (format!("sig1={inner}"), format!("sig1=:{signature}:"))
}
/// Go `signWebBotAuth`.
pub(in crate::gateway::credential) fn sign_wba(
    target: &Request<'_>,
    kid: &str,
    key: &SigningKey,
    created: u64,
    nonce: &str,
) -> (String, String) {
    let mut inner = format!(
        r#"("@authority" "@method" "@path");created={created};keyid="{kid}";alg="ed25519";tag="web-bot-auth""#
    );
    if !nonce.is_empty() {
        inner.push_str(&format!(r#";nonce="{nonce}""#));
    }
    sign(&inner, &COMPONENTS, target, key)
}
pub(in crate::gateway::credential) fn check(
    target: Request<'_>,
    headers: &(String, String),
    directory: &KeyDirectory,
    cache: &mut Nonces,
) -> Verdict {
    let presented = Request {
        signature_input: Some(&headers.0),
        signature: Some(&headers.1),
        ..target
    };
    verify(&presented, Some(directory), cache, now())
}
fn trusted() -> Verdict {
    Verdict::Trusted { keyid: KID.into() }
}
fn forged() -> Verdict {
    Verdict::Forged { keyid: KID.into() }
}

#[test]
fn verify_agent_trusted() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, KID, &key, NOW, "n1");
    assert_eq!(check(target, &headers, &dir, &mut nonces()), trusted());
}

#[test]
fn verify_agent_forged() {
    let dir = directory(KID, &signer(1));
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, KID, &signer(2), NOW, "n1");
    assert_eq!(check(target, &headers, &dir, &mut nonces()), forged());
}

#[test]
fn verify_agent_tampered_authority() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let headers = sign_wba(
        &request("example.com", "GET", "/data"),
        KID,
        &key,
        NOW,
        "n1",
    );
    let victim = request("victim.com", "GET", "/data");
    assert_eq!(check(victim, &headers, &dir, &mut nonces()), forged());
}

#[test]
fn verify_agent_replay_different_request_line() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let headers = sign_wba(
        &request("example.com", "GET", "/public"),
        KID,
        &key,
        NOW,
        "n1",
    );
    let other_path = request("example.com", "GET", "/admin");
    assert_eq!(check(other_path, &headers, &dir, &mut nonces()), forged());
    let other_method = request("example.com", "POST", "/public");
    assert_eq!(check(other_method, &headers, &dir, &mut nonces()), forged());
}

#[test]
fn verify_agent_nonce_single_use() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let mut cache = nonces();
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, KID, &key, NOW, "nonce-xyz");
    assert_eq!(check(target, &headers, &dir, &mut cache), trusted());
    assert_eq!(
        check(target, &headers, &dir, &mut cache),
        Verdict::Neutral {
            reason: Reason::Replay,
            keyid: Some(KID.into())
        }
    );
}

#[test]
fn verify_agent_authority_only_rejected() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let inner =
        format!(r#"("@authority");created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#);
    let headers = sign(&inner, &["@authority"], &target, &key);
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::Coverage)
    );
}

#[test]
fn verify_agent_unknown_key() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, "some-other-kid", &key, NOW, "n1");
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::UnknownKey)
    );
}

#[test]
fn verify_agent_expired() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let inner = format!(
        r#"("@authority" "@method" "@path");created=1000000000;expires=1000000100;keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
    );
    let headers = sign(&inner, &COMPONENTS, &target, &key);
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::Stale)
    );
}

#[test]
fn verify_agent_none() {
    let dir = directory(KID, &signer(1));
    let target = request("example.com", "GET", "/");
    assert_eq!(
        verify(&target, Some(&dir), &mut nonces(), now()),
        Verdict::Absent
    );
}

#[test]
fn verify_agent_lifetime_bound() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let old = format!(
        r#"("@authority" "@method" "@path");created=1000000000;keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
    );
    let headers = sign(&old, &COMPONENTS, &target, &key);
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::Stale)
    );
    let none = format!(
        r#"("@authority" "@method" "@path");keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
    );
    let headers = sign(&none, &COMPONENTS, &target, &key);
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::Unbounded)
    );
}

#[test]
fn lifetime_window_edges_follow_go() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let with = |params: &str| {
        let inner = format!(
            r#"("@authority" "@method" "@path");{params};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
        );
        check(
            target,
            &sign(&inner, &COMPONENTS, &target, &key),
            &dir,
            &mut nonces(),
        )
    };
    assert_eq!(with(&format!("created={}", NOW + SKEW)), trusted());
    assert_eq!(
        with(&format!("created={}", NOW + SKEW + 1)),
        neutral(Reason::Stale)
    );
    assert_eq!(with(&format!("created={}", NOW - MAX_LIFETIME)), trusted());
    assert_eq!(with(&format!("expires={}", NOW + MAX_LIFETIME)), trusted());
    assert_eq!(
        with(&format!("expires={}", NOW + MAX_LIFETIME + 1)),
        neutral(Reason::Unbounded)
    );
    assert_eq!(with(&format!("expires={}", NOW - SKEW)), trusted());
    assert_eq!(
        with(&format!("expires={}", NOW - SKEW - 1)),
        neutral(Reason::Stale)
    );
    assert_eq!(
        with(&format!("created={};expires={}", NOW - 3000, NOW + 1000)),
        neutral(Reason::Unbounded)
    );
    assert_eq!(
        with("created=0;expires=1700000100"),
        neutral(Reason::Unbounded)
    );
}

#[test]
fn absent_directory_refuses_instead_of_bypassing() {
    let key = signer(1);
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, KID, &key, NOW, "n1");
    let presented = Request {
        signature_input: Some(&headers.0),
        signature: Some(&headers.1),
        ..target
    };
    assert_eq!(
        verify(&presented, None, &mut nonces(), now()),
        neutral(Reason::NotConfigured)
    );
    assert_eq!(verify(&target, None, &mut nonces(), now()), Verdict::Absent);
    let half = Request {
        signature_input: Some(&headers.0),
        ..target
    };
    assert_eq!(
        verify(&half, None, &mut nonces(), now()),
        neutral(Reason::Malformed)
    );
}

#[test]
fn profile_and_shape_are_enforced() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let with = |inner: &str| {
        check(
            target,
            &sign(inner, &COMPONENTS, &target, &key),
            &dir,
            &mut nonces(),
        )
    };
    let line = r#"("@authority" "@method" "@path")"#;
    assert_eq!(
        with(&format!(
            r#"{line};created={NOW};keyid="{KID}";alg="rsa-pss-sha512";tag="web-bot-auth""#
        )),
        neutral(Reason::Profile)
    );
    assert_eq!(
        with(&format!(
            r#"{line};created={NOW};keyid="{KID}";alg="ed25519";tag="other""#
        )),
        neutral(Reason::Profile)
    );
    for malformed in [
        format!(r#"{line};created=+{NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#),
        format!(
            r#"{line};created={NOW};created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
        ),
        format!(r#"{line};created={NOW};keyid={KID};alg="ed25519";tag="web-bot-auth""#),
        format!(
            r#"("@authority" "@authority" "@method" "@path");created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
        ),
        format!(
            r#"{line};created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth", sig2=("@path")"#
        ),
        format!(r#"{line}x;created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#),
    ] {
        assert_eq!(with(&malformed), neutral(Reason::Malformed), "{malformed}");
    }
    let headers = sign_wba(&target, KID, &key, NOW, "");
    let renamed = (headers.0.replacen("sig1", "sig2", 1), headers.1.clone());
    assert_eq!(
        check(target, &renamed, &dir, &mut nonces()),
        neutral(Reason::Malformed)
    );
    let control = request("example.com", "GET", "/data\nx");
    assert_eq!(
        check(control, &headers, &dir, &mut nonces()),
        neutral(Reason::Malformed)
    );
    let huge = (
        format!("{}{}", headers.0, ";x=1".repeat(MAX_SIGNATURE_INPUT)),
        headers.1.clone(),
    );
    assert_eq!(
        check(target, &huge, &dir, &mut nonces()),
        neutral(Reason::Malformed)
    );
}

#[test]
fn covered_set_must_be_exact_and_any_order_verifies() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("Example.COM", "get", "/data");
    let reordered = ["@path", "@method", "@authority"];
    let inner = format!(
        r#"("@path" "@method" "@authority");created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
    );
    let headers = sign(&inner, &reordered, &target, &key);
    assert_eq!(check(target, &headers, &dir, &mut nonces()), trusted());
    let extra = format!(
        r#"("@authority" "@method" "@path" "accept");created={NOW};keyid="{KID}";alg="ed25519";tag="web-bot-auth""#
    );
    let headers = sign(&extra, &COMPONENTS, &target, &key);
    assert_eq!(
        check(target, &headers, &dir, &mut nonces()),
        neutral(Reason::Coverage)
    );
}

#[test]
fn signature_without_nonce_is_trusted_and_bad_length_is_forged() {
    let key = signer(1);
    let dir = directory(KID, &key);
    let target = request("example.com", "GET", "/data");
    let headers = sign_wba(&target, KID, &key, NOW, "");
    let mut cache = nonces();
    assert_eq!(check(target, &headers, &dir, &mut cache), trusted());
    assert_eq!(check(target, &headers, &dir, &mut cache), trusted());
    let short = (
        headers.0.clone(),
        format!("sig1=:{}:", STANDARD.encode([0u8; 63])),
    );
    assert_eq!(check(target, &short, &dir, &mut cache), forged());
}

#[test]
fn directory_configuration_is_strict() {
    let key = URL_SAFE_NO_PAD.encode(signer(1).verifying_key().to_bytes());
    assert!(KeyDirectory::parse(&format!("# agents\n\n{KID} {key}\n")).is_ok());
    assert_eq!(KeyDirectory::parse("").err(), Some(ConfigError::Empty));
    assert_eq!(
        KeyDirectory::parse("# only\n").err(),
        Some(ConfigError::Empty)
    );
    assert_eq!(KeyDirectory::parse(KID).err(), Some(ConfigError::Syntax));
    assert_eq!(
        KeyDirectory::parse(&format!("{KID} {key} extra")).err(),
        Some(ConfigError::Syntax)
    );
    assert_eq!(
        KeyDirectory::parse(&format!("{KID} AAAA")).err(),
        Some(ConfigError::Key)
    );
    assert_eq!(
        KeyDirectory::parse(&format!("{KID} {}", URL_SAFE_NO_PAD.encode([0u8; 32]))).err(),
        Some(ConfigError::Key)
    );
    assert_eq!(
        KeyDirectory::parse(&format!("{KID} {key}\n{KID} {key}")).err(),
        Some(ConfigError::Duplicate)
    );
}

/// Wargame R237 (legacy wargame_axis_cred_r201_test.go): the coverage helper
/// requires exactly the request line, with no unverifiable extra component.
#[test]
fn wargame_r237_covers_request_line_helper() {
    assert!(!covers_request_line(&["@authority"]));
    assert!(covers_request_line(&COMPONENTS));
    assert!(!covers_request_line(&[
        "@authority",
        "@method",
        "@path",
        "content-digest"
    ]));
}

/// Wargame R239 with Go's exact directory text. Go skipped `badline` and
/// loaded the good entry. Rust difference (fail closed, see module docs): a
/// malformed line refuses the whole directory as `Syntax`, so no partially
/// read allowlist is ever trusted; comments and blanks are still skipped.
#[test]
fn wargame_r239_bad_directory_line_refuses_whole_directory() {
    let key = URL_SAFE_NO_PAD.encode(signer(1).verifying_key().to_bytes());
    let good = format!("# comment\n\n{KID} {key}\n");
    assert!(KeyDirectory::parse(&good).is_ok_and(|d| d.lookup(KID).is_some()));
    let go_spec = format!("# comment\n\nbadline\n{KID} {key}\n");
    assert_eq!(
        KeyDirectory::parse(&go_spec).err(),
        Some(ConfigError::Syntax)
    );
}
