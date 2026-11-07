//! Consume the lab owner's single safe diagnostic vocabulary without raw detail.
use crate::{Result, require};
use serde_json::{Value, json};

#[path = "../../../../test/lab/browser_outcome.rs"]
mod browser_outcome;

pub fn validate(report: &Value) -> Result<Option<Value>> {
    let Some(diagnostic) = report.get("safe_diagnostics") else {
        return Ok(None);
    };
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../test/lab/runner/safe-diagnostics.json"
    ))
    .map_err(|e| e.to_string())?;
    require(
        contract["schema"] == "humanymous.lab-safe-diagnostics-contract/1",
        "lab diagnostic contract schema mismatch",
    )?;
    require(
        diagnostic["schema"] == contract["report_schema"] && diagnostic["partial"] == false,
        "incomplete or unknown lab diagnostic schema",
    )?;
    let rows = diagnostic["failures"]
        .as_array()
        .ok_or("lab diagnostic failures absent")?;
    let max_rows = contract["max_failures"]
        .as_u64()
        .ok_or("diagnostic bound absent")?;
    let max_id = contract["case_id_max"]
        .as_u64()
        .ok_or("diagnostic ID bound absent")?;
    let codes = contract["error_codes"]
        .as_array()
        .ok_or("diagnostic error vocabulary absent")?;
    require(
        rows.len() as u64 <= max_rows
            && diagnostic["failure_count"].as_u64() == Some(rows.len() as u64),
        "lab diagnostic count mismatch",
    )?;
    let mut failures = Vec::new();
    for row in rows {
        let id = row["case_id"].as_str().ok_or("diagnostic case ID absent")?;
        require(
            !id.is_empty()
                && id.len() as u64 <= max_id
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-')),
            "unsafe lab diagnostic case ID",
        )?;
        require(
            row["status"] == "failed" && codes.contains(&row["error_code"]),
            "unknown lab diagnostic status/error code",
        )?;
        failures.push(json!({"case_id":id,"status":"failed","error_code":row["error_code"]}));
    }
    let mut public = json!({"schema":diagnostic["schema"],"failures":failures,"failure_count":rows.len(),"partial":false});
    if let Some(outcome) = diagnostic.get("browser_actor") {
        require(
            contract["optional_fields"] == json!(["browser_actor"])
                && contract["browser_actor_contract"] == "browser-outcome-contract.json",
            "unknown optional lab diagnostic contract",
        )?;
        public["browser_actor"] = browser_outcome::public_outcome(outcome)
            .map_err(|_| "invalid owned browser outcome".to_owned())?;
    }
    Ok(Some(public))
}

pub fn public(report: &Value) -> Value {
    match validate(report) {
        Ok(Some(value)) => value,
        Ok(None) => Value::Null,
        Err(_) => json!({"status":"rejected","error_code":"invalid_lab_safe_diagnostics"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> Value {
        json!({"safe_diagnostics":{"schema":"humanymous.lab-safe-diagnostics/1","failures":[{"case_id":"browser_actor","status":"failed","error_code":"assertion_failed","detail":"private-canary"}],"failure_count":1,"partial":false,"raw":"private-canary"}})
    }
    #[test]
    fn only_structured_owner_diagnostics_are_public() {
        let value = public(&report());
        assert_eq!(value["failure_count"], 1);
        assert!(!value.to_string().contains("private-canary"));
        assert!(public(&json!({})).is_null());
    }
    #[test]
    fn invalid_and_partial_diagnostics_fail_closed_without_echo() {
        for mutation in 0..4 {
            let mut value = report();
            let diagnostic = &mut value["safe_diagnostics"];
            match mutation {
                0 => diagnostic["partial"] = json!(true),
                1 => diagnostic["failure_count"] = json!(0),
                2 => diagnostic["failures"][0]["case_id"] = json!("private-canary/path"),
                _ => diagnostic["failures"][0]["error_code"] = json!("private-canary"),
            }
            assert!(validate(&value).is_err());
            assert_eq!(public(&value)["status"], "rejected");
            assert!(!public(&value).to_string().contains("private-canary"));
        }
    }
    #[test]
    fn optional_browser_outcome_reuses_owner_validation_and_rejects_the_whole_projection() {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../../test/lab/runner/browser-outcome-contract.json"
        ))
        .unwrap();
        let mut outcome = json!({});
        for field in contract["fields"].as_array().unwrap() {
            outcome[field.as_str().unwrap()] = Value::Null;
        }
        outcome["schema"] = contract["outcome_schema"].clone();
        outcome["state"] = json!("not_observed");
        outcome["dns_finish"] = json!("unknown");
        outcome["source_prefix_authenticated"] = json!(false);
        outcome["full_product_qualified"] = json!(false);
        let mut value = report();
        value["safe_diagnostics"]["browser_actor"] = outcome.clone();
        assert_eq!(public(&value)["browser_actor"], outcome);
        assert!(public(&value)["browser_actor"]["inventory_count"].is_null());
        for mutation in 0..3 {
            let mut changed = value.clone();
            match mutation {
                0 => {
                    changed["safe_diagnostics"]["browser_actor"]["private-path"] =
                        json!("private-canary")
                }
                1 => changed["safe_diagnostics"]["browser_actor"]["inventory_count"] = json!(0),
                _ => {
                    changed["safe_diagnostics"]["browser_actor"]["full_product_qualified"] =
                        json!(true)
                }
            }
            assert!(validate(&changed).is_err());
            let rejected = public(&changed);
            assert_eq!(rejected["status"], "rejected");
            assert!(!rejected.to_string().contains("private-canary"));
            assert!(rejected.get("failures").is_none());
        }
    }
}
