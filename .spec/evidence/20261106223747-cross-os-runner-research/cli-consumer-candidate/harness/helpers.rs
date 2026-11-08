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
/// Declare an unexecuted privileged product case without granting a helper waiver.
pub(super) fn external_scope(
    entry: &Value,
    output: &str,
    unix: bool,
    packages: &Packages,
) -> Option<Value> {
    const PARENT: &str = "unix_other_user_is_denied_by_actual_socket_acl";
    const HELPER: &str = "unix_unprivileged_connect_probe";
    if !unix
        || entry["target"] != "authority-native-process"
        || entry["kind"] != json!(["test"])
        || !packages.matches(entry, Role::AuthorityNative)
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
        let support = Packages::fixture_entry(Role::PlatformLibrary);
        assert_eq!(captured(&support, &Packages::fixture()).unwrap().1.len(), 6);
        let audit = Packages::fixture_entry(Role::RuntimeLibrary);
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
    #[test]
    fn ignored_privileged_parent_is_required_and_never_a_verified_helper() {
        let entry = Packages::fixture_entry(Role::AuthorityNative);
        let log = "test unix_other_user_is_denied_by_actual_socket_acl ... ignored, Docker only\n";
        let scope = external_scope(&entry, log, true, &Packages::fixture()).unwrap();
        assert_eq!(scope["ignored_subprocess_helpers"], 0);
        assert_eq!(
            scope["required_not_run"][0],
            "unix_other_user_is_denied_by_actual_socket_acl"
        );
        assert!(external_scope(&entry, log, false, &Packages::fixture()).is_none());
        assert!(
            external_scope(
                &entry,
                "test unix_other_user_is_denied_by_actual_socket_acl ... ok",
                true,
                &Packages::fixture()
            )
            .is_none()
        );
        let mut wrong = entry;
        wrong["package_id"] = json!("path+file:///node#humanymous-node-adapter@0.1.0");
        assert!(external_scope(&wrong, log, true, &Packages::fixture()).is_none());
    }
}
