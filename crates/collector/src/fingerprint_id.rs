//! Pure reductions of the Go oracle's `web/js/advanced.js` `computeFingerprintId`
//! (981d25c). The identifier is an unscored, client-asserted report field for
//! cross-session correlation; it is neither a signal nor a device attestation.

/// SHA-256 digest bytes the oracle kept (`digest.slice(0, 12)`).
pub const DIGEST_PREFIX: usize = 12;

/// Lowercase hex of the first twelve SHA-256 bytes, as the oracle formatted them.
pub fn digest_id(digest: &[u8]) -> Result<String, &'static str> {
    if digest.len() != 32 {
        return Err("fingerprint digest length");
    }
    Ok(digest[..DIGEST_PREFIX]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// The oracle's non-cryptographic fallback when WebCrypto digest rejects:
/// `h = (h * 31 + s.charCodeAt(i)) | 0` over UTF-16 code units, then
/// `(h >>> 0).toString(16)`. Int32 wrapping equals JavaScript `| 0` here
/// because `h * 31 + unit` stays an exact double before the ToInt32 step.
pub fn fallback_id(units: &[u16]) -> String {
    let hash = units.iter().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(*unit))
    });
    format!("{:x}", hash as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn digest_prefix_matches_the_oracle_hex_formatting() {
        // SHA-256("abc") from FIPS 180-2; the oracle keeps 12 bytes.
        let digest = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(digest_id(&digest).unwrap(), "ba7816bf8f01cfea414140de");
        assert_eq!(digest_id(&[0; 32]).unwrap(), "0".repeat(24));
        for wrong in [&[][..], &[0; 12], &[0; 31], &[0; 33]] {
            assert!(digest_id(wrong).is_err());
        }
    }

    #[test]
    fn fallback_hash_wraps_like_javascript_int32_and_prints_unsigned_hex() {
        // Vectors computed independently with exact ToInt32 reduction.
        for (text, expected) in [
            ("", "0"),
            ("abc", "17862"),
            ("zzzzzzzzzzzz", "416a2b80"),
            // Surrogate pair counts as two code units; the result is negative
            // before `>>> 0`.
            ("humanymous-fp-\u{1F512}|1920x1080x24x1|UTC", "d8981817"),
        ] {
            assert_eq!(fallback_id(&units(text)), expected, "{text}");
        }
        // Lone surrogates are code units, never replaced before hashing.
        assert_eq!(fallback_id(&[0xD800]), "d800");
    }
}
