use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

const PROTOCOL: &str = "humanymous.observer-bootstrap/0.3";
const FORMAT: &str = "humanymous.observer-bootstrap.authorization/2";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    format: String,
    delegation: String,
    prepared: String,
    commit: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    protocol: String,
    phase: String,
    purpose: String,
    bootstrap_scope: String,
    grant_id: String,
    service_epoch: String,
    observer_epoch: String,
    credential_epoch: String,
    sender_pin: String,
    receiver_pin: String,
    hello_nonce: String,
    grant: Grant,
}

/// Canonical public Hello metadata, not an authenticated transport or native peer.
pub struct ParsedHello {
    metadata: Metadata,
    records: ParsedAuthorization,
    nonce: [u8; 16],
}
impl ParsedHello {
    /// Untrusted parsed16byte nonce. Actual exchange freshness is not proven here.
    pub fn nonce(&self) -> &[u8; 16] {
        &self.nonce
    }
}
fn lower_hex<const N: usize>(s: &str) -> Result<[u8; N]> {
    if s.len() != N * 2
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AuthorizationError::Encoding);
    }
    let mut bytes = [0; N];
    hex::decode_to_slice(s, &mut bytes).map_err(|_| AuthorizationError::Encoding)?;
    Ok(bytes)
}
fn decode(s: &str, cap: usize) -> Result<Vec<u8>> {
    let max = (cap * 4).div_ceil(3);
    if s.len() > max {
        return Err(AuthorizationError::Size);
    }
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(AuthorizationError::Encoding);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AuthorizationError::Encoding)?;
    if bytes.len() > cap {
        return Err(AuthorizationError::Size);
    }
    if URL_SAFE_NO_PAD.encode(&bytes) != s {
        return Err(AuthorizationError::Encoding);
    }
    Ok(bytes)
}
/// Parse only the Hello METADATA (not a control frame). Enforce3072 before serde.
/// HMBP0003 transport framing/aggregate startup budget remain separate ownership.
pub fn parse_hello(bytes: &[u8]) -> Result<ParsedHello> {
    if bytes.len() > 3072 {
        return Err(AuthorizationError::Size);
    }
    let metadata: Metadata =
        serde_json::from_slice(bytes).map_err(|_| AuthorizationError::Encoding)?;
    if metadata.protocol != PROTOCOL
        || metadata.phase != "hello"
        || metadata.purpose != "observer-status"
        || metadata.grant.format != FORMAT
        || serde_json::to_vec(&metadata).map_err(|_| AuthorizationError::Encoding)? != bytes
    {
        return Err(AuthorizationError::Encoding);
    }
    lower_hex::<32>(&metadata.bootstrap_scope)?;
    lower_hex::<32>(&metadata.sender_pin)?;
    lower_hex::<32>(&metadata.receiver_pin)?;
    let grant = lower_hex::<32>(&metadata.grant_id)?;
    let service = lower_hex::<16>(&metadata.service_epoch)?;
    let observer = lower_hex::<16>(&metadata.observer_epoch)?;
    let credential = lower_hex::<16>(&metadata.credential_epoch)?;
    let nonce = lower_hex::<16>(&metadata.hello_nonce)?;
    if [
        &grant[..],
        &service[..],
        &observer[..],
        &credential[..],
        &nonce[..],
    ]
    .iter()
    .any(|v| !binding::nonzero(v))
    {
        return Err(AuthorizationError::Encoding);
    }
    let delegation = decode(&metadata.grant.delegation, codec::CAPS[0])?;
    let prepared = decode(&metadata.grant.prepared, codec::CAPS[1])?;
    let commit = decode(&metadata.grant.commit, codec::CAPS[2])?;
    let records = parse_bundle(&delegation, &prepared, &commit)?;
    Ok(ParsedHello {
        metadata,
        records,
        nonce,
    })
}
/// Validate repeated Hello DATA and the bundle. Nonce freshness/native transport
/// are NOT implied; inspect/retain nonce separately before an actual exchange.
pub fn verify_hello(
    hello: ParsedHello,
    expected: &ExpectedAuthorizationData,
    now_ms: u64,
) -> Result<SignatureCheckedAuthorization> {
    binding::validate(expected)?;
    let m = &hello.metadata;
    if lower_hex::<32>(&m.bootstrap_scope)? != binding::scope(&expected.scope)
        || lower_hex::<32>(&m.grant_id)? != expected.attempt.grant_id
        || lower_hex::<16>(&m.service_epoch)? != expected.service.epoch
        || lower_hex::<16>(&m.observer_epoch)? != expected.observer.epoch
        || lower_hex::<16>(&m.credential_epoch)? != expected.attempt.credential_epoch
        || lower_hex::<32>(&m.sender_pin)? != binding::role(&expected.service)
        || lower_hex::<32>(&m.receiver_pin)? != binding::role(&expected.observer)
    {
        return Err(AuthorizationError::Binding);
    }
    verify_bundle(hello.records, expected, now_ms)
}

#[cfg(test)]
pub(super) fn canonical_fixture(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::from_value::<Metadata>(value).unwrap()).unwrap()
}
#[cfg(test)]
pub(super) fn fixture_hello(records: &[Vec<u8>; 3], e: &ExpectedAuthorizationData) -> Vec<u8> {
    serde_json::to_vec(&Metadata {
        protocol: PROTOCOL.into(),
        phase: "hello".into(),
        purpose: "observer-status".into(),
        bootstrap_scope: hex::encode(binding::scope(&e.scope)),
        grant_id: hex::encode(e.attempt.grant_id),
        service_epoch: hex::encode(e.service.epoch),
        observer_epoch: hex::encode(e.observer.epoch),
        credential_epoch: hex::encode(e.attempt.credential_epoch),
        sender_pin: hex::encode(binding::role(&e.service)),
        receiver_pin: hex::encode(binding::role(&e.observer)),
        hello_nonce: "11".repeat(16),
        grant: Grant {
            format: FORMAT.into(),
            delegation: URL_SAFE_NO_PAD.encode(&records[0]),
            prepared: URL_SAFE_NO_PAD.encode(&records[1]),
            commit: URL_SAFE_NO_PAD.encode(&records[2]),
        },
    })
    .unwrap()
}
