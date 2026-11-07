use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::target::meets_target;

/// Seconds in one challenge time bucket.
pub const WINDOW: u64 = 120;

/// Wire representation issued to a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// Lowercase hexadecimal HMAC seed.
    pub seed: String,
    /// Required leading zero bits.
    pub difficulty: i32,
    /// Issuance time bucket.
    pub bucket: u64,
}

impl Challenge {
    /// Compact header value, `seed:difficulty:bucket`.
    pub fn encode(&self) -> String {
        format!("{}:{}:{}", self.seed, self.difficulty, self.bucket)
    }
}

fn seed_bytes(master_key: &[u8], session_id: &str, bucket: u64) -> [u8; 32] {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(master_key).expect("HMAC accepts keys of any length");
    mac.update(b"pow");
    mac.update(session_id.as_bytes());
    mac.update(&bucket.to_be_bytes());
    mac.finalize().into_bytes().into()
}

/// Derive a challenge with caller-supplied policy and time.
pub fn issue(master_key: &[u8], session_id: &str, difficulty: i32, bucket: u64) -> Challenge {
    Challenge {
        seed: hex::encode(seed_bytes(master_key, session_id, bucket)),
        difficulty,
        bucket,
    }
}

/// Verify the work and accept only the current bucket or its immediate neighbors.
pub fn verify(
    master_key: &[u8],
    session_id: &str,
    difficulty: i32,
    bucket: u64,
    current_bucket: u64,
    nonce: &str,
) -> bool {
    bucket.abs_diff(current_bucket) <= 1
        && meets_target(
            &seed_bytes(master_key, session_id, bucket),
            nonce,
            difficulty,
        )
}

/// Parse `bucket:nonce`, preserving all bytes after the first colon.
pub fn parse_solution(value: &str) -> Option<(u64, &str)> {
    let (bucket, nonce) = value.split_once(':')?;
    // Go ParseUint permits digits only, including leading zeroes.
    let digits = bucket;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((digits.parse().ok()?, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::challenge::proof_of_work::solve;

    #[test]
    fn work_binds_session_and_time() {
        let challenge = issue(b"master-key", "sess-1", 12, 1000);
        let nonce = solve(&challenge.seed, 12, 1 << 24).unwrap();
        assert!(verify(b"master-key", "sess-1", 12, 1000, 1000, &nonce));
        assert!(!verify(b"master-key", "other-sess", 12, 1000, 1000, &nonce));
        assert!(!verify(b"master-key", "sess-1", 12, 1000, 1005, &nonce));
        assert!(!verify(
            b"master-key",
            "sess-1",
            12,
            1000,
            1000,
            "not-a-solution-xyz"
        ));
    }

    #[test]
    fn bucket_distance_does_not_wrap() {
        assert!(verify(b"k", "s", 0, 0, 1, ""));
        assert!(verify(b"k", "s", 0, u64::MAX, u64::MAX - 1, ""));
        assert!(!verify(b"k", "s", 0, 0, u64::MAX, ""));
    }

    #[test]
    fn parses_go_unsigned_decimal_and_opaque_nonce() {
        assert_eq!(parse_solution("001000:a:b"), Some((1000, "a:b")));
        assert_eq!(parse_solution("+1000:x"), None);
        assert_eq!(parse_solution("0:"), Some((0, "")));
        for invalid in ["garbage", "-1:x", " 1:x", "+:x", "18446744073709551616:x"] {
            assert_eq!(parse_solution(invalid), None);
        }
    }
}
