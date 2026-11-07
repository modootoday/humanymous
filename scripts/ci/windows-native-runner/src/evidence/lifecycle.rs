//! Whitelisted actor teardown facts, excluding identifiers and raw descriptors.
use serde_json::{Value, json};

fn sha256(value: &Value) -> Option<&str> {
    value
        .as_str()
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub(super) fn project(report: &Value, summary: &mut Value) {
    if let Some(actor) = report.get("standard_user_lifecycle") {
        summary["standard_user_lifecycle"] = json!({
            "account_deleted":actor["account_deleted"].as_bool(),
            "scratch_deleted":actor["scratch_deleted"].as_bool(),
            "child_exit_code":actor["child_exit_code"].as_i64(),
            "child_launch_attempted":actor["child_launch_attempted"].as_bool(),
            "job_kill_on_close":actor["job_kill_on_close"].as_bool(),
            "job_empty_observed":actor["job_empty_observed"].as_bool(),
            "scm_delegation_requested":actor["scm_delegation_requested"].as_bool(),
            "actor_case":match actor["actor_case"].as_str() {Some("normal")=>Some("normal"),Some("after-grant")=>Some("after-grant"),_=>None}
        });
    }
    if let Some(scm) = report.get("scm_delegation") {
        let hash = sha256(&scm["original_descriptor_sha256"]);
        summary["scm_delegation"] = json!({"rights":scm["rights"].as_u64(),"single_generated_sid":scm["single_generated_sid"].as_bool(),"restored":scm["restored"].as_bool(),"original_descriptor_sha256":hash});
    }
    if let Some(service) = report.get("service_execution") {
        summary["service_execution"] = json!({"exit_code":service["exit_code"].as_i64(),"passed":service["passed"].as_u64(),"failed":service["failed"].as_u64(),"ignored":service["ignored"].as_u64(),"measured":service["measured"].as_u64(),"nonselected_tests":service["nonselected_tests"].as_u64()});
        summary["service_execution"]["test"] = match service["test"].as_str() {
            Some("native_service_install_upgrade_rollback_and_remove") => {
                json!("native_service_install_upgrade_rollback_and_remove")
            }
            _ => Value::Null,
        };
        summary["service_execution"]["stdout_sha256"] = json!(sha256(&service["stdout_sha256"]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_failed_cleanup_unknowns_and_no_private_identifiers() {
        let raw = json!({"standard_user_lifecycle":{"sid":"PRIVATE-SID","account_deleted":false,"scratch_deleted":true,"child_exit_code":1,"job_empty_observed":false,"actor_case":"PRIVATE-PATH"},"scm_delegation":{"rights":3,"single_generated_sid":true,"restored":false,"original_descriptor_sha256":"a".repeat(64),"sddl":"PRIVATE-DESCRIPTOR"}});
        let mut summary = json!({});
        project(&raw, &mut summary);
        assert_eq!(summary["standard_user_lifecycle"]["account_deleted"], false);
        assert_eq!(summary["standard_user_lifecycle"]["scratch_deleted"], true);
        assert!(summary["standard_user_lifecycle"]["job_kill_on_close"].is_null());
        assert_eq!(summary["scm_delegation"]["restored"], false);
        assert!(!summary.to_string().contains("PRIVATE"));
    }
    #[test]
    fn prelaunch_negative_does_not_claim_an_observed_job() {
        let raw = json!({"standard_user_lifecycle":{"actor_case":"after-grant","child_launch_attempted":false,"child_exit_code":null,"job_empty_observed":null,"job_kill_on_close":null,"account_deleted":true,"scratch_deleted":true},"scm_delegation":{"restored":true,"rights":3,"single_generated_sid":true}});
        let mut summary = json!({});
        project(&raw, &mut summary);
        assert_eq!(
            summary["standard_user_lifecycle"]["child_launch_attempted"],
            false
        );
        assert!(summary["standard_user_lifecycle"]["job_empty_observed"].is_null());
        assert!(summary["standard_user_lifecycle"]["job_kill_on_close"].is_null());
        assert_eq!(summary["scm_delegation"]["restored"], true);
    }
}
