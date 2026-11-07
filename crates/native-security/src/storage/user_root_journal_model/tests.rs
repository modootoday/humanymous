use super::*;

fn caps() -> ModelCaps {
    ModelCaps {
        attempts: 8,
        namespaces: 8,
        tombstones: 8,
        bytes: 800,
        pending_operations: 8,
    }
}
fn cost() -> ModelCost {
    ModelCost {
        bytes: 100,
        pending_operations: 1,
    }
}
fn request(id: u64, namespace: u64) -> ModelRequest {
    ModelRequest {
        attempt: ModelAttempt { id, generation: 1 },
        owner: 10,
        anchor: ModelAnchor {
            volume: 11,
            file: 12,
        },
        selector: 13,
        policy: 14,
        namespace,
        nonce: u128::from(id) + 100,
        epoch: 1,
        original: ModelOriginalPair {
            start_tick: 1,
            run_until: 50,
            cleanup_until: 80,
        },
    }
}
fn ledger() -> ModelLedger {
    ModelLedger::new_fixture(caps(), 1, 1).unwrap()
}
fn relations(ledger: &ModelLedger) -> Vec<ModelNamespaceRelation> {
    ledger
        .namespaces
        .iter()
        .map(|slot| ModelNamespaceRelation {
            existing: slot.attempt,
            relation: ModelRelation::Disjoint,
        })
        .collect()
}
fn add(ledger: &mut ModelLedger, request: ModelRequest, now: u64) {
    let rows = relations(ledger);
    assert_eq!(
        ledger.propose_reservation(request, &rows, cost(), now),
        Ok(ModelReservationOutcome::ModelReserved)
    );
}
fn unknown(ledger: &mut ModelLedger, request: ModelRequest) {
    add(ledger, request, 2);
    ledger.mark_create_pending(request.attempt, 3).unwrap();
    ledger.mark_unknown(request.attempt, 4).unwrap();
}
fn terminal(request: ModelRequest, epoch: u64) -> ModelTerminalObservation {
    ModelTerminalObservation {
        attempt: request.attempt,
        owner: request.owner,
        anchor: request.anchor,
        namespace: request.namespace,
        epoch,
    }
}

#[test]
fn crashed_unopened_attempt_blocks_same_namespace_new_attempt() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(request(2, 1), &relations(&ledger), cost(), 5),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
    assert_eq!(ledger.records[0].request, a);
    assert_eq!(ledger.records[0].phase, ModelPhase::Unknown);
}

#[test]
fn alias_or_unknown_namespace_relation_cannot_bypass_reservation() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let before = ledger.snapshot();
    for relation in [ModelRelation::Same, ModelRelation::Unknown] {
        let rows = [ModelNamespaceRelation {
            existing: a.attempt,
            relation,
        }];
        assert_eq!(
            ledger.propose_reservation(request(2, 2), &rows, cost(), 5),
            Err(ModelError::Conflict)
        );
        assert_eq!(ledger.snapshot(), before);
    }
    for rows in [vec![], vec![relations(&ledger)[0]; 2]] {
        assert_eq!(
            ledger.propose_reservation(request(2, 2), &rows, cost(), 5),
            Err(ModelError::Conflict)
        );
        assert_eq!(ledger.snapshot(), before);
    }
    // Equal tokens cannot be declared disjoint by changing the selector/anchor.
    let mut alias = request(2, 1);
    alias.selector = 99;
    alias.anchor.file = 99;
    assert_eq!(
        ledger.propose_reservation(alias, &relations(&ledger), cost(), 5),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn replay_requires_identical_owner_anchor_selector_nonce_epoch_and_original_pair() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(a, &[], cost(), 5),
        Ok(ModelReservationOutcome::ModelExistingSameRecord)
    );
    assert_eq!(ledger.snapshot(), before);
    let mut changes = [a; 10];
    changes[0].owner += 1;
    changes[1].anchor.volume += 1;
    changes[2].anchor.file += 1;
    changes[3].selector += 1;
    changes[4].policy += 1;
    changes[5].nonce += 1;
    changes[6].epoch += 1;
    changes[7].attempt.generation += 1;
    changes[8].original.run_until += 1;
    changes[9].original.cleanup_until += 1;
    for changed in changes {
        assert!(
            ledger
                .propose_reservation(changed, &relations(&ledger), cost(), 5)
                .is_err()
        );
        assert_eq!(ledger.snapshot(), before);
    }
    let mut reused = a;
    reused.attempt.generation += 1;
    reused.namespace += 1;
    assert_eq!(
        ledger.propose_reservation(reused, &relations(&ledger), cost(), 5),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn later_not_created_never_resolves_another_attempt_unknown() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.observe_not_created(request(2, 1).attempt, 1, 5),
        Err(ModelError::UnknownAttempt)
    );
    assert_eq!(
        ledger.observe_not_created(a.attempt, 1, 5),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
    let b = request(2, 2);
    add(&mut ledger, b, 5);
    ledger.observe_not_created(b.attempt, 1, 6).unwrap();
    assert_eq!(ledger.records[0].request, a);
    assert_eq!(ledger.records[0].phase, ModelPhase::Unknown);
    assert_eq!(
        ledger.namespaces,
        vec![ModelNamespace {
            attempt: a.attempt,
            token: a.namespace
        }]
    );
}

#[test]
fn stale_ack_or_terminal_epoch_cannot_release_current_reservation() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    ledger
        .propose_handoff(2, ModelHistoryState::KnownFixtureHistory, 5)
        .unwrap();
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(a, &[], cost(), 6),
        Err(ModelError::StaleEpoch)
    );
    assert_eq!(
        ledger.propose_terminal(terminal(a, 1), 6),
        Err(ModelError::StaleEpoch)
    );
    let mut wrong = terminal(a, 2);
    wrong.owner += 1;
    assert_eq!(ledger.propose_terminal(wrong, 6), Err(ModelError::Conflict));
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn lost_ack_and_partial_indexes_remain_unknown_without_admission() {
    let mut ledger = ledger();
    let a = request(1, 1);
    add(&mut ledger, a, 2);
    // Lost delivery creates no admission retry or second create transition.
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(request(2, 1), &relations(&ledger), cost(), 3),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
    ledger.mark_unknown(a.attempt, 3).unwrap();
    for missing_attempt_index in [false, true] {
        // Synthetic partial durable facts: deliberately not a valid ledger.
        let mut partial = ledger.clone();
        if missing_attempt_index {
            partial.records.clear();
        } else {
            partial.namespaces.clear();
        }
        let before = partial.snapshot();
        assert_eq!(
            partial.propose_reservation(request(2, 2), &[], cost(), 4),
            Err(ModelError::UnknownHistory)
        );
        assert_eq!(partial.snapshot(), before);
    }
}

#[test]
fn crash_handoff_unknown_history_preserves_reservations_and_blocks_admission() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    ledger
        .propose_handoff(2, ModelHistoryState::UnknownFixtureHistory, 5)
        .unwrap();
    assert_eq!(ledger.records[0].request, a);
    assert_eq!(ledger.namespaces.len(), 1);
    let before = ledger.snapshot();
    let mut b = request(2, 2);
    b.epoch = 2;
    assert_eq!(
        ledger.propose_reservation(b, &relations(&ledger), cost(), 6),
        Err(ModelError::UnknownHistory)
    );
    assert_eq!(
        ledger.propose_handoff(3, ModelHistoryState::KnownFixtureHistory, 6),
        Err(ModelError::UnknownHistory)
    );
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn deadlines_expiry_rollback_and_handoff_never_reset_original_pair() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(a, &[], cost(), 50),
        Err(ModelError::Expired)
    );
    assert_eq!(ledger.snapshot(), before);
    ledger.mark_unknown(a.attempt, 80).unwrap();
    let expired = ledger.snapshot();
    assert_eq!(
        ledger.propose_terminal(terminal(a, 1), 80),
        Err(ModelError::Expired)
    );
    assert_eq!(ledger.mark_unknown(a.attempt, 79), Err(ModelError::Invalid));
    assert_eq!(ledger.snapshot(), expired);
    ledger
        .propose_handoff(2, ModelHistoryState::KnownFixtureHistory, 81)
        .unwrap();
    assert_eq!(ledger.records[0].request.original, a.original);
    let mut rebased = a;
    rebased.epoch = 2;
    rebased.original = ModelOriginalPair {
        start_tick: 81,
        run_until: 90,
        cleanup_until: 100,
    };
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(rebased, &[], cost(), 82),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn distinct_namespace_attempt_keeps_independent_original_pair() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    let mut b = request(2, 2);
    b.original = ModelOriginalPair {
        start_tick: 5,
        run_until: 60,
        cleanup_until: 90,
    };
    add(&mut ledger, b, 5);
    assert_eq!(ledger.records[0].request.original, a.original);
    assert_eq!(ledger.records[1].request.original, b.original);
    assert_eq!(ledger.namespaces.len(), 2);
    ledger.check_indexes().unwrap();
}

#[test]
fn every_capacity_saturates_before_admission_without_unknown_or_tombstone_eviction() {
    for dimension in 0..5 {
        let mut cap = caps();
        match dimension {
            0 => cap.attempts = 1,
            1 => cap.namespaces = 1,
            2 => cap.tombstones = 1,
            3 => cap.bytes = 100,
            _ => cap.pending_operations = 1,
        }
        let mut ledger = ModelLedger::new_fixture(cap, 1, 1).unwrap();
        let a = request(1, 1);
        unknown(&mut ledger, a);
        let before = ledger.snapshot();
        assert_eq!(
            ledger.propose_reservation(request(2, 2), &relations(&ledger), cost(), 5),
            Err(ModelError::Capacity)
        );
        assert_eq!(ledger.snapshot(), before);
        assert_eq!(
            ledger.propose_reservation(a, &[], cost(), 5),
            Ok(ModelReservationOutcome::ModelExistingSameRecord)
        );
        assert_eq!(ledger.snapshot(), before);
    }
    let mut cap = caps();
    cap.tombstones = 1;
    let mut ledger = ModelLedger::new_fixture(cap, 1, 1).unwrap();
    let a = request(1, 1);
    add(&mut ledger, a, 2);
    ledger.propose_terminal(terminal(a, 1), 3).unwrap();
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(request(2, 2), &[], cost(), 4),
        Err(ModelError::Capacity)
    );
    assert_eq!(ledger.snapshot(), before);
    assert_eq!(ledger.tombstones, vec![a]);
}

#[test]
fn checked_byte_counter_and_generation_overflow_preserve_exact_prior_snapshot() {
    for byte_overflow in [false, true] {
        let mut ledger = ledger();
        // Bounded synthetic counters, not huge vectors or allocations.
        if byte_overflow {
            ledger.used_bytes = u64::MAX;
        } else {
            ledger.pending_operations = usize::MAX;
        }
        let before = ledger.snapshot();
        assert_eq!(
            ledger.propose_reservation(request(1, 1), &[], cost(), 2),
            Err(ModelError::Overflow)
        );
        assert_eq!(ledger.snapshot(), before);
    }
    let mut ledger = ledger();
    ledger.epoch = u64::MAX;
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_handoff(1, ModelHistoryState::KnownFixtureHistory, 2),
        Err(ModelError::Overflow)
    );
    assert_eq!(ledger.snapshot(), before);
}

#[test]
fn terminal_release_retains_tombstone_and_denies_stale_replay() {
    let mut ledger = ledger();
    let a = request(1, 1);
    unknown(&mut ledger, a);
    ledger.propose_terminal(terminal(a, 1), 5).unwrap();
    assert!(ledger.namespaces.is_empty());
    assert_eq!(ledger.tombstones, vec![a]);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_reservation(a, &[], cost(), 6),
        Err(ModelError::Conflict)
    );
    assert_eq!(
        ledger.propose_terminal(terminal(a, 1), 6),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
    let b = request(2, 1);
    add(&mut ledger, b, 6);
    let before = ledger.snapshot();
    assert_eq!(
        ledger.propose_terminal(terminal(a, 1), 7),
        Err(ModelError::Conflict)
    );
    assert_eq!(ledger.snapshot(), before);
    assert_eq!(
        ledger.namespaces,
        vec![ModelNamespace {
            attempt: b.attempt,
            token: b.namespace
        }]
    );
    ledger.check_indexes().unwrap();
}
