//! Wargame rounds from legacy internal/gate wargame_escalation_r81, r121 and
//! r161 (baseline 981d25c) that exercise only this kernel. Go used 29-byte
//! keys; Rust refuses keys under `MIN_KEY`, so each seed is padded (the pinned
//! deliberate key-length difference; the asserted outcome is unchanged).
use super::*;

const T: u64 = 1_700_000_000;
const LATER: u64 = 1_800_000_000;
const HOUR: u64 = 3600;

/// Go TestWargameR086_VerdictNotValidAsStepUp.
#[test]
fn wargame_r086_verdict_not_valid_as_step_up() {
    let k = key(b"wargame-r86-key-material!!!!!");
    let vt = k.issue_verdict("sid", "bind", "e1", T + HOUR).unwrap();
    assert!(
        k.verify_step_up(&vt, "bind", "sid", T, &epochs(&["e1"]))
            .is_err()
    );
}

/// Go TestWargameR087_StepUpNotValidAsVerdict.
#[test]
fn wargame_r087_step_up_not_valid_as_verdict() {
    let k = key(b"wargame-r87-key-material!!!!!");
    let su = k.issue_step_up("sid", "bind", "e1", T + HOUR).unwrap();
    assert!(
        k.verify_verdict(&su, "bind", "sid", T, &epochs(&["e1"]))
            .is_err()
    );
}

/// Go TestWargameR088_ReceiptNotValidAsStepUpCookie.
#[test]
fn wargame_r088_receipt_not_valid_as_step_up_cookie() {
    let k = key(b"wargame-r88-key-material!!!!!");
    let rc = k.issue_receipt("sid", T + HOUR).unwrap();
    assert!(
        k.verify_step_up(&rc, "bind", "sid", T, &epochs(&["e1"]))
            .is_err()
    );
}

/// Go TestWargameR089_StepUpCookieNotValidAsReceipt.
#[test]
fn wargame_r089_step_up_cookie_not_valid_as_receipt() {
    let k = key(b"wargame-r89-key-material!!!!!");
    let su = k.issue_step_up("sid", "bind", "e1", T + HOUR).unwrap();
    assert!(k.verify_receipt(&su, "sid", T).is_err());
}

/// Go TestWargameR090_ReceiptSIDMismatchTyped.
#[test]
fn wargame_r090_receipt_sid_mismatch_typed() {
    let k = key(b"wargame-r90-key-material!!!!!");
    let rc = k.issue_receipt("sid-A", T + HOUR).unwrap();
    assert_eq!(
        k.verify_receipt(&rc, "sid-B", T),
        Err(Reason::BindingMismatch)
    );
}

/// Go TestWargameR091_ReceiptExpiredTyped.
#[test]
fn wargame_r091_receipt_expired_typed() {
    let k = key(b"wargame-r91-key-material!!!!!");
    let rc = k.issue_receipt("sid", T + 60).unwrap();
    assert_eq!(k.verify_receipt(&rc, "sid", T + 600), Err(Reason::Expired));
}

/// Go TestWargameR092_ReceiptSkewLeewayAllowsSlightDrift.
#[test]
fn wargame_r092_receipt_skew_leeway_allows_slight_drift() {
    let k = key(b"wargame-r92-key-material!!!!!");
    let rc = k.issue_receipt("sid", T).unwrap();
    assert_eq!(k.verify_receipt(&rc, "sid", T + 60), Ok(()));
}

/// Go TestWargameR093_EmptyBindKeySkipsCollapse and
/// TestWargameR151_VerdictStoreGCEmpty: without a User-Agent there is no
/// shared bind bucket.
#[test]
fn wargame_r093_r151_headerless_client_has_no_bind_key() {
    assert_eq!(bind_key("", "", ""), None);
    assert_eq!(bind_key("", "en-US", "\"Chromium\""), None);
}

/// Go TestWargameR094_TokenBindIncludesSubnet.
#[test]
fn wargame_r094_token_bind_includes_subnet() {
    let ua = bind_key("Chrome/126", "", "");
    let a = client_subnet("203.0.113.1".parse().unwrap());
    let b = client_subnet("198.51.100.1".parse().unwrap());
    assert_ne!(token_bind(ua.as_deref(), &a), token_bind(ua.as_deref(), &b));
}

/// Go TestWargameR095_TokenBindIncludesUA.
#[test]
fn wargame_r095_token_bind_includes_ua() {
    let net = client_subnet("203.0.113.9".parse().unwrap());
    let chrome = bind_key("Chrome/126", "", "");
    let headless = bind_key("HeadlessChrome/126", "", "");
    assert_ne!(
        token_bind(chrome.as_deref(), &net),
        token_bind(headless.as_deref(), &net)
    );
}

/// Go TestWargameR123_WebBotAuthNoneDistinct (bind residual): deriving the
/// bind key and token binding for an ordinary client is total and stable.
#[test]
fn wargame_r123_bind_derivation_is_total_and_stable() {
    let net = client_subnet("203.0.113.123".parse().unwrap());
    let bk = bind_key("Chrome/126", "", "");
    assert!(bk.is_some());
    assert_eq!(
        token_bind(bk.as_deref(), &net),
        token_bind(bind_key("Chrome/126", "", "").as_deref(), &net)
    );
}

/// Go TestWargameR153_TokenMalformedNoDot.
#[test]
fn wargame_r153_token_malformed_no_dot() {
    let k = key(b"wargame-r153-key-material!!!!");
    assert_eq!(
        k.verify_verdict("nodotpayload", "bind", "sid", T, &epochs(&["e1"])),
        Err(Reason::Malformed)
    );
}

/// Go TestWargameR154_TokenMalformedBadB64.
#[test]
fn wargame_r154_token_malformed_bad_b64() {
    let k = key(b"wargame-r154-key-material!!!!");
    assert_eq!(
        k.verify_verdict("!!!.!!!", "bind", "sid", T, &epochs(&["e1"])),
        Err(Reason::Malformed)
    );
}

/// Go TestWargameR155_StepUpMalformed.
#[test]
fn wargame_r155_step_up_malformed() {
    let k = key(b"wargame-r155-key-material!!!!");
    assert_eq!(
        k.verify_step_up("x", "bind", "sid", T, &epochs(&["e1"])),
        Err(Reason::Malformed)
    );
}

/// Go TestWargameR156_ReceiptMalformed (Go `receiptBad` is Rust `Malformed`).
#[test]
fn wargame_r156_receipt_malformed() {
    let k = key(b"wargame-r156-key-material!!!!");
    assert_eq!(k.verify_receipt("x", "sid", T), Err(Reason::Malformed));
}

/// Go TestWargameR157_EpochRejectsUnknownEpoch.
#[test]
fn wargame_r157_epoch_rejects_unknown_epoch() {
    let k = key(b"wargame-r157-key-material!!!!");
    let now = T + 100;
    let tok = k.issue_verdict("sid", "bind", "e-old", now + HOUR).unwrap();
    assert_eq!(
        k.verify_verdict(&tok, "bind", "sid", now, &epochs(&["e-new"])),
        Err(Reason::Expired)
    );
}

/// Go TestWargameR158_SidMismatchOnVerdict.
#[test]
fn wargame_r158_sid_mismatch_on_verdict() {
    let k = key(b"wargame-r158-key-material!!!!");
    let now = T + 100;
    let tok = k.issue_verdict("sid-A", "bind", "e1", now + HOUR).unwrap();
    assert_eq!(
        k.verify_verdict(&tok, "bind", "sid-B", now, &epochs(&["e1"])),
        Err(Reason::BindingMismatch)
    );
}

/// Go TestWargameR159_BindMismatchOnVerdict.
#[test]
fn wargame_r159_bind_mismatch_on_verdict() {
    let k = key(b"wargame-r159-key-material!!!!");
    let now = T + 100;
    let tok = k.issue_verdict("sid", "bind-A", "e1", now + HOUR).unwrap();
    assert_eq!(
        k.verify_verdict(&tok, "bind-B", "sid", now, &epochs(&["e1"])),
        Err(Reason::BindingMismatch)
    );
}

/// Go TestWargameR160_ExpiredVerdict.
#[test]
fn wargame_r160_expired_verdict() {
    let k = key(b"wargame-r160-key-material!!!!");
    let now = T + 100;
    let tok = k.issue_verdict("sid", "bind", "e1", now - 60).unwrap();
    assert_eq!(
        k.verify_verdict(&tok, "bind", "sid", now, &epochs(&["e1"])),
        Err(Reason::Expired)
    );
}

/// Go TestWargameR174_StepUpExpired.
#[test]
fn wargame_r174_step_up_expired() {
    let k = key(b"wargame-r174-key-material!!!!");
    let tok = k.issue_step_up("sid", "bind", "e1", LATER - 1).unwrap();
    assert_eq!(
        k.verify_step_up(&tok, "bind", "sid", LATER, &epochs(&["e1"])),
        Err(Reason::Expired)
    );
}

/// Go TestWargameR175_StepUpBindMismatch.
#[test]
fn wargame_r175_step_up_bind_mismatch() {
    let k = key(b"wargame-r175-key-material!!!!");
    let tok = k.issue_step_up("sid", "bindA", "e1", LATER + HOUR).unwrap();
    assert_eq!(
        k.verify_step_up(&tok, "bindB", "sid", LATER, &epochs(&["e1"])),
        Err(Reason::BindingMismatch)
    );
}

/// Go TestWargameR176_StepUpSidMismatch.
#[test]
fn wargame_r176_step_up_sid_mismatch() {
    let k = key(b"wargame-r176-key-material!!!!");
    let tok = k.issue_step_up("sidA", "bind", "e1", LATER + HOUR).unwrap();
    assert_eq!(
        k.verify_step_up(&tok, "bind", "sidB", LATER, &epochs(&["e1"])),
        Err(Reason::BindingMismatch)
    );
}

/// Go TestWargameR177_ReceiptBadSig: a tampered MAC character never verifies.
#[test]
fn wargame_r177_receipt_bad_sig() {
    let k = key(b"wargame-r177-key-material!!!!");
    let rc = k.issue_receipt("sid", LATER + HOUR).unwrap();
    let last = if rc.ends_with('A') { "B" } else { "A" };
    let bad = format!("{}{last}", &rc[..rc.len() - 1]);
    assert_ne!(bad, rc);
    assert!(k.verify_receipt(&bad, "sid", LATER).is_err());
}

/// Go TestWargameR194_TokenRoundTripEpochCurrent.
#[test]
fn wargame_r194_token_round_trip_epoch_current() {
    let k = key(b"wargame-r194-key-material!!!!");
    let now = LATER + 500;
    let tok = k.issue_verdict("sid", "bind", "e1", now + HOUR).unwrap();
    assert_eq!(
        k.verify_verdict(&tok, "bind", "sid", now, &epochs(&["e1", "e0"])),
        Ok(())
    );
}

/// Go TestWargameR195_AcceptLanguageAffectsBindKey.
#[test]
fn wargame_r195_accept_language_affects_bind_key() {
    assert_ne!(
        bind_key("Chrome/126", "en-US", ""),
        bind_key("Chrome/126", "ko-KR", "")
    );
}

/// Go TestWargameR196_SecCHUAAffectsBindKey.
#[test]
fn wargame_r196_sec_ch_ua_affects_bind_key() {
    assert_ne!(
        bind_key("Chrome/126", "", "\"Chromium\";v=\"126\""),
        bind_key("Chrome/126", "", "\"Not.A/Brand\";v=\"99\"")
    );
}
