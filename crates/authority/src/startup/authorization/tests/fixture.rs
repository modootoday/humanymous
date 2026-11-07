//! Independent test-only byte construction, never an issuer/native capability.
use super::super::*;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

pub(super) fn digest(kind: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"humanymous.observer-bootstrap.authorization/2/");
    hash.update(kind.as_bytes());
    hash.update([0]);
    hash.update(bytes);
    hash.finalize().into()
}
fn short(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(u8::try_from(bytes.len()).unwrap());
    out.extend_from_slice(bytes);
}
fn sid(tag: u8, max: bool) -> Vec<u8> {
    let mut bytes = vec![1, if max { 15 } else { 0 }, 0, 0, 0, 0, 0, tag];
    if max {
        for n in 0..15u32 {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
    }
    bytes
}
fn name(tag: char, max: bool) -> String {
    format!(
        "humanymous-lab-{}{}",
        if max { "a".repeat(47) } else { String::new() },
        tag
    )
}
pub(super) fn expected(max: bool) -> ExpectedAuthorizationData {
    let role = |tag, id, epoch| RoleData {
        role: tag,
        name: name(id, max),
        sid: sid(id as u8, max),
        pid: u32::from(id as u8),
        creation: u64::from(id as u8),
        epoch: [epoch; 16],
        base_plan: [epoch + 1; 32],
        artifact: [epoch + 2; 32],
    };
    ExpectedAuthorizationData {
        installer: InstallerData {
            public_key: SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes(),
            sid: sid(b'i', max),
            installation_epoch: [3; 16],
            delegation_commitment: [0; 32],
            delegation_id: [4; 32],
            not_before_ms: 500,
            not_after_ms: 10_000,
        },
        publisher: PublisherData {
            public_key: SigningKey::from_bytes(&[2; 32]).verifying_key().to_bytes(),
            sid: sid(b'p', max),
            name: name('p', max),
            artifact: [5; 32],
            pid: 10,
            creation: 100,
            epoch: [6; 16],
        },
        service: role(AudienceRole::Service, 's', 7),
        observer: role(AudienceRole::Observer, 'o', 8),
        scope: ScopeData {
            installation: "i".into(),
            tenant: "t".into(),
            origin: "o".into(),
            revision: "r".into(),
            application_authority: "example.test".into(),
        },
        attempt: AttemptData {
            grant_id: [9; 32],
            credential_epoch: [10; 16],
            startup_cap_ms: 1000,
            earliest_issued_ms: 1000,
            latest_not_after_ms: 5000,
        },
    }
}
pub(super) fn scope(e: &ExpectedAuthorizationData) -> [u8; 32] {
    let mut data = Vec::new();
    for s in [
        &e.scope.installation,
        &e.scope.tenant,
        &e.scope.origin,
        &e.scope.revision,
        &e.scope.application_authority,
    ] {
        data.extend_from_slice(&u32::try_from(s.len()).unwrap().to_be_bytes());
        data.extend_from_slice(s.as_bytes());
    }
    digest("scope", &data)
}
pub(super) fn role(r: &RoleData) -> [u8; 32] {
    let mut b = vec![match r.role {
        AudienceRole::Service => 1,
        AudienceRole::Observer => 2,
    }];
    short(&mut b, &r.sid);
    b.extend_from_slice(&r.pid.to_be_bytes());
    b.extend_from_slice(&r.creation.to_be_bytes());
    b.extend_from_slice(&r.epoch);
    b.extend_from_slice(&r.base_plan);
    digest("role-instance", &b)
}
pub(super) fn wrap(magic: &[u8; 8], kind: &str, body: &[u8], seed: u8) -> Vec<u8> {
    let mut message =
        format!("humanymous.observer-bootstrap.authorization/2/{kind}\0").into_bytes();
    message.extend_from_slice(&u32::try_from(body.len()).unwrap().to_be_bytes());
    message.extend_from_slice(body);
    let signature = SigningKey::from_bytes(&[seed; 32])
        .sign(&message)
        .to_bytes();
    let mut out = magic.to_vec();
    out.extend_from_slice(&u16::try_from(body.len()).unwrap().to_be_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(&signature);
    out
}
pub(super) fn records(e: &mut ExpectedAuthorizationData) -> [Vec<u8>; 3] {
    let cap = e.attempt.startup_cap_ms;
    timed(e, 1000, 1001, 5000, cap)
}
pub(super) fn timed(
    e: &mut ExpectedAuthorizationData,
    issued: u64,
    commit_issued: u64,
    expiry: u64,
    commit_cap: u32,
) -> [Vec<u8>; 3] {
    let i = &e.installer;
    let p = &e.publisher;
    let s = &e.service;
    let o = &e.observer;
    let key = digest("key", &p.public_key);
    let mut ip = Vec::new();
    short(&mut ip, &i.sid);
    ip.extend_from_slice(&i.installation_epoch);
    let mut pair = Vec::new();
    for (tag, r) in [(1u8, s), (2, o)] {
        pair.push(tag);
        short(&mut pair, r.name.as_bytes());
        short(&mut pair, &r.sid);
        pair.extend_from_slice(&r.artifact);
        pair.extend_from_slice(&r.base_plan);
    }
    let mut d = vec![0, 2];
    d.extend_from_slice(&i.installation_epoch);
    d.extend_from_slice(&scope(e));
    d.extend_from_slice(&digest("installer-principal", &ip));
    d.extend_from_slice(&digest("key", &i.public_key));
    short(&mut d, &p.sid);
    short(&mut d, p.name.as_bytes());
    d.extend_from_slice(&p.public_key);
    d.extend_from_slice(&p.artifact);
    d.extend_from_slice(&digest("pair-policy", &pair));
    d.extend_from_slice(&i.delegation_id);
    d.extend_from_slice(&i.not_before_ms.to_be_bytes());
    d.extend_from_slice(&i.not_after_ms.to_be_bytes());
    d.extend_from_slice(&[1, 0]);
    let delegation = wrap(b"HMBGD002", "delegation", &d, 1);
    let link = digest("delegation-record", &delegation);
    let mut installation = scope(e).to_vec();
    installation.extend_from_slice(&digest("key", &i.public_key));
    installation.extend_from_slice(&i.installation_epoch);
    short(&mut installation, s.name.as_bytes());
    short(&mut installation, o.name.as_bytes());
    let mut pi = Vec::new();
    short(&mut pi, &p.sid);
    pi.extend_from_slice(&p.pid.to_be_bytes());
    pi.extend_from_slice(&p.creation.to_be_bytes());
    pi.extend_from_slice(&p.epoch);
    let mut b = vec![0, 2];
    for h in [
        link,
        scope(e),
        digest("installation", &installation),
        key,
        e.attempt.grant_id,
        digest("publisher-instance", &pi),
    ] {
        b.extend_from_slice(&h);
    }
    b.extend_from_slice(&p.pid.to_be_bytes());
    b.extend_from_slice(&p.creation.to_be_bytes());
    b.extend_from_slice(&p.epoch);
    b.extend_from_slice(&s.epoch);
    b.extend_from_slice(&o.epoch);
    b.extend_from_slice(&e.attempt.credential_epoch);
    short(&mut b, s.name.as_bytes());
    short(&mut b, o.name.as_bytes());
    short(&mut b, &s.sid);
    short(&mut b, &o.sid);
    for r in [s, o] {
        b.extend_from_slice(&r.pid.to_be_bytes());
        b.extend_from_slice(&r.creation.to_be_bytes());
    }
    for h in [s.base_plan, o.base_plan, s.artifact, o.artifact] {
        b.extend_from_slice(&h);
    }
    b.extend_from_slice(&issued.to_be_bytes());
    b.extend_from_slice(&expiry.to_be_bytes());
    b.extend_from_slice(&e.attempt.startup_cap_ms.to_be_bytes());
    b.extend_from_slice(&[1, 0]);
    let prepared = wrap(b"HMBGP002", "prepared", &b, 2);
    let mut c = vec![0, 2];
    for h in [
        link,
        e.attempt.grant_id,
        digest("prepared-record", &prepared),
        key,
    ] {
        c.extend_from_slice(&h);
    }
    c.extend_from_slice(&s.epoch);
    c.extend_from_slice(&o.epoch);
    c.extend_from_slice(&e.attempt.credential_epoch);
    c.extend_from_slice(&commit_issued.to_be_bytes());
    c.extend_from_slice(&expiry.to_be_bytes());
    c.extend_from_slice(&commit_cap.to_be_bytes());
    c.extend_from_slice(&[1, 0, 3]);
    let commit = wrap(b"HMBGC002", "commit", &c, 2);
    e.installer.delegation_commitment = link;
    [delegation, prepared, commit]
}

pub(super) fn public_vector(
    e: &ExpectedAuthorizationData,
    records: &[Vec<u8>; 3],
) -> serde_json::Value {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let role = |r: &RoleData| {
        serde_json::json!({
            "role":match r.role {AudienceRole::Service=>1,AudienceRole::Observer=>2},
            "name":r.name,"sid":hex::encode(&r.sid),"pid":r.pid,"creation":r.creation,
            "epoch":hex::encode(r.epoch),"base_plan":hex::encode(r.base_plan),"artifact":hex::encode(r.artifact)
        })
    };
    serde_json::json!({
        "schema":"humanymous.authorization-test-vector/2","fixture_only":true,"now_ms":2000,
        "delegation":URL_SAFE_NO_PAD.encode(&records[0]),"prepared":URL_SAFE_NO_PAD.encode(&records[1]),"commit":URL_SAFE_NO_PAD.encode(&records[2]),
        "expected":{
            "installer":{"public_key":hex::encode(e.installer.public_key),"sid":hex::encode(&e.installer.sid),"installation_epoch":hex::encode(e.installer.installation_epoch),"delegation_commitment":hex::encode(e.installer.delegation_commitment),"delegation_id":hex::encode(e.installer.delegation_id),"not_before_ms":e.installer.not_before_ms,"not_after_ms":e.installer.not_after_ms},
            "publisher":{"name":e.publisher.name,"sid":hex::encode(&e.publisher.sid),"public_key":hex::encode(e.publisher.public_key),"artifact":hex::encode(e.publisher.artifact),"pid":e.publisher.pid,"creation":e.publisher.creation,"epoch":hex::encode(e.publisher.epoch)},
            "service":role(&e.service),"observer":role(&e.observer),
            "scope":{"installation":e.scope.installation,"tenant":e.scope.tenant,"origin":e.scope.origin,"revision":e.scope.revision,"application_authority":e.scope.application_authority},
            "attempt":{"grant_id":hex::encode(e.attempt.grant_id),"credential_epoch":hex::encode(e.attempt.credential_epoch),"startup_cap_ms":e.attempt.startup_cap_ms,"earliest_issued_ms":e.attempt.earliest_issued_ms,"latest_not_after_ms":e.attempt.latest_not_after_ms}
        }
    })
}
