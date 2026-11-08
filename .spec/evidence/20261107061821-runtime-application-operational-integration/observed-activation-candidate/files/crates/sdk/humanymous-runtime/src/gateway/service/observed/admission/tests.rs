//! Actual unavailable-provider path; never constructs NativeProof or Ready.
use super::*;

fn attempt() -> RequiredObserverStartup {
    ServiceHost::begin_required_observer(crate::gateway::tests::config())
}

#[test]
fn missing_original_provider_refuses_before_unseal_and_retains_same_input() {
    let mut a = attempt();
    let address = std::ptr::from_ref(a.owner.retained.resources());
    let passphrase = a
        .owner
        .retained
        .resources()
        .audit
        .keys
        .passphrase
        .as_str()
        .to_owned();
    assert_eq!(
        a.prepare_native_once(),
        Err(RequiredObserverRefusal::NativeUnavailable)
    );
    assert_eq!(a.stage(), RequiredObserverStage::Refused);
    assert!(matches!(
        a.native_unavailable,
        Some(NativePreparationUnavailable::Unsupported)
    ));
    assert_eq!(address, std::ptr::from_ref(a.owner.retained.resources()));
    assert_eq!(
        a.owner.retained.resources().audit.keys.passphrase.as_str(),
        passphrase
    );
    assert!(a.owner.retained.guard_mut().0.is_none());
}

#[test]
fn replay_of_native_attempt_cannot_reset_failure_or_replace_resources() {
    let mut a = attempt();
    let address = std::ptr::from_ref(a.owner.retained.resources());
    assert_eq!(
        a.prepare_native_once(),
        Err(RequiredObserverRefusal::NativeUnavailable)
    );
    for _ in 0..3 {
        assert_eq!(
            a.prepare_native_once(),
            Err(RequiredObserverRefusal::AlreadyConsumed)
        );
        assert_eq!(
            a.first_refusal(),
            Some(RequiredObserverRefusal::NativeUnavailable)
        );
        assert_eq!(address, std::ptr::from_ref(a.owner.retained.resources()));
    }
}

#[test]
fn offer_without_native_authority_burns_attempt_without_secret_generation() {
    let mut a = attempt();
    let address = std::ptr::from_ref(a.owner.retained.resources());
    assert_eq!(
        a.offer_ack_once(),
        Err(RequiredObserverRefusal::PreparationRequired)
    );
    assert_eq!(
        a.prepare_native_once(),
        Err(RequiredObserverRefusal::AlreadyConsumed)
    );
    assert!(a.native_unavailable.is_none());
    assert!(a.owner.retained.guard_mut().0.is_none());
    assert_eq!(address, std::ptr::from_ref(a.owner.retained.resources()));
}

#[test]
fn repeated_offer_requests_never_create_receipt_or_rearm_epoch() {
    let mut a = attempt();
    assert_eq!(
        a.prepare_native_once(),
        Err(RequiredObserverRefusal::NativeUnavailable)
    );
    for _ in 0..3 {
        assert_eq!(
            a.offer_ack_once(),
            Err(RequiredObserverRefusal::AlreadyConsumed)
        );
    }
    assert_eq!(
        a.first_refusal(),
        Some(RequiredObserverRefusal::NativeUnavailable)
    );
    assert!(a.owner.retained.guard_mut().0.is_none());
}

#[test]
fn cancellation_before_native_attempt_keeps_resources_in_failure_owner() {
    let a = attempt();
    let address = std::ptr::from_ref(a.owner.retained.resources());
    let mut failure = a.cancel();
    failure.revoke();
    failure.revoke();
    assert_eq!(failure.reason(), RequiredObserverRefusal::Cancelled);
    assert!(failure.native_unavailable().is_none());
    assert!(failure.authority_failure().is_none());
    assert_eq!(address, std::ptr::from_ref(failure.retained.resources()));
}

#[test]
fn cancellation_after_native_refusal_conserves_first_error_and_allocation() {
    let mut a = attempt();
    let address = std::ptr::from_ref(a.owner.retained.resources());
    assert_eq!(
        a.prepare_native_once(),
        Err(RequiredObserverRefusal::NativeUnavailable)
    );
    let mut failure = a.cancel();
    failure.revoke();
    assert_eq!(failure.reason(), RequiredObserverRefusal::NativeUnavailable);
    assert!(matches!(
        failure.native_unavailable(),
        Some(NativePreparationUnavailable::Unsupported)
    ));
    assert!(failure.authority_failure().is_none());
    assert_eq!(address, std::ptr::from_ref(failure.retained.resources()));
}
