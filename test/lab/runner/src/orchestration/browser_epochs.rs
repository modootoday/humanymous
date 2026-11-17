//! Roadmap 2d two-epoch feeding. One browser-product run holds two session
//! epochs (`browser_expectations::EPOCHS`): `stripped` (no guard hooks) and
//! `complete` (the lab page explicitly enables them). This module only splits
//! the run's observed evidence into per-epoch views so `missing-signals-closed`
//! reads the stripped epoch and `collector-parity` the complete one; neither
//! evaluator's checks change.
//!
//! The split is observed, never positional: both proxies open an epoch at each
//! bootstrap request and record every later API hit into it; the Gate frontend
//! assigns each collect entity to its epoch and attributes each authenticated
//! audit record by its `request_id` (SHA-256 of the consumed request's session
//! and counter). A run whose epochs are not exactly bound yields empty views.
use crate::browser_expectations::EPOCHS;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Finite reasons the run's evidence is not bound to its epochs; empty when bound.
pub(super) fn binding_gaps(gate: &Value, actor: &Value) -> Vec<String> {
    let mut gaps = Vec::new();
    for (side, value) in [("gate", gate), ("browser", actor)] {
        let epochs = value["epochs"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        if epochs.len() != EPOCHS.len() {
            gaps.push(format!("{side}_epoch_count"));
        }
        // Each epoch names exactly one session that no other epoch names.
        if epochs
            .iter()
            .any(|epoch| epoch["sessions"] != 1 || epoch["shared_sessions"] != 0)
        {
            gaps.push(format!("{side}_epoch_session"));
        }
        if value["proxy_hits"]["epoch_unattributed"]
            .as_u64()
            .is_some_and(|n| n > 0)
        {
            gaps.push(format!("{side}_request_unattributed"));
        }
    }
    let records: Vec<u64> = gate["audit_records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["seq"].as_u64())
        .collect();
    let mut attributed = Vec::new();
    for epoch in gate["epochs"].as_array().into_iter().flatten() {
        attributed.extend(
            epoch["audit_seqs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64),
        );
    }
    let unique: BTreeSet<_> = attributed.iter().collect();
    if gate["audit_unattributed"] != 0
        || unique.len() != attributed.len()
        || unique != records.iter().collect()
    {
        gaps.push("audit_unattributed".into());
    }
    if gate["missing_signal_controls"]["epoch"] != "stripped" {
        gaps.push("controls_epoch".into());
    }
    gaps
}

/// The (gate, browser, expected) evidence of one epoch, shaped like the
/// single-epoch run each evaluator consumes. Origin effects are not
/// attributable to an epoch, so every view carries the whole run's count.
pub(super) fn view(gate: &Value, actor: &Value, expected: &Value, id: &str) -> [Value; 3] {
    let bound = binding_gaps(gate, actor).is_empty();
    let index = EPOCHS.iter().position(|epoch| *epoch == id);
    let epoch = |side: &Value| match (bound, index) {
        (true, Some(index)) => side["epochs"][index].clone(),
        _ => Value::Null,
    };
    let (gate_epoch, actor_epoch) = (epoch(gate), epoch(actor));
    let reports: Vec<Value> = gate_epoch["collector_reports"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|index| gate["collector_reports"].get(index.as_u64()? as usize))
        .cloned()
        .collect();
    let seqs: BTreeSet<u64> = gate_epoch["audit_seqs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    let records: Vec<Value> = gate["audit_records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|record| {
            record["seq"]
                .as_u64()
                .is_some_and(|seq| seqs.contains(&seq))
        })
        .cloned()
        .collect();
    let sequence = seqs.last().copied().unwrap_or(0);
    let hits = |epoch: &Value| match &epoch["proxy_hits"] {
        Value::Object(hits) => Value::Object(hits.clone()),
        _ => json!({}),
    };
    [
        json!({"proxy_hits":hits(&gate_epoch),"collector_reports":reports,
            "audit_records":records,"audit_sequence":sequence,
            "origin_effects":gate["origin_effects"],
            "missing_signal_controls":gate["missing_signal_controls"]}),
        json!({"status":actor["status"],"proxy_hits":hits(&actor_epoch)}),
        expected["epochs"][id].clone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::collector_parity::{missing_signals, parity};
    use sha2::{Digest, Sha256};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../../crates/collector/pilot/fixture.json"
        ))
        .unwrap()
    }

    /// Standard padded base64, as the Gate actor encodes captured entities.
    fn encode(bytes: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    fn entity(report: &Value) -> Value {
        let bytes = serde_json::to_vec(report).unwrap();
        json!({"sha256":format!("{:x}", Sha256::digest(&bytes)),"bytes":bytes.len(),
            "base64":encode(&bytes),"oversize":false})
    }

    fn report(guard: bool) -> Value {
        json!({"sessionId":"","userAgent":"Mozilla/5.0 Firefox","uaClientHints":{},
            "signals":[{"id":"l1.navigator.webdriver","layer":"L1","value":true,"verdict":"BOT",
                "weight":15.0,"score":15.0,"confidence":1.0,"collected":"wasm"}],
            "behavior":{},"environment":{"probed":true},"guard":{"reported":guard},
            "advanced":{"probed":true},"fingerprintId":"0123456789abcdef01234567",
            "engineVersion":"rust-wasm-0.1.0"})
    }

    /// The hit counters one proxy records for the fixture's two epochs.
    fn epoch_hits() -> [Value; 2] {
        [
            json!({"/_humanymous/browser/session":1,"/effect":2,
                "/_humanymous/browser/collect":1,"/requires-collector":1,
                "response:/_humanymous/browser/session:200:ack_false":1,
                "response:/effect:403:ack_true":2,
                "response:/_humanymous/browser/collect:200:ack_true":1,
                "response:/requires-collector:422:ack_false":1}),
            json!({"/_humanymous/browser/session":1,
                "/_humanymous/browser/collect":1,"/requires-collector":1,
                "response:/_humanymous/browser/session:200:ack_false":1,
                "response:/_humanymous/browser/collect:200:ack_true":1,
                "response:/requires-collector:403:ack_true":1}),
        ]
    }

    /// A two-epoch run shaped like the Gate and browser actor frames.
    fn run() -> (Value, Value, Value) {
        let expected = crate::browser_expectations::from_fixture(&fixture()).unwrap();
        let [stripped, complete] = epoch_hits();
        let epoch = |hits: &Value, reports: Value, seqs: Value| {
            json!({"proxy_hits":hits,"sessions":1,"shared_sessions":0,
                "collector_reports":reports,"audit_seqs":seqs})
        };
        let controls = json!({"schema":"humanymous.browser-missing-signal-controls/1",
            "epoch":"stripped","captured_collect_requests":1,"fresh_session_distinct":true,
            "rows":[{"id":"fresh-session-bootstrap","http":200,"error":null,"ack":false,"failure":null},
                {"id":"fresh-session-required-collector","http":422,"error":"required_capability","ack":false,"failure":null},
                {"id":"replayed-collect","http":409,"error":"counter_replay_or_gap","ack":false,"failure":null},
                {"id":"foreign-session-collect","http":403,"error":"request_integrity","ack":false,"failure":null},
                {"id":"foreign-session-required-collector-after-replay","http":422,"error":"required_capability","ack":false,"failure":null}]});
        let gate = json!({"proxy_hits":{},"origin_effects":0,
            "collector_reports":[entity(&report(false)),entity(&report(true))],
            "epochs":[epoch(&stripped,json!([0]),json!([1,2,3])),epoch(&complete,json!([1]),json!([4,5]))],
            "audit_unattributed":0,"audit_records":expected["audit_records"],
            "audit_sequence":expected["audit_sequence"],"missing_signal_controls":controls});
        let actor = json!({"status":"passed","proxy_hits":{},
            "epochs":[epoch(&stripped,json!([]),Value::Null),epoch(&complete,json!([]),Value::Null)]});
        (gate, actor, expected)
    }

    fn evaluate(gate: &Value, actor: &Value, expected: &Value) -> (Value, Value) {
        let fixture = fixture();
        let [g, a, e] = view(gate, actor, expected, "stripped");
        let missing = missing_signals(&g, &a, &e, &fixture);
        let [g, a, e] = view(gate, actor, expected, "complete");
        (parity(&g, &a, &e, &fixture), missing)
    }

    #[test]
    fn both_cases_pass_only_on_their_own_bound_epoch() {
        let (gate, actor, expected) = run();
        assert_eq!(binding_gaps(&gate, &actor), Vec::<String>::new());
        let (parity, missing) = evaluate(&gate, &actor, &expected);
        assert_eq!(parity["gaps"], json!([]), "{parity}");
        assert_eq!(missing["gaps"], json!([]), "{missing}");
        // Feeding either case the other epoch never passes it.
        let fixture = fixture();
        let [g, a, e] = view(&gate, &actor, &expected, "stripped");
        assert!(!parity_of(&g, &a, &e, &fixture));
        let [g, a, e] = view(&gate, &actor, &expected, "complete");
        assert_eq!(missing_signals(&g, &a, &e, &fixture)["passed"], false);
        // An unknown epoch is empty evidence.
        let [g, a, e] = view(&gate, &actor, &expected, "other");
        assert!(!parity_of(&g, &a, &e, &fixture));
    }

    fn parity_of(gate: &Value, actor: &Value, expected: &Value, fixture: &Value) -> bool {
        parity(gate, actor, expected, fixture)["passed"] == true
    }

    #[test]
    fn unbound_epochs_feed_empty_evidence_and_fail_both_cases() {
        let (gate, actor, expected) = run();
        let mut cases: Vec<(&str, Value, Value)> = Vec::new();
        let mut shared = gate.clone();
        shared["epochs"][1]["shared_sessions"] = json!(1);
        cases.push(("gate_epoch_session", shared, actor.clone()));
        let mut two = gate.clone();
        two["epochs"][0]["sessions"] = json!(2);
        cases.push(("gate_epoch_session", two, actor.clone()));
        let mut one = actor.clone();
        one["epochs"].as_array_mut().unwrap().pop();
        cases.push(("browser_epoch_count", gate.clone(), one));
        let mut stray = gate.clone();
        stray["proxy_hits"]["epoch_unattributed"] = json!(1);
        cases.push(("gate_request_unattributed", stray, actor.clone()));
        let mut missing = gate.clone();
        missing["epochs"][1]["audit_seqs"] = json!([4]);
        cases.push(("audit_unattributed", missing, actor.clone()));
        let mut twice = gate.clone();
        twice["epochs"][1]["audit_seqs"] = json!([3, 4, 5]);
        cases.push(("audit_unattributed", twice, actor.clone()));
        let mut counted = gate.clone();
        counted["audit_unattributed"] = json!(1);
        cases.push(("audit_unattributed", counted, actor.clone()));
        let mut controls = gate.clone();
        controls["missing_signal_controls"]["epoch"] = json!("complete");
        cases.push(("controls_epoch", controls, actor.clone()));
        for (gap, gate, actor) in cases {
            assert!(
                binding_gaps(&gate, &actor).contains(&gap.to_owned()),
                "{gap}"
            );
            let (parity, missing) = evaluate(&gate, &actor, &expected);
            assert_eq!(parity["passed"], false, "{gap}");
            assert_eq!(missing["passed"], false, "{gap}");
        }
    }

    #[test]
    fn each_epoch_keeps_only_its_own_reports_records_and_hits() {
        let (gate, actor, expected) = run();
        let [g, a, e] = view(&gate, &actor, &expected, "complete");
        assert_eq!(
            g["collector_reports"],
            json!([gate["collector_reports"][1]])
        );
        assert_eq!(g["audit_sequence"], 5);
        assert_eq!(g["audit_records"], e["audit_records"]);
        assert_eq!(a["proxy_hits"], epoch_hits()[1]);
        assert_eq!(a["status"], "passed");
        let [g, _, e] = view(&gate, &actor, &expected, "stripped");
        assert_eq!(g["audit_sequence"], 3);
        assert_eq!(g["audit_records"], e["audit_records"]);
        assert_eq!(
            g["collector_reports"],
            json!([gate["collector_reports"][0]])
        );
        // Origin effects are charged to every epoch.
        let mut effect = gate.clone();
        effect["origin_effects"] = json!(1);
        assert_eq!(
            view(&effect, &actor, &expected, "stripped")[0]["origin_effects"],
            1
        );
        assert_eq!(evaluate(&effect, &actor, &expected).1["passed"], false);
    }
}
