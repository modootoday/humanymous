//! Sole Go public projection: terminal accounting only, no semantic probe clone.
use crate::{docker::Result, snapshot};
use humanymous_test_support::json::decode_strict_json;
use serde_json::{Value, json};
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn safe_code(value: &Value, allowed: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| {
            allowed
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(*s)))
        })
        .map(str::to_owned)
}
fn label(value: String, repository: bool) -> Result<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b) || (repository && b == b'/'))
        || repository && (value.split('/').count() != 2 || value.split('/').any(str::is_empty))
    {
        return Err("public Go run-context label refused".into());
    }
    Ok(value)
}
fn context_from(hosted: bool, act: bool, get: impl Fn(&str) -> Option<String>) -> Result<Value> {
    if !hosted {
        return Ok(
            json!({"kind":if act{"local-act"}else{"local-unqualified"},"hosted_qualification_admitted":false,"run_id":null,"run_attempt":null,"authenticated_provenance":false,"requires_external_job_verification":true}),
        );
    }
    let number = |name| -> Result<u64> {
        let value = get(name).ok_or("actual hosted run metadata missing")?;
        let number = value.parse::<u64>()?;
        if number == 0 {
            return Err("actual hosted run metadata invalid".into());
        }
        Ok(number)
    };
    let workflow_sha = get("GITHUB_WORKFLOW_SHA").ok_or("hosted workflow SHA missing")?;
    if !hex(&workflow_sha, 40) {
        return Err("hosted workflow SHA invalid".into());
    }
    Ok(
        json!({"kind":"github-hosted-linux","hosted_qualification_admitted":true,"repository":label(get("GITHUB_REPOSITORY").ok_or("hosted repository missing")?,true)?,"run_id":number("GITHUB_RUN_ID")?,"run_attempt":number("GITHUB_RUN_ATTEMPT")?,"job":label(get("GITHUB_JOB").ok_or("hosted job missing")?,false)?,"workflow_sha":workflow_sha,"authenticated_provenance":false,"requires_external_job_verification":true}),
    )
}
pub(super) fn run_context() -> Result<Value> {
    context_from(
        crate::docker::actual_hosted_linux(),
        std::env::var_os("ACT").is_some(),
        |name| std::env::var(name).ok(),
    )
}
pub(super) fn public_context(context: &Value) -> Result<Value> {
    if context["authenticated_provenance"] != false
        || context["requires_external_job_verification"] != true
    {
        return Err("public Go context cannot claim authenticated provenance".into());
    }
    match context["kind"].as_str() {
        Some("github-hosted-linux") => {
            if context["hosted_qualification_admitted"] != true {
                return Err("public Go hosted context not admitted".into());
            }
            context_from(true, false, |name| match name {
                "GITHUB_RUN_ID" => context["run_id"].as_u64().map(|v| v.to_string()),
                "GITHUB_RUN_ATTEMPT" => context["run_attempt"].as_u64().map(|v| v.to_string()),
                "GITHUB_REPOSITORY" => context["repository"].as_str().map(str::to_owned),
                "GITHUB_JOB" => context["job"].as_str().map(str::to_owned),
                "GITHUB_WORKFLOW_SHA" => context["workflow_sha"].as_str().map(str::to_owned),
                _ => None,
            })
        }
        Some("local-act" | "local-unqualified") => {
            if context["hosted_qualification_admitted"] != false
                || !context["run_id"].is_null()
                || !context["run_attempt"].is_null()
            {
                return Err("local Go context contains hosted-run attribution".into());
            }
            context_from(false, context["kind"] == "local-act", |_| None)
        }
        _ => Err("unknown public Go execution context".into()),
    }
}
pub(super) fn project(
    report: &Value,
    raw: &[u8],
    source: &Value,
    manifest_sha: &str,
    context: &Value,
) -> Result<Value> {
    let contract: Value = serde_json::from_str(include_str!("../../go-public-receipts.json"))?;
    let owner: Value = serde_json::from_str(include_str!(
        "../../../../../adapters/go/pilot-contract.json"
    ))?;
    let infrastructure: Value = serde_json::from_str(include_str!("../../safe-diagnostics.json"))?;
    let context = public_context(context)?;
    if decode_strict_json(raw, 1048576)? != *report
        || report["schema"] != contract["private_report_schema"]
        || report["scheduled"] != 8
        || report["full_product_qualified"] != false
        || !matches!(report["status"].as_str(), Some("passed" | "failed"))
    {
        return Err("public Go original report boundary".into());
    }
    let source_sha = source["head"]
        .as_str()
        .filter(|s| hex(s, 40))
        .ok_or("public Go source SHA invalid")?;
    if !hex(manifest_sha, 64) {
        return Err("public Go source-manifest SHA invalid".into());
    }
    let cases = report["cases"]
        .as_array()
        .filter(|v| v.len() == 8)
        .ok_or("public Go requires exactly eight terminal rows")?;
    let inventory = owner["cases"]
        .as_array()
        .filter(|v| v.len() == 8)
        .ok_or("Go owner inventory invalid")?;
    let mut seen = std::collections::BTreeSet::new();
    let (mut passed, mut failed, mut not_run, mut evaluated, mut assertions, mut unknown) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut rows = Vec::new();
    for (index, (row, expected)) in cases.iter().zip(inventory).enumerate() {
        let id = expected["id"].as_str().ok_or("public Go static case ID")?;
        if row["case"] != id || !seen.insert(id) || row["full_product_qualified"] != false {
            return Err("public Go case inventory/qualification mismatch".into());
        }
        if !row["status"].is_null()
            && !matches!(
                row["status"].as_str(),
                Some("not_run" | "error" | "attempted")
            )
        {
            return Err("public Go unknown private terminal status".into());
        }
        let evaluation = &row["evaluation"];
        let measured = matches!(evaluation["status"].as_str(), Some("passed" | "failed"));
        let count = if measured {
            if evaluation["schema"] != contract["evaluation_schema"] {
                return Err("public Go evaluator schema mismatch".into());
            }
            evaluated += 1;
            let value = if evaluation["status"] == "passed" {
                if evaluation["outcome"]["case"] != id
                    || evaluation["outcome"]["failures"] != 0
                    || evaluation["outcome"]["actual_host"] != true
                {
                    return Err("public Go evaluator outcome mismatch".into());
                }
                &evaluation["outcome"]["assertions"]
            } else {
                if evaluation["case"] != id {
                    return Err("public Go failed evaluator case mismatch".into());
                }
                &evaluation["assertions"]
            };
            let count = value
                .as_u64()
                .filter(|n| *n > 0 && *n <= 128)
                .ok_or("public Go assertion count invalid")?;
            assertions += count;
            Some(count)
        } else if row["status"] == "not_run" {
            Some(0)
        } else {
            unknown += 1;
            None
        };
        let (status, code) = if row["status"] == "not_run" {
            if measured || !evaluation.is_null() {
                return Err("public Go NOT_RUN row contains evaluation".into());
            }
            not_run += 1;
            ("not_run", None)
        } else if row["status"] == "error" || row["status"] == "attempted" {
            failed += 1;
            (
                "fail",
                Some(
                    safe_code(&row["error_code"], &infrastructure["error_codes"])
                        .unwrap_or_else(|| "execution_failed".into()),
                ),
            )
        } else if measured && evaluation["status"] == "passed" {
            passed += 1;
            ("pass", None)
        } else if measured {
            failed += 1;
            (
                "fail",
                Some(
                    safe_code(&evaluation["error_code"], &owner["evaluation_error_codes"])
                        .ok_or("Go semantic error code not in owner contract")?,
                ),
            )
        } else {
            return Err("public Go unknown terminal row".into());
        };
        rows.push(json!({"index":index,"id":id,"status":status,"evaluated":measured,"assertions":count,"assertions_known":count.is_some(),"error_code":code}));
    }
    let qualified = report["qualification"]["qualified_component"] == true;
    if qualified
        && (passed != 8
            || failed != 0
            || not_run != 0
            || evaluated != 8
            || context["kind"] != "github-hosted-linux"
            || context["hosted_qualification_admitted"] != true
            || report["qualification"]["schema"] != contract["qualification_schema"]
            || report["qualification"]["scheduled"] != 8
            || report["qualification"]["full_product_qualified"] != false)
        || qualified != (report["status"] == "passed")
    {
        return Err("public Go qualification/accounting/context mismatch".into());
    }
    let stage = if report.get("setup_or_qualification_error").is_some() {
        Some(
            safe_code(
                &report["setup_or_qualification_error"]["stage"],
                &infrastructure["error_codes"],
            )
            .unwrap_or_else(|| "execution_failed".into()),
        )
    } else {
        None
    };
    Ok(
        json!({"schema":contract["public_schema"],"profile":owner["qualification_profile"],"private_receipt_schema":contract["private_report_schema"],"private_receipt_sha256":snapshot::digest(raw),"source_sha":source_sha,"source_manifest_sha256":manifest_sha,"run_context":context,"case_statuses":rows,"totals":{"scheduled":8,"passed":passed,"failed":failed,"not_run":not_run,"evaluated":evaluated,"known_assertions":assertions,"unknown_assertion_cases":unknown},"setup_or_qualification_error_code":stage,"qualified_component":qualified,"raw_contents_included":false,"full_product_qualified":false}),
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn admits_exact_eight_hosted_rows_and_refuses_partial_qualification() {
        use serde_json::json;
        let owner: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../adapters/go/pilot-contract.json"
        ))
        .unwrap();
        let cases: Vec<_> = owner["cases"].as_array().unwrap().iter().map(|case| json!({"case":case["id"],"evaluation":{"schema":"humanymous.go-case-evaluation/1","status":"passed","outcome":{"case":case["id"],"assertions":3,"failures":0,"actual_host":true}},"full_product_qualified":false})).collect();
        let mut report = json!({"schema":"humanymous.go-product-ledger/1","scheduled":8,"status":"passed","cases":cases,"qualification":{"schema":"humanymous.go-lab-qualification/1","qualified_component":true,"scheduled":8,"full_product_qualified":false},"full_product_qualified":false});
        let context = super::context_from(true, false, |name| {
            Some(match name {
                "GITHUB_WORKFLOW_SHA" => "a".repeat(40),
                "GITHUB_REPOSITORY" => "owner/repository".into(),
                "GITHUB_JOB" => "go-component".into(),
                _ => "1".into(),
            })
        })
        .unwrap();
        let source = json!({"head":"a".repeat(40)});
        let manifest = "b".repeat(64);
        let raw = serde_json::to_vec(&report).unwrap();
        let public = super::project(&report, &raw, &source, &manifest, &context).unwrap();
        assert_eq!(public["qualified_component"], true);
        assert_eq!(public["totals"]["evaluated"], 8);
        assert_eq!(public["totals"]["known_assertions"], 24);
        assert_eq!(public["full_product_qualified"], false);
        report["cases"][7]["status"] = json!("not_run");
        report["cases"][7]["evaluation"] = serde_json::Value::Null;
        let partial = serde_json::to_vec(&report).unwrap();
        assert!(super::project(&report, &partial, &source, &manifest, &context).is_err());
        assert!(
            super::project(
                &serde_json::from_slice(&raw).unwrap(),
                &raw,
                &source,
                &manifest,
                &super::context_from(false, true, |_| None).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn keeps_eight_denominator_drops_private_data_and_rejects_false_green() {
        use serde_json::json;
        let owner: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../adapters/go/pilot-contract.json"
        ))
        .unwrap();
        let mut cases:Vec<_>=owner["cases"].as_array().unwrap().iter().map(|v|json!({"case":v["id"],"status":"not_run","evaluation":null,"detail":"PRIVATE_BODY","full_product_qualified":false})).collect();
        cases[0] = json!({"case":owner["cases"][0]["id"],"status":"error","error_code":"actor_writer_keys","evaluation":null,"detail":"PRIVATE_TOKEN","full_product_qualified":false});
        let mut report = json!({"schema":"humanymous.go-product-ledger/1","scheduled":8,"status":"failed","cases":cases,"setup_or_qualification_error":{"stage":"go_fixture","detail":"PRIVATE_PATH"},"full_product_qualified":false});
        let source = json!({"head":"a".repeat(40)});
        let manifest = "b".repeat(64);
        let mut context =
            super::context_from(false, true, |_| Some("PRIVATE_EMULATED_RUN".into())).unwrap();
        context["extra_private_field"] = json!("PRIVATE_CONTEXT");
        let raw = serde_json::to_vec(&report).unwrap();
        let public = super::project(&report, &raw, &source, &manifest, &context).unwrap();
        assert_eq!(public["totals"]["scheduled"], 8);
        assert_eq!(public["totals"]["failed"], 1);
        assert_eq!(public["totals"]["not_run"], 7);
        assert_eq!(public["totals"]["unknown_assertion_cases"], 1);
        assert!(public["case_statuses"][0]["assertions"].is_null());
        assert!(!public.to_string().contains("PRIVATE"));
        assert!(public["run_context"]["run_id"].is_null());
        report["cases"][0]["evaluation"] = json!({"schema":"humanymous.go-case-evaluation/1","status":"passed","outcome":{"case":owner["cases"][0]["id"],"assertions":3,"failures":0,"actual_host":true}});
        let measured = serde_json::to_vec(&report).unwrap();
        let retained = super::project(&report, &measured, &source, &manifest, &context).unwrap();
        assert_eq!(retained["totals"]["failed"], 1);
        assert_eq!(retained["totals"]["passed"], 0);
        assert_eq!(retained["totals"]["evaluated"], 1);
        assert_eq!(retained["case_statuses"][0]["assertions"], 3);
        report["cases"][0]["status"] = json!("unknown");
        let unknown = serde_json::to_vec(&report).unwrap();
        assert!(super::project(&report, &unknown, &source, &manifest, &context).is_err());
        report["cases"][0]["status"] = json!("error");
        report["qualification"] = json!({"qualified_component":true});
        let changed = serde_json::to_vec(&report).unwrap();
        assert!(super::project(&report, &changed, &source, &manifest, &context).is_err());
        assert!(super::project(&report, b"different bytes", &source, &manifest, &context).is_err());
        report["qualification"] = serde_json::Value::Null;
        report["cases"][1]["case"] = report["cases"][0]["case"].clone();
        let changed = serde_json::to_vec(&report).unwrap();
        assert!(super::project(&report, &changed, &source, &manifest, &context).is_err());
    }
}
