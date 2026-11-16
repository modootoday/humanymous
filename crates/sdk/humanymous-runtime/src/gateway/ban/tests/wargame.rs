//! Ban-store and ban-key wargame rounds from legacy internal/gate
//! wargame_escalation_r81, r121 and r161 (baseline 981d25c) that exercise
//! only this kernel. Go `validBanKey(k)` is `Key::parse(k).is_ok()`, Go
//! `isBroadKey` is `Key::is_broad` and Go `BanStore.Add` with a positive
//! duration is `Store::add_temporary`.
use super::*;

fn rejected(text: &str) {
    assert_eq!(Key::parse(text), Err(Refusal::Key), "{text:?}");
}

/// Go TestWargameR104_TempBanNotPermanent.
#[test]
fn wargame_r104_temp_ban_not_permanent() {
    let mut s = store();
    let e = s
        .add_temporary(
            &key("ip:203.0.113.104"),
            "tmp",
            Some("inc"),
            Duration::from_secs(1),
            T0,
        )
        .unwrap();
    assert!(!e.permanent());
    assert_eq!(e.until, Some(T0 + Duration::from_secs(1)));
}

/// Go TestWargameR105_PermanentBanHasZeroUntil. Deliberate difference: Go let
/// a single operator create a permanent ban with duration 0; Rust refuses the
/// zero term on the single-operator path, and a permanent ban exists only
/// through `add_controlled` with a consumed dual-control permit (AGENTS.md
/// hard constraint 2), pinned in the `dual_control` tests.
#[test]
fn wargame_r105_zero_duration_is_not_a_single_operator_permanent_ban() {
    let mut s = store();
    let k = key("ip:203.0.113.105");
    assert_eq!(
        s.add_temporary(&k, "perm", Some("inc"), Duration::ZERO, T0),
        Err(Refusal::Duration)
    );
    assert!(s.check(&k, T0).is_none());
}

/// Go TestWargameR106_LiftRemovesBan.
#[test]
fn wargame_r106_lift_removes_ban() {
    let mut s = store();
    let k = key("ip:203.0.113.106");
    s.add_temporary(&k, "x", None, HOUR, T0).unwrap();
    assert_eq!(s.lift(&k), Ok(true));
    assert!(s.check(&k, T0).is_none());
}

/// Go TestWargameR107_AutoBanEscalatesStrike and
/// TestWargameR193_EscalationMonotonicNonIncreasingDurationOrPermanent.
/// Terms 1h, 6h and 24h grow strictly as in Go. Deliberate difference: Go's
/// fourth strike returned 0 (permanent); Rust keeps 24h and reports
/// `dual_control_required` (see `auto_ban_escalates_and_permanent_tier_needs_dual_control`).
#[test]
fn wargame_r107_r193_escalation_terms() {
    assert_eq!(escalation(1), HOUR);
    assert_eq!(escalation(2), 6 * HOUR);
    assert_eq!(escalation(3), 24 * HOUR);
    assert!(escalation(1) < escalation(2) && escalation(2) < escalation(3));
    assert_eq!(escalation(4), 24 * HOUR);
}

/// Go TestWargameR118_BanKeyIPv4LoopbackAllowed.
#[test]
fn wargame_r118_ban_key_ipv4_loopback_allowed() {
    assert!(Key::parse("ip:127.0.0.1").is_ok());
}

/// Go TestWargameR119_BanKeyRejectsTabChar.
#[test]
fn wargame_r119_ban_key_rejects_tab_char() {
    rejected("ip:1.2.3.4\t");
}

/// Go TestWargameR120_FPWithSpacesRejected.
#[test]
fn wargame_r120_fp_with_spaces_rejected() {
    rejected("fp:abcd efgh");
}

/// Go TestWargameR121_WebAuthnNoneDoesNotTrust (ban-key residual): a long
/// printable fingerprint that looks like a credential id is still a key.
#[test]
fn wargame_r121_credential_like_fingerprint_is_a_valid_key() {
    assert!(Key::parse("fp:webauthn-cred-id-long-enough").is_ok());
}

/// Go TestWargameR122_PATDoubleSpendDistinctFromNone (ban-key residual).
#[test]
fn wargame_r122_token_like_fingerprint_keys() {
    rejected("fp:ab");
    assert!(Key::parse("fp:privacy-pass-token-hash-value").is_ok());
}

/// Go TestWargameR124_ProxyProtoGarbageRejected: the world range parses and
/// is broad (dual-controlled).
#[test]
fn wargame_r124_world_cidr_is_valid_and_broad() {
    let world = key("cidr:0.0.0.0/0");
    assert!(world.is_broad());
}

/// Go TestWargameR125_ProxyProtoLocalCommand: PROXY header bytes are no key.
#[test]
fn wargame_r125_proxy_tlv_bytes_are_no_ban_key() {
    rejected("ip:\r\n");
}

/// Go TestWargameR126_TrustGateRequiresProxyNet: a public test-net IP parses.
#[test]
fn wargame_r126_public_test_net_ip_parses() {
    assert!(Key::parse("ip:203.0.113.126").is_ok());
}

/// Go TestWargameR148_SecurityHeadersAddOnlyContract (ban-key residual).
#[test]
fn wargame_r148_empty_cidr_rejected() {
    rejected("cidr:");
}

/// Go TestWargameR149_ReadinessIndependentOfBans.
#[test]
fn wargame_r149_fresh_store_lists_no_bans() {
    assert!(store().list(T0).is_empty());
}

/// Go TestWargameR150_GCDoesNotPanicEmpty: GC of an empty store and of an
/// expired ban both succeed and leave nothing active.
#[test]
fn wargame_r150_gc_empty_and_expired() {
    let mut s = store();
    s.gc(T0).unwrap();
    let k = key("ip:203.0.113.150");
    s.add_temporary(&k, "t", None, Duration::from_nanos(1), T0)
        .unwrap();
    s.gc(T0 + HOUR).unwrap();
    assert!(s.list(T0 + HOUR).is_empty());
}

/// Go TestWargameR183_CIDRHostBitsValid: host bits in a CIDR still parse.
#[test]
fn wargame_r183_cidr_host_bits_valid() {
    assert!(Key::parse("cidr:203.0.113.15/24").is_ok());
}

/// Go TestWargameR184_RejectUnknownScheme.
#[test]
fn wargame_r184_reject_unknown_scheme() {
    for text in ["user:alice", "email:a@b.c", "asn:1234"] {
        rejected(text);
    }
}

/// Go TestWargameR185_RejectBareIPWithoutScheme.
#[test]
fn wargame_r185_reject_bare_ip_without_scheme() {
    rejected("203.0.113.185");
}

/// Go TestWargameR191_DoubleLiftSafe.
#[test]
fn wargame_r191_double_lift_safe() {
    let mut s = store();
    let k = key("ip:203.0.113.191");
    s.add_temporary(&k, "x", None, HOUR, T0).unwrap();
    assert_eq!(s.lift(&k), Ok(true));
    assert_eq!(s.lift(&k), Ok(false));
}

/// Go TestWargameR192_ListBansIncludesManual.
#[test]
fn wargame_r192_list_bans_includes_manual() {
    let mut s = store();
    let k = key("ip:203.0.113.192");
    s.add_temporary(&k, "manual", Some("inc"), HOUR, T0)
        .unwrap();
    assert!(
        s.list(T0)
            .iter()
            .any(|e| e.key == k && e.source == Source::Manual)
    );
}
