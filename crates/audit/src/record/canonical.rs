use super::{Record, normalization::nfc};
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

fn field(out: &mut Vec<u8>, key: &str, value: &str) {
    out.extend_from_slice(&(key.len() as u32).to_be_bytes());
    out.extend_from_slice(key.as_bytes());
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
fn scalar(out: &mut Vec<u8>, key: &str, value: &str) {
    if !value.is_empty() {
        field(out, key, &nfc(value));
    }
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
    scalar(&mut out, "event_id", &record.event_id);
    field(&mut out, "seq", &(record.seq as i64).to_string());
    for (key, value) in [
        ("node_id", &record.node_id),
        ("ts", &record.ts),
        ("event_type", &record.event_type),
        ("event_version", &record.event_version),
        ("actor.kind", &record.actor.kind),
        ("actor.id", &record.actor.id_pseudonym),
        ("tenant_id", &record.tenant_id),
        ("session_pseudonym", &record.session_pseudonym),
        ("request_id", &record.request_id),
        ("correlation_id", &record.correlation_id),
        ("host", &record.host),
        ("route_class", &record.route_class),
        ("verdict", &record.verdict),
    ] {
        scalar(&mut out, key, value);
    }
    field(&mut out, "risk_score", &record.risk_score.to_string());
    let mut rules = record.triggered_rules.clone().unwrap_or_default();
    rules.sort();
    // The frozen source normalizes scalar strings and signal IDs, but not the
    // rule array or verdict element strings. Preserve those exact bytes.
    field(
        &mut out,
        "triggered_rules",
        &format!("[{}]", rules.join(",")),
    );
    let mut signals = record
        .top_signals
        .as_deref()
        .unwrap_or_default()
        .iter()
        .collect::<Vec<_>>();
    super::ordering::sort_signals(&mut signals);
    let parts = signals
        .into_iter()
        .map(|signal| {
            format!(
                "{}|{}|{}",
                nfc(&signal.id),
                signal.verdict,
                confidence(signal.conf)
            )
        })
        .collect::<Vec<_>>();
    field(&mut out, "top_signals", &format!("[{}]", parts.join(",")));
    for (key, value) in [
        ("enforcement_action", &record.enforcement_action),
        ("enforcement_mode", &record.enforcement_mode),
        ("fail_mode", &record.fail_mode),
        ("fail_reason", &record.fail_reason),
    ] {
        scalar(&mut out, key, value);
    }
    field(&mut out, "latency_us", &record.latency_us.to_string());
    if let Some(upstream) = &record.upstream {
        field(&mut out, "upstream.status", &upstream.status.to_string());
        scalar(&mut out, "upstream.error_class", &upstream.error_class);
    }
    if let Some(tls) = &record.tls {
        for (key, value) in [
            ("tls.ja4", &tls.ja4_pseudonym),
            ("tls.h2fp", &tls.h2fp_pseudonym),
            ("tls.sni", &tls.sni_pseudonym),
        ] {
            scalar(&mut out, key, value);
        }
    }
    for (key, value) in [
        ("config_version", &record.config_version),
        ("key_id", &record.key_id),
        ("data_class", &record.data_class),
    ] {
        scalar(&mut out, key, value);
    }
    out
}

/// Hex SHA256(canonical semantic body || exact previous-hash text).
pub fn record_hash(record: &Record, previous_hash: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(canonical_bytes(record));
    digest.update(previous_hash.as_bytes());
    hex::encode(digest.finalize())
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
