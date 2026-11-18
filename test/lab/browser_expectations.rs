//! Pure expectations from the sole collector fixture; never observed outcomes.
//!
//! The selected pilot runs three session epochs in one browser, in this order:
//! `stripped` (no guard hooks: pre-report CHALLENGEs, a collected partial
//! report, then a pre-consumption 422 without ACK that closes the epoch),
//! `complete` (the page explicitly enables guard hooks: a collected report
//! with fresh provenance, then a consumed, acknowledged non-ALLOW decision on
//! the required-collector route) and `ceiling` (the T4 ceiling observation:
//! the same opt-in at the fixture's declared pace). The ceiling decision is
//! never an expectation: only its request shape, its collect and the audit
//! sequence slot it occupies are fixed here; its verdict, response and origin
//! effect are observed by the lab. The top-level counts are the whole run
//! without that observed decision; each epoch keeps the single-epoch shape its
//! evaluator consumes.
use serde_json::{Map, Value, json};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// Session epochs in browser execution order.
pub const EPOCHS: [&str; 3] = ["stripped", "complete", "ceiling"];
/// Verdicts a consumed ceiling decision may be observed with; none is expected.
pub const CEILING_VERDICTS: [&str; 3] = ["ALLOW", "DENY", "CHALLENGE"];
pub fn from_fixture(fixture: &Value) -> Result<Value> {
    let expected = fixture["expected"]
        .as_object()
        .ok_or("browser fixture expected object required")?;
    const FIELDS: [&str; 10] = [
        "before_report_status",
        "effect_requests",
        "before_report_verdict",
        "collect_status",
        "required_collector_status",
        "required_collector_ack",
        "complete_required_collector_status",
        "complete_required_collector_verdict",
        "complete_required_collector_ack",
        "origin_effects",
    ];
    const ROUTES: [&str; 4] = ["session", "collect", "effect", "required_collector"];
    let routes = fixture["routes"]
        .as_object()
        .ok_or("browser fixture routes required")?;
    if fixture["schema"] != "humanymous.browser-runtime-pilot/1"
        || expected.len() != FIELDS.len()
        || expected.keys().any(|k| !FIELDS.contains(&k.as_str()))
        || routes.len() != ROUTES.len()
        || routes.keys().any(|k| !ROUTES.contains(&k.as_str()))
    {
        return Err("browser fixture missing or unknown expectation/route fields".into());
    }
    let count = fixture["expected"]["effect_requests"]
        .as_u64()
        .filter(|n| (1..=32).contains(n))
        .ok_or("browser effect request count missing or invalid")?;
    let status = |key: &str| -> Result<u64> {
        fixture["expected"][key]
            .as_u64()
            .filter(|n| (100..=599).contains(n))
            .ok_or_else(|| "browser expected HTTP status invalid".into())
    };
    // The admitted decision must stay local (no origin effect): an ALLOW would
    // be forwarded, so only a consumed DENY or CHALLENGE fits this pilot.
    let admitted = &fixture["expected"]["complete_required_collector_verdict"];
    if fixture["expected"]["before_report_verdict"] != "CHALLENGE"
        || fixture["expected"]["required_collector_ack"] != false
        || fixture["expected"]["complete_required_collector_ack"] != true
        || !matches!(admitted.as_str(), Some("DENY" | "CHALLENGE"))
        || status("complete_required_collector_status")? == status("required_collector_status")?
        || fixture["expected"]["origin_effects"] != 0
    {
        return Err("browser fail-closed fixture expectations refused".into());
    }
    let response = |status: u64, ack: bool| json!({"status":status,"ack":ack});
    let record = |seq: u64, event: &str, verdict: &Value| json!({"seq":seq,"event_type":event,"verdict":verdict});
    let mut stripped_records: Vec<_> = (1..=count)
        .map(|seq| {
            record(
                seq,
                "runtime.decision",
                &fixture["expected"]["before_report_verdict"],
            )
        })
        .collect();
    stripped_records.push(record(count + 1, "runtime.collect", &json!("COLLECTED")));
    let stripped = json!({"effect_requests":count,
        "request_counts":{"session":1,"effect":count,"collect":1,"required_collector":1},
        "responses":{"effect":response(status("before_report_status")?,true),
            "collect":response(status("collect_status")?,true),
            "required_collector":response(status("required_collector_status")?,false)},
        "audit_sequence":count+1,"audit_records":stripped_records,"origin_effects":0});
    let complete_records = vec![
        record(count + 2, "runtime.collect", &json!("COLLECTED")),
        record(count + 3, "runtime.decision", admitted),
    ];
    let complete = json!({"effect_requests":0,
        "request_counts":{"session":1,"effect":0,"collect":1,"required_collector":1},
        "responses":{"collect":response(status("collect_status")?,true),
            "required_collector":response(status("complete_required_collector_status")?,true)},
        "audit_sequence":count+3,"audit_records":complete_records,"origin_effects":0});
    // The declared pace is the actor's definition, chosen before any run and
    // never adjusted to a verdict.
    let pace = fixture["ceiling"]
        .as_object()
        .filter(|pace| pace.len() == 2)
        .ok_or("browser ceiling pace must declare exactly dwell_ms and think_ms")?;
    let millis = |key: &str| {
        pace.get(key)
            .and_then(Value::as_u64)
            .filter(|ms| (1000..=30_000).contains(ms))
            .ok_or("browser ceiling pace invalid")
    };
    let (dwell, think) = (millis("dwell_ms")?, millis("think_ms")?);
    let ceiling = json!({"effect_requests":0,
        "request_counts":{"session":1,"effect":0,"collect":1,"required_collector":1},
        "responses":{"collect":response(status("collect_status")?,true)},
        "audit_sequence":count+5,
        "audit_records":[record(count + 4, "runtime.collect", &json!("COLLECTED"))],
        "observed":{"route":"required_collector","decision_seq":count+5,
            "verdicts":CEILING_VERDICTS},
        "pace":{"dwell_ms":dwell,"think_ms":think,"minimum_elapsed_ms":dwell+think}});
    // Whole-run accounting: summed request counts and every distinct response.
    let mut request_counts = Map::new();
    let mut responses = Map::new();
    for epoch in [&stripped, &complete, &ceiling] {
        for route in ROUTES {
            let n = epoch["request_counts"][route].as_u64().unwrap_or(0);
            let total = request_counts.entry(route).or_insert(json!(0));
            *total = json!(total.as_u64().unwrap_or(0) + n);
            let one = &epoch["responses"][route];
            if one.is_null() || n == 0 {
                continue;
            }
            let list = responses
                .entry(route)
                .or_insert(json!([]))
                .as_array_mut()
                .ok_or("browser response list")?;
            match list
                .iter_mut()
                .find(|r| r["status"] == one["status"] && r["ack"] == one["ack"])
            {
                Some(found) => found["count"] = json!(found["count"].as_u64().unwrap_or(0) + n),
                None => list.push(json!({"status":one["status"],"ack":one["ack"],"count":n})),
            }
        }
    }
    let records: Vec<Value> = [&stripped, &complete, &ceiling]
        .iter()
        .flat_map(|epoch| {
            epoch["audit_records"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    // `origin_effects` is the known epochs' zero; a forwarded ceiling ALLOW
    // is an observation added by the lab, never part of this expectation.
    Ok(
        json!({"effect_requests":count,"request_counts":request_counts,"responses":responses,
        "audit_sequence":count+5,"audit_records":records,"origin_effects":0,
        "epochs":{"stripped":stripped,"complete":complete,"ceiling":ceiling}}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../crates/collector/pilot/fixture.json")).unwrap()
    }
    #[test]
    fn one_authority_controls_requests_responses_and_authenticated_audit_denominator() {
        for count in [1, 2] {
            let mut fixture = fixture();
            fixture["expected"]["effect_requests"] = json!(count);
            let value = from_fixture(&fixture).unwrap();
            assert_eq!(value["request_counts"]["effect"], count);
            assert_eq!(value["request_counts"]["session"], 3);
            assert_eq!(value["request_counts"]["collect"], 3);
            assert_eq!(value["request_counts"]["required_collector"], 3);
            // The sequence includes the ceiling decision slot; its record is
            // observed, so the known records stop one short of it.
            assert_eq!(value["audit_sequence"], count + 5);
            assert_eq!(
                value["audit_records"].as_array().unwrap().len(),
                count as usize + 4
            );
            assert_eq!(
                value["responses"]["effect"],
                json!([{"status":fixture["expected"]["before_report_status"],"ack":true,"count":count}])
            );
            assert_eq!(
                value["responses"]["required_collector"],
                json!([{"status":422,"ack":false,"count":1},{"status":403,"ack":true,"count":1}])
            );
            assert_eq!(
                value["responses"]["collect"],
                json!([{"status":200,"ack":true,"count":3}])
            );
            let ceiling = &value["epochs"]["ceiling"];
            assert_eq!(ceiling["responses"]["required_collector"], Value::Null);
            assert_eq!(ceiling["request_counts"]["required_collector"], 1);
            assert_eq!(
                ceiling["audit_records"],
                json!([{"seq":count+4,"event_type":"runtime.collect","verdict":"COLLECTED"}])
            );
            assert_eq!(ceiling["observed"]["decision_seq"], count + 5);
            assert_eq!(
                ceiling["pace"],
                json!({"dwell_ms":4000,"think_ms":1500,"minimum_elapsed_ms":5500})
            );
            assert_eq!(ceiling["origin_effects"], Value::Null);
            // Each epoch keeps the single-epoch shape and owns a disjoint,
            // contiguous slice of the authenticated audit.
            let stripped = &value["epochs"]["stripped"];
            let complete = &value["epochs"]["complete"];
            assert_eq!(stripped["request_counts"]["collect"], 1);
            assert_eq!(stripped["audit_sequence"], count + 1);
            assert_eq!(stripped["responses"]["required_collector"]["status"], 422);
            assert_eq!(stripped["responses"]["required_collector"]["ack"], false);
            assert_eq!(complete["effect_requests"], 0);
            assert_eq!(complete["audit_sequence"], count + 3);
            assert_eq!(complete["responses"]["required_collector"]["ack"], true);
            assert_eq!(
                complete["audit_records"],
                json!([{"seq":count+2,"event_type":"runtime.collect","verdict":"COLLECTED"},
                    {"seq":count+3,"event_type":"runtime.decision","verdict":"DENY"}])
            );
            let joined: Vec<Value> = EPOCHS
                .iter()
                .flat_map(|id| {
                    value["epochs"][id]["audit_records"]
                        .as_array()
                        .unwrap()
                        .clone()
                })
                .collect();
            assert_eq!(json!(joined), value["audit_records"]);
        }
    }
    #[test]
    fn missing_zero_unknown_or_unbounded_expectations_never_fall_back_to_one() {
        for invalid in [
            Value::Null,
            json!(0),
            json!("unknown"),
            json!(33),
            json!(true),
        ] {
            let mut fixture = fixture();
            fixture["expected"]["effect_requests"] = invalid;
            assert!(from_fixture(&fixture).is_err());
        }
        let mut extra = fixture();
        extra["expected"]["unknown"] = json!(1);
        assert!(from_fixture(&extra).is_err());
        let mut extra = fixture();
        extra["routes"]["unknown"] = json!("/unknown");
        assert!(from_fixture(&extra).is_err());
        let mut missing = fixture();
        missing["expected"]
            .as_object_mut()
            .unwrap()
            .remove("effect_requests");
        assert!(from_fixture(&missing).is_err());
    }
    #[test]
    fn ceiling_pace_is_declared_exactly_and_carries_no_verdict() {
        for (field, value) in [
            ("dwell_ms", json!(999)),
            ("think_ms", json!(30_001)),
            ("dwell_ms", json!("4000")),
            ("think_ms", Value::Null),
            ("verdict", json!("ALLOW")),
        ] {
            let mut fixture = fixture();
            fixture["ceiling"][field] = value;
            assert!(from_fixture(&fixture).is_err(), "{field}");
        }
        let mut missing = fixture();
        missing.as_object_mut().unwrap().remove("ceiling");
        assert!(from_fixture(&missing).is_err());
    }
    #[test]
    fn complete_epoch_must_be_a_consumed_local_decision() {
        for (field, value) in [
            ("complete_required_collector_ack", json!(false)),
            ("complete_required_collector_verdict", json!("ALLOW")),
            ("complete_required_collector_verdict", json!("")),
            ("complete_required_collector_status", json!(422)),
            ("complete_required_collector_status", json!(99)),
        ] {
            let mut fixture = fixture();
            fixture["expected"][field] = value;
            assert!(from_fixture(&fixture).is_err(), "{field}");
        }
        let mut missing = fixture();
        missing["expected"]
            .as_object_mut()
            .unwrap()
            .remove("complete_required_collector_verdict");
        assert!(from_fixture(&missing).is_err());
    }
}
