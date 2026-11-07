//! Closed publication vocabulary for named receipts and indexed failures.
use serde_json::{Value, json};
const NATIVE_TESTS: [&str; 5] = [
    "native_binary_session_collection_origin_restart_and_wal_authentication",
    "real_ipc_auth_replay_body_scope_capture_and_restart_epoch",
    "concurrent_nonce_reservation_and_durable_failure",
    "normalized_producers_are_bound_by_native_authority_credentials",
    "sealed_startup_retains_custody_and_reuses_existing_identity",
];
pub(super) fn row(raw: &Value, public: &mut Value) {
    public["index"] = json!(raw["index"].as_u64());
    public["required_native_tests_verified"] =
        json!(raw["required_native_tests_verified"].as_bool());
    public["required_native_tests"] = json!(
        raw["required_native_tests"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|name| NATIVE_TESTS.contains(name))
            .collect::<Vec<_>>()
    );
    public["required_execution_scope"] = json!(
        raw["required_execution_scope"]
            .as_str()
            .filter(|value| *value == "privileged-docker")
    );
    public["declared_subprocess_helpers"] = json!(
        raw["declared_subprocess_helpers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|name| *name == "unix_unprivileged_connect_probe")
            .collect::<Vec<_>>()
    );
}
pub(super) fn failures(raw: &Value) -> Value {
    json!(
        raw.as_array()
            .into_iter()
            .flatten()
            .map(|failure| {
                let code = failure["code"]
                    .as_str()
                    .filter(|code| {
                        matches!(
                            *code,
                            "coverage-accounting"
                                | "required-native-assertion"
                                | "required-native-harness"
                                | "final-report-absent"
                        )
                    })
                    .unwrap_or("unknown-failure");
                json!({"index":failure["index"].as_u64(),"code":code})
            })
            .collect::<Vec<_>>()
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_assertions_are_preserved_without_publishing_private_strings() {
        let raw = json!({"index":4,"required_native_tests_verified":true,"required_native_tests":[NATIVE_TESTS[0],"private-path"],"required_execution_scope":"privileged-docker","declared_subprocess_helpers":["unix_unprivileged_connect_probe","private-path"]});
        let mut public = json!({});
        row(&raw, &mut public);
        assert_eq!(public["index"], 4);
        assert_eq!(public["required_native_tests"].as_array().unwrap().len(), 1);
        assert_eq!(public["required_native_tests_verified"], true);
        assert!(!public.to_string().contains("private-path"));
    }
    #[test]
    fn indexed_failure_projection_excludes_private_diagnostics() {
        let projected = failures(
            &json!([{"index":2,"code":"coverage-accounting","target":"private-path","error":"private-path"},{"index":null,"code":"private-path"}]),
        );
        assert_eq!(projected[0]["index"], 2);
        assert_eq!(projected[1]["code"], "unknown-failure");
        assert!(!projected.to_string().contains("private-path"));
    }
}
