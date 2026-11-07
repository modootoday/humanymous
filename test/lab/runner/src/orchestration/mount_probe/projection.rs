//! Pure custody observation validation and public byte-bound projection.
use crate::{
    docker::Result,
    mount_policy::{Facts, Lookup},
    snapshot,
};
use humanymous_test_support::json::decode_strict_json;
use serde_json::{Value, json};
pub(super) fn contract() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../mount-probe-contract.json"
    ))?)
}
fn hex(value: &Value, n: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == n
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub(super) fn role<'a>(contract: &'a Value, name: &Value) -> Result<&'a Value> {
    contract["roles"]
        .as_array()
        .and_then(|r| r.iter().find(|v| v["name"] == *name))
        .ok_or_else(|| "mount probe unknown role".into())
}
fn optional(value: &Value) -> Result<Option<u32>> {
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(u32::try_from(
            value.as_u64().ok_or("mount integer invalid")?,
        )?))
    }
}
pub(super) fn paths(observation: &Value, role: &Value, contract: &Value) -> Result<Vec<Value>> {
    if observation["schema"] != contract["observation_schema"]
        || observation["role"] != role["name"]
        || observation["kernel_identity_verified"] != true
        || observation["kernel_hardening_verified"] != true
        || observation["full_product_qualified"] != false
    {
        return Err("mount observation identity/schema refused".into());
    }
    let rows = observation["paths"]
        .as_array()
        .filter(|r| r.len() == 2)
        .ok_or("two mount paths required")?;
    let expected = role["paths"]
        .as_array()
        .ok_or("mount role path inventory")?;
    let uid = u32::try_from(role["uid"].as_u64().ok_or("mount role UID")?)?;
    let mut result = Vec::new();
    for (row, expected) in rows.iter().zip(expected) {
        if row["id"] != expected["id"] {
            return Err("mount fixed path inventory mismatch".into());
        }
        let lookup = match row["lookup"].as_str() {
            Some("Found") => Lookup::Found,
            Some("Missing") => Lookup::Missing,
            Some("Denied") => Lookup::Denied,
            Some("Other") => Lookup::Other,
            _ => return Err("mount lookup classification invalid".into()),
        };
        let facts = Facts {
            lookup,
            directory: row["directory"].as_bool().ok_or("mount directory flag")?,
            symlink: row["symlink"].as_bool().ok_or("mount symlink flag")?,
            uid: optional(&row["uid"])?,
            gid: optional(&row["gid"])?,
            mode: optional(&row["mode"])?,
            os_error: if row["os_error"].is_null() {
                None
            } else {
                Some(i32::try_from(
                    row["os_error"].as_i64().ok_or("mount OS error invalid")?,
                )?)
            },
        };
        let populated = facts.uid.is_some() && facts.gid.is_some() && facts.mode.is_some();
        if lookup == Lookup::Found && (!populated || facts.os_error.is_some())
            || lookup != Lookup::Found
                && (facts.uid.is_some()
                    || facts.gid.is_some()
                    || facts.mode.is_some()
                    || facts.directory
                    || facts.symlink)
        {
            return Err("mount unknown facts must remain absent".into());
        }
        let refusal = facts.refusal(uid).map(|v| v.code());
        if row["policy_code"] != json!(refusal) {
            return Err("mount claimed predicate differs from shared evaluator".into());
        }
        result.push(json!({"id":expected["id"],"status":if refusal.is_some(){"observed_refusal"}else{"observed_accept"},"code":refusal}));
    }
    Ok(result)
}
pub(super) fn project(
    ledger: &Value,
    raw: &[u8],
    source: &Value,
    manifest: &str,
    recipe: &str,
    context: &Value,
) -> Result<Value> {
    let contract = contract()?;
    let context = crate::orchestration::go_public::public_context(context)?;
    if decode_strict_json(raw, 1048576)? != *ledger
        || ledger["schema"] != contract["private_schema"]
        || ledger["full_product_qualified"] != false
        || !hex(&source["head"], 40)
        || !hex(&json!(manifest), 64)
        || !hex(&json!(recipe), 64)
    {
        return Err("mount original/source boundary refused".into());
    }
    if !ledger["image_sha256"].is_null()
        && !ledger["image_sha256"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .is_some_and(|s| hex(&json!(s), 64))
    {
        return Err("mount image ID invalid".into());
    }
    let rows = ledger["observations"]
        .as_array()
        .filter(|r| r.len() == 4)
        .ok_or("four mount observations required")?;
    let mut observations = Vec::new();
    let (mut accepted, mut refused, mut unobserved) = (0u64, 0u64, 0u64);
    for (row, expected) in rows
        .iter()
        .zip(contract["cases"].as_array().ok_or("mount inventory")?)
    {
        if row["role"] != expected["role"] || row["surface"] != expected["surface"] {
            return Err("mount role/surface accounting mismatch".into());
        }
        let (status, paths) = if row["observation"].is_null() {
            unobserved += 1;
            ("not_observed", json!([]))
        } else {
            if ledger["image_sha256"].is_null() {
                return Err("observed mounts require actual image binding".into());
            }
            let paths = paths(
                &row["observation"],
                role(&contract, &row["role"])?,
                &contract,
            )?;
            if paths.iter().any(|p| p["status"] == "observed_refusal") {
                refused += 1;
                ("observed_refusal", json!(paths))
            } else {
                accepted += 1;
                ("observed_accept", json!(paths))
            }
        };
        observations.push(json!({"role":expected["role"],"surface":expected["surface"],"status":status,"paths":paths}));
    }
    Ok(
        json!({"schema":contract["report_schema"],"source_sha":source["head"],"source_manifest_sha256":manifest,"recipe_sha256":recipe,"image_sha256":ledger["image_sha256"],"private_receipt_sha256":snapshot::digest(raw),"run_context":context,"observations":observations,"totals":{"scheduled":4,"observed":accepted+refused,"observed_accept":accepted,"observed_refusal":refused,"unobserved":unobserved},"observations_complete":unobserved==0,"raw_contents_included":false,"full_product_qualified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observed_refusal_is_complete_diagnostic_but_never_qualification() {
        let contract = contract().unwrap();
        let rows:Vec<_>=contract["cases"].as_array().unwrap().iter().map(|case|{
            let role=role(&contract,&case["role"]).unwrap();
            let paths:Vec<_>=role["paths"].as_array().unwrap().iter().map(|path|json!({"id":path["id"],"lookup":"Found","directory":true,"symlink":false,"uid":role["uid"],"gid":role["uid"],"mode":0o40700,"policy_code":null,"private":"PRIVATE_KEY"})).collect();
            json!({"role":case["role"],"surface":case["surface"],"observation":{"schema":contract["observation_schema"],"role":case["role"],"kernel_identity_verified":true,"kernel_hardening_verified":true,"paths":paths,"full_product_qualified":false}})
        }).collect();
        let mut ledger = json!({"schema":contract["private_schema"],"image_sha256":format!("sha256:{}","d".repeat(64)),"observations":rows,"full_product_qualified":false});
        ledger["observations"][0]["observation"]["paths"][0]["uid"] = json!(0);
        ledger["observations"][0]["observation"]["paths"][0]["policy_code"] =
            json!("actor_mount_owner");
        let source = json!({"head":"a".repeat(40)});
        let manifest = "b".repeat(64);
        let recipe = "c".repeat(64);
        let mut context=crate::orchestration::go_public::public_context(&json!({"kind":"local-unqualified","hosted_qualification_admitted":false,"run_id":null,"run_attempt":null,"authenticated_provenance":false,"requires_external_job_verification":true})).unwrap();
        context["private"] = json!("PRIVATE_CONTEXT");
        let raw = serde_json::to_vec(&ledger).unwrap();
        let public = project(&ledger, &raw, &source, &manifest, &recipe, &context).unwrap();
        assert_eq!(public["totals"]["observed"], 4);
        assert_eq!(public["totals"]["observed_refusal"], 1);
        assert_eq!(public["observations_complete"], true);
        assert_eq!(public["full_product_qualified"], false);
        assert!(!public.to_string().contains("PRIVATE"));
        assert!(public["observations"][0]["paths"][0].get("uid").is_none());
        assert!(
            project(
                &ledger,
                b"changed original",
                &source,
                &manifest,
                &recipe,
                &context
            )
            .is_err()
        );
        ledger["observations"][0]["observation"]["paths"][0]["policy_code"] = Value::Null;
        assert!(
            project(
                &ledger,
                &serde_json::to_vec(&ledger).unwrap(),
                &source,
                &manifest,
                &recipe,
                &context
            )
            .is_err()
        );
        ledger["observations"][0]["observation"] = Value::Null;
        let partial = project(
            &ledger,
            &serde_json::to_vec(&ledger).unwrap(),
            &source,
            &manifest,
            &recipe,
            &context,
        )
        .unwrap();
        assert_eq!(partial["observations_complete"], false);
        assert_eq!(partial["totals"]["unobserved"], 1);
    }
    #[test]
    fn diagnostic_target_is_runtime_ancestor_with_one_unchanged_copy_recipe() {
        let recipe = include_str!("../../../../../../deployments/lab/images/product.Dockerfile");
        assert!(recipe.contains("FROM custody-layout AS runtime"));
        let copies: Vec<_> = recipe
            .lines()
            .filter(|line| line.starts_with("COPY --from=custody-directories "))
            .collect();
        assert_eq!(copies.len(), 4);
        for line in copies {
            assert!(line.contains("--chmod=0700 /empty/"));
        }
        let contract = contract().unwrap();
        let codes: Value =
            serde_json::from_str(include_str!("../../../safe-diagnostics.json")).unwrap();
        for code in contract["refusal_codes"].as_array().unwrap() {
            assert!(codes["error_codes"].as_array().unwrap().contains(code));
        }
    }
}
