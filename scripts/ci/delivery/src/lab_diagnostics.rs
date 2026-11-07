//! Consume the lab owner's single safe diagnostic vocabulary without raw detail.
use crate::{Result, require};
use serde_json::{Value, json};

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
    Ok(Some(
        json!({"schema":diagnostic["schema"],"failures":failures,"failure_count":rows.len(),"partial":false}),
    ))
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
}
