//! Canonical RFC4648 bytes; no whitespace, alternate padding or lossy decoding.
use crate::{Result, require};
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub(super) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for part in bytes.chunks(3) {
        let a = part[0];
        let b = part.get(1).copied().unwrap_or(0);
        let c = part.get(2).copied().unwrap_or(0);
        out.push(ALPHABET[(a >> 2) as usize] as char);
        out.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if part.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if part.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
pub(super) fn decode(text: &str, cap: usize) -> Result<Vec<u8>> {
    require(
        text.len().is_multiple_of(4) && text.len() <= cap.div_ceil(3) * 4,
        "proof base64 bound refused",
    )?;
    let mut out = Vec::new();
    for part in text.as_bytes().as_chunks::<4>().0 {
        let mut values = [0u8; 4];
        for (index, byte) in part.iter().enumerate() {
            values[index] = if *byte == b'=' {
                0
            } else {
                ALPHABET
                    .iter()
                    .position(|candidate| candidate == byte)
                    .ok_or("proof base64 alphabet refused")? as u8
            };
        }
        out.push((values[0] << 2) | (values[1] >> 4));
        if part[2] != b'=' {
            out.push((values[1] << 4) | (values[2] >> 2));
        }
        if part[3] != b'=' {
            out.push((values[2] << 6) | values[3]);
        }
    }
    require(
        out.len() <= cap && encode(&out) == text,
        "proof base64 padding refused",
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_vectors_and_binary_bytes_round_trip_without_normalization() {
        for (raw, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
        ] {
            assert_eq!(encode(raw.as_bytes()), coded);
            assert_eq!(decode(coded, 16).unwrap(), raw.as_bytes());
        }
        let bytes: Vec<_> = (0..=255).collect();
        assert_eq!(decode(&encode(&bytes), 256).unwrap(), bytes);
        for text in [
            "Zg", "Zg=", "Zg===", "Zh==", "Zm9=", "=m9v", "Zg==\n", "Zg==AAAA", "Zg--", "====",
        ] {
            assert!(decode(text, 64).is_err(), "{text}");
        }
        assert!(decode("Zm9v", 2).is_err());
    }
}
