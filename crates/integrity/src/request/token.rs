use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::Mac;
use subtle::ConstantTimeEq;

use super::rotation::mac;

/// Sign canonical UTF-8 followed by big-endian counter and bucket, without padding.
pub fn compute_token(seed: &[u8], canonical: &str, counter: u64, bucket: u64) -> String {
    let mut value = mac(seed);
    value.update(canonical.as_bytes());
    value.update(&counter.to_be_bytes());
    value.update(&bucket.to_be_bytes());
    URL_SAFE_NO_PAD.encode(value.finalize().into_bytes())
}

/// Compare the exact encoded token in constant time, preserving Go string semantics.
pub fn verify_token(seed: &[u8], canonical: &str, counter: u64, bucket: u64, token: &str) -> bool {
    compute_token(seed, canonical, counter, bucket)
        .as_bytes()
        .ct_eq(token.as_bytes())
        .into()
}

/// Clamp pre-epoch time to zero and use ten seconds for a nonpositive window.
pub fn time_bucket(unix_seconds: i64, window: i64) -> u64 {
    (unix_seconds.max(0) / if window <= 0 { 10 } else { window }) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_binds_every_input_and_exact_encoding() {
        let token = compute_token(b"seed", "request", 1, 9);
        assert!(verify_token(b"seed", "request", 1, 9, &token));
        assert!(!verify_token(b"other", "request", 1, 9, &token));
        assert!(!verify_token(b"seed", "tampered", 1, 9, &token));
        assert!(!verify_token(b"seed", "request", 2, 9, &token));
        assert!(!verify_token(b"seed", "request", 1, 10, &token));
        assert!(!verify_token(b"seed", "request", 1, 9, &(token + "=")));
    }

    #[test]
    fn bucket_handles_signed_boundaries() {
        assert_eq!(time_bucket(-1, 10), 0);
        assert_eq!(time_bucket(19, 0), 1);
        assert_eq!(time_bucket(20, -1), 2);
        assert_eq!(time_bucket(i64::MAX, 1), i64::MAX as u64);
    }
}
