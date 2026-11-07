//! Extract only reviewed scalar observations from the original private actor JSON.
use crate::browser_outcome::public_outcome;
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};
use std::collections::HashSet;

pub(super) fn unavailable(state: &str) -> Value {
    public_outcome(
        &json!({"schema":"humanymous.lab-browser-actor-outcome/1","state":state,
        "process_success":null,"exit_code":null,"inventory_matched":null,"inventory_count":null,
        "selected_count":null,"filtered_count":null,"parser_summary_matched":null,
        "test_passed":null,"dns_finish":"unknown","source_prefix_authenticated":false,
        "full_product_qualified":false}),
    )
    .expect("owned non-observation contract")
}

fn extract(actor: &Value, fixture: &Value, expected: &Value) -> Result<Value, &'static str> {
    let contract: Value = serde_json::from_str(include_str!("../../browser-outcome-contract.json"))
        .map_err(|_| "owned actor contract invalid")?;
    let fields = contract["actor_fields"]
        .as_array()
        .ok_or("actor fields absent")?;
    let object = actor.as_object().ok_or("actor object absent")?;
    if object.len() != fields.len()
        || object
            .keys()
            .any(|key| !fields.iter().any(|field| field.as_str() == Some(key)))
        || actor["schema"] != contract["actor_schema"]
        || actor["expected"] != *expected
        || actor["full_product_qualified"] != false
        || actor["declared_selected"] != 1
    {
        return Err("private actor shape rejected");
    }
    for field in ["test_output", "test_stderr", "selected_test"] {
        if actor[field].as_str().is_none() {
            return Err("private actor string type rejected");
        }
    }
    for field in [
        "proxy_hits",
        "namespace_controls",
        "connection_refusals",
        "hardening",
        "dns_control",
    ] {
        if !actor[field].is_object() {
            return Err("private actor object type rejected");
        }
    }
    let name = fixture["test_selection"]["exact_name"]
        .as_str()
        .ok_or("owned test absent")?;
    if actor["selected_test"] != name {
        return Err("foreign selected test");
    }
    let inventory = actor["test_inventory"]
        .as_array()
        .ok_or("test inventory absent")?;
    let maximum = contract["maximum_inventory"]
        .as_u64()
        .ok_or("inventory bound absent")?;
    if inventory.len() as u64 > maximum {
        return Err("test inventory over bound");
    }
    let mut unique = HashSet::new();
    let mut selected = 0_u64;
    let name_bound = contract["maximum_name_bytes"]
        .as_u64()
        .ok_or("inventory name bound absent")?;
    for value in inventory {
        let item = value
            .as_str()
            .filter(|item| !item.is_empty() && item.len() as u64 <= name_bound)
            .ok_or("inventory name rejected")?;
        if !unique.insert(item) {
            return Err("ambiguous test inventory");
        }
        selected += u64::from(item == name);
    }
    let filtered = actor["declared_filtered"]
        .as_u64()
        .ok_or("filtered count absent")?;
    if filtered
        > contract["maximum_filtered"]
            .as_u64()
            .ok_or("filtered bound absent")?
        || selected
            > contract["maximum_selected"]
                .as_u64()
                .ok_or("selected bound absent")?
    {
        return Err("private actor count over bound");
    }
    let matched = selected == 1
        && fixture["test_selection"]["selected"] == 1
        && fixture["test_selection"]["component_tests_filtered"].as_u64() == Some(filtered)
        && inventory.len() as u64 == filtered + selected;
    let process = actor["test_process_success"]
        .as_bool()
        .ok_or("process result absent")?;
    let summary = actor["parser_summary_matched"]
        .as_bool()
        .ok_or("parser result absent")?;
    let verified = actor["summary_verified"]
        .as_bool()
        .ok_or("combined result absent")?;
    if verified != (process && summary)
        || actor["status"] != if verified { "passed" } else { "failed" }
        || match actor["dns_finish"].as_str() {
            Some("succeeded") => !actor["dns_policy"].is_object(),
            Some("failed") => !actor["dns_policy"].is_null(),
            _ => true,
        }
    {
        return Err("contradictory private actor result");
    }
    public_outcome(
        &json!({"schema":contract["outcome_schema"],"state":"observed",
        "process_success":process,"exit_code":actor["test_exit_code"],"inventory_matched":matched,
        "inventory_count":inventory.len(),"selected_count":selected,"filtered_count":filtered,
        "parser_summary_matched":summary,"test_passed":process&&summary&&matched,
        "dns_finish":actor["dns_finish"],"source_prefix_authenticated":false,
        "full_product_qualified":false}),
    )
}

pub(super) fn from_stdout(
    bytes: &[u8],
    fixture: &Value,
    expected: &Value,
) -> (Value, Option<&'static str>) {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return (unavailable("missing"), None);
    }
    let Ok(actor) = decode_strict_json(bytes, 2 * 1024 * 1024) else {
        return (unavailable("invalid"), None);
    };
    let Ok(value) = extract(&actor, fixture, expected) else {
        return (unavailable("invalid"), None);
    };
    let failure = if value["process_success"] == false
        && value["inventory_matched"] == true
        && value["dns_finish"] == "succeeded"
    {
        super::browser_failure::classify(
            actor["test_output"]
                .as_str()
                .expect("validated actor output"),
            actor["test_stderr"]
                .as_str()
                .expect("validated actor stderr"),
            actor["selected_test"]
                .as_str()
                .expect("validated owned name"),
        )
    } else {
        None
    };
    (value, failure)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> (Value, Value, Value) {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../crates/collector/pilot/fixture.json"
        ))
        .unwrap();
        let expected = crate::browser_expectations::from_fixture(&fixture).unwrap();
        let name = fixture["test_selection"]["exact_name"].as_str().unwrap();
        let mut inventory: Vec<Value> = (0..66)
            .map(|index| json!(format!("owned::component_{index}")))
            .collect();
        inventory.push(json!(name));
        let actor = json!({"schema":"humanymous.browser-product-actor/1","expected":expected,
            "status":"failed","test_output":"PRIVATE_BODY","test_stderr":"PRIVATE_HEADER",
            "test_inventory":inventory,"selected_test":name,"declared_selected":1,
            "declared_filtered":66,"summary_verified":false,"proxy_hits":{},"namespace_controls":{},
            "connection_refusals":{},"hardening":{},"dns_control":{},"dns_policy":{},
            "full_product_qualified":false,"test_process_success":false,"test_exit_code":1,
            "parser_summary_matched":false,"dns_finish":"succeeded"});
        (actor, fixture, expected)
    }

    #[test]
    fn original_actor_report_rebuilds_only_scalars_before_any_witness_claim() {
        let (mut actor, fixture, expected) = actor();
        let (value, failure) =
            from_stdout(&serde_json::to_vec(&actor).unwrap(), &fixture, &expected);
        assert_eq!(failure, None);
        assert_eq!(value["state"], "observed");
        assert_eq!(value["exit_code"], 1);
        assert_eq!(value["inventory_count"], 67);
        assert_eq!(value["test_passed"], false);
        assert!(!value.to_string().contains("PRIVATE"));
        actor["dns_finish"] = json!("failed");
        actor["dns_policy"] = Value::Null;
        assert_eq!(
            from_stdout(&serde_json::to_vec(&actor).unwrap(), &fixture, &expected).0["dns_finish"],
            "failed"
        );
        actor["test_stderr"] = json!("Error: webdriver POST /session request failed");
        assert_eq!(
            from_stdout(&serde_json::to_vec(&actor).unwrap(), &fixture, &expected).1,
            None
        );
        actor["dns_finish"] = json!("succeeded");
        actor["dns_policy"] = json!({});
        assert_eq!(
            from_stdout(&serde_json::to_vec(&actor).unwrap(), &fixture, &expected).1,
            Some("browser_driver_session_request_failed")
        );
        actor["test_process_success"] = json!(true);
        actor["test_exit_code"] = json!(0);
        assert_eq!(
            from_stdout(&serde_json::to_vec(&actor).unwrap(), &fixture, &expected).1,
            None
        );
    }

    #[test]
    fn duplicate_unknown_and_ambiguous_private_reports_remain_unobserved() {
        let (actor, fixture, expected) = actor();
        let raw = serde_json::to_string(&actor).unwrap();
        let duplicate = raw.replacen(
            "\"schema\":\"humanymous.browser-product-actor/1\"",
            "\"schema\":\"humanymous.browser-product-actor/1\",\"schema\":\"humanymous.browser-product-actor/1\"",
            1,
        );
        for bytes in [format!("{raw}\n{raw}"), duplicate] {
            assert_eq!(
                from_stdout(bytes.as_bytes(), &fixture, &expected).0["state"],
                "invalid"
            );
        }
        for mutation in 0..8 {
            let mut value = actor.clone();
            match mutation {
                0 => value["PRIVATE_EXTRA"] = json!("PRIVATE_PAYLOAD"),
                1 => value["test_inventory"][0] = value["test_inventory"][1].clone(),
                2 => value["test_exit_code"] = json!("PRIVATE_EXIT"),
                3 => value["selected_test"] = json!("PRIVATE_TEST"),
                4 => value["declared_filtered"] = json!(10001),
                5 => value["declared_filtered"] = json!(u64::MAX),
                6 => value["test_inventory"][0] = json!("x".repeat(1025)),
                _ => value["test_output"] = json!({"PRIVATE_BODY":"PRIVATE_VALUE"}),
            }
            let public = from_stdout(&serde_json::to_vec(&value).unwrap(), &fixture, &expected).0;
            assert_eq!(public["state"], "invalid");
            assert!(public["inventory_count"].is_null());
            assert!(!public.to_string().contains("PRIVATE"));
        }
        assert_eq!(
            from_stdout(b" \n", &fixture, &expected),
            (unavailable("missing"), None)
        );
    }
}
