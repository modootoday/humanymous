//! Signed URL compatibility, including Unicode and signed expiry boundaries.
use humanymous_integrity::resource::{sign, signed_url, verify};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    key: String,
    session: String,
    asset: String,
    expiry: i64,
    signature: String,
    url: String,
    at_max: bool,
}

#[test]
fn signed_urls_and_expiry_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/resource.json")).unwrap();
    assert_eq!(cases.len(), 81);
    for case in cases {
        assert_eq!(
            sign(case.key.as_bytes(), &case.session, &case.asset, case.expiry),
            case.signature
        );
        assert_eq!(
            signed_url(case.key.as_bytes(), &case.session, &case.asset, case.expiry),
            case.url
        );
        assert_eq!(
            verify(
                case.key.as_bytes(),
                &case.session,
                &case.asset,
                case.expiry,
                i64::MAX,
                &case.signature
            ),
            case.at_max
        );
    }
}
