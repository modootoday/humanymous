//! Wire goldens measured by running the Go oracle (internal/gate token.go at
//! baseline 981d25c, Go 1.25.3) with the same inputs, so a Go Gate and this
//! kernel accept each other's tokens, receipts and bindings byte for byte.
use super::*;

const GO_KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
const EXP: u64 = 1_700_000_000;
const VERDICT: &str =
    "c2lkMXxiaW5kQXwxNzAwMDAwMDAwfGUxODg4ODg4.1zpfkNmUEgOpO9YN8ZjQugRVlJrdkLBjtct5JMchakA";
const STEP_UP: &str =
    "c3V8c2lkMXxiaW5kQXwxNzAwMDAwMDAwfGUxODg4ODg4.Vbfj8ORl5UmTGmo-Wm6o0pP1l8x-H9mrPHeX5cbgmcE";
const RECEIPT: &str = "c3VyfHNpZDF8MTcwMDAwMDAwMA.ayDXcyFKEfGOkoOUnZfTAwzJ_whtjyRXjqAiKGJwkuY";

#[test]
fn tokens_and_receipts_match_go_bytes() {
    let k = TokenKey::new(GO_KEY.to_vec()).unwrap();
    assert_eq!(
        k.issue_verdict("sid1", "bindA", "e1888888", EXP).unwrap(),
        VERDICT
    );
    assert_eq!(
        k.issue_step_up("sid1", "bindA", "e1888888", EXP).unwrap(),
        STEP_UP
    );
    assert_eq!(k.issue_receipt("sid1", EXP).unwrap(), RECEIPT);
    let accepted = vec!["e1888888".to_owned()];
    assert_eq!(
        k.verify_verdict(VERDICT, "bindA", "sid1", EXP, &accepted),
        Ok(())
    );
    assert_eq!(
        k.verify_step_up(STEP_UP, "bindA", "sid1", EXP, &accepted),
        Ok(())
    );
    assert_eq!(k.verify_receipt(RECEIPT, "sid1", EXP), Ok(()));
    // The domain separation holds on Go-minted bytes too.
    assert!(
        k.verify_step_up(VERDICT, "bindA", "sid1", EXP, &accepted)
            .is_err()
    );
}

#[test]
fn bindings_match_go_bytes() {
    let fingerprint = bind_key(
        "Mozilla/5.0 Chrome/126.0 Safari/537.36",
        "en-US,en;q=0.9",
        "\"Chromium\";v=\"126\"",
    );
    assert_eq!(fingerprint.as_deref(), Some("d_Tues8bRjsNLZHw"));
    let v4 = client_subnet("203.0.113.10".parse().unwrap());
    assert_eq!(v4, "203.0.113.0/24");
    assert_eq!(
        token_bind(fingerprint.as_deref(), &v4),
        "wiBbwDRLYsKZSZDElqZUzw"
    );
    let v6 = client_subnet("2001:db8:1:2::9".parse().unwrap());
    assert_eq!(
        token_bind(fingerprint.as_deref(), &v6),
        "6iny1kTxQ0uYx2ug3o4Zww"
    );
    // Go binds a User-Agent-less request with an empty fingerprint.
    assert_eq!(token_bind(None, &v4), "FQpeMcfByt6PE5cazvB14Q");
}

#[test]
fn wire_names_match_go() {
    assert_eq!(
        (VERDICT_COOKIE, STEP_UP_COOKIE, STEP_UP_RECEIPT_HEADER),
        ("hmn_vt", "hmn_su", "X-Hmn-Stepup-Receipt")
    );
}
