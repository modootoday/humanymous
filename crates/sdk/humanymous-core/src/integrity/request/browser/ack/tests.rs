use super::*;
const TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
#[test]
fn fixed_ack_vector_matches_independent_dotnet_hmac_derivation() {
    let text = format!("humanymous.browser/0.1/ack\n43:{TOKEN}\n200\n");
    use sha2::{Digest, Sha256};
    assert_eq!(
        hex::encode(Sha256::digest(text.as_bytes())),
        "f2b7005370c6bc779abe03d0044ac7e52537383ed85ac51c3af499413fe90c5b"
    );
    assert_eq!(
        acknowledgement(&[1; 32], TOKEN, 1, 9, 200).unwrap(),
        "OOdSOlV17YY6x0gF37ZIL3SVSumfO6xPoa3XpGZLcS4"
    );
}
#[test]
fn every_ack_input_and_exact_encoding_are_bound_without_request_reflection() {
    let ack = acknowledgement(&[1; 32], TOKEN, 1, 9, 503).unwrap();
    assert!(verify_acknowledgement(&[1; 32], TOKEN, 1, 9, 503, &ack));
    assert!(!verify_acknowledgement(&[2; 32], TOKEN, 1, 9, 503, &ack));
    assert!(!verify_acknowledgement(
        &[1; 32],
        "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE",
        1,
        9,
        503,
        &ack
    ));
    assert!(!verify_acknowledgement(&[1; 32], TOKEN, 2, 9, 503, &ack));
    assert!(!verify_acknowledgement(&[1; 32], TOKEN, 1, 10, 503, &ack));
    assert!(!verify_acknowledgement(&[1; 32], TOKEN, 1, 9, 200, &ack));
    assert!(!verify_acknowledgement(
        &[1; 32],
        TOKEN,
        1,
        9,
        503,
        &(ack.clone() + "=")
    ));
    assert!(!verify_acknowledgement(&[1; 32], TOKEN, 1, 9, 503, TOKEN));
}
#[test]
fn malformed_ack_bounds_cannot_be_signed() {
    for (key, n, bucket, status, token) in [
        (&[1u8; 31][..], 1, 9, 200, TOKEN),
        (&[1; 32][..], 0, 9, 200, TOKEN),
        (&[1; 32][..], MAX_SAFE_INTEGER + 1, 9, 200, TOKEN),
        (&[1; 32][..], 1, MAX_SAFE_INTEGER + 1, 200, TOKEN),
        (&[1; 32][..], 1, 9, 99, TOKEN),
        (&[1; 32][..], 1, 9, 600, TOKEN),
        (
            &[1; 32][..],
            1,
            9,
            200,
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB",
        ),
    ] {
        assert!(acknowledgement(key, token, n, bucket, status).is_err());
    }
}
