use super::{state::Owner, *};
use std::time::{Duration, Instant};

const PHASES: [StartupStage; 6] = [
    StartupStage::NativePrepared,
    StartupStage::WriterPrepared,
    StartupStage::OfferPending,
    StartupStage::ReceiptBound,
    StartupStage::ReadyEnqueuePending,
    StartupStage::Active,
];

#[test]
fn native_producer_is_unsupported_even_in_tests() {
    assert!(matches!(
        native_preparation(),
        Err(NativePreparationUnavailable::Unsupported)
    ));
}

#[test]
fn private_phase_model_requires_writer_offer_receipt_and_complete_ready_order() {
    let owner = Owner::model();
    for pair in PHASES.windows(2) {
        assert_eq!(owner.stage(), pair[0]);
        assert!(owner.advance(pair[0], pair[1]));
        assert!(!owner.advance(pair[0], pair[1]));
    }
    assert_eq!(owner.stage(), StartupStage::Active);
}

#[test]
fn no_ack_before_writer_or_ready_from_claimed_phase() {
    let owner = Owner::model();
    assert!(!owner.advance(StartupStage::NativePrepared, StartupStage::ReceiptBound));
    assert!(!owner.advance(StartupStage::NativePrepared, StartupStage::Active));
    assert!(!owner.advance(
        StartupStage::ReceiptBound,
        StartupStage::ReadyEnqueuePending
    ));
    assert_eq!(owner.stage(), StartupStage::NativePrepared);
}

#[test]
fn cancellation_in_every_phase_is_irreversible_and_preserves_first_cause() {
    let first = Instant::now();
    for index in 0..PHASES.len() {
        let owner = Owner::model();
        for pair in PHASES[..=index].windows(2) {
            assert!(owner.advance(pair[0], pair[1]));
        }
        let gate = owner.gate();
        owner.lose_at(first, LossCause::WitnessLost);
        owner.lose_at(first + Duration::from_secs(1), LossCause::ObserverLost);
        assert!(!owner.advance(StartupStage::TerminalRevoked, StartupStage::Active));
        drop(owner);
        let state = gate.snapshot().unwrap();
        assert_eq!(state.stage, StartupStage::TerminalRevoked);
        assert_eq!(state.revocation.unwrap().cause, LossCause::WitnessLost);
        assert_eq!(state.revocation.unwrap().at, first);
    }
}

#[test]
fn owner_drop_revokes_restricted_clones_without_claiming_cleanup() {
    let owner = Owner::model();
    let gate = owner.gate();
    let other = gate.clone();
    drop(owner);
    for view in [gate, other] {
        let state = view.snapshot().unwrap();
        assert_eq!(state.stage, StartupStage::TerminalRevoked);
        assert_eq!(state.revocation.unwrap().cause, LossCause::OwnerDropped);
    }
}

#[test]
fn first_loss_close_deadline_never_extends_original_budget() {
    let at = Instant::now();
    let loss = Revocation {
        at,
        cause: LossCause::EpochExpired,
    };
    let original = at + Duration::from_secs(3);
    assert_eq!(loss.close_deadline(original, Duration::ZERO), Some(at));
    assert_eq!(
        loss.close_deadline(original, Duration::from_secs(2)),
        Some(at + Duration::from_secs(2))
    );
    assert_eq!(
        loss.close_deadline(original, Duration::from_secs(5)),
        Some(original)
    );
    assert_eq!(loss.close_deadline(original, Duration::MAX), None);
}

#[test]
fn ready_transition_racing_loss_cannot_leave_a_live_epoch() {
    let owner = Owner::model();
    for pair in PHASES[..5].windows(2) {
        assert!(owner.advance(pair[0], pair[1]));
    }
    std::thread::scope(|scope| {
        scope.spawn(|| {
            owner.advance(StartupStage::ReadyEnqueuePending, StartupStage::Active);
        });
        scope.spawn(|| owner.revoke(LossCause::ObserverLost));
    });
    assert_eq!(owner.stage(), StartupStage::TerminalRevoked);
    assert!(!owner.advance(StartupStage::TerminalRevoked, StartupStage::Active));
}
