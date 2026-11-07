//! Full engine and additive explanation against the frozen Go oracle.
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use humanymous_test_support::assert_json_equivalent;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Case {
    report: SessionReport,
    modes: Option<BTreeMap<String, String>>,
    weights: Option<BTreeMap<String, f64>>,
    net: Option<BTreeMap<String, String>>,
    expected: Value,
    trace: Value,
}

#[test]
fn complete_engine_results_and_traces_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/engine.json")).unwrap();
    assert_eq!(cases.len(), 3788);
    let mut matched_rules = std::collections::BTreeSet::new();
    for mut case in cases {
        let engine = Engine {
            rule_modes: case.modes.unwrap_or_default(),
            weight_multipliers: case.weights.unwrap_or_default(),
            net_policy: case.net,
            ..Default::default()
        };
        let (result, trace) = engine.score_with_trace(&mut case.report).unwrap();
        for rule in &trace.hard_rules {
            if rule.matched {
                matched_rules.insert(rule.rule.clone());
            }
        }
        assert_json_equivalent(&serde_json::to_value(&result).unwrap(), &case.expected);
        assert_json_equivalent(&serde_json::to_value(trace).unwrap(), &case.trace);
        assert_eq!(result, engine.score(&mut case.report).unwrap());
    }
    assert_eq!(
        matched_rules.len(),
        24,
        "every frozen predicate must be exercised"
    );
}

#[test]
fn original_freeze_goldens_remain_unchanged() {
    for raw in [
        include_str!("fixtures/frozen/bot-webdriver-cdp-hr9.json"),
        include_str!("fixtures/frozen/go-http-client.json"),
        include_str!("fixtures/frozen/human-chrome-allow.json"),
        include_str!("fixtures/frozen/no-client-hr10.json"),
        include_str!("fixtures/frozen/selenium-hr1.json"),
        include_str!("fixtures/frozen/ua-tls-h2-hr2.json"),
    ] {
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let mut report: SessionReport = serde_json::from_value(fixture["report"].clone()).unwrap();
        let result = serde_json::to_value(Engine::default().score(&mut report).unwrap()).unwrap();
        for field in ["riskScore", "verdict", "hardRuleFired", "policyVersion"] {
            assert_json_equivalent(&result[field], &fixture["want"][field]);
        }
    }
}
