//! Bounded padded RFC4648 base32; byte encoding, not a cryptographic primitive.

use crate::{Error, frame::MAX_CARRIER_BYTES};

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const MAX_RAW: usize = MAX_CARRIER_BYTES + 10;
const MAX_ENCODED: usize = MAX_RAW.div_ceil(5) * 8;

pub(super) fn encode(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.len() > MAX_RAW {
        return Err(Error::ResourceLimit);
    }
    let mut out = Vec::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize]);
        }
    }
    if bits != 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize]);
    }
    while !out.len().is_multiple_of(8) {
        out.push(b'=');
    }
    Ok(out)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    if bytes.len() > MAX_ENCODED {
        return Err(Error::ResourceLimit);
    }
    if !bytes.len().is_multiple_of(8) {
        return Ok(None);
    }
    let significant = bytes
        .iter()
        .position(|byte| *byte == b'=')
        .unwrap_or(bytes.len());
    let padding = bytes.len() - significant;
    if !matches!(
        (significant % 8, padding),
        (0, 0) | (2, 6) | (4, 4) | (5, 3) | (7, 1)
    ) || bytes[significant..].iter().any(|byte| *byte != b'=')
    {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(significant * 5 / 8);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in &bytes[..significant] {
        let value = match *byte {
            b'A'..=b'Z' => *byte - b'A',
            b'2'..=b'7' => *byte - b'2' + 26,
            _ => return Ok(None),
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    // Historical Go decoder did not require zero unused tail bits. New encoding
    // emits canonical zeros; recovery still requires the decoded frame's CRC.
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_base32_vectors_and_bad_padding() {
        // Primary vectors: https://www.rfc-editor.org/rfc/rfc4648#section-10
        for (plain, encoded) in [
            ("f", "MY======"),
            ("fo", "MZXQ===="),
            ("foo", "MZXW6==="),
            ("foob", "MZXW6YQ="),
            ("fooba", "MZXW6YTB"),
            ("foobar", "MZXW6YTBOI======"),
        ] {
            assert_eq!(encode(plain.as_bytes()).unwrap(), encoded.as_bytes());
            assert_eq!(
                decode(encoded.as_bytes()).unwrap().unwrap(),
                plain.as_bytes()
            );
        }
        for encoded in [
            "MY=====",
            "MY====A=",
            "M=======",
            "my======",
            "MY======MY======",
        ] {
            assert!(decode(encoded.as_bytes()).unwrap().is_none());
        }
    }
}
