//! Ledger invariants independent of MIME codecs, HTTP, scoring and wall clocks.

use super::*;

fn time(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn configuration(capacity: usize) -> Configuration {
    Configuration {
        record_ttl: time(10),
        reservation_ttl: time(2),
        capacity,
    }
}

fn ledger(capacity: usize) -> Ledger {
    Ledger::new(configuration(capacity), Duration::ZERO).unwrap()
}

fn issuance(id: u32) -> Issuance {
    Issuance {
        payload: Payload {
            tag_id: id,
            session_id: "session-one".into(),
            asset_id: "/asset.bin".into(),
            bucket: 20_300,
        },
        mime: "application/octet-stream".into(),
        channels: vec!["bin-trailer".into()],
        ip_pseudonym: Some("keyed-pseudonym".into()),
    }
}

#[test]
fn configuration_has_no_implicit_ttl_or_unbounded_capacity() {
    for capacity in [0, MAX_ENTRIES + 1] {
        assert!(matches!(
            Ledger::new(configuration(capacity), time(0)),
            Err(Error::InvalidConfiguration)
        ));
    }
    let mut config = configuration(1);
    config.record_ttl = Duration::ZERO;
    assert!(matches!(
        Ledger::new(config, time(0)),
        Err(Error::InvalidConfiguration)
    ));
    config = configuration(1);
    config.reservation_ttl = Duration::ZERO;
    assert!(matches!(
        Ledger::new(config, time(0)),
        Err(Error::InvalidConfiguration)
    ));
}

#[test]
fn pending_is_not_issued_and_drop_cancel_release_capacity_without_reusing_ids() {
    let ledger = ledger(1);
    let first = ledger.reserve(time(0)).unwrap();
    assert_eq!(first.tag_id(), 1);
    assert!(matches!(
        ledger.lookup(1, time(0)).unwrap(),
        Lookup::Unknown
    ));
    assert!(matches!(ledger.reserve(time(0)), Err(Error::Capacity)));
    drop(first);
    let second = ledger.reserve(time(0)).unwrap();
    assert_eq!(second.tag_id(), 2);
    second.cancel().unwrap();
    assert_eq!(ledger.reserve(time(0)).unwrap().tag_id(), 3);
}

#[test]
fn commit_retains_actual_metadata_until_exact_record_ttl_boundary() {
    let ledger = ledger(1);
    let reservation = ledger.reserve(time(0)).unwrap();
    reservation.commit(issuance(1), time(1)).unwrap();
    let Lookup::Live(record) = ledger.lookup(1, time(11)).unwrap() else {
        panic!("live at exact TTL")
    };
    assert_eq!(record.payload().session_id, "session-one");
    assert_eq!(record.mime(), "application/octet-stream");
    assert_eq!(record.channels(), &["bin-trailer".to_string()]);
    assert_eq!(record.ip_pseudonym(), Some("keyed-pseudonym"));
    assert_eq!(record.issued_at(), time(1));
    assert!(matches!(
        ledger.lookup(1, time(12)).unwrap(),
        Lookup::Expired
    ));
    assert!(matches!(
        ledger.lookup(1, time(12)).unwrap(),
        Lookup::Unknown
    ));
    assert_eq!(ledger.reserve(time(12)).unwrap().tag_id(), 2);
}

#[test]
fn gc_expires_pending_and_committed_entries_without_evicting_live_records() {
    let ledger = ledger(2);
    let pending = ledger.reserve(time(0)).unwrap();
    let committed = ledger.reserve(time(0)).unwrap();
    committed.commit(issuance(2), time(0)).unwrap();
    assert_eq!(ledger.gc(time(2)).unwrap(), 0);
    assert_eq!(ledger.gc(time(3)).unwrap(), 1);
    assert_eq!(
        pending.commit(issuance(1), time(3)),
        Err(Error::ReservationExpired)
    );
    let replacement = ledger.reserve(time(3)).unwrap();
    assert!(matches!(ledger.reserve(time(3)), Err(Error::Capacity)));
    assert!(matches!(
        ledger.lookup(2, time(3)).unwrap(),
        Lookup::Live(_)
    ));
    drop(replacement);
    assert_eq!(ledger.gc(time(11)).unwrap(), 1);
}

#[test]
fn reservation_exact_ttl_may_commit_but_late_or_mismatched_commit_cannot_issue() {
    let ledger = ledger(1);
    ledger
        .reserve(time(0))
        .unwrap()
        .commit(issuance(1), time(2))
        .unwrap();
    assert!(matches!(
        ledger.lookup(1, time(2)).unwrap(),
        Lookup::Live(_)
    ));
    let other = Ledger::new(configuration(1), time(0)).unwrap();
    assert_eq!(
        other.reserve(time(0)).unwrap().commit(issuance(1), time(3)),
        Err(Error::ReservationExpired)
    );
    let reserved = other.reserve(time(3)).unwrap();
    assert_eq!(
        reserved.commit(issuance(1), time(3)),
        Err(Error::ReservationMismatch)
    );
    assert!(matches!(other.lookup(2, time(3)).unwrap(), Lookup::Unknown));
    assert_eq!(other.reserve(time(3)).unwrap().tag_id(), 3);
}

#[test]
fn rejected_metadata_and_payload_cancel_the_reservation() {
    let ledger = ledger(1);
    let mut invalid = issuance(1);
    invalid.channels = vec!["bin-trailer".into(), "bin-trailer".into()];
    assert_eq!(
        ledger.reserve(time(0)).unwrap().commit(invalid, time(0)),
        Err(Error::InvalidMetadata)
    );
    let mut invalid = issuance(2);
    invalid.payload.session_id = "a\0b".into();
    assert_eq!(
        ledger.reserve(time(0)).unwrap().commit(invalid, time(0)),
        Err(Error::InvalidPayload)
    );
    let mut invalid = issuance(3);
    invalid.mime = "m".repeat(MAX_MIME_BYTES + 1);
    assert_eq!(
        ledger.reserve(time(0)).unwrap().commit(invalid, time(0)),
        Err(Error::InvalidMetadata)
    );
    let mut invalid = issuance(4);
    invalid.channels = vec!["c".repeat(MAX_CHANNEL_BYTES + 1)];
    assert_eq!(
        ledger.reserve(time(0)).unwrap().commit(invalid, time(0)),
        Err(Error::InvalidMetadata)
    );
    let mut invalid = issuance(5);
    invalid.ip_pseudonym = Some("p".repeat(MAX_PSEUDONYM_BYTES + 1));
    assert_eq!(
        ledger.reserve(time(0)).unwrap().commit(invalid, time(0)),
        Err(Error::InvalidMetadata)
    );
    assert_eq!(ledger.reserve(time(0)).unwrap().tag_id(), 6);
}

#[test]
fn clock_regression_fails_closed_and_failed_commit_does_not_leave_capacity() {
    let ledger = ledger(1);
    let reservation = ledger.reserve(time(5)).unwrap();
    assert!(matches!(
        ledger.lookup(1, time(6)).unwrap(),
        Lookup::Unknown
    ));
    assert_eq!(
        reservation.commit(issuance(1), time(5)),
        Err(Error::ClockRegression)
    );
    assert!(matches!(
        ledger.reserve(time(5)),
        Err(Error::ClockRegression)
    ));
    assert!(matches!(
        ledger.lookup(1, time(5)),
        Err(Error::ClockRegression)
    ));
    assert_eq!(ledger.gc(time(5)), Err(Error::ClockRegression));
    assert_eq!(ledger.reserve(time(6)).unwrap().tag_id(), 2);
}

#[test]
fn final_u32_id_is_usable_once_and_never_wraps_to_zero() {
    let ledger = ledger(1);
    ledger.inner.lock().unwrap().next_id = u64::from(u32::MAX);
    let reservation = ledger.reserve(time(0)).unwrap();
    assert_eq!(reservation.tag_id(), u32::MAX);
    reservation.cancel().unwrap();
    assert!(matches!(ledger.reserve(time(0)), Err(Error::IdExhausted)));
    assert!(matches!(ledger.reserve(time(1)), Err(Error::IdExhausted)));
}

#[test]
fn concurrent_clones_share_capacity_and_unique_monotonic_ids() {
    let ledger = ledger(32);
    let workers: Vec<_> = (0..32)
        .map(|_| {
            let shared = ledger.clone();
            std::thread::spawn(move || shared.reserve(time(0)).unwrap())
        })
        .collect();
    let reservations: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let mut ids: Vec<_> = reservations.iter().map(Reservation::tag_id).collect();
    ids.sort_unstable();
    assert_eq!(ids, (1..=32).collect::<Vec<_>>());
    assert!(matches!(ledger.reserve(time(0)), Err(Error::Capacity)));
    drop(reservations);
    assert_eq!(ledger.reserve(time(0)).unwrap().tag_id(), 33);
}

#[test]
fn unwind_cancels_pending_and_poisoned_state_is_never_used_for_lookup() {
    let ledger = ledger(1);
    let result = std::panic::catch_unwind(|| {
        let _reservation = ledger.reserve(time(0)).unwrap();
        panic!("simulated encoder failure");
    });
    assert!(result.is_err());
    let reservation = ledger.reserve(time(0)).unwrap();
    assert_eq!(reservation.tag_id(), 2);
    let result = std::panic::catch_unwind(|| {
        let _guard = ledger.inner.lock().unwrap();
        panic!("simulated state failure");
    });
    assert!(result.is_err());
    drop(reservation);
    assert!(matches!(ledger.lookup(2, time(0)), Err(Error::Poisoned)));
    assert!(matches!(ledger.reserve(time(0)), Err(Error::Poisoned)));
    assert_eq!(ledger.gc(time(0)), Err(Error::Poisoned));
}
