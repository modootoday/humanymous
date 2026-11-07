//! Consume the lab's sole eight-case projection; never reimplement Go semantics.
use crate::{
    Result, command, evidence,
    lab_receipts::{digest, fields, hosted_context},
    read_json, require, source, write_json,
};
use serde_json::{Value, json};
use std::path::Path;
fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../test/lab/runner/go-public-receipts.json"
    ))
    .map_err(|e| e.to_string())
}
fn owner() -> Result<Value> {
    serde_json::from_str(include_str!("../../../../adapters/go/pilot-contract.json"))
        .map_err(|e| e.to_string())
}
fn requirement(value: &Value) -> Result<Value> {
    let contract = contract()?;
    require(
        contract["bounded_cli_flag"] == "--require-go-component"
            && contract["final_requirement_field"] == "requirement",
        "Go bounded CLI contract mismatch",
    )?;
    fields(value, &contract["requirement_fields"])?;
    require(
        value["schema"] == contract["requirement_schema"]
            && value["kind"] == contract["bounded_requirement_kind"]
            && value["satisfied"].is_boolean()
            && value["full_product_qualified"] == false,
        "Go bounded requirement mismatch",
    )?;
    Ok(value.clone())
}
fn validate(
    value: &Value,
    head: &str,
    manifest: &str,
    raw: &str,
    expected_context: &Value,
) -> Result<Value> {
    let contract = contract()?;
    let owner = owner()?;
    require(
        contract["schema"] == "humanymous.lab-go-public-contract/1",
        "Go public contract schema mismatch",
    )?;
    fields(value, &contract["public_fields"])?;
    require(
        value["schema"] == contract["public_schema"]
            && value["private_receipt_schema"] == contract["private_report_schema"]
            && value["profile"] == owner["qualification_profile"]
            && value["source_sha"] == head
            && value["source_manifest_sha256"] == manifest
            && value["private_receipt_sha256"] == raw
            && digest(&value["source_sha"], 40)
            && digest(&value["source_manifest_sha256"], 64)
            && digest(&value["private_receipt_sha256"], 64)
            && value["raw_contents_included"] == false
            && value["full_product_qualified"] == false,
        "Go source/private-receipt binding mismatch",
    )?;
    let ctx = &value["run_context"];
    fields(ctx, &contract["hosted_context_fields"])?;
    require(
        ctx == expected_context
            && ctx["kind"] == "github-hosted-linux"
            && ctx["hosted_qualification_admitted"] == true
            && ctx["repository"] == "modootoday/humanymous"
            && ctx["workflow_sha"] == head
            && ctx["authenticated_provenance"] == false
            && ctx["requires_external_job_verification"] == true,
        "Go receipt does not match this hosted run",
    )?;
    let infrastructure: Value = serde_json::from_str(include_str!(
        "../../../../test/lab/runner/safe-diagnostics.json"
    ))
    .map_err(|e| e.to_string())?;
    let safe = |code: &Value| {
        code.is_null()
            || infrastructure["error_codes"]
                .as_array()
                .is_some_and(|codes| codes.contains(code))
            || owner["evaluation_error_codes"]
                .as_array()
                .is_some_and(|codes| codes.contains(code))
    };
    require(
        safe(&value["setup_or_qualification_error_code"]),
        "Go setup error not allowlisted",
    )?;
    let rows = value["case_statuses"]
        .as_array()
        .ok_or("Go public rows absent")?;
    let inventory = owner["cases"]
        .as_array()
        .ok_or("Go owner inventory absent")?;
    require(
        rows.len() == inventory.len()
            && rows.len() as u64
                == contract["scheduled"]
                    .as_u64()
                    .ok_or("Go scheduled count absent")?,
        "Go exact denominator mismatch",
    )?;
    let (mut passed, mut failed, mut not_run, mut evaluated, mut assertions, mut unknown) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut names = std::collections::BTreeSet::new();
    for (index, (row, expected)) in rows.iter().zip(inventory).enumerate() {
        fields(row, &contract["case_fields"])?;
        require(
            row["index"].as_u64() == Some(index as u64)
                && row["id"] == expected["id"]
                && names.insert(row["id"].as_str().ok_or("Go ID absent")?)
                && safe(&row["error_code"]),
            "Go case inventory/error mismatch",
        )?;
        let measured = row["evaluated"]
            .as_bool()
            .ok_or("Go evaluated flag absent")?;
        let known = row["assertions_known"]
            .as_bool()
            .ok_or("Go assertion knowledge absent")?;
        require(
            known == row["assertions"].as_u64().is_some() && (known || row["assertions"].is_null()),
            "Go assertion knowledge inconsistent",
        )?;
        let count = row["assertions"].as_u64();
        if measured {
            require(
                count.is_some_and(|n| {
                    n > 0 && Some(n) <= contract["max_assertions_per_case"].as_u64()
                }),
                "Go measured assertion bound mismatch",
            )?;
            evaluated += 1;
        }
        match row["status"].as_str() {
            Some("pass") => {
                require(
                    measured && known && row["error_code"].is_null(),
                    "Go pass lacks evaluation",
                )?;
                passed += 1;
            }
            Some("fail") => {
                require(
                    !row["error_code"].is_null() && (measured || !known),
                    "Go failure accounting invalid",
                )?;
                failed += 1;
            }
            Some("not_run") => {
                require(
                    !measured && known && count == Some(0) && row["error_code"].is_null(),
                    "Go NOT_RUN invented execution",
                )?;
                not_run += 1;
            }
            _ => return Err("Go unknown terminal status".into()),
        }
        assertions += count.unwrap_or(0);
        unknown += u64::from(!known);
    }
    fields(&value["totals"], &contract["totals_fields"])?;
    require(
        value["totals"]
            == json!({"scheduled":rows.len(),"passed":passed,"failed":failed,"not_run":not_run,"evaluated":evaluated,"known_assertions":assertions,"unknown_assertion_cases":unknown}),
        "Go totals mismatch",
    )?;
    let qualified = value["qualified_component"]
        .as_bool()
        .ok_or("Go owner qualification absent")?;
    require(
        !qualified
            || (passed == rows.len() as u64
                && failed == 0
                && not_run == 0
                && evaluated == passed
                && unknown == 0
                && value["setup_or_qualification_error_code"].is_null()),
        "Go false component qualification",
    )?;
    Ok(value.clone())
}
pub fn run(root: &Path) -> Result<()> {
    require(
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var_os("ACT").is_none(),
        "Go product requires genuine hosted Linux context",
    )?;
    let before = source::snapshot(root)?;
    require(
        before["dirty"] == false
            && before["source_sha"] == std::env::var("GITHUB_SHA").unwrap_or_default(),
        "Go product needs clean exact workflow source",
    )?;
    let dir = evidence::bundle(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    write_json(&dir.join("delivery-source-before.json"), &before)?;
    let mut summary = json!({"schema":"humanymous.go-delivery-summary/1","source_sha":before["source_sha"],"source_manifest_sha256":before["manifest_sha256"],"owner_exit_code":null,"receipt":null,"receipt_valid":false,"cleanup_observed":false,"component_status":"not_verified","full_product_gate":"not_requested_in_bounded_profile","full_product_qualified":false,"release_ready":false});
    let result: Result<()> = (|| {
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
                dir.to_str().ok_or("Go private path invalid")?,
                "--go-product",
                "--require-go-component",
            ],
            &dir.join("go-owner.log"),
        )?;
        summary["owner_exit_code"] = json!(code);
        let lab = read_json(&dir.join("lab-report.json"))?;
        summary["requirement"] = requirement(&lab["requirement"])?;
        summary["diagnostics"] =
            crate::lab_diagnostics::validate(&lab)?.ok_or("Go lab diagnostics missing")?;
        require(
            lab["schema"] == "humanymous.product-lab/2" && lab["full_product_qualified"] == false,
            "Go lab report boundary mismatch",
        )?;
        let manifest = read_json(&dir.join("source-manifest.json"))?;
        require(
            manifest["head"] == before["source_sha"],
            "Go frozen source differs",
        )?;
        let raw = dir.join("go-product-report.json");
        let metadata = std::fs::symlink_metadata(&raw).map_err(|e| e.to_string())?;
        require(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 1048576,
            "Go original report not bounded regular file",
        )?;
        let receipt = validate(
            &lab["public_receipts"]["go"],
            before["source_sha"].as_str().ok_or("Go source missing")?,
            &source::file(&dir.join("source-manifest.json"))?,
            &source::file(&raw)?,
            &hosted_context()?,
        )?;
        summary["receipt"] = receipt;
        summary["receipt_valid"] = json!(true);
        let cleanup = lab["cases"]
            .as_array()
            .ok_or("Go lab case ledger missing")?
            .iter()
            .filter(|case| case["id"] == "cleanup-owned-project")
            .collect::<Vec<_>>();
        let cleaned = cleanup.len() == 1 && cleanup[0]["status"] == "passed";
        summary["cleanup_observed"] = json!(cleaned);
        require(cleaned, "Go owned cleanup not observed")?;
        require(code == 0, "Go owning bounded requirement failed")?;
        require(
            summary["requirement"]["satisfied"] == true,
            "Go bounded owner requirement not satisfied",
        )?;
        require(
            summary["diagnostics"]["failure_count"] == 0,
            "Go lab assertions failed",
        )?;
        require(
            summary["receipt"]["qualified_component"] == true,
            "Go eight-case component incomplete",
        )?;
        summary["component_status"] = json!("passed");
        Ok(())
    })();
    let after = source::snapshot(root)?;
    summary["source_changed"] = json!(before != after);
    if before != after {
        summary["component_status"] = json!("source_changed");
    }
    write_json(&dir.join("delivery-source-after.json"), &after)?;
    write_json(&dir.join("go-delivery-summary.json"), &summary)?;
    write_json(&root.join("target/delivery-go/summary.json"), &summary)?;
    evidence::register(&dir, "Source bound Go eight case hosted execution")?;
    evidence::emit_public_summary("Source-bound Go eight-case component", &summary)?;
    result?;
    require(before == after, "Go source changed during execution")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        let owner = owner().unwrap();
        json!({"schema":"humanymous.lab-go-public/1","profile":owner["qualification_profile"],"private_receipt_schema":"humanymous.go-product-ledger/1","private_receipt_sha256":"c".repeat(64),"source_sha":"a".repeat(40),"source_manifest_sha256":"b".repeat(64),"run_context":{"kind":"github-hosted-linux","hosted_qualification_admitted":true,"repository":"modootoday/humanymous","run_id":1,"run_attempt":1,"job":"product","workflow_sha":"a".repeat(40),"authenticated_provenance":false,"requires_external_job_verification":true},"case_statuses":owner["cases"].as_array().unwrap().iter().enumerate().map(|(index,case)|json!({"index":index,"id":case["id"],"status":"not_run","evaluated":false,"assertions":0,"assertions_known":true,"error_code":null})).collect::<Vec<_>>(),"totals":{"scheduled":8,"passed":0,"failed":0,"not_run":8,"evaluated":0,"known_assertions":0,"unknown_assertion_cases":0},"setup_or_qualification_error_code":null,"qualified_component":false,"raw_contents_included":false,"full_product_qualified":false})
    }
    fn check(v: &Value) -> Result<Value> {
        validate(
            v,
            &"a".repeat(40),
            &"b".repeat(64),
            &"c".repeat(64),
            &fixture()["run_context"],
        )
    }
    #[test]
    fn not_run_stays_unqualified_and_private_fields_are_refused() {
        let mut typed = json!({"schema":"humanymous.lab-requirement/1","kind":"go-component","satisfied":true,"full_product_qualified":false});
        assert!(requirement(&typed).is_ok());
        typed["kind"] = json!("full-product");
        assert!(requirement(&typed).is_err());
        let original = fixture();
        assert!(check(&original).is_ok());
        for mutation in 0..4 {
            let mut v = original.clone();
            match mutation {
                0 => v["qualified_component"] = json!(true),
                1 => v["private_body"] = json!("private-canary"),
                2 => v["case_statuses"][1]["index"] = json!(0),
                _ => v["run_context"]["run_id"] = json!(2),
            }
            assert!(check(&v).is_err());
        }
        let mut qualified = original;
        for row in qualified["case_statuses"].as_array_mut().unwrap() {
            row["status"] = json!("pass");
            row["evaluated"] = json!(true);
            row["assertions"] = json!(1);
        }
        qualified["totals"] = json!({"scheduled":8,"passed":8,"failed":0,"not_run":0,"evaluated":8,"known_assertions":8,"unknown_assertion_cases":0});
        qualified["qualified_component"] = json!(true);
        assert!(check(&qualified).is_ok());
    }
    #[test]
    fn measured_failures_cannot_invent_assertions_or_error_codes() {
        let mut v = fixture();
        v["case_statuses"][0] = json!({"index":0,"id":v["case_statuses"][0]["id"],"status":"fail","evaluated":false,"assertions":null,"assertions_known":false,"error_code":"execution_failed"});
        v["totals"] = json!({"scheduled":8,"passed":0,"failed":1,"not_run":7,"evaluated":0,"known_assertions":0,"unknown_assertion_cases":1});
        assert!(check(&v).is_ok());
        v["case_statuses"][0]["error_code"] = json!("private-canary");
        assert!(check(&v).is_err());
    }
}
