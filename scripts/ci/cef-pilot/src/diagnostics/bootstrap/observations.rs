//! Typed fields from the sole owning bootstrap marker contract.
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};

fn fields(value: &Value, names: &Value) -> Option<()> {
    let object = value.as_object()?;
    let names = names.as_array()?;
    (object.len() == names.len()
        && object
            .keys()
            .all(|key| names.iter().any(|name| name.as_str() == Some(key.as_str()))))
    .then_some(())
}
fn record(value: &Value, config: &Value) -> Option<Value> {
    if value["schema"] != config["observation_schema"] {
        return None;
    }
    let error = |key: &str| {
        config["error_kinds"]
            .as_array()
            .is_some_and(|kinds| kinds.contains(&value[key]))
    };
    match value["kind"].as_str()? {
        "case" => {
            fields(value, &config["case_fields"])?;
            if !config["cases"]
                .as_array()?
                .iter()
                .any(|case| case["id"] == value["case_id"])
                || ["spawn_ok", "wait_completed", "cleanup_attempted"]
                    .iter()
                    .any(|key| !value[*key].is_boolean())
                || !(value["deadline_expired"].is_null() || value["deadline_expired"].is_boolean())
                || !(value["reaped"].is_null() || value["reaped"] == true)
                || !(value["exit_code"].is_null()
                    || value["exit_code"]
                        .as_i64()
                        .is_some_and(|code| i32::try_from(code).is_ok()))
                || ["spawn_error_kind", "wait_error_kind", "cleanup_error_kind"]
                    .iter()
                    .any(|key| !error(key))
                || (value["wait_completed"] == true
                    && (value["spawn_ok"] != true || value["reaped"] != true))
                || (!value["exit_code"].is_null() && value["reaped"] != true)
                || (value["spawn_ok"] == false
                    && (value["wait_completed"] == true
                        || !value["reaped"].is_null()
                        || !value["exit_code"].is_null()
                        || !value["deadline_expired"].is_null()
                        || value["cleanup_attempted"] == true))
            {
                return None;
            }
            Some(
                json!({"schema":1,"kind":"case","case_id":value["case_id"],"spawn_ok":value["spawn_ok"],"wait_completed":value["wait_completed"],
                "deadline_expired":value["deadline_expired"],"reaped":value["reaped"],"exit_code":value["exit_code"],"cleanup_attempted":value["cleanup_attempted"],
                "spawn_error_kind":value["spawn_error_kind"],"wait_error_kind":value["wait_error_kind"],"cleanup_error_kind":value["cleanup_error_kind"]}),
            )
        }
        "profiles" => {
            fields(value, &config["profile_fields"])?;
            if !config["profile_phases"]
                .as_array()?
                .contains(&value["phase"])
                || !value["inventory_ok"].is_boolean()
                || !(value["changed"].is_null() || value["changed"].is_boolean())
                || !error("error_kind")
                || (value["phase"] == "before" && !value["changed"].is_null())
                || (value["inventory_ok"] == false && !value["changed"].is_null())
            {
                return None;
            }
            Some(
                json!({"schema":1,"kind":"profiles","phase":value["phase"],"inventory_ok":value["inventory_ok"],"changed":value["changed"],"error_kind":value["error_kind"]}),
            )
        }
        _ => None,
    }
}
fn rejected(truncated: bool) -> Value {
    json!({"status":"rejected","error_code":"invalid_bootstrap_observation","observations_complete":false,"cases":null,"profiles":null,"read_truncated":truncated,"full_product_qualified":false})
}
pub(super) fn read(bytes: &[u8], truncated: bool) -> Value {
    let parsed = (|| -> Option<Value> {
        let config = super::config().ok()?;
        let prefix = config["marker_prefix"].as_str()?;
        let limit = usize::try_from(config["max_record_bytes"].as_u64()?).ok()?;
        let ids: Vec<_> = config["cases"]
            .as_array()?
            .iter()
            .map(|case| case["id"].clone())
            .collect();
        let phases = config["profile_phases"].as_array()?;
        if ids.len() != 4 || phases.len() != 2 {
            return None;
        }
        let mut cases = vec![Value::Null; 4];
        let mut profiles = vec![Value::Null; 2];
        for line in bytes.split(|byte| *byte == b'\n') {
            if !line.starts_with(prefix.as_bytes()) {
                continue;
            }
            if line.len() + 1 > limit {
                return None;
            }
            let raw = decode_strict_json(line.get(prefix.len()..)?, limit).ok()?;
            let value = record(&raw, &config)?;
            let slot = if value["kind"] == "case" {
                &mut cases[ids.iter().position(|id| *id == value["case_id"])?]
            } else {
                &mut profiles[phases.iter().position(|phase| *phase == value["phase"])?]
            };
            if !slot.is_null() {
                return None;
            }
            *slot = value;
        }
        let observed_cases = cases.iter().filter(|case| !case.is_null()).count();
        let observed_profiles = profiles.iter().filter(|phase| !phase.is_null()).count();
        let complete = observed_cases == 4 && observed_profiles == 2 && !truncated;
        Some(
            json!({"status":if complete{"complete"}else if observed_cases+observed_profiles==0{"missing"}else{"partial"},
            "observations_complete":complete,"case_ids":ids,"cases":cases,"profile_phases":phases,"profiles":profiles,
            "observed_cases":observed_cases,"observed_profiles":observed_profiles,"read_truncated":truncated,"full_product_qualified":false}),
        )
    })();
    parsed.unwrap_or_else(|| rejected(truncated))
}
pub(super) fn public(value: &Value) -> Value {
    if value["status"] == "rejected" {
        return rejected(value["read_truncated"].as_bool().unwrap_or(true));
    }
    let Some(prefix) = super::config()
        .ok()
        .and_then(|config| config["marker_prefix"].as_str().map(str::to_owned))
    else {
        return read(&[], true);
    };
    let mut bytes = Vec::new();
    for key in ["cases", "profiles"] {
        for record in value[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|record| !record.is_null())
        {
            bytes.extend_from_slice(prefix.as_bytes());
            bytes.extend_from_slice(record.to_string().as_bytes());
            bytes.push(b'\n');
        }
    }
    read(&bytes, value["read_truncated"].as_bool().unwrap_or(true))
}
