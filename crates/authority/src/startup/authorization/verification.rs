use super::*;
use ed25519_dalek::{Signature, VerifyingKey};

fn signature<T>(record: &Signed<T>, key: &[u8; 32], kind: &str) -> Result<()> {
    let key = VerifyingKey::from_bytes(key).map_err(|_| AuthorizationError::Signature)?;
    let signature =
        Signature::from_slice(&record.signature).map_err(|_| AuthorizationError::Signature)?;
    let mut message =
        format!("humanymous.observer-bootstrap.authorization/2/{kind}\0").into_bytes();
    message.extend_from_slice(&(record.body.len() as u32).to_be_bytes());
    message.extend_from_slice(&record.body);
    key.verify_strict(&message, &signature)
        .map_err(|_| AuthorizationError::Signature)
}

/// Check strict signatures, linkage, explicit expected DATA and reference time.
/// This cannot certify native provenance, entropy, consumption, liveness or Ready.
pub fn verify_bundle(
    parsed: ParsedAuthorization,
    expected: &ExpectedAuthorizationData,
    now_ms: u64,
) -> Result<SignatureCheckedAuthorization> {
    binding::validate(expected)?;
    let d = &parsed.delegation.claims;
    let p = &parsed.prepared.claims;
    let c = &parsed.commit.claims;
    let i = &expected.installer;
    let publisher = &expected.publisher;
    let delegation = binding::hash("delegation-record", &parsed.delegation.artifact);
    let prepared = binding::hash("prepared-record", &parsed.prepared.artifact);
    signature(&parsed.delegation, &i.public_key, "delegation")?;
    if delegation != i.delegation_commitment
        || d.id != i.delegation_id
        || d.installation_epoch != i.installation_epoch
        || d.scope != binding::scope(&expected.scope)
        || d.installer != binding::installer(i)
        || d.installer_key != binding::key(&i.public_key)
        || d.publisher_sid != publisher.sid
        || d.publisher_name != publisher.name
        || d.publisher_key != publisher.public_key
        || d.publisher_artifact != publisher.artifact
        || d.pair != binding::pair(expected)
        || d.not_before != i.not_before_ms
        || d.not_after != i.not_after_ms
    {
        return Err(AuthorizationError::Binding);
    }
    signature(&parsed.prepared, &publisher.public_key, "prepared")?;
    signature(&parsed.commit, &publisher.public_key, "commit")?;
    if p.delegation != delegation
        || c.delegation != delegation
        || p.scope != d.scope
        || p.installation != binding::installation(expected)
        || p.publisher_key != binding::key(&publisher.public_key)
        || c.publisher_key != p.publisher_key
        || p.grant != expected.attempt.grant_id
        || c.grant != p.grant
        || p.publisher_instance != binding::publisher(publisher)
        || p.publisher_pid != publisher.pid
        || p.publisher_creation != publisher.creation
        || p.publisher_epoch != publisher.epoch
        || p.credential_epoch != expected.attempt.credential_epoch
        || c.credential_epoch != p.credential_epoch
        || p.service != expected.service
        || p.observer != expected.observer
        || c.service_epoch != p.service.epoch
        || c.observer_epoch != p.observer.epoch
        || c.prepared != prepared
    {
        return Err(AuthorizationError::Binding);
    }
    let window = p
        .not_after
        .checked_sub(p.issued)
        .ok_or(AuthorizationError::Lifetime)?;
    if d.not_before >= d.not_after
        || p.issued < d.not_before
        || p.issued < expected.attempt.earliest_issued_ms
        || c.issued < p.issued
        || c.issued >= p.not_after
        || now_ms < c.issued
        || now_ms >= p.not_after
        || p.not_after > d.not_after
        || p.not_after > expected.attempt.latest_not_after_ms
        || c.not_after != p.not_after
        || p.cap != expected.attempt.startup_cap_ms
        || c.cap != p.cap
        || p.cap == 0
        || u64::from(p.cap) > window
    {
        return Err(AuthorizationError::Lifetime);
    }
    Ok(SignatureCheckedAuthorization {
        records: parsed,
        scope: expected.scope.clone(),
    })
}
