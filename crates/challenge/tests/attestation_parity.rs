//! Frozen independent Go attestation vectors.
use humanymous_challenge::attestation::{issue, verify};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    key: String,
    subject: String,
    nonce: String,
    window: u64,
    token: String,
    at_zero: bool,
    at_max: bool,
}

#[test]
fn matches_go_attestation_vectors() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/attestation.json")).unwrap();
    assert_eq!(cases.len(), 81);
    for case in cases {
        assert_eq!(
            issue(case.key.as_bytes(), &case.subject, &case.nonce, case.window),
            case.token
        );
        assert_eq!(
            verify(
                case.key.as_bytes(),
                &case.subject,
                &case.nonce,
                &case.token,
                0
            ),
            case.at_zero
        );
        assert_eq!(
            verify(
                case.key.as_bytes(),
                &case.subject,
                &case.nonce,
                &case.token,
                u64::MAX
            ),
            case.at_max
        );
    }
}
