mod encoder;
use super::Record;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Audit writers require an actual nonempty authentication key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyMacKey;
impl std::fmt::Display for EmptyMacKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("audit authentication key is empty")
    }
}
impl std::error::Error for EmptyMacKey {}

/// Secondary HMAC over the exact lowercase record-hash text, not decoded bytes.
pub fn record_mac(hash: &str, key: &[u8]) -> Result<String, EmptyMacKey> {
    if key.is_empty() {
        return Err(EmptyMacKey);
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts nonempty keys");
    mac.update(hash.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Verify the exact hex wire text in constant time; uppercase is not silently
/// accepted because historical records compare the encoded lowercase value.
pub fn verify_record_mac(hash: &str, encoded: &str, key: &[u8]) -> bool {
    record_mac(hash, key)
        .is_ok_and(|expected| bool::from(expected.as_bytes().ct_eq(encoded.as_bytes())))
}

fn confidence(value: f64) -> String {
    if value.is_nan() {
        "NaN".into()
    } else if value == f64::INFINITY {
        "+Inf".into()
    } else if value == f64::NEG_INFINITY {
        "-Inf".into()
    } else {
        format!("{value:.3}")
    }
}

/// Serialize semantic fields in frozen order with UTF-8 byte-length framing.
/// Collection order in the original record and integrity/read-time fields stay
/// untouched. Empty scalars are omitted; canonical array fields always exist.
pub fn canonical_bytes(record: &Record) -> Vec<u8> {
    let mut out = Vec::new();
    encoder::write_record(record, &mut out, encoder::Limits::compatibility())
        .expect("canonical field byte lengths must fit u32");
    out
}

/// Hex SHA256(canonical semantic body || exact previous-hash text).
pub fn record_hash(record: &Record, previous_hash: &str) -> String {
    record_hash_bounded(record, previous_hash, usize::MAX, usize::MAX)
        .expect("canonical field byte lengths must fit u32")
}

pub(crate) fn record_hash_bounded(
    record: &Record,
    previous_hash: &str,
    bytes: usize,
    items: usize,
) -> std::io::Result<String> {
    let mut digest = Sha256::new();
    struct HashSink<'a>(&'a mut Sha256);
    impl std::io::Write for HashSink<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    {
        let mut sink = std::io::BufWriter::with_capacity(8192, HashSink(&mut digest));
        encoder::write_record(record, &mut sink, encoder::Limits { bytes, items })?;
        std::io::Write::flush(&mut sink)?;
    }
    digest.update(previous_hash.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Upstream;
    #[test]
    fn excluded_fields_do_not_change_hash_but_present_empty_objects_do() {
        let mut record = Record::default();
        let expected = record_hash(&record, "");
        record.record_hash = "ignored".into();
        record.prev_hash = "ignored".into();
        record.hmac = "ignored".into();
        record.incident = "ignored".into();
        assert_eq!(record_hash(&record, ""), expected);
        record.upstream = Some(Upstream::default());
        assert_ne!(record_hash(&record, ""), expected);
        assert_ne!(record_hash(&record, "different"), record_hash(&record, ""));
    }
}
