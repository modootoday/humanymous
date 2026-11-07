//! Model evidence only; no native role, real socket or ServiceHost qualification.
use super::*;

const EPOCH: [u8; 16] = [7; 16];
fn ready() -> Gate {
    let gate = Gate::closed(EPOCH);
    gate.activate_for_test().unwrap();
    gate
}
#[test]
fn starting_and_foreign_instances_cannot_admit_or_revive() {
    let gate = Gate::closed(EPOCH);
    assert!(!gate.live());
    assert!(matches!(gate.admit(EPOCH), Err(Refusal::Closed)));
    gate.revoke(EPOCH, Cause::ObserverLost).unwrap();
    assert!(gate.activate_for_test().is_err());
    let first = ready();
    let second = ready();
    let mut work = first.admit(EPOCH).unwrap();
    assert!(matches!(first.admit([8; 16]), Err(Refusal::ForeignEpoch)));
    assert!(matches!(
        second.connect(&mut work, || panic!("foreign operation")),
        Err(Refusal::ForeignEpoch)
    ));
}
#[test]
fn first_loss_closes_readiness_and_admission_without_extending_its_clock() {
    let gate = ready();
    let loss = gate.revoke(EPOCH, Cause::ObserverLost).unwrap();
    assert!(!gate.live());
    assert!(gate.admit(EPOCH).is_err());
    gate.manager_stop().unwrap();
    assert_eq!(gate.revoke(EPOCH, Cause::EpochExpired).unwrap(), loss);
    let original = loss.at + std::time::Duration::from_secs(2);
    assert_eq!(
        loss.deadline(original, std::time::Duration::from_secs(3))
            .unwrap(),
        original
    );
    assert_eq!(
        loss.deadline(original, std::time::Duration::from_secs(1))
            .unwrap(),
        loss.at + std::time::Duration::from_secs(1)
    );
    assert!(loss.deadline(original, std::time::Duration::MAX).is_err());
    assert!(gate.activate_for_test().is_err());
}
#[test]
fn revoked_work_cannot_initiate_connect_or_accept_first_bytes() {
    let gate = ready();
    let mut unconnected = gate.admit(EPOCH).unwrap();
    let mut connected = gate.admit(EPOCH).unwrap();
    gate.connect(&mut connected, || Ok(())).unwrap();
    gate.revoke(EPOCH, Cause::ObserverLost).unwrap();
    assert!(
        gate.connect(&mut unconnected, || panic!("connect after loss"))
            .is_err()
    );
    assert!(
        gate.write(&mut connected, b"body", |_| panic!("write after loss"))
            .is_err()
    );
    assert_eq!(connected.accepted, 0);
}
#[test]
fn would_block_has_no_bytes_and_reacquisition_rechecks_revocation() {
    let gate = ready();
    let mut work = gate.admit(EPOCH).unwrap();
    gate.connect(&mut work, || Ok(())).unwrap();
    let result = gate.write(
        &mut work,
        b"body",
        |_| Err(io::ErrorKind::WouldBlock.into()),
    );
    assert!(matches!(result, Err(Refusal::Io(error)) if error.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(work.accepted, 0);
    gate.revoke(EPOCH, Cause::EpochExpired).unwrap();
    assert!(
        gate.write(&mut work, b"body", |_| panic!("stale writable permit"))
            .is_err()
    );
}
#[test]
fn prior_partial_submission_is_retained_and_manager_stop_is_not_observer_loss() {
    let gate = ready();
    let mut uncertain = gate.admit(EPOCH).unwrap();
    gate.connect(&mut uncertain, || Ok(())).unwrap();
    assert!(gate.write(&mut uncertain, b"x", |_| Ok(2)).is_err());
    assert!(uncertain.unknown);
    assert!(
        gate.write(&mut uncertain, b"x", |_| panic!("uncertain work reused"))
            .is_err()
    );
    let mut work = gate.admit(EPOCH).unwrap();
    gate.connect(&mut work, || Ok(())).unwrap();
    assert_eq!(gate.write(&mut work, b"body", |_| Ok(2)).unwrap(), 2);
    gate.manager_stop().unwrap();
    assert!(!gate.live());
    assert!(gate.0.lock().unwrap().loss.is_none());
    gate.revoke(EPOCH, Cause::ObserverLost).unwrap();
    assert!(
        gate.write(&mut work, b"dy", |_| panic!("continued after loss"))
            .is_err()
    );
    assert_eq!(work.accepted, 2);
}

#[test]
fn concurrent_first_writes_linearize_before_first_loss_or_are_refused() {
    let gate = ready();
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let effects = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..8 {
            let gate = gate.clone();
            let barrier = barrier.clone();
            let effects = effects.clone();
            workers.push(scope.spawn(move || {
                let mut work = gate.admit(EPOCH).unwrap();
                gate.connect(&mut work, || Ok(())).unwrap();
                barrier.wait();
                let result = gate.write(&mut work, b"x", |_| {
                    effects.lock().unwrap().push(Instant::now());
                    Ok(1)
                });
                assert!(matches!(result, Ok(1) | Err(Refusal::Closed)));
            }));
        }
        barrier.wait();
        let loss = gate.revoke(EPOCH, Cause::ObserverLost).unwrap();
        for worker in workers {
            worker.join().unwrap();
        }
        assert!(
            effects
                .lock()
                .unwrap()
                .iter()
                .all(|accepted| *accepted <= loss.at)
        );
    });
    assert!(!gate.live());
}

#[test]
fn poisoned_gate_never_recovers_admission() {
    let gate = ready();
    let owner = gate.clone();
    assert!(
        std::thread::spawn(move || {
            let _state = owner.0.lock().unwrap();
            panic!("injected owner panic");
        })
        .join()
        .is_err()
    );
    assert!(!gate.live());
    assert!(matches!(gate.admit(EPOCH), Err(Refusal::Poisoned)));
    assert!(matches!(
        gate.revoke(EPOCH, Cause::ObserverLost),
        Err(Refusal::Poisoned)
    ));
}
