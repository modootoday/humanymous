//! Ports of legacy internal/gate/ws3_test.go (baseline 981d25c) at the hold
//! queue, with the real reference vault for pseudonyms and the reverse index.
//!
//! An `ErasurePermit` is mintable only by the audit control ledger, so these
//! tests drive the queue with a test capability and model `Vault::shred` by
//! rebuilding the subject's vault without its key; the ledger-to-queue
//! composition is a separate runtime obligation.
use super::*;
use crate::audit::keys::vault::{Mode, Vault};
use std::convert::Infallible;

const T0: Duration = Duration::from_secs(1_000);

/// Test stand-in for the ledger-issued permit.
struct Permit;

fn vault() -> Vault {
    Vault::new(Mode::Hmac, 64).unwrap()
}

#[test]
fn erasure_hold_window_and_reverse_resolve() {
    let live = vault();
    let p0 = live.pseudonymize("subjX", "1.2.3.4").unwrap();
    live.link("psn-handle-X", "subjX").unwrap();

    // The operator names the console handle; it resolves to the subject.
    let subject = resolve_subject("psn-handle-X", |h| live.resolve(h).unwrap());
    assert_eq!(subject, "subjX");
    let mut queue = HoldQueue::new(DEFAULT_HOLD).unwrap();
    let scheduled = queue
        .schedule(&subject, "op-1", "dpo-1", "GDPR Art.17", Permit, T0)
        .unwrap();
    assert_eq!(scheduled.execute_at, T0 + DEFAULT_HOLD);

    // During the hold window nothing executes and the pseudonym is stable.
    let mut shredded = Vec::new();
    let mut shred = |s: &str, _: &Permit| -> Result<bool, Infallible> {
        shredded.push(s.to_owned());
        Ok(true)
    };
    assert!(
        queue
            .run_due(T0, ("abcdef0123456789", 41), &mut shred)
            .unwrap()
            .is_empty()
    );
    assert_eq!(live.pseudonymize("subjX", "1.2.3.4").unwrap(), p0);

    // After the window the shred executes once and completion evidence is sealed.
    let done = queue
        .run_due(
            T0 + Duration::from_secs(6 * 60),
            ("abcdef0123456789", 41),
            &mut shred,
        )
        .unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(shredded, ["subjX"]);
    assert_eq!(done[0].event_type, "erasure.completed");
    assert!(done[0].certificate.contains("erasure-cert"));
    assert!(done[0].certificate.contains("GDPR Art.17"));
    assert_eq!(
        done[0].certificate,
        "erasure-cert: sth_root=abcdef012345 tree_size=41 legal_basis=GDPR Art.17 op-1->dpo-1"
    );
    assert!(queue.list().is_empty());
}

#[test]
fn erasure_cancel() {
    let live = vault();
    let p0 = live.pseudonymize("subjY", "9.9.9.9").unwrap();
    let subject = resolve_subject("subjY", |h| live.resolve(h).unwrap());
    assert_eq!(
        subject, "subjY",
        "an unindexed value is already a subject id"
    );
    let mut queue = HoldQueue::new(DEFAULT_HOLD).unwrap();
    let scheduled = queue
        .schedule(&subject, "op-1", "dpo-1", "PIPA", Permit, T0)
        .unwrap();
    assert_eq!(queue.cancel(&scheduled.id), Some(scheduled.clone()));
    assert_eq!(queue.cancel(&scheduled.id), None);
    let done = queue
        .run_due(
            T0 + Duration::from_secs(10 * 60),
            ("", 0),
            |_, _: &Permit| -> Result<bool, Infallible> {
                panic!("a cancelled erasure must not execute")
            },
        )
        .unwrap();
    assert!(done.is_empty());
    assert_eq!(live.pseudonymize("subjY", "9.9.9.9").unwrap(), p0);
}

#[test]
fn failed_shred_stays_pending_and_missing_subject_is_reported() {
    let mut queue = HoldQueue::new(Duration::from_secs(1)).unwrap();
    let a = queue.schedule("a", "r", "d", "PIPA", Permit, T0).unwrap();
    assert_eq!(
        queue.run_due(T0 + Duration::from_secs(1), ("", 0), |_, _| Err("kms down")),
        Err("kms down")
    );
    assert_eq!(queue.list(), std::slice::from_ref(&a));
    let done = queue
        .run_due(T0 + Duration::from_secs(2), ("", 0), |_, _| {
            Ok::<_, ()>(false)
        })
        .unwrap();
    assert_eq!(done[0].event_type, "erasure.no_subject");
    assert_eq!(done[0].scheduled, a);
}

#[test]
fn schedule_refusals_and_unique_ids() {
    assert!(matches!(
        HoldQueue::<Permit>::new(Duration::ZERO),
        Err(Refusal::Hold)
    ));
    let mut queue = HoldQueue::new(DEFAULT_HOLD).unwrap();
    assert_eq!(
        queue.schedule("", "r", "d", "PIPA", Permit, T0).err(),
        Some(Refusal::Missing)
    );
    assert_eq!(
        queue.schedule("s", "r", "d", "", Permit, T0).err(),
        Some(Refusal::Missing)
    );
    let mut ids = std::collections::BTreeSet::new();
    for i in 0..MAX_PENDING {
        ids.insert(
            queue
                .schedule(&format!("s{i}"), "r", "d", "PIPA", Permit, T0)
                .unwrap()
                .id,
        );
    }
    assert_eq!(ids.len(), MAX_PENDING);
    assert_eq!(
        queue.schedule("over", "r", "d", "PIPA", Permit, T0).err(),
        Some(Refusal::Capacity)
    );
}
