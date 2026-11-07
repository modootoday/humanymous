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
}
