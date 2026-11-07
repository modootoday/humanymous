//! Reference HMAC attestation protocol. The caller enforces issuance rate limits
//! and challenge single use; this primitive alone provides neither property.
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Seconds per validity bucket.
pub const WINDOW: u64 = 60;

/// Issue a token bound to subject, challenge nonce and validity window.
pub fn issue(master_key: &[u8], subject: &str, nonce: &str, window: u64) -> String {
    let mut value =
        Hmac::<Sha256>::new_from_slice(master_key).expect("HMAC accepts keys of any length");
    value.update(b"attest");
    value.update(subject.as_bytes());
    value.update(b"|");
    value.update(nonce.as_bytes());
    value.update(b"|");
    value.update(&window.to_be_bytes());
    hex::encode(value.finalize().into_bytes())
}

/// Verify the exact encoded token in the current or adjacent wrapping u64 window.
/// Wrapping at integer boundaries preserves the existing Go byte protocol.
pub fn verify(master_key: &[u8], subject: &str, nonce: &str, token: &str, current: u64) -> bool {
    if token.is_empty() {
        return false;
    }
    [current, current.wrapping_sub(1), current.wrapping_add(1)]
        .into_iter()
        .any(|window| {
            bool::from(
                issue(master_key, subject, nonce, window)
                    .as_bytes()
                    .ct_eq(token.as_bytes()),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_key_subject_nonce_and_window() {
        let token = issue(b"master", "fp-alice", "nonce-a", 100);
        for current in [99, 100, 101] {
            assert!(verify(b"master", "fp-alice", "nonce-a", &token, current));
        }
        assert!(!verify(b"master", "fp-alice", "nonce-a", &token, 200));
        assert!(!verify(b"master", "fp-bob", "nonce-a", &token, 100));
        assert!(!verify(b"master", "fp-alice", "nonce-b", &token, 100));
        assert!(!verify(b"other", "fp-alice", "nonce-a", &token, 100));
        assert!(!verify(b"master", "fp-alice", "nonce-a", "", 100));
        assert!(!verify(
            b"master",
            "fp-alice",
            "nonce-a",
            &token.to_uppercase(),
            100
        ));
    }

    #[test]
    fn preserves_go_unsigned_window_boundaries() {
        let token = issue(b"master", "subject", "nonce", u64::MAX);
        assert!(verify(b"master", "subject", "nonce", &token, 0));
        let token = issue(b"master", "subject", "nonce", 0);
        assert!(verify(b"master", "subject", "nonce", &token, u64::MAX));
    }
}
