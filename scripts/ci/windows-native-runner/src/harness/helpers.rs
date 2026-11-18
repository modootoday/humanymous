//! Explicit subprocess entry points, never ignored product integration cases.
use super::packages::{Packages, Role};
use serde_json::{Value, json};
pub(super) fn captured(
    entry: &Value,
    packages: &Packages,
) -> Option<(&'static str, &'static [&'static str])> {
    if (packages.matches(entry, Role::AuditDurableWitness)
        || packages.matches(entry, Role::AuditKeyBackups))
        && entry["kind"] == json!(["test"])
    {
        return match entry["target"].as_str()? {
            "audit-durable-witness" => Some((
                "child_witness_probe",
                &["witness_process_lifetime_and_interrupted_recovery"],
            )),
            "audit-key-backups" => Some((
                "child_retire_without_pin_acknowledgement",
                &["crash_reconcile_preserves_consumed_authorization"],
            )),
            _ => None,
        };
    }
    if packages.matches(entry, Role::RuntimeLibrary)
        && entry["kind"] == json!(["lib"])
        && entry["target"] == "humanymous_runtime"
    {
        return Some((
            "audit::storage::wal::tests::process_lock_probe",
            &["audit::storage::wal::tests::exclusive_writer_rotation_checkpoint_and_restart"],
        ));
    }
    if packages.matches(entry, Role::PlatformLibrary)
        && entry["kind"] == json!(["lib"])
        && entry["target"] == "humanymous_platform"
    {
        return Some((
            "testing::process::tests::subprocess_fixture",
            &[
                "testing::process::tests::actual_exit_and_output_are_observed",
                "testing::process::tests::captured_pid_matches_the_actual_direct_child_not_the_parent",
                "testing::process::tests::timeout_is_failure_and_child_is_reaped",
                "testing::process::tests::output_limit_is_not_truncated_success",
                "testing::process::tests::explicit_stderr_is_concurrent_bounded_and_not_in_debug",
                "testing::process::tests::duplex_caller_owns_pipes_and_guard_owns_lifetime",
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
        let support = json!({"package_id":"path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0","kind":["lib"],"target":"humanymous_platform"});
        assert_eq!(captured(&support, &Packages::fixture()).unwrap().1.len(), 6);
        let audit = json!({"package_id":"path+file:///C:/workspaces/humanymous/crates/sdk/humanymous-runtime#0.1.0","kind":["lib"],"target":"humanymous_runtime"});
        assert_eq!(
            captured(&audit, &Packages::fixture()).unwrap().0,
            "audit::storage::wal::tests::process_lock_probe"
        );
        let mut wrong = audit;
        wrong["target"] = json!("different_lib");
        assert!(captured(&wrong, &Packages::fixture()).is_none());
        for target in ["host_process", "runtime_service_process"] {
            assert!(captured(&json!({"package_id":"path+file:///node#humanymous-node-adapter@0.1.0","kind":["test"],"target":target}), &Packages::fixture()).is_none());
        }
    }
}
