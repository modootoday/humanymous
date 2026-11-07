//! Recover private startup logs while projecting only fixed stage identifiers.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::json;
pub(super) fn classify(logs: &str) -> Option<&'static str> {
    logs.lines()
        .filter_map(
            |line| match line.trim().strip_prefix("HM-LAB-ACTOR-STEP ")? {
                "actor_hardening" => Some("actor_hardening"),
                "actor_root_probe_eacces" => Some("actor_root_probe_eacces"),
                "actor_root_probe_unknown" => Some("actor_root_probe_unknown"),
                "actor_installation" => Some("actor_installation"),
                "actor_writer_mount" => Some("actor_writer_mount"),
                "actor_mount_missing" => Some("actor_mount_missing"),
                "actor_mount_lookup_denied" => Some("actor_mount_lookup_denied"),
                "actor_mount_lookup_other" => Some("actor_mount_lookup_other"),
                "actor_mount_symlink" => Some("actor_mount_symlink"),
                "actor_mount_not_directory" => Some("actor_mount_not_directory"),
                "actor_mount_owner" => Some("actor_mount_owner"),
                "actor_mount_mode" => Some("actor_mount_mode"),
                "actor_writer_keys" => Some("actor_writer_keys"),
                "actor_writer_identity" => Some("actor_writer_identity"),
                "actor_witness_bootstrap" => Some("actor_witness_bootstrap"),
                "actor_browser_hardening" => Some("actor_browser_hardening"),
                "actor_browser_config" => Some("actor_browser_config"),
                "actor_browser_dns_bind" => Some("actor_browser_dns_bind"),
                "actor_browser_dns_probe" => Some("actor_browser_dns_probe"),
                "actor_browser_namespace" => Some("actor_browser_namespace"),
                "actor_browser_proxy" => Some("actor_browser_proxy"),
                "actor_browser_inventory" => Some("actor_browser_inventory"),
                "actor_browser_test" => Some("actor_browser_test"),
                "actor_browser_test_passed" => Some("actor_browser_test_passed"),
                "actor_browser_test_failed" => Some("actor_browser_test_failed"),
                "actor_browser_negative_proxy" => Some("actor_browser_negative_proxy"),
                "actor_browser_dns_finish" => Some("actor_browser_dns_finish"),
                "actor_browser_dns_finished" => Some("actor_browser_dns_finished"),
                "actor_browser_dns_finish_failed" => Some("actor_browser_dns_finish_failed"),
                _ => None,
            },
        )
        .next_back()
}
pub(super) fn capture(lab: &mut Lab, id: &str) -> Result<()> {
    let output = lab.docker.invoke(
        &["logs".into(), "--tail".into(), "32".into(), id.into()],
        &[],
        false,
    )?;
    let logs = format!("{}\n{}", output.stdout, output.stderr);
    if let Some(code) = classify(&logs) {
        lab.stage = code;
    }
    let state = lab
        .docker
        .invoke(&["inspect".into(), id.into()], &[], false)?;
    snapshot::write(
        &lab.evidence,
        "actor-startup-failure.private.json",
        &json!({"stdout":output.stdout,"stderr":output.stderr,"inspect_stdout":state.stdout,"inspect_stderr":state.stderr,"safe_error_code":lab.stage,"private_contents_included":true}),
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn only_complete_static_stage_lines_can_escape_private_logs() {
        assert_eq!(
            super::classify(
                "HM-LAB-ACTOR-STEP actor_installation\nsecret/path\nHM-LAB-ACTOR-STEP actor_writer_keys"
            ),
            Some("actor_writer_keys")
        );
        for value in [
            "HM-LAB-ACTOR-STEP private@example.com",
            "prefix HM-LAB-ACTOR-STEP actor_writer_keys",
            "HM-LAB-ACTOR-STEP actor_writer_keys Bearer private",
        ] {
            assert_eq!(super::classify(value), None);
        }
        let contract: serde_json::Value =
            serde_json::from_str(include_str!("../../safe-diagnostics.json")).unwrap();
        for stage in [
            "actor_hardening",
            "actor_root_probe_eacces",
            "actor_root_probe_unknown",
            "actor_installation",
            "actor_writer_mount",
            "actor_mount_missing",
            "actor_mount_lookup_denied",
            "actor_mount_lookup_other",
            "actor_mount_symlink",
            "actor_mount_not_directory",
            "actor_mount_owner",
            "actor_mount_mode",
            "actor_writer_keys",
            "actor_writer_identity",
            "actor_witness_bootstrap",
            "actor_browser_hardening",
            "actor_browser_config",
            "actor_browser_dns_bind",
            "actor_browser_dns_probe",
            "actor_browser_namespace",
            "actor_browser_proxy",
            "actor_browser_inventory",
            "actor_browser_test",
            "actor_browser_test_passed",
            "actor_browser_test_failed",
            "actor_browser_negative_proxy",
            "actor_browser_dns_finish",
            "actor_browser_dns_finished",
            "actor_browser_dns_finish_failed",
        ] {
            assert_eq!(
                super::classify(&format!("HM-LAB-ACTOR-STEP {stage}")),
                Some(stage)
            );
            assert_eq!(
                super::classify(&format!("HM-LAB-ACTOR-STEP {stage} private payload")),
                None
            );
            assert!(
                contract["error_codes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == stage)
            );
        }
    }
}
