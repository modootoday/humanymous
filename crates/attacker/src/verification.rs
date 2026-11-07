//! Rust report checking: replay the pure engine or check owned-network evidence.
mod json;
use crate::network::NetworkReport;
use humanymous_defender::lab::LabError;
use serde::Serialize;
use serde_json::Value;

/// Internally consistent evidence remains separate from product qualification.
#[derive(Debug, Serialize)]
pub struct Verification {
    /// Stable verifier output vocabulary.
    pub schema: &'static str,
    /// The input matched its current-source contract.
    pub internally_consistent: bool,
    /// Pure replay or owned-process network observations, never interchangeable.
    pub mode: &'static str,
    /// Scoped regression acceptance only.
    pub scoped_passed: bool,
    /// False for both currently implemented evidence modes.
    pub full_product_qualified: bool,
    /// Original denominator categories, not a reduced success percentage.
    pub totals: Value,
}

fn equivalent(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
                left == right
            } else if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
                left == right
            } else {
                left.as_f64()
                    .zip(right.as_f64())
                    .is_some_and(|(left, right)| left == right)
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| equivalent(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, left)| right.get(key).is_some_and(|right| equivalent(left, right)))
        }
        _ => left == right,
    }
}

/// Validate bounded report bytes. V1 runs the existing engine again; v2 checks
/// measured evidence without manufacturing a network run. Duplicate decoded
/// object keys and unknown fields fail; whitespace/key order and equivalent
/// JSON numeric spellings are not a canonical-byte or signature proof.
pub fn verify_bytes(bytes: &[u8]) -> Result<Verification, LabError> {
    let value = json::decode(bytes)?;
    let (mode, scoped_passed, full_product_qualified, totals) = match value["schema"].as_str() {
        Some("humanymous.wargame.v1") => {
            let seed = value["seed"].as_u64().ok_or(LabError("simulation seed"))?;
            let actual = crate::run(seed);
            let expected = serde_json::to_value(&actual)
                .map_err(|_| LabError("engine replay serialization"))?;
            if !equivalent(&value, &expected) {
                return Err(LabError(
                    "simulation report differs from current engine replay",
                ));
            }
            (
                "pure_engine_simulation",
                actual.regressions_pass(),
                false,
                value["totals"].clone(),
            )
        }
        Some("humanymous.wargame-network.v2") => {
            // Decode typed fields from original bytes, preserving duplicate-field
            // rejection rather than decoding a lossy intermediate object.
            let actual: NetworkReport =
                serde_json::from_slice(bytes).map_err(|_| LabError("network report schema"))?;
            actual.validate()?;
            (
                "owned_process_network",
                actual.scoped_passed,
                actual.full_product_qualified,
                value["totals"].clone(),
            )
        }
        _ => return Err(LabError("unsupported report schema")),
    };
    Ok(Verification {
        schema: "humanymous.wargame-verification.v1",
        internally_consistent: true,
        mode,
        scoped_passed,
        full_product_qualified,
        totals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_failure_fixture() -> Value {
        // A unit-only failed observation ledger, never a mock runtime success.
        let mut report = serde_json::to_value(crate::network::run(
            &mut crate::network::tests::Unavailable,
            42,
        ))
        .unwrap();
        let row = report["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == "missing-client-report")
            .unwrap();
        row["steps"] = serde_json::json!([{
            "id":"protected-request", "role":"attack", "http_status":null,
            "response_verdict":null, "error":"unit-only transport failure"
        }]);
        row["processes"] = serde_json::json!([{
            "runtime_sha256":"1".repeat(64), "supervisor_sha256":"2".repeat(64),
            "installation":"unit-synthetic", "revision":"r1",
            "binding":"supervisor-owned-child/private-pipe/same-loopback-only-namespace"
        }]);
        row["effects"] = serde_json::json!({
            "origin_effects":0, "audit":[{
                "seq":1, "event_type":"runtime.collect", "verdict":"COLLECTED",
                "risk_score":0, "rules":[], "record_hash":"3".repeat(64)
            }]
        });
        let verification = verify_bytes(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(!verification.scoped_passed);
        assert!(!verification.full_product_qualified);
        report
    }

    #[test]
    fn network_rejects_relabelled_reordered_and_extra_steps_but_retains_failure_prefixes() {
        let mut report = network_failure_fixture();
        let rows = report["rows"].as_array_mut().unwrap();
        let index = rows
            .iter()
            .position(|row| row["id"] == "rit-replay-after-control")
            .unwrap();
        // Failed unit ledger only: no process is run and no successful round is claimed.
        rows[index]["steps"] = serde_json::json!([
            {"id":"collect-asserted-fixture", "role":"setup", "http_status":200,
             "response_verdict":null, "error":null},
            {"id":"protected-request", "role":"synthetic_control", "http_status":200,
             "response_verdict":null, "error":null},
            {"id":"replayed-request", "role":"attack", "http_status":null,
             "response_verdict":null, "error":"unit-only transport failure"}
        ]);
        let check = |value: &Value| verify_bytes(&serde_json::to_vec(value).unwrap());
        assert!(!check(&report).unwrap().scoped_passed);
        for (field, value) in [("role", "attack"), ("id", "replayed-request")] {
            let mut changed = report.clone();
            changed["rows"][index]["steps"][1][field] = value.into();
            assert!(check(&changed).is_err(), "{field}");
        }
        let mut changed = report.clone();
        changed["rows"][index]["steps"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert!(check(&changed).is_err());
        let mut changed = report.clone();
        let steps = changed["rows"][index]["steps"].as_array_mut().unwrap();
        steps.push(steps[2].clone());
        assert!(check(&changed).is_err());
        for length in 0..=3 {
            let mut prefix = report.clone();
            prefix["rows"][index]["steps"]
                .as_array_mut()
                .unwrap()
                .truncate(length);
            assert!(!check(&prefix).unwrap().scoped_passed);
        }
    }

    #[test]
    fn simulation_rejects_duplicate_keys_before_semantic_replay() {
        let value = serde_json::to_value(crate::run(42)).unwrap();
        let raw = serde_json::to_string(&value).unwrap();
        for prefix in [r#""seed":0,"#, r#""\u0073chema":"humanymous.wargame.v1","#] {
            let duplicate = format!("{{{prefix}{}", &raw[1..]);
            assert!(verify_bytes(duplicate.as_bytes()).is_err());
        }
        let scheduled = format!("\"scheduled\":{}", value["totals"]["scheduled"]);
        let nested = raw.replacen(&scheduled, &format!("\"scheduled\":999,{scheduled}"), 1);
        assert_ne!(nested, raw);
        assert!(verify_bytes(nested.as_bytes()).is_err());
    }

    #[test]
    fn both_modes_reject_unknown_fields_at_every_report_structure() {
        let base = network_failure_fixture();
        let error_row = base["rows"]
            .as_array()
            .unwrap()
            .iter()
            .position(|row| row["id"] == "missing-client-report")
            .unwrap();
        for pointer in [
            String::new(),
            "/totals".into(),
            "/by_class/attack".into(),
            format!("/rows/{error_row}"),
            format!("/rows/{error_row}/steps/0"),
            format!("/rows/{error_row}/processes/0"),
            format!("/rows/{error_row}/effects"),
            format!("/rows/{error_row}/effects/audit/0"),
        ] {
            let mut changed = base.clone();
            changed
                .pointer_mut(&pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unknown_extension".into(), Value::Bool(true));
            assert!(
                verify_bytes(&serde_json::to_vec(&changed).unwrap()).is_err(),
                "{pointer}"
            );
        }
        let mut simulation = serde_json::to_value(crate::run(42)).unwrap();
        simulation["unknown_extension"] = Value::Bool(true);
        assert!(verify_bytes(&serde_json::to_vec(&simulation).unwrap()).is_err());
        let mut simulation = serde_json::to_value(crate::run(42)).unwrap();
        simulation["rows"][0]["unknown_extension"] = Value::Bool(true);
        assert!(verify_bytes(&serde_json::to_vec(&simulation).unwrap()).is_err());
    }

    #[test]
    fn network_rejects_duplicate_fields_and_dynamic_map_keys() {
        let base = network_failure_fixture();
        let raw = serde_json::to_string(&base).unwrap();
        let duplicate = format!(
            "{{\"schema\":\"humanymous.wargame-network.v2\",{}",
            &raw[1..]
        );
        assert!(verify_bytes(duplicate.as_bytes()).is_err());
        let entry = format!("\"attack\":{}", base["by_class"]["attack"]);
        let duplicate = raw.replacen(&entry, &format!("{entry},{entry}"), 1);
        assert_ne!(duplicate, raw);
        assert!(verify_bytes(duplicate.as_bytes()).is_err());
        let duplicate = raw.replacen("\"seq\":1", "\"seq\":0,\"seq\":1", 1);
        assert_ne!(duplicate, raw);
        assert!(verify_bytes(duplicate.as_bytes()).is_err());
    }
    #[test]
    fn simulation_replays_real_engine_and_preserves_integer_float_json_semantics() {
        let mut report = serde_json::to_value(crate::run(u64::MAX)).unwrap();
        let result = verify_bytes(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(result.scoped_passed);
        assert_eq!(result.mode, "pure_engine_simulation");
        assert!(!result.full_product_qualified);
        let row = report["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == "human-zero-motor")
            .unwrap();
        row["actual"]["result"]["riskScore"] = serde_json::json!(0);
        assert!(verify_bytes(&serde_json::to_vec(&report).unwrap()).is_ok());
    }
    #[test]
    fn mutated_counters_trace_and_unknown_schemas_are_not_replay_evidence() {
        let mut report = serde_json::to_value(crate::run(42)).unwrap();
        report["totals"]["allow"] = serde_json::json!(999);
        assert!(verify_bytes(&serde_json::to_vec(&report).unwrap()).is_err());
        let mut report = serde_json::to_value(crate::run(42)).unwrap();
        report["rows"][0]["provenance"] = serde_json::json!("real human");
        assert!(verify_bytes(&serde_json::to_vec(&report).unwrap()).is_err());
        let mut report = serde_json::to_value(crate::run(42)).unwrap();
        let row = report["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == "human-zero-motor")
            .unwrap();
        row["actual"]["trace"]["score"] = serde_json::json!(99);
        assert!(verify_bytes(&serde_json::to_vec(&report).unwrap()).is_err());
        for raw in [
            br#"{"schema":"unknown","seed":42}"#.as_slice(),
            b"not JSON",
            br#"{"schema":"humanymous.wargame.v1","seed":-1}"#,
        ] {
            assert!(verify_bytes(raw).is_err());
        }
    }
}
