//! Execute the owning Node/Bun Docker fixture without granting package trust.
use super::invoke;
use crate::{Result, require};
use serde_json::{Value, json};
use std::path::Path;
fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../../adapters/node/host-pilot-contract.json"
    ))
    .map_err(|e| e.to_string())
}
fn counts(text: &str) -> Result<Value> {
    let contract = contract()?;
    require(
        contract["schema"] == "humanymous.node-host-pilot-contract/1"
            && contract["requiredHarnessCounts"]["passed"] == 5,
        "Node host contract mismatch",
    )?;
    let required = contract["requiredActualGroups"]
        .as_array()
        .ok_or("Node required groups absent")?;
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    require(
        required.len() == 3
            && summaries.len() == 1
            && summaries[0].starts_with(
                "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
            )
            && required.iter().all(|name| {
                name.as_str()
                    .is_some_and(|name| text.matches(&format!("test {name} ... ")).count() == 1)
            }),
        "required five Node host groups incomplete",
    )?;
    let rows: Vec<Value> = text
        .lines()
        .filter_map(|line| {
            line.find('{')
                .and_then(|start| serde_json::from_str(&line[start..]).ok())
        })
        .filter(|row: &Value| row.get("authority").is_some())
        .collect();
    require(
        rows.len() as u64
            == contract["requiredDistinctRows"]
                .as_u64()
                .ok_or("Node row contract absent")?,
        "Node runtime row count mismatch",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    let mut totals = [0u64; 4];
    for row in rows {
        let runtime = row["runtime"].as_str().ok_or("Node runtime row absent")?;
        let framework = row["framework"]
            .as_str()
            .ok_or("Node framework row absent")?;
        let authority = row["authority"]
            .as_str()
            .ok_or("Node authority row absent")?;
        require(
            contract["requiredTuples"]
                .as_array()
                .ok_or("Node tuple contract absent")?
                .iter()
                .any(|tuple| tuple["runtime"] == runtime && tuple["framework"] == framework),
            "unknown Node runtime tuple",
        )?;
        require(
            seen.insert((
                runtime.to_owned(),
                framework.to_owned(),
                authority.to_owned(),
            )),
            "duplicate Node runtime row",
        )?;
        require(
            row.get("fullProduct").is_none_or(|value| value == false),
            "Node component cannot claim full product",
        )?;
        let expected = contract["requiredRowsPerTuple"]
            .as_array()
            .ok_or("Node row contract absent")?
            .iter()
            .find(|expected| expected["authority"] == authority)
            .ok_or("unknown Node authority profile")?;
        require(
            row["originEffects"] == expected["originEffects"],
            "Node origin effect mismatch",
        )?;
        match authority {
            "real-engine-synthetic-evidence" => {
                require(
                    row["allowCases"] == expected["allowCases"]
                        && row["auditRecords"] == expected["auditRecords"]
                        && row["negativeCases"] == expected["negativeCasesByFramework"][framework]
                        && ["binaryBody", "sse", "asyncIterator"]
                            .iter()
                            .all(|key| row[*key] == "preserved"),
                    "Node synthetic authority receipt mismatch",
                )?;
                totals[1] += row["allowCases"]
                    .as_u64()
                    .ok_or("Node ALLOW count invalid")?;
                totals[2] += row["negativeCases"]
                    .as_u64()
                    .ok_or("Node negative count invalid")?;
            }
            "production-application-http" => {
                require(row["status"] == "passed", "Node production row failed")?;
                totals[0] += expected["cases"]
                    .as_u64()
                    .ok_or("Node production count invalid")?;
            }
            "unavailable-negative" => {
                require(
                    row["status"] == "passed" && row["cases"] == expected["cases"],
                    "Node unavailable row failed",
                )?;
                totals[3] += row["cases"]
                    .as_u64()
                    .ok_or("Node unavailable count invalid")?;
            }
            _ => return Err("unsupported Node profile".into()),
        }
    }
    let actual = json!({"productionNonAllow":totals[0],"syntheticAllow":totals[1],"syntheticAuthorityNegative":totals[2],"unavailableNegative":totals[3],"total":totals.iter().sum::<u64>()});
    require(
        actual == contract["requiredActualCaseCounts"],
        "Node actual case totals mismatch",
    )?;
    Ok(
        json!({"required_groups":5,"passed_groups":5,"failed":0,"ignored":0,"filtered":0,"case_assertions":actual,"distinct_runtime_rows":seen.len(),"production_browser_evidence":false}),
    )
}
fn image(value: &Value, sha: &str, manifest: &str) -> Result<String> {
    let id = value[0]["Id"]
        .as_str()
        .ok_or("Node host image identity absent")?;
    require(
        crate::supply::digest(id)
            && value[0]["Config"]["User"] == "65532:65532"
            && value[0]["Config"]["Labels"]["org.opencontainers.image.revision"] == sha
            && value[0]["Config"]["Labels"]["org.humanymous.source-manifest-sha256"] == manifest
            && value[0]["Config"]["Cmd"]
                == json!([
                    "/source/target/debug/humanymous-node-package",
                    "run-host-tests"
                ]),
        "Node image actor/source/command contract mismatch",
    )?;
    Ok(id.into())
}
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let dockerfile = std::fs::read_to_string(root.join("adapters/node/Dockerfile.pilot"))
        .map_err(|e| e.to_string())?;
    require(
        dockerfile.contains("CARGO_BUILD_JOBS=1") && dockerfile.contains("CARGO_INCREMENTAL=0"),
        "Node Docker build must use one bounded Cargo job",
    )?;
    let stamp = dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.get(..14))
        .ok_or("Node evidence identifier absent")?;
    require(
        stamp.bytes().all(|c| c.is_ascii_digit()),
        "Node evidence identifier invalid",
    )?;
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    );
    let name = format!("hm-ci-node-{stamp}-{nonce}");
    let ownership_label = format!("org.humanymous.delivery-owner={nonce}");
    let tag = format!("humanymous-node-host:ci-{stamp}");
    let revision = report["source_sha"]
        .as_str()
        .ok_or("Node source absent")?
        .to_owned();
    let manifest = report["source_manifest_sha256"]
        .as_str()
        .ok_or("Node manifest absent")?
        .to_owned();
    let revision_label = format!("org.opencontainers.image.revision={revision}");
    let manifest_label = format!("org.humanymous.source-manifest-sha256={manifest}");
    invoke(
        root,
        dir,
        report,
        "node-image-build",
        "docker",
        &[
            "build",
            "--file",
            "adapters/node/Dockerfile.pilot",
            "--tag",
            &tag,
            "--label",
            &revision_label,
            "--label",
            &manifest_label,
            ".",
        ],
        "1200s",
    )?;
    invoke(
        root,
        dir,
        report,
        "node-image-inspect",
        "docker",
        &["image", "inspect", &tag],
        "30s",
    )?;
    let inspected: Value = serde_json::from_slice(
        &std::fs::read(dir.join("node-image-inspect.log")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let id = image(&inspected, &revision, &manifest)?;
    report["image_id"] = json!(id);
    let mut owned_names = Vec::new();
    let execution: Result<()> = (|| {
        for (tool, stage) in [("node", "node-version"), ("bun", "bun-version")] {
            let version_name = format!("{name}-{tool}-version");
            owned_names.push(version_name.clone());
            invoke(
                root,
                dir,
                report,
                stage,
                "docker",
                &[
                    "run",
                    "--name",
                    &version_name,
                    "--label",
                    &ownership_label,
                    "--network",
                    "none",
                    "--read-only",
                    "--cap-drop",
                    "ALL",
                    "--security-opt",
                    "no-new-privileges",
                    "--pids-limit",
                    "32",
                    "--memory",
                    "256m",
                    "--entrypoint",
                    tool,
                    &id,
                    "--version",
                ],
                "30s",
            )?;
            let version = std::fs::read_to_string(dir.join(format!("{stage}.log")))
                .map_err(|e| e.to_string())?;
            let version = version.trim();
            require(
                if tool == "node" {
                    version.starts_with("v24.")
                        && version
                            .bytes()
                            .all(|c| c.is_ascii_digit() || matches!(c, b'v' | b'.'))
                } else {
                    version == "1.3.14"
                },
                "Node/Bun observed version mismatch",
            )?;
            report["runtime_versions"][tool] = json!(version);
        }
        owned_names.push(name.clone());
        invoke(
            root,
            dir,
            report,
            "node-host-tests",
            "docker",
            &[
                "run",
                "--name",
                &name,
                "--label",
                &ownership_label,
                "--network",
                "none",
                "--read-only",
                "--tmpfs",
                "/tmp:rw,nosuid,nodev,size=268435456",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges",
                "--pids-limit",
                "128",
                "--memory",
                "1g",
                "--cpus",
                "2",
                "--init",
                &id,
            ],
            "300s",
        )?;
        report["test_counts"] = counts(
            &std::fs::read_to_string(dir.join("node-host-tests.log")).map_err(|e| e.to_string())?,
        )?;
        invoke(
            root,
            dir,
            report,
            "node-container-inspect",
            "docker",
            &["container", "inspect", &name],
            "30s",
        )?;
        let container: Value = serde_json::from_slice(
            &std::fs::read(dir.join("node-container-inspect.log")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        require(
            container[0]["Image"] == id
                && container[0]["State"]["Running"] == false
                && container[0]["State"]["ExitCode"] == 0,
            "Node container did not stop with exact tested image",
        )
    })();
    if execution.is_err() {
        report["failure_stage"] = report["stage"].clone();
    }
    // Include version probes: killing a Docker client is not container cleanup.
    let mut all_absent = true;
    for (index, owned) in owned_names.iter().enumerate() {
        let ownership_stage = format!("node-ownership-{index}");
        let ownership: Result<bool> = (|| {
            invoke(
                root,
                dir,
                report,
                &ownership_stage,
                "docker",
                &["container", "inspect", owned],
                "30s",
            )?;
            let inspected: Value = serde_json::from_slice(
                &std::fs::read(dir.join(format!("{ownership_stage}.log")))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(
                inspected[0]["Config"]["Labels"]["org.humanymous.delivery-owner"] == nonce
                    && inspected[0]["Image"] == id,
            )
        })();
        // Never remove a container whose exact image and invocation label are unproven.
        if ownership == Ok(true) {
            let _cleanup = invoke(
                root,
                dir,
                report,
                &format!("node-cleanup-{index}"),
                "docker",
                &["rm", "--force", owned],
                "30s",
            );
        }
        let label = format!("node-absence-{index}");
        let filter = format!("name=^/{owned}$");
        let inventory = invoke(
            root,
            dir,
            report,
            &label,
            "docker",
            &[
                "container",
                "ls",
                "--all",
                "--filter",
                &filter,
                "--format",
                "{{.ID}}",
            ],
            "30s",
        );
        let absent = std::fs::read_to_string(dir.join(format!("{label}.log")))
            .is_ok_and(|text| text.trim().is_empty());
        all_absent &= inventory.is_ok() && absent;
    }
    report["container_cleanup_observed"] = json!(all_absent);
    report["full_native_qualified"] = json!(false);
    report["package_qualification_receipt_created"] = json!(false);
    execution?;
    require(
        report["container_cleanup_observed"] == true,
        "owned Node container cleanup incomplete",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_groups_are_not_ignored_helpers_or_case_totals() {
        let contract = contract().unwrap();
        let names = contract["requiredActualGroups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| format!("test {} ... ok\n", name.as_str().unwrap()))
            .collect::<String>();
        let mut rows = String::new();
        for tuple in contract["requiredTuples"].as_array().unwrap() {
            for expected in contract["requiredRowsPerTuple"].as_array().unwrap() {
                let mut row = expected.clone();
                row["runtime"] = tuple["runtime"].clone();
                row["framework"] = tuple["framework"].clone();
                row["fullProduct"] = json!(false);
                if row["authority"] == "real-engine-synthetic-evidence" {
                    row["negativeCases"] = expected["negativeCasesByFramework"]
                        [tuple["framework"].as_str().unwrap()]
                    .clone();
                    for key in ["binaryBody", "sse", "asyncIterator"] {
                        row[key] = json!("preserved");
                    }
                }
                rows.push_str(&format!("{row}\n"));
            }
        }
        let log = format!(
            "{names}{rows}test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        );
        assert_eq!(counts(&log).unwrap()["case_assertions"]["total"], 60);
        assert!(counts(&log.replace("0 ignored", "3 ignored")).is_err());
        assert!(
            counts(&log.replace(
                contract["requiredActualGroups"][0].as_str().unwrap(),
                "unknown"
            ))
            .is_err()
        );
        assert!(counts(&format!("{log}\n{log}")).is_err());
        assert!(counts(&log.replace("\"allowCases\":7", "\"allowCases\":0")).is_err());
        assert!(
            counts(&log.replace("production-application-http", "unavailable-negative")).is_err()
        );
    }
    #[test]
    fn image_identity_cannot_substitute_other_actor_or_source() {
        let value = json!([{"Id":format!("sha256:{}","a".repeat(64)),"Config":{"User":"65532:65532","Labels":{"org.opencontainers.image.revision":"source","org.humanymous.source-manifest-sha256":"manifest"},"Cmd":["/source/target/debug/humanymous-node-package","run-host-tests"]}}]);
        assert!(image(&value, "source", "manifest").is_ok());
        assert!(image(&value, "stale", "manifest").is_err());
        let mut elevated = value;
        elevated[0]["Config"]["User"] = json!("0");
        assert!(image(&elevated, "source", "manifest").is_err());
    }
}
