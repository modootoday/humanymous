use super::*;
use crate::CASES;

fn observation(case: Case) -> CaseObservation {
    let mut o = CaseObservation {
        case,
        http_status: None,
        decoded_entity: vec![],
        header_pairs: vec![],
        origin_contacts: Some(0),
        actual_host_verified: true,
        pre_handoff_cancelled: false,
        observed_past_deadline: false,
        write_half_closed: false,
        backend_and_witness_kernel_denied: false,
    };
    match case {
        Case::BinaryAllow => {
            o.origin_contacts = Some(1);
            o.http_status = Some(200);
            o.decoded_entity =
                format!("POST {BINARY_TARGET} HTTP/1.1\r\nContent-Length: 7\r\n\r\n").into_bytes();
            o.decoded_entity.extend_from_slice(BINARY_ENTITY);
        }
        Case::ResponseSemantics => {
            o.origin_contacts = Some(1);
            o.http_status = Some(200);
            o.header_pairs = vec![
                ("Set-Cookie".into(), "a=1".into()),
                ("set-cookie".into(), "b=2".into()),
            ];
        }
        Case::DependencyDown | Case::ForgedProvenance => {
            o.http_status = Some(422);
            o.decoded_entity = br#"{"error":"required_capability"}"#.to_vec();
        }
        Case::Cancellation => {
            o.pre_handoff_cancelled = true;
            o.observed_past_deadline = true;
        }
        Case::IncompleteBody => {
            o.write_half_closed = true;
            o.http_status = Some(400);
            o.decoded_entity = br#"{"error":"truncated_body"}"#.to_vec();
        }
        Case::OriginBypass => {
            o.backend_and_witness_kernel_denied = true;
        }
        Case::Upgrade => {
            o.http_status = Some(400);
            o.decoded_entity = br#"{"error":"http_framing"}"#.to_vec();
        }
    }
    o
}

#[test]
fn every_required_case_needs_matching_measured_observations() {
    for case in CASES {
        let outcome = evaluate(&observation(case)).unwrap();
        assert_eq!(outcome.case, case);
        assert!(outcome.assertions >= 4);
        assert_eq!(outcome.failures, 0);
        assert!(outcome.actual_host);
    }
}
#[test]
fn absent_counter_or_unverified_fixture_cannot_become_zero_contacts() {
    for case in CASES {
        let mut o = observation(case);
        o.origin_contacts = None;
        assert_eq!(evaluate(&o).unwrap_err().code, "origin_counter_missing");
        o.origin_contacts = Some(0);
        o.actual_host_verified = false;
        assert_eq!(evaluate(&o).unwrap_err().code, "actual_host_unverified");
    }
    let mut o = observation(Case::DependencyDown);
    o.origin_contacts = Some(1);
    assert_eq!(evaluate(&o).unwrap_err().code, "origin_contacts_mismatch");
}
#[test]
fn unrelated_error_and_missing_phase_or_kernel_proofs_reject() {
    let mut o = observation(Case::DependencyDown);
    o.decoded_entity = br#"{"error":"unrelated"}"#.to_vec();
    assert_eq!(evaluate(&o).unwrap_err().code, "capability_reason_mismatch");
    let mut o = observation(Case::Cancellation);
    o.observed_past_deadline = false;
    assert_eq!(
        evaluate(&o).unwrap_err().code,
        "deadline_observation_missing"
    );
    let mut o = observation(Case::IncompleteBody);
    o.write_half_closed = false;
    assert_eq!(evaluate(&o).unwrap_err().code, "half_close_unobserved");
    let mut o = observation(Case::OriginBypass);
    o.backend_and_witness_kernel_denied = false;
    assert_eq!(evaluate(&o).unwrap_err().code, "kernel_denial_unobserved");
}
#[test]
fn real_host_marker_and_contact_count_do_not_replace_response_proofs() {
    let mut o = observation(Case::BinaryAllow);
    *o.decoded_entity.last_mut().unwrap() ^= 1;
    assert_eq!(evaluate(&o).unwrap_err().code, "binary_dump_mismatch");
    let mut o = observation(Case::ResponseSemantics);
    o.header_pairs = vec![("Set-Cookie".into(), "a=1, b=2".into())];
    assert_eq!(evaluate(&o).unwrap_err().code, "head_cookie_mismatch");
    let mut o = observation(Case::Cancellation);
    o.http_status = Some(503);
    assert_eq!(evaluate(&o).unwrap_err().code, "unexpected_http_response");
}
