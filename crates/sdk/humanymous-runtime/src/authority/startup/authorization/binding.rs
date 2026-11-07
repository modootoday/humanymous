use super::*;
use sha2::{Digest, Sha256};

pub(super) fn nonzero(bytes: &[u8]) -> bool {
    bytes.iter().any(|b| *b != 0)
}
pub(super) fn valid_name(bytes: &[u8]) -> bool {
    bytes.strip_prefix(b"humanymous-lab-").is_some_and(|tail| {
        !tail.is_empty()
            && tail.len() <= 48
            && tail
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
    })
}
pub(super) fn valid_sid(bytes: &[u8]) -> bool {
    (8..=68).contains(&bytes.len())
        && bytes[0] == 1
        && bytes[1] <= 15
        && bytes.len() == 8 + 4 * usize::from(bytes[1])
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
pub(super) fn validate(e: &ExpectedAuthorizationData) -> Result<()> {
    let i = &e.installer;
    let p = &e.publisher;
    let s = &e.service;
    let o = &e.observer;
    let authority = e.scope.application_authority.as_bytes();
    if [
        i.sid.as_slice(),
        p.sid.as_slice(),
        s.sid.as_slice(),
        o.sid.as_slice(),
    ]
    .iter()
    .any(|sid| !valid_sid(sid))
        || [&p.name, &s.name, &o.name]
            .iter()
            .any(|n| !valid_name(n.as_bytes()))
        || p.name == s.name
        || p.name == o.name
        || s.name == o.name
        || p.sid == s.sid
        || p.sid == o.sid
        || s.sid == o.sid
        || i.sid == p.sid
        || i.public_key == p.public_key
        || key(&i.public_key) == key(&p.public_key)
        || p.pid == 0
        || s.pid == 0
        || o.pid == 0
        || p.pid == s.pid
        || p.pid == o.pid
        || s.pid == o.pid
        || p.creation == 0
        || s.creation == 0
        || o.creation == 0
        || s.role != AudienceRole::Service
        || o.role != AudienceRole::Observer
        || [
            i.installation_epoch,
            p.epoch,
            s.epoch,
            o.epoch,
            e.attempt.credential_epoch,
        ]
        .iter()
        .any(|v| !nonzero(v))
        || !nonzero(&i.delegation_id)
        || !nonzero(&e.attempt.grant_id)
        || [
            &e.scope.installation,
            &e.scope.tenant,
            &e.scope.origin,
            &e.scope.revision,
        ]
        .iter()
        .any(|v| !identity(v))
        || authority.is_empty()
        || authority.len() > 1024
        || authority
            .iter()
            .any(|b| !(33..=126).contains(b) || b"@/#?\\".contains(b))
        || i.not_before_ms >= i.not_after_ms
        || e.attempt.earliest_issued_ms >= e.attempt.latest_not_after_ms
        || e.attempt.startup_cap_ms == 0
    {
        return Err(AuthorizationError::Context);
    }
    Ok(())
}
pub(super) fn hash(kind: &str, bytes: &[u8]) -> [u8; 32] {
    let mut h = hasher(kind);
    h.update(bytes);
    h.finalize().into()
}
fn hasher(kind: &str) -> Sha256 {
    let mut h = Sha256::new();
    h.update(b"humanymous.observer-bootstrap.authorization/2/");
    h.update(kind.as_bytes());
    h.update([0]);
    h
}
pub(super) fn key(bytes: &[u8; 32]) -> [u8; 32] {
    hash("key", bytes)
}
fn short(h: &mut Sha256, bytes: &[u8]) {
    h.update([bytes.len() as u8]);
    h.update(bytes);
}
pub(super) fn scope(e: &ScopeData) -> [u8; 32] {
    let mut h = hasher("scope");
    for s in [
        &e.installation,
        &e.tenant,
        &e.origin,
        &e.revision,
        &e.application_authority,
    ] {
        h.update((s.len() as u32).to_be_bytes());
        h.update(s.as_bytes());
    }
    h.finalize().into()
}
pub(super) fn installer(e: &InstallerData) -> [u8; 32] {
    let mut h = hasher("installer-principal");
    short(&mut h, &e.sid);
    h.update(e.installation_epoch);
    h.finalize().into()
}
pub(super) fn installation(e: &ExpectedAuthorizationData) -> [u8; 32] {
    let mut h = hasher("installation");
    h.update(scope(&e.scope));
    h.update(key(&e.installer.public_key));
    h.update(e.installer.installation_epoch);
    short(&mut h, e.service.name.as_bytes());
    short(&mut h, e.observer.name.as_bytes());
    h.finalize().into()
}
pub(super) fn pair(e: &ExpectedAuthorizationData) -> [u8; 32] {
    let mut h = hasher("pair-policy");
    for (tag, r) in [(1u8, &e.service), (2, &e.observer)] {
        h.update([tag]);
        short(&mut h, r.name.as_bytes());
        short(&mut h, &r.sid);
        h.update(r.artifact);
        h.update(r.base_plan);
    }
    h.finalize().into()
}
pub(super) fn publisher(p: &PublisherData) -> [u8; 32] {
    let mut h = hasher("publisher-instance");
    short(&mut h, &p.sid);
    h.update(p.pid.to_be_bytes());
    h.update(p.creation.to_be_bytes());
    h.update(p.epoch);
    h.finalize().into()
}
pub(super) fn role(r: &RoleData) -> [u8; 32] {
    let mut h = hasher("role-instance");
    h.update([match r.role {
        AudienceRole::Service => 1u8,
        AudienceRole::Observer => 2,
    }]);
    short(&mut h, &r.sid);
    h.update(r.pid.to_be_bytes());
    h.update(r.creation.to_be_bytes());
    h.update(r.epoch);
    h.update(r.base_plan);
    h.finalize().into()
}
