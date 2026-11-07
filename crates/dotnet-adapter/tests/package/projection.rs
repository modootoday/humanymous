//! Pure allowlisted projection. A host observation never proves actor cleanup.
use serde_json::{Value, json};
use std::collections::BTreeSet;

type Checked<T> = Result<T, &'static str>;

fn hexadecimal(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn count(value: &Value, key: &str) -> Checked<u64> {
    value[key]
        .as_u64()
        .filter(|n| *n <= 1_000_000)
        .ok_or("invalid_count")
}
fn case_set(value: &Value, known: &BTreeSet<String>) -> Checked<BTreeSet<String>> {
    let rows = value.as_array().ok_or("missing_cases")?;
    if rows.len() > known.len() {
        return Err("case_budget");
    }
    let mut found = BTreeSet::new();
    for row in rows {
        let name = row.as_str().ok_or("invalid_case")?;
        if !known.contains(name) || !found.insert(name.to_owned()) {
            return Err("unknown_or_duplicate_case");
        }
    }
    Ok(found)
}
fn runtime(value: &Value) -> Checked<&str> {
    let version = value
        .as_str()
        .and_then(|s| s.strip_prefix(".NET "))
        .ok_or("runtime_identity")?;
    if version.len() > 24 {
        return Err("runtime_identity");
    }
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts[0] != "10"
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("runtime_identity");
    }
    Ok(version)
}

/// Project only fixed facts; the caller separately proves artifact/source identity.
pub(super) fn project(
    receipt: &Value,
    expected_source: &str,
    expected_native: &str,
) -> Checked<Value> {
    if !hexadecimal(expected_source, 40)
        || !hexadecimal(expected_native, 64)
        || receipt["schema"] != "humanymous.dotnet-package-observation/1"
        || receipt["source_sha"] != expected_source
        || receipt["sdk"] != "10.0.403"
        || receipt["full_product"] != false
        || receipt["publication"] != false
    {
        return Err("receipt_identity");
    }
    let venue = match receipt["execution_environment"].as_str() {
        Some("github-hosted") => "github-hosted",
        Some("root-allocated-local") => "root-allocated-local",
        _ => return Err("venue_identity"),
    };
    let os = match receipt["rid"].as_str() {
        Some("win-x64") => "windows",
        Some("linux-x64") => "linux",
        _ => return Err("tuple_identity"),
    };
    let target = if os == "windows" {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    if receipt["target"] != target
        || receipt["native_profile"] != "release"
        || !receipt["rustc"]
            .as_str()
            .is_some_and(|s| s.starts_with("rustc 1.99.1 "))
    {
        return Err("native_toolchain_identity");
    }
    let mut commitments = serde_json::Map::new();
    for key in ["cargo_lock_sha256", "harness_sha256", "controller_sha256"] {
        let hash = receipt[key]
            .as_str()
            .filter(|s| hexadecimal(s, 64))
            .ok_or("artifact_commitment")?;
        commitments.insert(key.to_owned(), json!(hash));
    }
    let package = receipt["package_sha256"]
        .as_str()
        .filter(|s| hexadecimal(s, 64))
        .ok_or("package_identity")?;
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../adapters/dotnet/pilot-contract.json"
    ))
    .map_err(|_| "contract_identity")?;
    let known: BTreeSet<String> = contract["hostCases"]
        .as_array()
        .ok_or("contract_identity")?
        .iter()
        .map(|row| {
            row["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or("contract_identity")
        })
        .collect::<Checked<_>>()?;
    let runs = receipt["runs"]
        .as_array()
        .filter(|r| r.len() == 4)
        .ok_or("mode_denominator")?;
    let mut modes = BTreeSet::new();
    let mut projected = Vec::new();
    let mut complete = true;
    for run in runs {
        let (name, aot, packed) = match run["mode"].as_str() {
            Some("jit-source") => ("jit-source", false, false),
            Some("jit-packed") => ("jit-packed", false, true),
            Some("aot-source") => ("aot-source", true, false),
            Some("aot-packed") => ("aot-packed", true, true),
            _ => return Err("mode_identity"),
        };
        if !modes.insert(name)
            || run["aot"] != aot
            || run["packed"] != packed
            || run["package_install_verified"] != packed
            || run["native_sha256"] != expected_native
        {
            return Err("mode_identity");
        }
        let host = run["host_sha256"]
            .as_str()
            .filter(|s| hexadecimal(s, 64))
            .ok_or("host_identity")?;
        let profiles = run["profiles"]
            .as_array()
            .filter(|p| p.len() == 2)
            .ok_or("profile_denominator")?;
        let mut profile_names = BTreeSet::new();
        let mut rows = Vec::new();
        let mut missing = None;
        for profile in profiles {
            let synthetic = match profile["profile"].as_str() {
                Some("production-application-http") => false,
                Some("real-engine-audit-synthetic-evidence") => true,
                _ => return Err("profile_identity"),
            };
            if !profile_names.insert(synthetic)
                || profile["schema"] != "humanymous.dotnet-host-observation/1"
                || profile["execution_environment"] != venue
                || profile["os"] != os
                || profile["arch"] != "x86_64"
                || profile["dynamic_code"] != !aot
                || profile["full_product"] != false
                || profile["package_install"] != false
                || count(profile, "failed")? != 0
                || count(profile, "ignored")? != 0
                || profile["remaining_native_resources_before_dispose"] != 0
            {
                return Err("profile_outcome");
            }
            let passed = count(profile, "passed")?;
            let contacts = count(profile, "origin_contacts")?;
            let mutations = count(profile, "origin_mutations")?;
            let collections = count(profile, "gc_collections")?;
            if passed == 0
                || mutations != 0
                || (!synthetic && contacts != 0)
                || (synthetic && (contacts == 0 || collections == 0))
            {
                return Err("profile_counts");
            }
            let covered = case_set(&profile["covered_contract_cases"], &known)?;
            let not_run = case_set(&profile["not_run_contract_cases"], &known)?;
            if covered.contains("package-install")
                || !covered.is_disjoint(&not_run)
                || covered.union(&not_run).cloned().collect::<BTreeSet<_>>() != known
            {
                return Err("case_partition");
            }
            let verdicts = profile["verdict_counts"]
                .as_object()
                .ok_or("verdict_counts")?;
            if verdicts.len() > 3
                || verdicts
                    .keys()
                    .any(|key| !matches!(key.as_str(), "ALLOW" | "CHALLENGE" | "DENY"))
                || verdicts
                    .values()
                    .any(|v| v.as_u64().is_none_or(|n| n > 1_000_000))
            {
                return Err("verdict_counts");
            }
            let verdict_count = |key: &str| verdicts.get(key).and_then(Value::as_u64).unwrap_or(0);
            if (!synthetic && verdict_count("ALLOW") != 0)
                || (synthetic && verdict_count("ALLOW") == 0)
                || (synthetic
                    && covered.contains("deny-challenge")
                    && (verdict_count("DENY") == 0 || verdict_count("CHALLENGE") == 0))
            {
                return Err("verdict_counts");
            }
            if synthetic {
                let mut required = not_run.clone();
                // Installation belongs to the paired packed consumer, not its HTTP delegate.
                required.remove("package-install");
                complete &= required.is_empty();
                missing = Some(required);
            }
            rows.push(json!({"synthetic_evidence":synthetic,"runtime_version":runtime(&profile["runtime"])? ,"assertions_passed":passed,"failed":0,"ignored":0,"covered_contract_cases":covered,"not_run_contract_cases":not_run,"origin_contacts":contacts,"origin_mutations":mutations,"gc_collections":collections,"remaining_request_resources":0,"verdict_counts":{"ALLOW":verdict_count("ALLOW"),"CHALLENGE":verdict_count("CHALLENGE"),"DENY":verdict_count("DENY")}}));
        }
        projected.push(json!({"mode":name,"aot":aot,"packed":packed,"host_sha256":host,"native_sha256":expected_native,"package_install_verified":packed,"required_host_cases_not_run":missing.ok_or("missing_synthetic_profile")?,"profiles":rows}));
    }
    Ok(
        json!({"schema":"humanymous.dotnet-component-summary/1","source_sha":expected_source,"execution_environment":venue,"os":os,"arch":"x86_64","sdk":"10.0.403","rust_toolchain":"1.99.1","target":target,"native_profile":"release","commitments":commitments,"package_sha256":package,"native_sha256":expected_native,"host_contract_complete":complete,"status":if complete{"fixture-complete-cleanup-unverified"}else{"incomplete"},"cleanup":null,"component_qualified":false,"full_product":false,"runs":projected}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Value {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../../adapters/dotnet/pilot-contract.json"
        ))
        .unwrap();
        let all: Vec<Value> = contract["hostCases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].clone())
            .collect();
        let covered: Vec<Value> = all
            .iter()
            .filter(|id| **id != "package-install")
            .cloned()
            .collect();
        let mut runs = Vec::new();
        for (name, aot, packed) in [
            ("jit-source", false, false),
            ("jit-packed", false, true),
            ("aot-source", true, false),
            ("aot-packed", true, true),
        ] {
            let mut profiles = Vec::new();
            for synthetic in [false, true] {
                profiles.push(json!({"schema":"humanymous.dotnet-host-observation/1","execution_environment":"github-hosted","os":"windows","arch":"x86_64","profile":if synthetic{"real-engine-audit-synthetic-evidence"}else{"production-application-http"},"runtime":".NET 10.0.14","dynamic_code":!aot,"passed":1,"failed":0,"ignored":0,"covered_contract_cases":if synthetic{covered.clone()}else{Vec::new()},"not_run_contract_cases":if synthetic{vec![json!("package-install")]}else{all.clone()},"origin_contacts":u64::from(synthetic),"origin_mutations":0,"gc_collections":1,"remaining_native_resources_before_dispose":0,"verdict_counts":if synthetic{json!({"ALLOW":1,"DENY":1,"CHALLENGE":1})}else{json!({"DENY":1})},"full_product":false,"package_install":false,"credential":"secret-marker","private_path":"C:/secret-marker"}));
            }
            runs.push(json!({"mode":name,"aot":aot,"packed":packed,"host_sha256":"c".repeat(64),"native_sha256":"b".repeat(64),"profiles":profiles,"package_install_verified":packed,"all_contract_cases_qualified":true}));
        }
        json!({"schema":"humanymous.dotnet-package-observation/1","execution_environment":"github-hosted","source_sha":"a".repeat(40),"cargo_lock_sha256":"e".repeat(64),"harness_sha256":"f".repeat(64),"controller_sha256":"a".repeat(64),"rustc":"rustc 1.99.1 (unit fixture)","target":"x86_64-pc-windows-msvc","native_profile":"release","rid":"win-x64","sdk":"10.0.403","package_sha256":"d".repeat(64),"runs":runs,"full_product":false,"publication":false,"credential":"secret-marker"})
    }
    #[test]
    fn projection_excludes_private_fields_and_never_invents_cleanup() {
        let projected = project(&sample(), &"a".repeat(40), &"b".repeat(64)).unwrap();
        assert!(!projected.to_string().contains("secret-marker"));
        assert_eq!(projected["host_contract_complete"], true);
        assert_eq!(projected["component_qualified"], false);
        assert_eq!(projected["cleanup"], Value::Null);
    }
    #[test]
    fn missing_cases_remain_incomplete_and_duplicate_modes_are_refused() {
        let mut input = sample();
        input["runs"][0]["profiles"][1]["covered_contract_cases"]
            .as_array_mut()
            .unwrap()
            .retain(|id| *id != "wrong-peer");
        input["runs"][0]["profiles"][1]["not_run_contract_cases"]
            .as_array_mut()
            .unwrap()
            .push(json!("wrong-peer"));
        let projected = project(&input, &"a".repeat(40), &"b".repeat(64)).unwrap();
        assert_eq!(projected["status"], "incomplete");
        input["runs"][1] = input["runs"][0].clone();
        assert_eq!(
            project(&input, &"a".repeat(40), &"b".repeat(64)),
            Err("mode_identity")
        );
    }
    #[test]
    fn wrong_source_failed_counts_or_unknown_case_never_project_as_success() {
        assert_eq!(
            project(&sample(), &"e".repeat(40), &"b".repeat(64)),
            Err("receipt_identity")
        );
        let mut input = sample();
        input["runs"][0]["profiles"][0]["ignored"] = json!(1);
        assert_eq!(
            project(&input, &"a".repeat(40), &"b".repeat(64)),
            Err("profile_outcome")
        );
        let mut input = sample();
        input["runs"][0]["profiles"][1]["covered_contract_cases"][0] = json!("secret-marker");
        assert_eq!(
            project(&input, &"a".repeat(40), &"b".repeat(64)),
            Err("unknown_or_duplicate_case")
        );
    }
    #[test]
    fn oversized_runtime_and_contradictory_verdicts_fail_closed() {
        let mut input = sample();
        input["runs"][0]["profiles"][1]["runtime"] =
            json!(format!(".NET 10.0.{}", "1".repeat(100_000)));
        assert_eq!(
            project(&input, &"a".repeat(40), &"b".repeat(64)),
            Err("runtime_identity")
        );
        let mut input = sample();
        input["runs"][0]["profiles"][1]["verdict_counts"]["CHALLENGE"] = json!(0);
        assert_eq!(
            project(&input, &"a".repeat(40), &"b".repeat(64)),
            Err("verdict_counts")
        );
    }
}
