//! Frozen HTTP/2 rendering and engine resolution, including duplicate settings.
use humanymous_runtime::network::http2::Fingerprint;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    input: Fingerprint,
    akamai: String,
    engine: String,
    sorted: Vec<u16>,
}

#[test]
fn h2_wire_profile_matches_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/network/h2.json")).unwrap();
    assert_eq!(cases.len(), 144);
    for case in cases {
        assert_eq!(case.input.akamai(), case.akamai);
        assert_eq!(case.input.engine(), case.engine);
        assert_eq!(case.input.sorted_setting_ids(), case.sorted);
    }
}
