//! Complete L5 report, header predicates and audit residuals against frozen Go.
use humanymous_network::{
    report::{Observation, build},
    residual,
};
use humanymous_test_support::assert_json_equivalent;
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
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/reports.json")).unwrap();
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
