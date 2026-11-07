//! Rust report checking: replay the pure engine or check owned-network evidence.
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
/// measured evidence without manufacturing a network run. Unknown schemas fail.
pub fn verify_bytes(bytes: &[u8]) -> Result<Verification, LabError> {
    if bytes.len() > 4_194_304 {
        return Err(LabError("report size budget"));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| LabError("report JSON"))?;
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
