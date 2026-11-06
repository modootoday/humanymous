use sha2::{Digest, Sha256};

pub(super) fn meets_target(seed: &[u8], nonce: &str, difficulty: i32) -> bool {
    let mut hash = Sha256::new();
    hash.update(seed);
    hash.update(nonce.as_bytes());
    leading_zero_bits(&hash.finalize()) >= difficulty
}

fn leading_zero_bits(bytes: &[u8]) -> i32 {
    let mut count = 0;
    for byte in bytes {
        count += byte.leading_zeros() as i32;
        if *byte != 0 {
            break;
        }
    }
    count
}

/// Find the first decimal nonce in a bounded search, or return `None`.
pub fn solve(seed_hex: &str, difficulty: i32, max_iterations: i64) -> Option<String> {
    let seed = hex::decode(seed_hex).ok()?;
    (0..max_iterations)
        .map(|number| number.to_string())
        .find(|nonce| meets_target(&seed, nonce, difficulty))
}

/// Frozen risk-to-work policy from the Go implementation.
pub fn difficulty_for(risk: f64) -> i32 {
    if risk >= 70.0 {
        17
    } else if risk >= 50.0 {
        16
    } else if risk >= 30.0 {
        14
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_bits_cover_byte_boundaries() {
        for (bytes, expected) in [
            (&[0xff][..], 0),
            (&[0x0f], 4),
            (&[0, 0xff], 8),
            (&[0, 0x0f], 12),
            (&[0x80], 0),
            (&[1], 7),
            (&[0, 0], 16),
            (&[], 0),
        ] {
            assert_eq!(leading_zero_bits(bytes), expected);
        }
    }

    #[test]
    fn frozen_thresholds_and_nonfinite_risk() {
        for (risk, expected) in [
            (-1.0, 0),
            (29.999, 0),
            (30.0, 14),
            (49.999, 14),
            (50.0, 16),
            (69.999, 16),
            (70.0, 17),
            (f64::NAN, 0),
            (f64::INFINITY, 17),
        ] {
            assert_eq!(difficulty_for(risk), expected);
        }
    }

    #[test]
    fn solver_rejects_invalid_hex_and_exhaustion() {
        assert_eq!(solve("odd", 0, 1), None);
        assert_eq!(solve("00", 0, 0), None);
        assert_eq!(solve("00", 0, -1), None);
        assert_eq!(solve("00", 257, 3), None);
        assert_eq!(solve("00", 0, 1), Some("0".to_owned()));
    }
}
