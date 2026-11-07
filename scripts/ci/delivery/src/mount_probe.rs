//! Consume closed mount observations, never the owning custody predicates.
use crate::{
    Result, command, evidence,
    lab_receipts::{digest, fields, hosted_context},
    read_json, require, source, write_json,
};
use serde_json::{Value, json};
use std::path::Path;

fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../test/lab/runner/mount-probe-contract.json"
    ))
    .map_err(|error| error.to_string())
}
fn validate(
    value: &Value,
    head: &str,
    manifest: &str,
    raw: &str,
    recipe: &str,
    context: &Value,
) -> Result<Value> {
    let contract = contract()?;
    require(
        contract["schema"] == "humanymous.lab-mount-probe-contract/1" && contract["scheduled"] == 4,
        "mount diagnostic contract mismatch",
    )?;
    fields(value, &contract["public_fields"])?;
    require(
        value["schema"] == contract["report_schema"]
            && value["source_sha"] == head
            && value["source_manifest_sha256"] == manifest
            && value["private_receipt_sha256"] == raw
            && value["recipe_sha256"] == recipe
            && digest(&value["source_sha"], 40)
            && [
                "source_manifest_sha256",
                "private_receipt_sha256",
                "recipe_sha256",
            ]
            .iter()
            .all(|key| digest(&value[*key], 64))
            && value["raw_contents_included"] == false
            && value["full_product_qualified"] == false,
        "mount source/original/recipe binding mismatch",
    )?;
    require(
        value["run_context"] == *context
            && context["repository"] == "modootoday/humanymous"
            && context["kind"] == "github-hosted-linux"
            && context["hosted_qualification_admitted"] == true
            && context["workflow_sha"] == head
            && context["authenticated_provenance"] == false
            && context["requires_external_job_verification"] == true,
        "mount receipt does not match this hosted run",
    )?;
    let rows = value["observations"]
        .as_array()
        .ok_or("mount observations absent")?;
    let cases = contract["cases"]
        .as_array()
        .ok_or("mount case contract absent")?;
    require(
        rows.len() == 4 && cases.len() == 4,
        "four ordered mount observations required",
    )?;
    let (mut accepted, mut refused, mut unobserved) = (0u64, 0u64, 0u64);
    for (row, case) in rows.iter().zip(cases) {
        fields(row, &contract["observation_fields"])?;
        require(
            row["role"] == case["role"] && row["surface"] == case["surface"],
            "mount role/surface order mismatch",
        )?;
        let paths = row["paths"].as_array().ok_or("mount paths absent")?;
        if row["status"] == "not_observed" {
            require(
                paths.is_empty(),
                "unobserved mounts cannot claim path facts",
            )?;
            unobserved += 1;
            continue;
        }
        let role = contract["roles"]
            .as_array()
            .and_then(|roles| roles.iter().find(|role| role["name"] == case["role"]))
            .ok_or("mount role contract absent")?;
        let expected = role["paths"]
            .as_array()
            .ok_or("mount path contract absent")?;
        require(
            paths.len() == expected.len() && paths.len() == 2,
            "two fixed mount path observations required",
        )?;
        let mut has_refusal = false;
        for (path, expected) in paths.iter().zip(expected) {
            fields(path, &contract["path_fields"])?;
            require(path["id"] == expected["id"], "mount path identity mismatch")?;
            match path["status"].as_str() {
                Some("observed_accept") => require(
                    path["code"].is_null(),
                    "accepted observation cannot claim refusal",
                )?,
                Some("observed_refusal") => {
                    require(
                        contract["refusal_codes"]
                            .as_array()
                            .is_some_and(|codes| codes.contains(&path["code"])),
                        "unknown mount refusal code",
                    )?;
                    has_refusal = true;
                }
                _ => return Err("unknown mount path status".into()),
            }
        }
        require(
            row["status"]
                == if has_refusal {
                    "observed_refusal"
                } else {
                    "observed_accept"
                },
            "mount observation status/count mismatch",
        )?;
        if has_refusal {
            refused += 1;
        } else {
            accepted += 1;
        }
    }
    fields(&value["totals"], &contract["totals_fields"])?;
    require(
        value["totals"]
            == json!({"scheduled":4,"observed":accepted+refused,"observed_accept":accepted,"observed_refusal":refused,"unobserved":unobserved})
            && value["observations_complete"] == (unobserved == 0),
        "mount diagnostic totals mismatch",
    )?;
    let image = value["image_sha256"]
        .as_str()
        .and_then(|image| image.strip_prefix("sha256:"));
    require(
        image.is_some_and(|image| digest(&json!(image), 64))
            || value["image_sha256"].is_null() && unobserved == 4,
        "observed mounts require a bound image",
    )?;
    Ok(value.clone())
}
fn bounded(path: &Path, limit: u64) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= limit,
        "mount original report must be bounded regular file",
    )
}
fn collection_complete(summary: &Value) -> bool {
    summary["owner_exit_code"] == 0
        && summary["receipt_valid"] == true
        && summary["cleanup_observed"] == true
        && summary["requirement"]["satisfied"] == true
        && summary["diagnostics"]["failure_count"] == 0
        && summary["receipt"]["observations_complete"] == true
        && summary["custody_policy_qualified"] == false
        && summary["full_product_qualified"] == false
        && summary["release_ready"] == false
}
pub fn run(root: &Path) -> Result<()> {
    require(
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var_os("ACT").is_none(),
        "mount diagnostic requires genuine hosted Linux",
    )?;
    let before = source::snapshot(root)?;
    require(
        before["dirty"] == false
            && before["source_sha"] == std::env::var("GITHUB_SHA").unwrap_or_default(),
        "mount diagnostic requires clean exact workflow source",
    )?;
    let context = hosted_context()?;
    let dir = evidence::bundle(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    write_json(&dir.join("delivery-source-before.json"), &before)?;
    let mut summary = json!({"schema":"humanymous.mount-delivery-summary/1","source_sha":before["source_sha"],"source_manifest_sha256":before["manifest_sha256"],
        "collection_status":"incomplete","owner_exit_code":null,"receipt":null,"receipt_valid":false,"cleanup_observed":false,
        "custody_policy_qualified":false,"full_product_qualified":false,"release_ready":false});
    let result: Result<()> = (|| {
        require(
            contract()?["cli_flag"] == "--mount-probe"
                && contract()?["build_target"] == "custody-layout",
            "mount owner CLI contract mismatch",
        )?;
        let code = command::json_log(
            root,
            "cargo",
            &[
                "run",
                "--locked",
                "-p",
                "humanymous-lab-runner",
                "--",
                "run",
                "--evidence",
                dir.to_str().ok_or("mount private path invalid")?,
                "--mount-probe",
            ],
            &dir.join("mount-owner.log"),
        )?;
        summary["owner_exit_code"] = json!(code);
        bounded(&dir.join("lab-report.json"), 16 * 1024 * 1024)?;
        let lab = read_json(&dir.join("lab-report.json"))?;
        require(
            lab["schema"] == "humanymous.product-lab/2"
                && lab["full_product_qualified"] == false
                && lab["release_ready"] == false,
            "mount lab scope mismatch",
        )?;
        let requirement = &lab["requirement"];
        fields(
            requirement,
            &json!(["schema", "kind", "satisfied", "full_product_qualified"]),
        )?;
        require(
            requirement["schema"] == "humanymous.lab-requirement/1"
                && requirement["kind"] == "scoped"
                && requirement["satisfied"].is_boolean()
                && requirement["full_product_qualified"] == false,
            "mount scoped requirement mismatch",
        )?;
        summary["requirement"] = requirement.clone();
        summary["diagnostics"] =
            crate::lab_diagnostics::validate(&lab)?.ok_or("mount safe diagnostics absent")?;
        let manifest = dir.join("source-manifest.json");
        let ledger = dir.join("mount-probe-ledger.json");
        bounded(&manifest, 16 * 1024 * 1024)?;
        bounded(&ledger, 1024 * 1024)?;
        require(
            read_json(&manifest)?["head"] == before["source_sha"],
            "mount frozen source differs",
        )?;
        let receipt = validate(
            &lab["public_receipts"]["mount_probe"],
            before["source_sha"].as_str().ok_or("mount source absent")?,
            &source::file(&manifest)?,
            &source::file(&ledger)?,
            &source::file(&root.join("deployments/lab/images/product.Dockerfile"))?,
            &context,
        )?;
        let original = read_json(&ledger)?;
        require(
            original["schema"] == contract()?["private_schema"]
                && original["full_product_qualified"] == false
                && original["image_sha256"] == receipt["image_sha256"],
            "mount original image identity mismatch",
        )?;
        if !receipt["image_sha256"].is_null() {
            let image = dir.join("mount-probe-image.private.json");
            bounded(&image, 1024 * 1024)?;
            let inspected = read_json(&image)?;
            require(
                inspected.as_array().is_some_and(|rows| rows.len() == 1)
                    && inspected[0]["Id"] == receipt["image_sha256"],
                "mount inspected image identity mismatch",
            )?;
            summary["private_image_inspection_sha256"] = json!(source::file(&image)?);
        }
        summary["receipt"] = receipt;
        summary["receipt_valid"] = json!(true);
        let cleanup: Vec<_> = lab["cases"]
            .as_array()
            .ok_or("mount lab cases absent")?
            .iter()
            .filter(|case| case["id"] == "cleanup-owned-project")
            .collect();
        let cleaned = cleanup.len() == 1 && cleanup[0]["status"] == "passed";
        summary["cleanup_observed"] = json!(cleaned);
        require(
            collection_complete(&summary),
            "mount diagnostic collection incomplete or cleanup failed",
        )?;
        summary["collection_status"] = json!("complete");
        Ok(())
    })();
    let after = source::snapshot(root)?;
    summary["source_changed"] = json!(before != after);
    if before != after {
        summary["collection_status"] = json!("source_changed");
    }
    write_json(&dir.join("delivery-source-after.json"), &after)?;
    write_json(&dir.join("mount-delivery-summary.json"), &summary)?;
    write_json(&root.join("target/delivery-mount/summary.json"), &summary)?;
    evidence::register(&dir, "Source bound mount diagnostic collection")?;
    evidence::emit_public_summary("Source-bound mount diagnostic collection", &summary)?;
    result?;
    require(
        before == after,
        "mount diagnostic source changed during execution",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collection_requires_owner_receipt_cleanup_and_no_qualification_promotion() {
        let summary = json!({"owner_exit_code":0,"receipt_valid":true,"cleanup_observed":true,
            "requirement":{"satisfied":true},"diagnostics":{"failure_count":0},"receipt":{"observations_complete":true},
            "custody_policy_qualified":false,"full_product_qualified":false,"release_ready":false});
        assert!(collection_complete(&summary));
        for mutation in 0..7 {
            let mut changed = summary.clone();
            match mutation {
                0 => changed["owner_exit_code"] = json!(1),
                1 => changed["cleanup_observed"] = json!(false),
                2 => changed["receipt_valid"] = json!(false),
                3 => changed["requirement"]["satisfied"] = json!(false),
                4 => changed["diagnostics"]["failure_count"] = json!(1),
                5 => changed["receipt"]["observations_complete"] = json!(false),
                _ => changed["custody_policy_qualified"] = json!(true),
            }
            assert!(!collection_complete(&changed));
        }
    }
    fn fixture() -> (Value, Value) {
        let contract = contract().unwrap();
        let context = json!({"kind":"github-hosted-linux","hosted_qualification_admitted":true,"repository":"modootoday/humanymous","run_id":1,"run_attempt":1,"job":"probe","workflow_sha":"a".repeat(40),"authenticated_provenance":false,"requires_external_job_verification":true});
        let rows: Vec<_> = contract["cases"].as_array().unwrap().iter().map(|case| {
            let role = contract["roles"].as_array().unwrap().iter().find(|role| role["name"] == case["role"]).unwrap();
            json!({"role":case["role"],"surface":case["surface"],"status":"observed_refusal","paths":role["paths"].as_array().unwrap().iter().map(|path|json!({"id":path["id"],"status":"observed_refusal","code":"actor_mount_lookup_denied"})).collect::<Vec<_>>()})
        }).collect();
        (
            json!({"schema":contract["report_schema"],"source_sha":"a".repeat(40),"source_manifest_sha256":"b".repeat(64),"recipe_sha256":"c".repeat(64),"image_sha256":format!("sha256:{}","d".repeat(64)),"private_receipt_sha256":"e".repeat(64),"run_context":context,"observations":rows,"totals":{"scheduled":4,"observed":4,"observed_accept":0,"observed_refusal":4,"unobserved":0},"observations_complete":true,"raw_contents_included":false,"full_product_qualified":false}),
            context,
        )
    }
    fn check(value: &Value, context: &Value) -> Result<Value> {
        validate(
            value,
            &"a".repeat(40),
            &"b".repeat(64),
            &"e".repeat(64),
            &"c".repeat(64),
            context,
        )
    }
    #[test]
    fn observed_denial_is_complete_data_and_never_product_qualification() {
        let (value, context) = fixture();
        let safe = check(&value, &context).unwrap();
        assert_eq!(safe["observations_complete"], true);
        assert_eq!(safe["totals"]["observed_refusal"], 4);
        assert_eq!(safe["full_product_qualified"], false);
    }
    #[test]
    fn mount_consumer_rejects_forged_counts_unknown_codes_and_private_fields() {
        let (value, context) = fixture();
        for mutation in 0..6 {
            let mut changed = value.clone();
            match mutation {
                0 => changed["totals"]["observed_accept"] = json!(4),
                1 => changed["observations"][0]["paths"][0]["code"] = json!("PRIVATE_CANARY"),
                2 => changed["observations"][0]["paths"][0]["raw"] = json!("PRIVATE_CANARY"),
                3 => changed["run_context"]["run_id"] = json!(2),
                4 => changed["private_receipt_sha256"] = json!("f".repeat(64)),
                _ => changed["full_product_qualified"] = json!(true),
            }
            assert!(check(&changed, &context).is_err());
        }
    }
    #[test]
    fn unobserved_data_stays_incomplete_without_invented_image_or_paths() {
        let (mut value, context) = fixture();
        for row in value["observations"].as_array_mut().unwrap() {
            row["status"] = json!("not_observed");
            row["paths"] = json!([]);
        }
        value["image_sha256"] = Value::Null;
        value["totals"] = json!({"scheduled":4,"observed":0,"observed_accept":0,"observed_refusal":0,"unobserved":4});
        value["observations_complete"] = json!(false);
        assert_eq!(
            check(&value, &context).unwrap()["observations_complete"],
            false
        );
        value["observations_complete"] = json!(true);
        assert!(check(&value, &context).is_err());
    }
}
