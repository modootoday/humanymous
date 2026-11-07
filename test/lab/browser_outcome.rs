//! One closed scalar diagnostic contract; never browser or witness qualification.
use serde_json::{Value, json};

pub fn public_outcome(value: &Value) -> Result<Value, &'static str> {
    let contract: Value =
        serde_json::from_str(include_str!("runner/browser-outcome-contract.json"))
            .map_err(|_| "invalid owned browser outcome contract")?;
    let object = value.as_object().ok_or("browser outcome object required")?;
    let fields = contract["fields"]
        .as_array()
        .ok_or("outcome fields absent")?;
    if object.len() != fields.len()
        || object
            .keys()
            .any(|key| !fields.iter().any(|field| field.as_str() == Some(key)))
        || value["schema"] != contract["outcome_schema"]
        || value["source_prefix_authenticated"] != false
        || value["full_product_qualified"] != false
    {
        return Err("browser outcome fields or constants rejected");
    }
    let state = value["state"].as_str().ok_or("outcome state absent")?;
    if !contract["states"]
        .as_array()
        .ok_or("outcome states absent")?
        .iter()
        .any(|allowed| allowed.as_str() == Some(state))
        || !contract["dns_states"]
            .as_array()
            .ok_or("DNS states absent")?
            .contains(&value["dns_finish"])
    {
        return Err("unknown browser outcome state");
    }
    for field in contract["nullable_booleans"]
        .as_array()
        .ok_or("outcome boolean contract absent")?
    {
        let field = field.as_str().ok_or("outcome boolean field absent")?;
        if !value[field].is_null() && value[field].as_bool().is_none() {
            return Err("outcome boolean type rejected");
        }
    }
    if !value["exit_code"].is_null()
        && !value["exit_code"]
            .as_i64()
            .is_some_and(|code| i32::try_from(code).is_ok())
    {
        return Err("outcome exit code rejected");
    }
    for (field, bound) in [
        ("inventory_count", "maximum_inventory"),
        ("selected_count", "maximum_selected"),
        ("filtered_count", "maximum_filtered"),
    ] {
        let maximum = contract[bound].as_u64().ok_or("outcome bound absent")?;
        if !value[field].is_null() && !value[field].as_u64().is_some_and(|count| count <= maximum) {
            return Err("outcome count rejected");
        }
    }
    if state == "observed" {
        let process = value["process_success"]
            .as_bool()
            .ok_or("process unknown")?;
        let inventory = value["inventory_matched"]
            .as_bool()
            .ok_or("inventory unknown")?;
        let summary = value["parser_summary_matched"]
            .as_bool()
            .ok_or("summary unknown")?;
        let passed = value["test_passed"]
            .as_bool()
            .ok_or("test result unknown")?;
        let count = value["inventory_count"]
            .as_u64()
            .ok_or("inventory count unknown")?;
        let selected = value["selected_count"]
            .as_u64()
            .ok_or("selected count unknown")?;
        let filtered = value["filtered_count"]
            .as_u64()
            .ok_or("filtered count unknown")?;
        if count != selected + filtered
            || inventory && selected != 1
            || process && value["exit_code"].as_i64() != Some(0)
            || !process && value["exit_code"].as_i64() == Some(0)
            || passed != (process && inventory && summary)
            || value["dns_finish"] == "unknown"
        {
            return Err("contradictory browser outcome facts");
        }
    } else if contract["nullable_booleans"]
        .as_array()
        .ok_or("outcome boolean contract absent")?
        .iter()
        .chain(
            contract["nullable_counts"]
                .as_array()
                .ok_or("count fields absent")?,
        )
        .any(|field| field.as_str().is_none_or(|field| !value[field].is_null()))
        || !value["exit_code"].is_null()
        || value["dns_finish"] != "unknown"
    {
        return Err("unobserved facts must remain unknown");
    }
    let mut public = json!({});
    for field in fields {
        let field = field.as_str().ok_or("outcome field name absent")?;
        public[field] = value[field].clone();
    }
    Ok(public)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed() -> Value {
        json!({"schema":"humanymous.lab-browser-actor-outcome/1","state":"observed",
            "process_success":false,"exit_code":1,"inventory_matched":true,
            "inventory_count":67,"selected_count":1,"filtered_count":66,
            "parser_summary_matched":false,"test_passed":false,"dns_finish":"succeeded",
            "source_prefix_authenticated":false,"full_product_qualified":false})
    }

    #[test]
    fn scalar_outcomes_never_promote_process_exit_or_echo_extra_payloads() {
        let value = observed();
        assert_eq!(public_outcome(&value).unwrap(), value);
        let mut success = value.clone();
        success["process_success"] = json!(true);
        success["exit_code"] = json!(0);
        assert!(
            !public_outcome(&success).unwrap()["test_passed"]
                .as_bool()
                .unwrap()
        );
        success["parser_summary_matched"] = json!(true);
        success["test_passed"] = json!(true);
        assert_eq!(
            public_outcome(&success).unwrap()["full_product_qualified"],
            false
        );
        for (field, bad) in [
            ("raw", json!("PRIVATE_HEADER_BODY_PATH")),
            ("schema", json!("PRIVATE_SCHEMA")),
            ("state", json!("PRIVATE_STATUS")),
            ("process_success", json!(1)),
            ("exit_code", json!(2147483648_i64)),
            ("exit_code", json!(0)),
            ("exit_code", json!(1.5)),
            ("inventory_count", json!(10001)),
            ("selected_count", json!(2)),
            ("filtered_count", json!(-1)),
            ("filtered_count", json!(10001)),
            ("test_passed", json!(true)),
            ("dns_finish", json!("PRIVATE_DNS")),
            ("source_prefix_authenticated", json!(true)),
            ("full_product_qualified", json!(true)),
        ] {
            let mut changed = value.clone();
            changed[field] = bad;
            let error = public_outcome(&changed).unwrap_err();
            assert!(!error.contains("PRIVATE"));
        }
        let mut changed = value.clone();
        changed.as_object_mut().unwrap().remove("exit_code");
        assert!(public_outcome(&changed).is_err());
        changed = value;
        changed["exit_code"] = Value::Null;
        assert!(public_outcome(&changed).is_ok());
    }

    #[test]
    fn missing_invalid_and_capture_states_cannot_fabricate_zero_observations() {
        for state in [
            "not_observed",
            "missing",
            "invalid",
            "capture_deadline",
            "capture_failed",
        ] {
            let mut value = observed();
            value["state"] = json!(state);
            for field in [
                "process_success",
                "exit_code",
                "inventory_matched",
                "inventory_count",
                "selected_count",
                "filtered_count",
                "parser_summary_matched",
                "test_passed",
            ] {
                value[field] = Value::Null;
            }
            value["dns_finish"] = json!("unknown");
            assert_eq!(public_outcome(&value).unwrap(), value);
            value["inventory_count"] = json!(0);
            assert!(public_outcome(&value).is_err());
        }
    }
}
