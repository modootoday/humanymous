//! Explicit required service pilots. Ignored product cases are never helpers.
use serde_json::{Value, json};

pub(crate) const FEATURES: [&str; 2] = [
    "humanymous-cli/service-pilot",
    "humanymous-cli/runtime-service-pilot",
];
pub(crate) struct Case {
    pub target: &'static str,
    pub test: &'static str,
    pub id: &'static str,
    pub field: &'static str,
    pub log_prefix: &'static str,
    pub windows_only: bool,
}
impl Case {
    pub fn implemented_for_target(&self) -> bool {
        !self.windows_only || cfg!(windows)
    }
}
pub(crate) static CASES: [Case; 2] = [
    Case {
        target: "cli-service-process",
        test: "native_service_install_upgrade_rollback_and_remove",
        id: "native-service-lifecycle",
        field: "service_execution",
        log_prefix: "service",
        windows_only: false,
    },
    Case {
        target: "cli-runtime-service-process",
        test: "native_runtime_service_lifecycle_and_missing_credential_fail_closed",
        id: "native-runtime-service-lifecycle-and-missing-credential",
        field: "runtime_service_execution",
        log_prefix: "runtime-service",
        windows_only: true,
    },
];
pub(crate) fn required_case(
    entry: &Value,
    packages: &crate::native_runner::harness::packages::Packages,
) -> Option<&'static Case> {
    if entry["kind"] != json!(["test"])
        || !(packages.matches(
            entry,
            crate::native_runner::harness::packages::Role::CliService,
        ) || packages.matches(
            entry,
            crate::native_runner::harness::packages::Role::CliRuntimeService,
        ))
    {
        return None;
    }
    CASES.iter().find(|case| entry["target"] == case.target)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_cases_match_exact_owned_package_target_and_test_kind() {
        let packages = crate::native_runner::harness::packages::Packages::fixture();
        for case in &CASES {
            let role = if case.target == "cli-service-process" {
                crate::native_runner::harness::packages::Role::CliService
            } else {
                crate::native_runner::harness::packages::Role::CliRuntimeService
            };
            let mut entry = crate::native_runner::harness::packages::Packages::fixture_entry(role);
            assert_eq!(required_case(&entry, &packages).unwrap().test, case.test);
            entry["package_id"] = json!("path+file:///other#humanymous-cli-extension@0.1.0");
            assert!(required_case(&entry, &packages).is_none());
        }
        assert!(required_case(&json!({"package_id":"opaque-humanymous-cli","target":"cli-service-process","kind":["bin"]}), &packages).is_none());
    }
}
