//! External namespace/type compatibility, not actual Go host execution.
#![cfg(feature = "adapter-go")]
use humanymous::adapters::go::{self, Configuration, Deployment, Host, Rejection};

const _: fn(Host, Configuration) -> Result<Deployment, Rejection> = Deployment::new;
const _: fn(Deployment) -> Configuration = Deployment::into_configuration;
const _: fn(&go::scenario::CaseObservation) -> Result<go::Outcome, go::scenario::EvaluationError> =
    go::scenario::evaluate;

#[test]
fn reexport_preserves_leaf_types_and_bounded_profile() {
    let _: fn(
        Host,
        humanymous_adapter_go::Configuration,
    ) -> Result<Deployment, humanymous_adapter_go::Rejection> = Deployment::new;
    assert_eq!(
        go::QUALIFICATION_PROFILE,
        "humanymous.go-gate/bounded-http-8-case/2"
    );
    assert_eq!(go::CASES.len(), 8);
    assert_eq!(go::CASES, humanymous_adapter_go::CASES);
    assert!(!Host::NetHttp.pooled_context());
    assert!(Host::FastHttp.borrows_entity_slices());
    let _: Option<go::ExpectedTuple<'_>> = None;
    let _: Option<go::Receipt> = None;
    let _ = go::probe::binary_dump;
}

#[test]
fn public_evaluator_preserves_required_observation_rejection() {
    let mut observation = go::scenario::CaseObservation {
        case: go::Case::DependencyDown,
        http_status: Some(422),
        decoded_entity: br#"{"error":"required_capability"}"#.to_vec(),
        header_pairs: Vec::new(),
        origin_contacts: Some(0),
        actual_host_verified: true,
        pre_handoff_cancelled: false,
        observed_past_deadline: false,
        write_half_closed: false,
        backend_and_witness_kernel_denied: false,
    };
    // Synthetic evaluator inputs prove alias behavior, not actual host facts.
    let outcome = go::scenario::evaluate(&observation).unwrap();
    assert_eq!(outcome.case, go::Case::DependencyDown);
    assert_eq!(outcome.failures, 0);
    observation.origin_contacts = None;
    assert_eq!(
        go::scenario::evaluate(&observation).unwrap_err().code,
        "origin_counter_missing"
    );
    observation.origin_contacts = Some(1);
    assert_eq!(
        go::scenario::evaluate(&observation).unwrap_err().code,
        "origin_contacts_mismatch"
    );
}
