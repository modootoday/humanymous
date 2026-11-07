//! Explicit subprocess entry points, never ignored product integration cases.
use serde_json::{Value, json};
pub(super) fn captured(entry: &Value) -> Option<(&'static str, &'static [&'static str])> {
    let package = entry["package_id"].as_str()?;
    if package.contains("#humanymous-audit@") && entry["kind"] == json!(["test"]) {
        return match entry["target"].as_str()? {
            "durable_witness" => Some((
                "child_witness_probe",
                &["witness_process_lifetime_and_interrupted_recovery"],
            )),
            "key_backups" => Some((
                "child_retire_without_pin_acknowledgement",
                &["crash_reconcile_preserves_consumed_authorization"],
            )),
            _ => None,
        };
    }
    if package.contains("#humanymous-test-support@")
        && entry["kind"] == json!(["lib"])
        && entry["target"] == "humanymous_test_support"
    {
        return Some((
            "process::tests::subprocess_fixture",
            &[
                "process::tests::actual_exit_and_output_are_observed",
                "process::tests::captured_pid_matches_the_actual_direct_child_not_the_parent",
                "process::tests::timeout_is_failure_and_child_is_reaped",
                "process::tests::output_limit_is_not_truncated_success",
                "process::tests::explicit_stderr_is_concurrent_bounded_and_not_in_debug",
                "process::tests::duplex_caller_owns_pipes_and_guard_owns_lifetime",
            ],
        ));
    }
    None
}
/// Declare an unexecuted privileged product case without granting a helper waiver.
pub(super) fn external_scope(entry: &Value, output: &str, unix: bool) -> Option<Value> {
    const PARENT: &str = "unix_other_user_is_denied_by_actual_socket_acl";
    const HELPER: &str = "unix_unprivileged_connect_probe";
    if !unix
        || entry["target"] != "native_process"
        || entry["kind"] != json!(["test"])
        || !entry["package_id"]
            .as_str()
            .is_some_and(|p| p.contains("#humanymous-authority@"))
        || !output
            .lines()
            .any(|line| line.starts_with(&format!("test {PARENT} ... ignored")))
    {
        return None;
    }
    Some(
        json!({"required_not_run":[PARENT],"required_execution_scope":"privileged-docker","declared_subprocess_helpers":[HELPER],"ignored_subprocess_helpers":0}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_helper_owners_do_not_include_product_host_pilots() {
        let support = json!({"package_id":"path+file:///support#humanymous-test-support@0.1.0","kind":["lib"],"target":"humanymous_test_support"});
        assert_eq!(captured(&support).unwrap().1.len(), 6);
        for target in ["host_process", "runtime_service_process"] {
            assert!(captured(&json!({"package_id":"path+file:///node#humanymous-node-adapter@0.1.0","kind":["test"],"target":target})).is_none());
        }
    }
    #[test]
    fn ignored_privileged_parent_is_required_and_never_a_verified_helper() {
        let entry = json!({"package_id":"path+file:///authority#humanymous-authority@0.1.0","kind":["test"],"target":"native_process"});
        let log = "test unix_other_user_is_denied_by_actual_socket_acl ... ignored, Docker only\n";
        let scope = external_scope(&entry, log, true).unwrap();
        assert_eq!(scope["ignored_subprocess_helpers"], 0);
        assert_eq!(
            scope["required_not_run"][0],
            "unix_other_user_is_denied_by_actual_socket_acl"
        );
        assert!(external_scope(&entry, log, false).is_none());
        assert!(
            external_scope(
                &entry,
                "test unix_other_user_is_denied_by_actual_socket_acl ... ok",
                true
            )
            .is_none()
        );
        let mut wrong = entry;
        wrong["package_id"] = json!("path+file:///node#humanymous-node-adapter@0.1.0");
        assert!(external_scope(&wrong, log, true).is_none());
    }
}
