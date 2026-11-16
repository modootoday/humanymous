//! Ports of legacy internal/gate token_test.go and epoch_test.go (baseline
//! 981d25c) plus step-up, receipt, incident and fail-closed boundaries.
use super::*;

const NOW: u64 = 1_000_000;

fn key(seed: &[u8]) -> TokenKey {
    let mut bytes = seed.to_vec();
    bytes.resize(MIN_KEY, b'.');
    TokenKey::new(bytes).unwrap()
}
fn epochs(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}
fn fingerprint(ua: &str, lang: &str, ch: &str) -> String {
    bind_key(ua, lang, ch).unwrap_or_default()
}

#[test]
fn verdict_token_round_trip() {
    let k = key(b"server-only-token-key");
    let bind = fingerprint("Chrome/126", "en-US", r#""Chromium";v="126""#);
    let token = k.issue_verdict("sid1", &bind, "e1", NOW + 60).unwrap();
    assert_eq!(
        k.verify_verdict(&token, &bind, "sid1", NOW, &epochs(&["e1", "e0"])),
        Ok(())
    );
}

#[test]
fn lifted_token_fails_fingerprint_binding() {
    let k = key(b"server-only-token-key");
    let human = fingerprint("Chrome/126", "en-US", r#""Chromium";v="126""#);
    let bot = fingerprint("HeadlessChrome/126", "", "");
    let token = k.issue_verdict("sid1", &human, "e1", NOW + 60).unwrap();
    assert_eq!(
        k.verify_verdict(&token, &bot, "sid1", NOW, &epochs(&["e1", "e0"])),
        Err(Reason::BindingMismatch)
    );
}

#[test]
fn token_is_bound_to_subnet_and_session() {
    let k = key(b"server-only-token-key");
    let fp = bind_key("Chrome/126", "en-US", r#""Chromium";v="126""#);
    let at = |ip: &str| token_bind(fp.as_deref(), &client_subnet(ip.parse().unwrap()));
    let token = k
        .issue_verdict("sidH", &at("203.0.113.10"), "e1", NOW + 60)
        .unwrap();
    let accepted = epochs(&["e1"]);
    assert_eq!(
        k.verify_verdict(&token, &at("203.0.113.99"), "sidH", NOW, &accepted),
        Ok(())
    );
    assert_eq!(
        k.verify_verdict(&token, &at("198.51.100.7"), "sidH", NOW, &accepted),
        Err(Reason::BindingMismatch)
    );
    assert_eq!(
        k.verify_verdict(&token, &at("203.0.113.99"), "sidBOT", NOW, &accepted),
        Err(Reason::BindingMismatch)
    );
}

#[test]
fn forged_token_fails_signature() {
    let bind = fingerprint("Chrome/126", "en-US", "");
    let forged = key(b"attacker-key")
        .issue_verdict("sid1", &bind, "e1", NOW + 60)
        .unwrap();
    assert_eq!(
        key(b"server-only-token-key").verify_verdict(&forged, &bind, "sid1", NOW, &epochs(&["e1"])),
        Err(Reason::BadSignature)
    );
}

#[test]
fn expired_token_is_rejected() {
    let k = key(b"k");
    let bind = fingerprint("Chrome/126", "en-US", "");
    let token = k.issue_verdict("sid1", &bind, "e1", NOW - 1).unwrap();
    assert_eq!(
        k.verify_verdict(&token, &bind, "sid1", NOW, &epochs(&["e1"])),
        Err(Reason::Expired)
    );
}

#[test]
fn nonce_is_single_use_across_bindings() {
    let mut nonces = Nonces::new(Duration::from_secs(60), 16);
    let now = Duration::from_secs(NOW);
    assert!(nonces.use_once("n1", now));
    assert!(!nonces.use_once("n1", now), "same nonce replay");
    assert!(!nonces.use_once("n1", now), "replay under any binding");
    assert!(nonces.use_once("n2", now));
    assert!(
        !nonces.use_once("", now),
        "empty nonce refuses (Go accepted)"
    );
}

#[test]
fn nonce_cache_expires_and_saturates_closed() {
    let mut nonces = Nonces::new(Duration::from_secs(60), 2);
    let now = Duration::from_secs(NOW);
    assert!(nonces.use_once("a", now));
    assert!(nonces.use_once("b", now));
    assert!(!nonces.use_once("c", now), "saturated cache fails safe");
    let later = now + Duration::from_secs(61);
    assert!(nonces.use_once("c", later), "expired entries free capacity");
    assert!(nonces.use_once("a", later + Duration::from_secs(1)));
}

#[test]
fn epoch_rotation_grants_one_grace_bucket() {
    let k = key(b"server-token-key");
    let bind = token_bind(
        bind_key("Chrome/126", "", "").as_deref(),
        &client_subnet("203.0.113.4".parse().unwrap()),
    );
    let mut e = Epochs::default();
    let token = k
        .issue_verdict("sid1", &bind, &e.current(NOW), NOW + 3600)
        .unwrap();
    assert_eq!(
        k.verify_verdict(&token, &bind, "sid1", NOW, &e.accepted(NOW)),
        Ok(())
    );
    e.advance();
    assert_eq!(
        k.verify_verdict(&token, &bind, "sid1", NOW, &e.accepted(NOW)),
        Ok(())
    );
    e.advance();
    assert_eq!(
        k.verify_verdict(&token, &bind, "sid1", NOW, &e.accepted(NOW)),
        Err(Reason::Expired)
    );
    let fresh = k
        .issue_verdict("sid1", &bind, &e.current(NOW), NOW + 3600)
        .unwrap();
    assert_eq!(
        k.verify_verdict(&fresh, &bind, "sid1", NOW, &e.accepted(NOW)),
        Ok(())
    );
}

#[test]
fn epochs_follow_the_wall_clock_bucket() {
    let e = Epochs::default();
    assert_eq!(e.current(NOW), format!("e{}", NOW / EPOCH_WINDOW));
    assert_eq!(
        e.accepted(NOW + EPOCH_WINDOW),
        vec![
            format!("e{}", NOW / EPOCH_WINDOW + 1),
            format!("e{}", NOW / EPOCH_WINDOW)
        ]
    );
}

#[test]
fn step_up_and_verdict_tokens_are_domain_separated() {
    let k = key(b"server-only-token-key");
    let bind = fingerprint("Chrome/126", "en-US", "");
    let accepted = epochs(&["e1"]);
    let verdict = k.issue_verdict("sid1", &bind, "e1", NOW + 60).unwrap();
    let step_up = k.issue_step_up("sid1", &bind, "e1", NOW + 60).unwrap();
    assert_eq!(
        k.verify_step_up(&step_up, &bind, "sid1", NOW, &accepted),
        Ok(())
    );
    assert_eq!(
        k.verify_step_up(&verdict, &bind, "sid1", NOW, &accepted),
        Err(Reason::Malformed)
    );
    assert_eq!(
        k.verify_verdict(&step_up, &bind, "sid1", NOW, &accepted),
        Err(Reason::Malformed)
    );
    assert_eq!(
        k.verify_step_up(&step_up, &bind, "other", NOW, &accepted),
        Err(Reason::BindingMismatch)
    );
}

#[test]
fn receipt_checks_session_expiry_with_skew_and_domain() {
    let k = key(b"server-only-token-key");
    let receipt = k.issue_receipt("sid1", NOW).unwrap();
    assert_eq!(
        k.verify_receipt(&receipt, "sid1", NOW + RECEIPT_SKEW),
        Ok(())
    );
    assert_eq!(
        k.verify_receipt(&receipt, "sid1", NOW + RECEIPT_SKEW + 1),
        Err(Reason::Expired)
    );
    assert_eq!(
        k.verify_receipt(&receipt, "sid2", NOW),
        Err(Reason::BindingMismatch)
    );
    assert_eq!(
        k.verify_receipt(&receipt, "", NOW),
        Err(Reason::BindingMismatch)
    );
    let verdict = k.issue_verdict("sid1", "b", "e1", NOW).unwrap();
    assert_eq!(
        k.verify_receipt(&verdict, "sid1", NOW),
        Err(Reason::Malformed)
    );
}

#[test]
fn malformed_and_ambiguous_tokens_fail_closed() {
    let k = key(b"server-only-token-key");
    let accepted = epochs(&["e1"]);
    for bad in ["", "no-dot", "!!.!!", "a.b"] {
        assert!(
            k.verify_verdict(bad, "b", "s", NOW, &accepted).is_err(),
            "{bad}"
        );
    }
    assert_eq!(k.issue_verdict("s|x", "b", "e1", NOW), Err(Refusal::Field));
    assert_eq!(k.issue_verdict("", "b", "e1", NOW), Err(Refusal::Field));
    let token = k.issue_verdict("sid1", "b", "e1", NOW + 60).unwrap();
    assert_eq!(
        k.verify_verdict(&token, "b", "", NOW, &accepted),
        Err(Reason::BindingMismatch),
        "empty expected session refuses (Go skipped the check)"
    );
    let padded = k.seal(&["sid1", "b", "0060", "e1"]).unwrap();
    assert_eq!(
        k.verify_verdict(&padded, "b", "sid1", 0, &accepted),
        Err(Reason::Malformed)
    );
    assert!(TokenKey::new(vec![0; MIN_KEY - 1]).is_err());
}

#[test]
fn bindings_and_subnets_match_legacy_shapes() {
    assert_eq!(bind_key("", "en", "x"), None);
    let a = bind_key("Chrome/126", "en-US", "").unwrap();
    assert_eq!(a.len(), 16);
    assert_ne!(Some(a), bind_key("Chrome/126", "en-US", "x"));
    assert_eq!(
        client_subnet("203.0.113.10".parse().unwrap()),
        "203.0.113.0/24"
    );
    assert_eq!(
        client_subnet("2001:db8:1:2::9".parse().unwrap()),
        "2001:db8:1::/48"
    );
    assert_eq!(
        client_subnet("::ffff:203.0.113.10".parse().unwrap()),
        "203.0.113.0/24"
    );
    assert_eq!(token_bind(None, "203.0.113.0/24").len(), 22);
}

#[test]
fn incident_handles_are_stable_opaque_and_keyed() {
    let k = key(b"server-only-token-key");
    let h = k.incident_handle(7);
    assert!(h.starts_with("INC-") && h.len() == 14);
    assert!(
        h[4..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
    );
    assert_eq!(h, k.incident_handle(7));
    assert_ne!(h, k.incident_handle(8));
    assert_ne!(h, key(b"other").incident_handle(7));
}

#[test]
fn step_up_rejects_wrong_bind_expiry_and_garbage_receipts() {
    let k = key(b"server-only-token-key");
    let accepted = epochs(&["e1"]);
    let token = k.issue_step_up("sidX", "bindX", "e1", NOW + 60).unwrap();
    assert_eq!(
        k.verify_step_up(&token, "bindX", "sidX", NOW, &accepted),
        Ok(())
    );
    assert_eq!(
        k.verify_step_up(&token, "bindY", "sidX", NOW, &accepted),
        Err(Reason::BindingMismatch)
    );
    assert_eq!(
        k.verify_step_up(&token, "bindX", "sidX", NOW + 120, &accepted),
        Err(Reason::Expired)
    );
    let receipt = k.issue_receipt("sidX", NOW + 60).unwrap();
    assert_eq!(k.verify_receipt(&receipt, "sidX", NOW + 90), Ok(()));
    assert_eq!(
        k.verify_receipt(&receipt, "sidX", NOW + 600),
        Err(Reason::Expired)
    );
    assert!(k.verify_receipt("garbage", "sidX", NOW).is_err());
}

#[test]
fn lifted_step_up_proof_fails_binding() {
    let k = key(b"server-only-token-key");
    let victim = token_bind(
        bind_key("Chrome/126-victim", "", "").as_deref(),
        &client_subnet("203.0.113.5".parse().unwrap()),
    );
    let bot = token_bind(
        bind_key("HeadlessChrome/126", "", "").as_deref(),
        &client_subnet("45.66.77.88".parse().unwrap()),
    );
    let accepted = epochs(&["e1"]);
    let proof = k
        .issue_step_up("victim", &victim, "e1", NOW + 1800)
        .unwrap();
    assert_eq!(
        k.verify_step_up(&proof, &bot, "victim", NOW, &accepted),
        Err(Reason::BindingMismatch)
    );
}

mod wargame;
