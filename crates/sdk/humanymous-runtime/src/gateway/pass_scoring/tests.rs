//! B1 differential tests against the Go oracle at 981d25c.
use super::*;
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::observation::lookup;
use humanymous_core::signals::report::SessionReport;
use serde_json::Value;

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/pass-flow-go-golden.json"
    ))
    .unwrap()
}

fn bits(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

/// Legacy TestPassVelocitySignalsRegistered: the velocity signals carry
/// weight in the frozen registry and the solved upgrade carries none.
#[test]
fn pass_signals_are_registered_with_go_weights() {
    for (id, weight) in [
        (SOLVED_SIGNAL, 0.0),
        (VELOCITY_SIGNAL, 25.0),
        (FLOOD_SIGNAL, 45.0),
    ] {
        let definition = lookup(id).unwrap_or_else(|| panic!("{id} not registered"));
        assert_eq!(definition.weight, weight, "{id}");
    }
    for id in [
        "l4.key.machine_speed",
        "l4.key.flight_std",
        "l4.event.perfect_timing",
        "l2.adv.mobile_inconsistent",
    ] {
        assert!(lookup(id).is_some(), "{id} must already be registered");
    }
}

/// B1 differential: `applyPassBehavior` emits the same signals as Go (ids,
/// verdicts, confidences, registry weights, scores, notes, order) and the
/// frozen engine scores the stored report to Go's risk, verdict and rule.
#[test]
fn behavior_tells_and_scores_match_go_golden() {
    let golden = golden();
    let cases = golden["outputs"]["behavior"].as_array().unwrap();
    assert!(cases.len() >= 18);
    let engine = Engine::default();
    for case in cases {
        let text = case["proof"].as_str().unwrap();
        let proof: PassProof = serde_json::from_str(text).unwrap();
        let expected: Vec<Signal> = serde_json::from_value(case["signals"].clone()).unwrap();
        let signals = behavior_signals(&proof);
        assert_eq!(signals, expected, "{text}");
        assert_eq!(
            !signals.is_empty(),
            case["stored"].as_bool().unwrap(),
            "{text}"
        );
        if signals.is_empty() {
            continue;
        }
        let mut report = SessionReport::default();
        report.network.signals = Some(signals);
        let scored = engine.score(&mut report).unwrap();
        let go = &case["scoring"];
        assert_eq!(
            scored.risk_score,
            go["riskScore"].as_f64().unwrap(),
            "{text}"
        );
        assert_eq!(scored.verdict, go["verdict"].as_str().unwrap(), "{text}");
        assert_eq!(
            scored.hard_rule_fired,
            go["hardRuleFired"].as_str().unwrap(),
            "{text}"
        );
    }
}

/// B1 differential: `passDifficulty` and `passLabel` over risk bands, NaN,
/// infinities and int64 overflow (Go amd64 yields the minimum int64, so the
/// difficulty clamps to 0), with and without a red-team header.
#[test]
fn difficulty_and_label_match_go_golden() {
    let golden = golden();
    let outputs = &golden["outputs"]["difficulty"];
    let cases = outputs["difficulty"].as_array().unwrap();
    assert!(cases.len() >= 78);
    for case in cases {
        let risk = bits(&case["risk"]);
        let header = case["redteam"].as_str().unwrap();
        assert_eq!(
            u64::from(difficulty(risk, !header.is_empty())),
            case["difficulty"].as_u64().unwrap(),
            "risk {risk} header {header:?}"
        );
    }
    for case in outputs["label"].as_array().unwrap() {
        let risk = bits(&case["risk"]);
        let header = case["redteam"].as_str().unwrap();
        assert_eq!(
            label(risk, Some(header)),
            case["label"].as_str().unwrap(),
            "risk {risk} header {header:?}"
        );
    }
    for case in outputs["isBotLabel"].as_array().unwrap() {
        let name = case["label"].as_str().unwrap();
        assert_eq!(
            is_bot_label(name),
            case["isBot"].as_bool().unwrap(),
            "{name}"
        );
    }
}

/// Velocity escalation folds one signal per new level and never repeats or
/// downgrades (Go `applyPassVelocity` dedupe on `velLevel`).
#[test]
fn velocity_escalation_folds_each_level_once() {
    assert_eq!(velocity_escalation(0, 0), None);
    let soft = velocity_escalation(1, 0).unwrap();
    assert_eq!(
        (
            soft.id.as_str(),
            soft.verdict.clone(),
            soft.weight,
            soft.score
        ),
        (VELOCITY_SIGNAL, Verdict::Suspicious, 25.0, 12.5)
    );
    assert_eq!(velocity_escalation(1, 1), None);
    let hard = velocity_escalation(2, 1).unwrap();
    assert_eq!(
        (
            hard.id.as_str(),
            hard.verdict.clone(),
            hard.weight,
            hard.score
        ),
        (FLOOD_SIGNAL, Verdict::Bot, 45.0, 45.0)
    );
    assert_eq!(velocity_escalation(2, 2), None);
    assert_eq!(velocity_escalation(1, 2), None);
    assert_eq!(level_number(Level::Normal), 0);
    assert_eq!(level_number(Level::Soft), 1);
    assert_eq!(level_number(Level::Hard), 2);
}

/// Proof-of-work cost: 14 + difficulty + 2 per velocity level, capped at 20;
/// velocity never changes the puzzle itself.
#[test]
fn pow_cost_scales_with_difficulty_and_velocity_up_to_twenty() {
    assert_eq!(pow_difficulty(0, 0), 14);
    assert_eq!(pow_difficulty(1, 1), 17);
    assert_eq!(pow_difficulty(3, 1), 19);
    assert_eq!(pow_difficulty(2, 2), 20);
    assert_eq!(pow_difficulty(3, 2), 20);
}

/// The solved signal is the zero-weight OK trust upgrade.
#[test]
fn solved_signal_is_an_ok_server_upgrade() {
    let signal = solved_signal();
    assert_eq!(signal.id, SOLVED_SIGNAL);
    assert_eq!(signal.verdict, Verdict::Ok);
    assert_eq!(signal.value, Value::Bool(true));
    assert_eq!(signal.collected, Source::Server);
    assert_eq!(signal.notes, "humanymous Pass cleared");
    assert_eq!(fingerprint("t13d", "203.0.113"), "t13d|203.0.113");
}
