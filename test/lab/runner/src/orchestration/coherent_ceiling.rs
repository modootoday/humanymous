//! Roadmap 4 `coherent-browser-ceiling`: the intentional T4 ceiling measured
//! and recorded, never claimed as detection. The browser-product run's third
//! session epoch (`ceiling`) is the same real Firefox and Rust WASM collector
//! as the complete epoch (guard hooks opted in by the page) at the fixture's
//! declared dwell and think times. The Gate's consumed decision on the
//! required-collector route is read from the authenticated audit record the
//! epoch binding attributes to that session, its response from both proxies
//! and its origin effect from the origin's own count.
//!
//! The case records whatever was observed. ALLOW (forwarded once) is
//! `ceiling_reached`; DENY or CHALLENGE (nothing forwarded) is
//! `ceiling_not_reached` with the actor's own reported BOT signals listed, so
//! a non-coherent actor is visible as such. Neither outcome is block credit
//! or a detection claim. The case fails when the observation is missing,
//! unbound to its epoch/session/images, unpaced or inconsistent.
//!
//! Owner decision C9 (#75, 2026-11-18): the case also requires the ceiling
//! epoch's actor to be a coherent T4 actor. The bound browser epoch must carry
//! `actor_coherent: true`; `false` (or any non-boolean value) fails, and an
//! absent attestation leaves the case `not_evaluated`, never passed. No
//! current lab actor emits it (headless WebDriver Firefox is not coherent), so
//! the lab records the observation without a classification and the case
//! stays unevaluated until a coherent actor is defined.
use super::browser_epochs::{binding_gaps, view};
use super::collector_parity::captured;
use crate::browser_expectations::{CEILING_VERDICTS, EPOCHS};
use serde_json::{Value, json};

const SCHEMA: &str = "humanymous.coherent-browser-ceiling/1";
const EPOCH: &str = "ceiling";

fn index() -> usize {
    EPOCHS
        .iter()
        .position(|epoch| *epoch == EPOCH)
        .expect("ceiling epoch is declared")
}

fn route(fixture: &Value, key: &str) -> String {
    fixture["routes"][key]
        .as_str()
        .unwrap_or("\u{0}")
        .to_owned()
}

/// Response keys with a nonzero count recorded for one route.
fn responses(hits: &Value, path: &str) -> Vec<(String, u64)> {
    let prefix = format!("response:{path}:");
    hits.as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(key, n)| (key.clone(), n.as_u64().unwrap_or(0)))
        .filter(|(_, n)| *n > 0)
        .collect()
}

/// The authenticated audit records attributed to the bound ceiling epoch.
fn records(gate: &Value) -> Vec<Value> {
    let seqs: Vec<u64> = gate["epochs"][index()]["audit_seqs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    gate["audit_records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|record| record["seq"].as_u64().is_some_and(|s| seqs.contains(&s)))
        .cloned()
        .collect()
}

fn allows(records: &[Value]) -> u64 {
    records
        .iter()
        .filter(|r| r["event_type"] == "runtime.decision" && r["verdict"] == "ALLOW")
        .count() as u64
}

/// ALLOW decisions attributed to the ceiling epoch, zero when epochs are
/// unbound. The origin forwards only ALLOW decisions, so these are the only
/// origin effects the run may charge to the ceiling.
pub(super) fn ceiling_allows(gate: &Value, actor: &Value) -> u64 {
    if binding_gaps(gate, actor).is_empty() {
        allows(&records(gate))
    } else {
        0
    }
}

/// The ceiling epoch's consumed decision as bound evidence, or the finite
/// reasons it is not bound.
pub(crate) fn decision(
    gate: &Value,
    actor: &Value,
    expected: &Value,
    fixture: &Value,
) -> Result<Value, Vec<String>> {
    let binding = binding_gaps(gate, actor);
    if !binding.is_empty() {
        return Err(binding
            .into_iter()
            .map(|gap| format!("epoch_binding:{gap}"))
            .collect());
    }
    let mut gaps = Vec::new();
    let known = &expected["epochs"][EPOCH];
    let decision_seq = known["observed"]["decision_seq"].as_u64();
    let found = records(gate);
    let record = match found.as_slice() {
        [collect, decision]
            if Some(collect) == known["audit_records"].get(0)
                && decision["seq"].as_u64() == decision_seq
                && decision["event_type"] == "runtime.decision"
                && decision["verdict"]
                    .as_str()
                    .is_some_and(|v| CEILING_VERDICTS.contains(&v)) =>
        {
            Some(decision)
        }
        _ => {
            gaps.push("ceiling_decision_unbound".to_owned());
            None
        }
    };
    let path = route(fixture, "required_collector");
    let index = index();
    let gate_keys = responses(&gate["epochs"][index]["proxy_hits"], &path);
    let actor_keys = responses(&actor["epochs"][index]["proxy_hits"], &path);
    let response = match gate_keys.as_slice() {
        [(key, 1)] if gate_keys == actor_keys => key
            .strip_prefix(&format!("response:{path}:"))
            .and_then(|rest| rest.split_once(":ack_"))
            .and_then(|(status, ack)| Some((status.parse::<u64>().ok()?, ack == "true"))),
        _ => None,
    };
    match response {
        Some((_, true)) => {}
        Some((_, false)) => gaps.push("ceiling_decision_unacknowledged".to_owned()),
        None => gaps.push("ceiling_response_unbound".to_owned()),
    }
    // Every origin effect must be one ALLOW decision, and every ALLOW decision
    // of the run must belong to the ceiling epoch.
    let all: Vec<Value> = gate["audit_records"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let ceiling = allows(&found);
    if gate["origin_effects"].as_u64() != Some(allows(&all)) || allows(&all) != ceiling {
        gaps.push("origin_effect_unattributed".to_owned());
    }
    match (record, response) {
        (Some(record), Some((status, ack))) if gaps.is_empty() => Ok(json!({
            "seq":record["seq"],"verdict":record["verdict"],"status":status,"ack":ack,
            "origin_effects":ceiling})),
        _ => Err(gaps),
    }
}

/// The whole-run expectation with the observed ceiling decision filled in, so
/// whole-run exact checks cover it without ever expecting a verdict. Without
/// a bound decision the expectation is returned unchanged and those checks
/// fail against the run's extra record.
pub(crate) fn with_decision(expected: &Value, decision: Option<&Value>) -> Value {
    let mut whole = expected.clone();
    let Some(decision) = decision else {
        return whole;
    };
    let record = json!({"seq":decision["seq"],"event_type":"runtime.decision",
        "verdict":decision["verdict"]});
    let response = json!({"status":decision["status"],"ack":decision["ack"]});
    if let Some(records) = whole["audit_records"].as_array_mut() {
        records.push(record.clone());
    }
    if let Some(records) = whole["epochs"][EPOCH]["audit_records"].as_array_mut() {
        records.push(record);
    }
    whole["epochs"][EPOCH]["responses"]["required_collector"] = response.clone();
    whole["epochs"][EPOCH]["origin_effects"] = decision["origin_effects"].clone();
    whole["origin_effects"] = decision["origin_effects"].clone();
    if let Some(list) = whole["responses"]["required_collector"].as_array_mut() {
        match list
            .iter_mut()
            .find(|r| r["status"] == response["status"] && r["ack"] == response["ack"])
        {
            Some(found) => found["count"] = json!(found["count"].as_u64().unwrap_or(0) + 1),
            None => list.push(json!({"status":response["status"],"ack":response["ack"],"count":1})),
        }
    }
    whole
}

/// Whether the bound ceiling epoch's actor attests coherence: `None` when
/// epochs are unbound or the attestation is absent, `Some(false)` for an
/// explicit `false` or a malformed value, `Some(true)` only for `true`.
fn actor_coherent(gate: &Value, actor: &Value) -> Option<bool> {
    if !binding_gaps(gate, actor).is_empty() {
        return None;
    }
    match actor["epochs"][index()].get("actor_coherent")? {
        Value::Bool(coherent) => Some(*coherent),
        _ => Some(false),
    }
}

/// `coherent-browser-ceiling`: complete, bound, consistently classified and
/// observed from a coherent actor. `status` is `passed`, `failed` or
/// `not_evaluated`; any evidence gap fails, and only otherwise-sound evidence
/// without an `actor_coherent` attestation is `not_evaluated`.
/// `provenance` carries the observed browser and runtime image IDs.
pub(super) fn evaluate(
    gate: &Value,
    actor: &Value,
    expected: &Value,
    fixture: &Value,
    provenance: &Value,
) -> Value {
    let (decision, mut gaps) = match decision(gate, actor, expected, fixture) {
        Ok(decision) => (Some(decision), Vec::new()),
        Err(gaps) => (None, gaps),
    };
    let [epoch_gate, epoch_actor, known] = view(gate, actor, expected, EPOCH);
    for (key, count) in known["request_counts"].as_object().into_iter().flatten() {
        let path = route(fixture, key);
        let hits = |side: &Value| side["proxy_hits"][&path].as_u64().unwrap_or(0);
        if count.as_u64() != Some(hits(&epoch_gate)) || count.as_u64() != Some(hits(&epoch_actor)) {
            gaps.push(format!("ceiling_request_shape:{key}"));
        }
    }
    let collect_ok = format!("response:{}:200:ack_true", route(fixture, "collect"));
    if epoch_gate["proxy_hits"][&collect_ok] != 1 || epoch_actor["proxy_hits"][&collect_ok] != 1 {
        gaps.push("ceiling_collect_unacknowledged".to_owned());
    }
    // Observed pacing on both proxies' own clocks, never the declared values.
    let minimum = known["pace"]["minimum_elapsed_ms"].as_u64();
    let elapsed = |side: &Value| {
        binding_gaps(gate, actor)
            .is_empty()
            .then(|| side["epochs"][index()]["elapsed_ms"].as_u64())
            .flatten()
    };
    let (gate_elapsed, actor_elapsed) = (elapsed(gate), elapsed(actor));
    let paced = |observed: Option<u64>| matches!((observed, minimum), (Some(observed), Some(minimum)) if observed >= minimum);
    if !(paced(gate_elapsed) && paced(actor_elapsed)) {
        gaps.push("pace_not_observed".to_owned());
    }
    let report = captured(&epoch_gate);
    let user_agent = report.as_ref().and_then(|r| r["userAgent"].as_str());
    let engine = report.as_ref().and_then(|r| r["engineVersion"].as_str());
    match &report {
        None => gaps.push("ceiling_report_unavailable".to_owned()),
        Some(report) if report["guard"]["reported"] != true => {
            gaps.push("ceiling_hooks_not_opted_in".to_owned());
        }
        Some(_) => {}
    }
    if !user_agent.is_some_and(|ua| !ua.is_empty())
        || !engine.is_some_and(|v| v.starts_with("rust-wasm-"))
    {
        gaps.push("browser_version_unbound".to_owned());
    }
    let image = |key: &str| {
        provenance[key]
            .as_str()
            .filter(|id| id.starts_with("sha256:") && id.len() > 7)
    };
    if image("browser_image").is_none() || image("runtime_image").is_none() {
        gaps.push("image_unbound".to_owned());
    }
    if actor["status"] != "passed" {
        gaps.push("actor_test_incomplete".to_owned());
    }
    let reported_bot: Vec<Value> = report
        .as_ref()
        .map(|r| {
            r["signals"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| s["verdict"] == "BOT")
                .map(|s| s["id"].clone())
                .collect()
        })
        .unwrap_or_default();
    let verdict = decision.as_ref().and_then(|d| d["verdict"].as_str());
    let effects = decision.as_ref().and_then(|d| d["origin_effects"].as_u64());
    let outcome = match (verdict, effects) {
        (Some("ALLOW"), Some(1)) => Some("ceiling_reached"),
        (Some("DENY" | "CHALLENGE"), Some(0)) => Some("ceiling_not_reached"),
        (None, _) => None,
        _ => {
            gaps.push("misclassified".to_owned());
            None
        }
    };
    let coherent = actor_coherent(gate, actor);
    if coherent == Some(false) {
        gaps.push("actor_not_coherent".to_owned());
    }
    gaps.sort();
    gaps.dedup();
    // A classification is a T4 ceiling statement only for a coherent actor.
    let classification = outcome.filter(|_| coherent == Some(true));
    let status = if !gaps.is_empty() {
        "failed"
    } else if coherent == Some(true) {
        "passed"
    } else {
        "not_evaluated"
    };
    let observation = json!({"schema":SCHEMA,"tier":"T4","epoch":EPOCH,
        "classification":classification,"verdict":verdict,
        "status":decision.as_ref().map(|d| d["status"].clone()),
        "ack":decision.as_ref().map(|d| d["ack"].clone()),
        "audit_seq":decision.as_ref().map(|d| d["seq"].clone()),
        "origin_effects":effects,
        "block_credit":0,"detection_claim":false,
        "pace":{"declared":known["pace"],"gate_elapsed_ms":gate_elapsed,
            "browser_elapsed_ms":actor_elapsed},
        "actor":{"browser_image":provenance["browser_image"],"user_agent":user_agent,
            "engine_version":engine,"guard_reported":report.as_ref().map(|r| r["guard"]["reported"].clone()),
            "report_sha256":epoch_gate["collector_reports"].get(0).map(|r| r["sha256"].clone()),
            "reported_bot_signals":reported_bot,
            "automation_disclosed":!reported_bot.is_empty(),
            "actor_coherent":coherent},
        "runtime_image":provenance["runtime_image"],
        "session":{"sessions":gate["epochs"][index()]["sessions"],
            "shared_sessions":gate["epochs"][index()]["shared_sessions"]}});
    json!({"status":status,"passed":status == "passed","gaps":gaps,"observation":observation})
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../../crates/collector/pilot/fixture.json"
        ))
        .unwrap()
    }

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

    fn entity(guard: bool) -> Value {
        let report = json!({"userAgent":"Mozilla/5.0 (X11; Linux x86_64; rv:153.0) Gecko/20100101 Firefox/153.0",
            "engineVersion":"rust-wasm-0.1.0","guard":{"reported":guard},
            "signals":[{"id":"l1.navigator.webdriver","verdict":"BOT"},
                {"id":"l1.plugins","verdict":"HUMAN"}]});
        let bytes = serde_json::to_vec(&report).unwrap();
        json!({"sha256":format!("{:x}", Sha256::digest(&bytes)),"bytes":bytes.len(),
            "base64":encode(&bytes),"oversize":false})
    }

    fn provenance() -> Value {
        json!({"browser_image":"sha256:b","runtime_image":"sha256:r"})
    }

    /// A bound three-epoch run whose ceiling decision was observed as
    /// `verdict` with `status`, and `effects` origin contacts, from an actor
    /// attesting `actor_coherent: true`.
    fn run(verdict: &str, status: u64, effects: u64) -> (Value, Value, Value) {
        let (gate, mut actor, expected) = incoherent_run(verdict, status, effects);
        actor["epochs"][2]["actor_coherent"] = json!(true);
        (gate, actor, expected)
    }

    /// The same run as the current lab produces it: no `actor_coherent`.
    fn incoherent_run(verdict: &str, status: u64, effects: u64) -> (Value, Value, Value) {
        let fixture = fixture();
        let expected = crate::browser_expectations::from_fixture(&fixture).unwrap();
        let required = route(&fixture, "required_collector");
        let ceiling_hits = json!({"/_humanymous/browser/session":1,
            "/_humanymous/browser/collect":1,"/requires-collector":1,
            "response:/_humanymous/browser/session:200:ack_false":1,
            "response:/_humanymous/browser/collect:200:ack_true":1,
            format!("response:{required}:{status}:ack_true"):1});
        let epoch = |hits: Value, reports: Value, seqs: Value| {
            json!({"proxy_hits":hits,"sessions":1,"shared_sessions":0,
                "collector_reports":reports,"audit_seqs":seqs,"elapsed_ms":5600})
        };
        let mut records = expected["audit_records"].as_array().unwrap().clone();
        records.push(json!({"seq":7,"event_type":"runtime.decision","verdict":verdict}));
        let gate = json!({"proxy_hits":{},"origin_effects":effects,
            "collector_reports":[entity(false),entity(true),entity(true)],
            "epochs":[epoch(json!({}),json!([0]),json!([1,2,3])),
                epoch(json!({}),json!([1]),json!([4,5])),
                epoch(ceiling_hits.clone(),json!([2]),json!([6,7]))],
            "audit_unattributed":0,"audit_records":records,
            "missing_signal_controls":{"epoch":"stripped"}});
        let actor = json!({"status":"passed","proxy_hits":{},
            "epochs":[epoch(json!({}),json!([]),Value::Null),
                epoch(json!({}),json!([]),Value::Null),
                epoch(ceiling_hits,json!([]),Value::Null)]});
        (gate, actor, expected)
    }

    fn evaluated(gate: &Value, actor: &Value, expected: &Value) -> Value {
        evaluate(gate, actor, expected, &fixture(), &provenance())
    }

    #[test]
    fn absent_actor_coherent_is_not_evaluated_never_passed() {
        // The current lab: sound, bound evidence but no coherent actor.
        for (verdict, status, effects) in
            [("DENY", 403, 0), ("CHALLENGE", 403, 0), ("ALLOW", 200, 1)]
        {
            let (gate, actor, expected) = incoherent_run(verdict, status, effects);
            let result = evaluated(&gate, &actor, &expected);
            assert_eq!(result["gaps"], json!([]), "{result}");
            assert_eq!(result["status"], "not_evaluated", "{verdict}");
            assert_eq!(result["passed"], false, "{verdict}");
            let observation = &result["observation"];
            assert_eq!(observation["verdict"], verdict);
            assert_eq!(observation["classification"], Value::Null);
            assert_eq!(observation["actor"]["actor_coherent"], Value::Null);
            assert_eq!(observation["block_credit"], 0);
            assert_eq!(observation["detection_claim"], false);
        }
        // Absence never masks broken evidence: any gap still fails.
        let (mut gate, actor, expected) = incoherent_run("DENY", 403, 0);
        gate["epochs"][2]["elapsed_ms"] = json!(5499);
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(result["status"], "failed", "{result}");
        assert_eq!(result["gaps"], json!(["pace_not_observed"]));
        // An attestation on unbound epochs is not read at all.
        let (mut gate, actor, expected) = run("DENY", 403, 0);
        gate["epochs"][2]["shared_sessions"] = json!(1);
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(result["status"], "failed");
        assert_eq!(
            result["observation"]["actor"]["actor_coherent"],
            Value::Null
        );
    }

    #[test]
    fn actor_coherent_false_or_malformed_fails() {
        for value in [json!(false), json!("true"), json!(1), Value::Null] {
            let (gate, mut actor, expected) = run("DENY", 403, 0);
            actor["epochs"][2]["actor_coherent"] = value.clone();
            let result = evaluated(&gate, &actor, &expected);
            assert_eq!(result["status"], "failed", "{value}");
            assert_eq!(result["passed"], false, "{value}");
            assert_eq!(result["gaps"], json!(["actor_not_coherent"]), "{value}");
            assert_eq!(result["observation"]["classification"], Value::Null);
            assert_eq!(result["observation"]["actor"]["actor_coherent"], false);
        }
        // An attestation on another epoch does not count for the ceiling.
        let (gate, mut actor, expected) = incoherent_run("ALLOW", 200, 1);
        actor["epochs"][1]["actor_coherent"] = json!(true);
        assert_eq!(
            evaluated(&gate, &actor, &expected)["status"],
            "not_evaluated"
        );
    }

    #[test]
    fn actor_coherent_true_with_the_required_evidence_passes() {
        for (verdict, status, effects, classification) in [
            ("ALLOW", 200, 1, "ceiling_reached"),
            ("DENY", 403, 0, "ceiling_not_reached"),
        ] {
            let (gate, actor, expected) = run(verdict, status, effects);
            let result = evaluated(&gate, &actor, &expected);
            assert_eq!(result["gaps"], json!([]), "{result}");
            assert_eq!(result["status"], "passed");
            assert_eq!(result["passed"], true);
            assert_eq!(result["observation"]["classification"], classification);
            assert_eq!(result["observation"]["actor"]["actor_coherent"], true);
        }
        // Coherence never excuses a missing piece of the other evidence.
        let (gate, mut actor, expected) = run("DENY", 403, 0);
        actor["status"] = json!("failed");
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(result["status"], "failed");
        assert_eq!(result["gaps"], json!(["actor_test_incomplete"]));
    }

    #[test]
    fn deny_is_recorded_as_is_with_the_actors_own_disclosed_signals() {
        let (gate, actor, expected) = run("DENY", 403, 0);
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(result["gaps"], json!([]), "{result}");
        let observation = &result["observation"];
        assert_eq!(observation["classification"], "ceiling_not_reached");
        assert_eq!(observation["verdict"], "DENY");
        assert_eq!(observation["status"], 403);
        assert_eq!(observation["origin_effects"], 0);
        assert_eq!(observation["block_credit"], 0);
        assert_eq!(observation["detection_claim"], false);
        assert_eq!(
            observation["actor"]["reported_bot_signals"],
            json!(["l1.navigator.webdriver"])
        );
        assert_eq!(observation["actor"]["automation_disclosed"], true);
        assert_eq!(observation["pace"]["gate_elapsed_ms"], 5600);
        assert_eq!(observation["runtime_image"], "sha256:r");
        let (gate, actor, expected) = run("CHALLENGE", 403, 0);
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(
            result["observation"]["classification"],
            "ceiling_not_reached"
        );
        assert_eq!(result["passed"], true);
    }

    #[test]
    fn allow_forwarded_once_is_ceiling_reached_and_never_block_credit() {
        let (gate, actor, expected) = run("ALLOW", 200, 1);
        let result = evaluated(&gate, &actor, &expected);
        assert_eq!(result["gaps"], json!([]), "{result}");
        assert_eq!(result["observation"]["classification"], "ceiling_reached");
        assert_eq!(result["observation"]["block_credit"], 0);
        assert_eq!(result["observation"]["detection_claim"], false);
        // The filled expectation now matches the whole run exactly, and only
        // the ceiling's own forwarded effect is charged to the whole run.
        let observed = super::decision(&gate, &actor, &expected, &fixture()).unwrap();
        let whole = with_decision(&expected, Some(&observed));
        assert_eq!(whole["audit_records"], gate["audit_records"]);
        assert_eq!(whole["origin_effects"], 1);
        assert_eq!(
            whole["responses"]["required_collector"],
            json!([{"status":422,"ack":false,"count":1},{"status":403,"ack":true,"count":1},
                {"status":200,"ack":true,"count":1}])
        );
        assert_eq!(ceiling_allows(&gate, &actor), 1);
        // A DENY with the same 403 as the complete epoch merges its count.
        let (gate, actor, expected) = run("DENY", 403, 0);
        let observed = super::decision(&gate, &actor, &expected, &fixture()).unwrap();
        let whole = with_decision(&expected, Some(&observed));
        assert_eq!(whole["responses"]["required_collector"][1]["count"], 2);
        assert_eq!(whole["origin_effects"], 0);
        assert_eq!(with_decision(&expected, None), expected);
    }

    #[test]
    fn inconsistent_effects_are_misclassification_or_unattributed() {
        for (verdict, status, effects) in
            [("ALLOW", 200, 0), ("DENY", 403, 1), ("CHALLENGE", 403, 2)]
        {
            let (gate, actor, expected) = run(verdict, status, effects);
            let result = evaluated(&gate, &actor, &expected);
            assert_eq!(result["passed"], false, "{verdict}");
            assert!(
                result["gaps"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("origin_effect_unattributed")),
                "{result}"
            );
            assert_eq!(result["observation"]["classification"], Value::Null);
        }
        // An ALLOW anywhere but the ceiling epoch is never charged to it.
        let (mut gate, actor, expected) = run("ALLOW", 200, 2);
        gate["audit_records"][4]["verdict"] = json!("ALLOW");
        assert_eq!(evaluated(&gate, &actor, &expected)["passed"], false);
    }

    #[test]
    fn missing_unbound_unpaced_or_unprovenanced_evidence_fails() {
        let (gate, actor, expected) = run("DENY", 403, 0);
        let gap = |gate: &Value, actor: &Value, provenance: &Value, name: &str| {
            let result = evaluate(gate, actor, &expected, &fixture(), provenance);
            assert_eq!(result["passed"], false, "{name}");
            assert!(
                result["gaps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g.as_str().is_some_and(|g| g.starts_with(name))),
                "{name}: {result}"
            );
        };
        let mut missing = gate.clone();
        missing["epochs"][2]["audit_seqs"] = json!([6]);
        gap(
            &missing,
            &actor,
            &provenance(),
            "epoch_binding:audit_unattributed",
        );
        let mut unknown = gate.clone();
        unknown["audit_records"][6]["verdict"] = json!("PASS");
        gap(&unknown, &actor, &provenance(), "ceiling_decision_unbound");
        let mut shared = gate.clone();
        shared["epochs"][2]["shared_sessions"] = json!(1);
        gap(
            &shared,
            &actor,
            &provenance(),
            "epoch_binding:gate_epoch_session",
        );
        let mut two = actor.clone();
        two["epochs"].as_array_mut().unwrap().pop();
        gap(
            &gate,
            &two,
            &provenance(),
            "epoch_binding:browser_epoch_count",
        );
        let mut unacked = gate.clone();
        let mut hits = unacked["epochs"][2]["proxy_hits"]
            .as_object()
            .unwrap()
            .clone();
        hits.remove("response:/requires-collector:403:ack_true");
        hits.insert(
            "response:/requires-collector:403:ack_false".into(),
            json!(1),
        );
        unacked["epochs"][2]["proxy_hits"] = json!(hits);
        let mut actor_unacked = actor.clone();
        actor_unacked["epochs"][2]["proxy_hits"] = json!(hits);
        gap(
            &unacked,
            &actor_unacked,
            &provenance(),
            "ceiling_decision_unacknowledged",
        );
        let mut diverged = actor.clone();
        diverged["epochs"][2]["proxy_hits"]["response:/requires-collector:403:ack_true"] = json!(2);
        gap(&gate, &diverged, &provenance(), "ceiling_response_unbound");
        let mut fast = gate.clone();
        fast["epochs"][2]["elapsed_ms"] = json!(5499);
        gap(&fast, &actor, &provenance(), "pace_not_observed");
        let mut unmeasured = actor.clone();
        unmeasured["epochs"][2]
            .as_object_mut()
            .unwrap()
            .remove("elapsed_ms");
        gap(&gate, &unmeasured, &provenance(), "pace_not_observed");
        let mut stripped = gate.clone();
        stripped["collector_reports"][2] = entity(false);
        gap(
            &stripped,
            &actor,
            &provenance(),
            "ceiling_hooks_not_opted_in",
        );
        let mut tampered = gate.clone();
        tampered["collector_reports"][2]["sha256"] = json!("00");
        gap(
            &tampered,
            &actor,
            &provenance(),
            "ceiling_report_unavailable",
        );
        let mut extra = gate.clone();
        extra["epochs"][2]["proxy_hits"]["/effect"] = json!(1);
        gap(
            &extra,
            &actor,
            &provenance(),
            "ceiling_request_shape:effect",
        );
        gap(
            &gate,
            &actor,
            &json!({"browser_image":"sha256:b"}),
            "image_unbound",
        );
        let mut failed = actor.clone();
        failed["status"] = json!("failed");
        gap(&gate, &failed, &provenance(), "actor_test_incomplete");
    }
}
