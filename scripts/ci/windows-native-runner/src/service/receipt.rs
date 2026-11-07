//! Validate the controller's independently observed, sanitized runtime pilot result.
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};
use std::io;

pub(crate) const FACTS: [&str; 13] = [
    "missing_credential_fail_closed",
    "service_principal_verified",
    "readiness_verified",
    "graceful_stop_verified",
    "host_exit_stopped",
    "credential_cleanup_verified",
    "state_cleanup_verified",
    "receipt_binding_verified",
    "scm_registration_removed",
    "runtime_upgrade_verified",
    "runtime_rollback_verified",
    "replay_floor_verified",
    "full_readiness_manifest_verified",
];

pub(super) fn validate(text: &str, source_sha: &str, run_id: u64) -> io::Result<Value> {
    let records: Vec<_> = text
        .lines()
        .filter(|line| line.trim_start().starts_with('{'))
        .map(|line| {
            decode_strict_json(line.trim().as_bytes(), 64 * 1024)
                .map_err(|_| io::Error::other("invalid runtime pilot JSON line"))
        })
        .collect::<io::Result<_>>()?;
    if records.len() != 1 {
        return Err(io::Error::other(
            "exactly one runtime pilot result required",
        ));
    }
    let receipt = &records[0];
    if receipt["schema"] != "humanymous.runtime-service-pilot/1"
        || receipt["case"] != "native-runtime-service-lifecycle-and-missing-credential"
        || receipt["source_sha"] != source_sha
        || receipt["run_id"].as_u64() != Some(run_id)
        || receipt["product_qualified"] != false
        || FACTS.iter().any(|key| receipt[*key] != true)
    {
        return Err(io::Error::other(
            "runtime pilot facts or source binding incomplete",
        ));
    }
    let mut output = json!({"verified":true,"run_id":run_id,"product_qualified":false});
    for key in FACTS {
        output[key] = json!(true);
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn good() -> Value {
        let mut value = json!({"schema":"humanymous.runtime-service-pilot/1","case":"native-runtime-service-lifecycle-and-missing-credential","source_sha":"a".repeat(40),"run_id":1,"product_qualified":false,"private_path":"PRIVATE"});
        for key in FACTS {
            value[key] = json!(true);
        }
        value
    }
    #[test]
    fn complete_bound_receipt_is_whitelisted_and_missing_or_false_facts_fail() {
        let line = good().to_string();
        assert!(
            !validate(&line, &"a".repeat(40), 1)
                .unwrap()
                .to_string()
                .contains("PRIVATE")
        );
        for key in FACTS {
            let mut bad = good();
            bad[key] = Value::Null;
            assert!(validate(&bad.to_string(), &"a".repeat(40), 1).is_err());
            bad[key] = json!(false);
            assert!(validate(&bad.to_string(), &"a".repeat(40), 1).is_err());
        }
        assert!(validate(&line, &"b".repeat(40), 1).is_err());
        assert!(validate(&line, &"a".repeat(40), 2).is_err());
    }
    #[test]
    fn duplicate_records_and_duplicate_decoded_keys_are_refused() {
        let line = good().to_string();
        assert!(validate(&format!("{line}\n{line}"), &"a".repeat(40), 1).is_err());
        assert!(validate("{\"run_id\":1,\"run_id\":1}", &"a".repeat(40), 1).is_err());
        assert!(validate("no receipt", &"a".repeat(40), 1).is_err());
    }
}
