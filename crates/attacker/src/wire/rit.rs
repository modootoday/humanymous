//! RIT anti-tamper token construction matching the Go server contract.
//!
//! Defense-only: this reproduces the token the Go red-team clients
//! (`cmd/redteam`) computed for the project's own `/api/collect` so the Rust
//! port can exercise the same anti-tamper plane. It mints no capability of its
//! own; the canonical form mirrors `cmd/server/rit.go` exactly.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

/// RIT rotation window in seconds (Go `ritW`).
pub const RIT_WINDOW_S: u64 = 10;

/// Time bucket for a unix-seconds instant (Go `nowTB`).
pub fn now_tb(now_unix_s: u64) -> u64 {
    now_unix_s / RIT_WINDOW_S
}

/// Compute the live RIT token for a body, matching `cmd/server/rit.go`:
/// `base64url(HMAC-SHA256(seed, sid "\n" n "\n" tb "\n" hex(sha256(body))))`.
pub fn rit_token(seed: &[u8], sid: &str, n: u64, tb: u64, body: &[u8]) -> String {
    let body_hash = Sha256::digest(body);
    let canonical = format!("{sid}\n{n}\n{tb}\n{}", hex::encode(body_hash));
    let mut mac = Hmac::<Sha256>::new_from_slice(seed).expect("HMAC accepts any key length");
    mac.update(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &[u8] = b"owned-lab-rit-seed-bytes-000000000";
    const SID: &str = "session-abc";
    const BODY: &[u8] = b"{\"userAgent\":\"Chrome\",\"signals\":[]}";

    #[test]
    fn window_bucketing_matches_go_ritw() {
        assert_eq!(now_tb(0), 0);
        assert_eq!(now_tb(9), 0);
        assert_eq!(now_tb(10), 1);
        assert_eq!(now_tb(1_700_000_005), 170_000_000);
    }

    #[test]
    fn token_is_deterministic_url_safe_and_fixed_width() {
        let a = rit_token(SEED, SID, 1, 5, BODY);
        let b = rit_token(SEED, SID, 1, 5, BODY);
        assert_eq!(a, b);
        // HMAC-SHA256 is 32 bytes -> 43 base64url chars, no padding.
        assert_eq!(a.len(), 43);
        assert!(!a.contains('='));
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        );
    }

    #[test]
    fn golden_vector_pins_the_canonical_form() {
        // Regression anchor for the exact sid\nn\ntb\nhex(sha256(body)) layout.
        assert_eq!(
            rit_token(SEED, SID, 7, 170_000_000, BODY),
            "dw45k1NincVSyP7tGgaGZabnyoagsbAI56IMduIKZgw"
        );
    }

    #[test]
    fn tamper_replay_and_counter_each_change_the_token() {
        let base = rit_token(SEED, SID, 1, 5, BODY);
        // rit-tamper: same token intent, different body -> different HMAC.
        assert_ne!(base, rit_token(SEED, SID, 1, 5, b"{\"injected\":\"evil\"}"));
        // counter advance -> different token (so a stale replay is detectable).
        assert_ne!(base, rit_token(SEED, SID, 2, 5, BODY));
        // bucket rotation -> different token.
        assert_ne!(base, rit_token(SEED, SID, 1, 6, BODY));
        // different seed -> different token.
        assert_ne!(
            base,
            rit_token(b"other-seed-bytes-0000000000000000", SID, 1, 5, BODY)
        );
    }
}
