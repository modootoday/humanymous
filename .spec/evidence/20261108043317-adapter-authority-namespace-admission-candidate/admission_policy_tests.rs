//! Authored component tests only; none creates native proof or Ready.
use super::{check, fixture};
use super::super::{admission_policy::*, binding, SignatureCheckedAuthorization};
use ed25519_dalek::{Signer, SigningKey};
use std::time::{Duration, Instant};

fn case() -> (SignatureCheckedAuthorization, OriginalNamespaceData, OriginalPinData, ChannelPolicyData) {
    case_for(0, 0)
}
fn case_for(grant: u8, instance: u8) -> (SignatureCheckedAuthorization, OriginalNamespaceData, OriginalPinData, ChannelPolicyData) {
    let mut expected = fixture::expected(false);
    expected.attempt.grant_id = [9 + grant; 32];
    expected.attempt.credential_epoch = [10 + grant; 16];
    expected.service.epoch = [7 + instance; 16];
    expected.observer.epoch = [8 + instance; 16];
    let records = fixture::records(&mut expected);
    let checked = check(&records, &expected, 2000).unwrap();
    let original = OriginalNamespaceData {
        namespace: [21; 32], generation: 1,
        installer_key: expected.installer.public_key,
        installation_epoch: expected.installer.installation_epoch,
        delegation: *checked.delegation_commitment(),
        delegation_id: expected.installer.delegation_id,
        scope: binding::scope(&expected.scope),
    };
    let pin = OriginalPinData {
        namespace: original.namespace, generation: 1, record_digest: original_digest(&original),
    };
    let policy = ChannelPolicyData {
        peer_key: SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes(),
        channel: [22 + grant; 32], peer_nonce: [23 + grant; 16], witness_nonce: [24 + grant; 16],
        witness: WitnessData {
            sid: vec![1, 0, 0, 0, 0, 0, 0, b'w'], pid: 800, creation: 900,
            epoch: [25; 16], key: SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes(),
            lease: [26; 32], not_after_ms: 4900,
        },
    };
    (checked, original, pin, policy)
}

// Independent test encoder intentionally does not call production transcript().
fn message(role: u8, c: &SignatureCheckedAuthorization, o: &OriginalNamespaceData, p: &ChannelPolicyData) -> Vec<u8> {
    let mut out = b"humanymous.observer-bootstrap.admission-candidate/1/challenge\0".to_vec();
    out.push(role);
    for bytes in [original_digest(o), *c.delegation_commitment(), *c.grant_id()] { out.extend(bytes); }
    out.extend(c.credential_epoch());
    out.extend(binding::role(c.service())); out.extend(binding::role(c.observer()));
    out.extend(p.channel); out.extend(p.peer_key); out.extend(p.peer_nonce); out.extend(p.witness_nonce);
    let w = &p.witness;
    out.push(u8::try_from(w.sid.len()).unwrap()); out.extend(&w.sid);
    out.extend(w.pid.to_be_bytes()); out.extend(w.creation.to_be_bytes()); out.extend(w.epoch);
    out.extend(w.key); out.extend(w.lease); out.extend(w.not_after_ms.to_be_bytes()); out
}
fn response(c: &SignatureCheckedAuthorization, o: &OriginalNamespaceData, p: &ChannelPolicyData) -> ResponseData {
    ResponseData {
        peer_signature: SigningKey::from_bytes(&[11; 32]).sign(&message(1, c, o, p)).to_bytes(),
        witness_signature: SigningKey::from_bytes(&[12; 32]).sign(&message(2, c, o, p)).to_bytes(),
    }
}

#[test]
fn verified_data_and_reservation_still_cannot_mint_native_admission() {
    let (c, o, pin, p) = case(); let r = response(&c, &o, &p);
    let data = review_data(&c, &o, &pin, &p, &r, 2000).unwrap();
    let now = Instant::now(); let ledger = ReviewLedger::new(1, now, Duration::from_secs(1), 2000).unwrap();
    let reservation = ledger.reserve(data, now, 2000).unwrap();
    assert_eq!(native_admission(&reservation), Err(Refusal::NativeUnsupported));
    assert!(matches!(crate::authority::startup::observer::native_preparation(),
        Err(crate::authority::startup::observer::NativePreparationUnavailable::Unsupported)));
}

#[test]
fn original_namespace_key_epoch_scope_delegation_and_generation_substitution_refuse() {
    let changes: &[fn(&mut OriginalNamespaceData)] = &[
        |o| o.namespace[0] ^= 1, |o| o.generation += 1, |o| o.installer_key[0] ^= 1,
        |o| o.installation_epoch[0] ^= 1, |o| o.delegation[0] ^= 1,
        |o| o.delegation_id[0] ^= 1, |o| o.scope[0] ^= 1,
    ];
    for change in changes {
        let (c, mut o, pin, p) = case(); change(&mut o);
        assert!(matches!(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000), Err(Refusal::Original)));
    }
}

#[test]
fn matching_fake_record_digest_still_cannot_replace_the_signed_installer_delegation() {
    let (c, mut o, mut pin, p) = case(); o.installer_key[0] ^= 1;
    pin.record_digest = original_digest(&o);
    assert!(matches!(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000), Err(Refusal::Original)));
}

#[test]
fn every_channel_and_witness_field_mutation_invalidates_original_signatures() {
    let changes: &[fn(&mut ChannelPolicyData)] = &[
        |p| p.channel[0] ^= 1, |p| p.peer_key[0] ^= 1, |p| p.peer_nonce[0] ^= 1,
        |p| p.witness_nonce[0] ^= 1, |p| p.witness.sid[7] ^= 1,
        |p| p.witness.pid += 1, |p| p.witness.creation += 1, |p| p.witness.epoch[0] ^= 1,
        |p| p.witness.key[0] ^= 1, |p| p.witness.lease[0] ^= 1, |p| p.witness.not_after_ms -= 1,
    ];
    for change in changes {
        let (c, o, pin, mut p) = case(); let r = response(&c, &o, &p); change(&mut p);
        assert!(matches!(review_data(&c, &o, &pin, &p, &r, 2000), Err(Refusal::Signature)));
    }
}

#[test]
fn wrong_key_weak_signature_and_cross_role_reflection_refuse() {
    let (c, o, pin, p) = case();
    let mut r = response(&c, &o, &p); r.peer_signature = r.witness_signature;
    assert!(matches!(review_data(&c, &o, &pin, &p, &r, 2000), Err(Refusal::Signature)));
    let mut r = response(&c, &o, &p); r.witness_signature = [0; 64];
    assert!(matches!(review_data(&c, &o, &pin, &p, &r, 2000), Err(Refusal::Signature)));
    let mut r = response(&c, &o, &p);
    r.peer_signature = SigningKey::from_bytes(&[13; 32]).sign(&message(1, &c, &o, &p)).to_bytes();
    assert!(matches!(review_data(&c, &o, &pin, &p, &r, 2000), Err(Refusal::Signature)));
}

#[test]
fn witness_role_alias_zero_nonce_and_unbounded_sid_refuse_before_encoding() {
    let changes: &[fn(&mut ChannelPolicyData)] = &[
        |p| p.witness.pid = u32::from(b's'), |p| p.witness.sid[7] = b'o',
        |p| p.witness.key = p.peer_key, |p| p.peer_nonce = [0; 16],
        |p| p.witness_nonce = p.peer_nonce, |p| p.witness.sid = vec![1; 256],
        |p| p.witness.lease = [0; 32],
    ];
    for change in changes {
        let (c, o, pin, mut p) = case(); let r = response(&c, &o, &p); change(&mut p);
        assert!(matches!(review_data(&c, &o, &pin, &p, &r, 2000), Err(Refusal::Binding)));
    }
}

#[test]
fn invalid_signature_cannot_burn_legitimate_reservation_and_drop_cannot_restore_it() {
    let (c, o, pin, p) = case(); let good = response(&c, &o, &p);
    let now = Instant::now(); let ledger = ReviewLedger::new(1, now, Duration::from_secs(1), 2000).unwrap();
    let bad = ResponseData { peer_signature: [0; 64], witness_signature: [0; 64] };
    assert!(review_data(&c, &o, &pin, &p, &bad, 2000).is_err());
    drop(ledger.reserve(review_data(&c, &o, &pin, &p, &good, 2000).unwrap(), now, 2000).unwrap());
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &good, 2000).unwrap(), now, 2000), Err(Refusal::Replay)));
    assert!(matches!(ReviewLedger::recover(b"successful ACK cursor"), Err(Refusal::Recovery)));
}

#[test]
fn concurrent_exact_replay_has_only_one_ephemeral_reservation_not_native_consumption() {
    let (c, o, pin, p) = case(); let r = response(&c, &o, &p);
    let first = review_data(&c, &o, &pin, &p, &r, 2000).unwrap();
    let second = review_data(&c, &o, &pin, &p, &r, 2000).unwrap();
    let now = Instant::now(); let ledger = ReviewLedger::new(2, now, Duration::from_secs(1), 2000).unwrap();
    let wins = std::thread::scope(|s| {
        let a = s.spawn(|| ledger.reserve(first, now, 2000).is_ok());
        let b = s.spawn(|| ledger.reserve(second, now, 2000).is_ok());
        usize::from(a.join().unwrap()) + usize::from(b.join().unwrap())
    });
    assert_eq!(wins, 1);
}

#[test]
fn absolute_deadline_and_wall_rollback_are_latched_and_cannot_renew() {
    let (c, o, pin, p) = case(); let r = response(&c, &o, &p);
    let now = Instant::now(); let budget = Duration::from_millis(10);
    let ledger = ReviewLedger::new(2, now, budget, 2001).unwrap();
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2000).unwrap(), now, 2000), Err(Refusal::Expired)));
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2002).unwrap(), now, 2002), Err(Refusal::Expired)));
    let ledger = ReviewLedger::new(2, now, budget, 2000).unwrap();
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2000).unwrap(), now + budget, 2000), Err(Refusal::Expired)));
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2000).unwrap(), now, 2000), Err(Refusal::Expired)));
}

#[test]
fn witness_expiry_exact_boundary_future_commit_and_unbounded_lease_refuse() {
    let (c, o, pin, mut p) = case(); let r = response(&c, &o, &p);
    assert!(matches!(review_data(&c, &o, &pin, &p, &r, 4900), Err(Refusal::Expired)));
    assert!(matches!(review_data(&c, &o, &pin, &p, &r, 0), Err(Refusal::Expired)));
    p.witness.not_after_ms = 5001;
    assert!(matches!(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000), Err(Refusal::Expired)));
}

#[test]
fn fresh_grant_cannot_republish_same_process_epoch_and_capacity_never_evicts_tombstones() {
    let now = Instant::now(); let ledger = ReviewLedger::new(1, now, Duration::from_secs(1), 2000).unwrap();
    let (c, o, pin, p) = case();
    drop(ledger.reserve(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000).unwrap(), now, 2000).unwrap());
    let (c, o, pin, p) = case_for(1, 0);
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000).unwrap(), now, 2000), Err(Refusal::Replay)));
    // Distinct role epochs and nonces pass signatures but cannot evict the old entry.
    let (c, o, pin, p) = case_for(4, 1);
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &response(&c, &o, &p), 2000).unwrap(), now, 2000), Err(Refusal::Capacity)));
}

#[test]
fn reservation_rechecks_witness_expiry_and_signed_startup_cap_without_renewal() {
    let (c, o, pin, p) = case(); let r = response(&c, &o, &p); let now = Instant::now();
    let ledger = ReviewLedger::new(1, now, Duration::from_secs(1), 2000).unwrap();
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2000).unwrap(), now, 4900), Err(Refusal::Expired)));
    let ledger = ReviewLedger::new(1, now, Duration::from_millis(1001), 2000).unwrap();
    assert!(matches!(ledger.reserve(review_data(&c, &o, &pin, &p, &r, 2000).unwrap(), now, 2000), Err(Refusal::Expired)));
    assert!(matches!(ReviewLedger::new(129, now, Duration::from_secs(1), 2000), Err(Refusal::Binding)));
    assert!(matches!(ReviewLedger::new(1, now, Duration::ZERO, 2000), Err(Refusal::Binding)));
}
