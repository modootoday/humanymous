//! Complete L5 report, header predicates and audit residuals against frozen Go.
use humanymous_platform::testing::assert_json_equivalent;
use humanymous_runtime::network::{
    report::{Observation, build},
    residual,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Case {
    input: Observation,
    expected: Value,
    audit: Value,
    actionable: bool,
    header_checks: Value,
}

#[test]
fn all_report_signals_and_admin_observations_match_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/network/reports.json")).unwrap();
    assert_eq!(cases.len(), 1024);
    for case in cases {
        let report = build(&case.input);
        assert_json_equivalent(&serde_json::to_value(&report).unwrap(), &case.expected);
        let signals = report.signals.as_deref().unwrap();
        assert_json_equivalent(
            &serde_json::to_value(residual::audit_records(signals)).unwrap(),
            &case.audit,
        );
        assert_eq!(residual::has_actionable_residual(signals), case.actionable);
        let h = &case.input.header;
        assert_json_equivalent(
            &json!({"order":h.order(),"sec_fetch":h.sec_fetch_present(),"sec_ch_ua":h.sec_ch_ua_present(),"uppercase":h.has_uppercase_in_h2(),"encoding":h.chrome_accept_encoding_ok(),"proxy":h.proxy_hop_kind(),"spoof":h.client_ip_spoof_kind(),"hops":h.xff_hop_count()}),
            &case.header_checks,
        );
    }
}

/// The `browser_tls_over_h1` residual (legacy report.go: `obs.Hello != nil &&
/// isBrowserEngine(nr.JA4Engine) && !h.IsH2`) has no case in the frozen Go
/// corpus, so it is pinned here by flipping corpus inputs one predicate term
/// at a time.
#[test]
fn browser_tls_over_h1_needs_hello_browser_ja4_and_h1() {
    const ID: &str = "l5.http.browser_tls_over_h1";
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/network/reports.json")).unwrap();
    let fired = |input: &Value| -> Option<String> {
        let observation: Observation = serde_json::from_value(input.clone()).unwrap();
        build(&observation)
            .signals
            .unwrap_or_default()
            .into_iter()
            .find(|signal| signal.id == ID)
            .map(|signal| signal.verdict.as_str().to_owned())
    };
    let mut browsers = 0;
    let mut others = 0;
    for case in &cases {
        let input = &case["input"];
        if input["hello"].is_null() {
            continue;
        }
        let mut h1 = input.clone();
        h1["header"]["is_h2"] = json!(false);
        let browser = matches!(
            case["expected"]["ja4Engine"].as_str(),
            Some("chrome" | "firefox" | "safari")
        );
        if browser {
            browsers += 1;
            assert_eq!(fired(&h1).as_deref(), Some("SUSPICIOUS"));
            let mut no_hello = h1.clone();
            no_hello["hello"] = Value::Null;
            assert_eq!(
                fired(&no_hello),
                None,
                "a CDN-terminated hello never trips it"
            );
        } else {
            others += 1;
            assert_eq!(fired(&h1), None, "non-browser JA4 engines never trip it");
        }
        if input["header"]["is_h2"] == json!(true) {
            assert_eq!(fired(input), None, "h2 delivery never trips it");
        }
    }
    assert!(
        browsers > 0 && others > 0,
        "corpus must exercise both branches"
    );
}
