//! Integrates owning Rust runners; component success cannot qualify a product.
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
pub fn required(kind: &str) -> Result<&'static [&'static str]> {
    match kind {
        "native" => Ok(&[
            "startup",
            "configuration-invalid",
            "doctor",
            "install-dry-run-no-mutation",
            "local-only-admin",
            "restart-replay",
            "storage-failure-closed",
            "documentation-examples",
        ]),
        "browser" | "browser-product" => Ok(&[
            "collector-parity",
            "challenge-keyboard",
            "challenge-assistive",
            "missing-signals-closed",
            "human-control",
            "coherent-browser-ceiling",
            "documentation-examples",
        ]),
        "docker" => Ok(&[
            "attack-catalog",
            "gate",
            "pass",
            "swarm",
            "deployment",
            "human-control",
            "raw-tls-h2",
            "audit-restart",
            "origin-isolation",
            "documentation-examples",
        ]),
        _ => Err("unknown pilot kind".into()),
    }
}
pub fn validate(report: &Value, kind: &str, head: &str, review: &str) -> Result<()> {
    require(
        report["schema"] == 1 && report["kind"] == kind,
        "pilot schema/kind mismatch",
    )?;
    require(
        report["source_sha"] == head && report["documentation_review_sha256"] == review,
        "pilot source/documentation digest stale",
    )?;
    require(
        report["status"] == "passed" && report["qualification"] == "full-product",
        "component/blocked pilot cannot qualify full product",
    )?;
    require(
        report["errors"].as_u64() == Some(0) && report["skips"].as_u64() == Some(0),
        "pilot execution errors/skips are not success",
    )?;
    let cases = report["cases"].as_array().ok_or("pilot cases absent")?;
    let ids: BTreeSet<_> = cases
        .iter()
        .filter_map(|case| case["id"].as_str())
        .collect();
    require(
        ids.len() == cases.len() && required(kind)?.iter().all(|id| ids.contains(id)),
        "duplicate/missing required pilot cases",
    )?;
    require(
        cases.iter().all(|case| {
            case["status"] == "passed" && case["evidence"].as_str().is_some_and(|s| !s.is_empty())
        }),
        "non-passing or unreferenced pilot case",
    )?;
    if kind == "docker" {
        require(
            report["isolation"] == "docker" && report["runtime"] == "rust",
            "only isolated Rust Docker is authoritative",
        )?;
    }
    if kind != "native" {
        require(
            report["images"].as_array().is_some_and(|images| {
                !images.is_empty()
                    && images.iter().all(|image| {
                        image.as_str().is_some_and(|image| {
                            image.strip_prefix("sha256:").is_some_and(|digest| {
                                digest.len() == 64
                                    && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                            })
                        })
                    })
            }),
            "immutable Docker image identities required",
        )?;
    }
    Ok(())
}
pub fn run(root: &Path, kind: &str) -> Result<()> {
    let ids = required(kind)?;
    let before = source::snapshot(root)?;
    let head = before["source_sha"].as_str().ok_or("source SHA missing")?;
    let review = source::file(&root.join("docs/reviews.json"))?;
    let dir = evidence::bundle(root)?;
    write_json(&dir.join("delivery-source-before.json"), &before)?;
    let owner_report = dir.join("owner-report.json");
    let output = owner_report.to_str().ok_or("non-UTF8 report destination")?;
    let mut components = Vec::new();
    if kind == "native" {
        let manifest = root.join("scripts/ci/windows-native-runner/Cargo.toml");
        if manifest.is_file() {
            // The owning runner compiles exact artifacts and verifies their
            // Cargo JSON hashes. Do not repeat its build/accounting authority.
            let binary_dir = root.join("target/release");
            let actor_mode =
                if cfg!(windows) && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
                    "standard-user"
                } else {
                    "native"
                };
            let code = command::log(
                root,
                "cargo",
                &[
                    "run",
                    "--locked",
                    "--manifest-path",
                    "scripts/ci/windows-native-runner/Cargo.toml",
                    "--",
                    actor_mode,
                    "--report",
                    output,
                    "--binaries",
                    binary_dir.to_str().ok_or("non-UTF8 binary directory")?,
                ],
                &dir.join("native-owner.log"),
            )?;
            let report = if owner_report.is_file() {
                read_json(&owner_report)?
            } else {
                json!({"status":"failed","reason":"owner runner produced no report"})
            };
            components.push(json!({"owner":"native-runner","exit_code":code,"report":report}));
        } else {
            components.push(json!({"owner":"native-runner","status":"blocked","reason":"owning Rust native runner not delivered"}));
        }
    } else {
        // Compose/testlab owns orchestration, topology and actual sandbox use.
        // Do not clone that runner into CI or launch a host loopback substitute.
        let manifest = root.join("test/lab/runner/Cargo.toml");
        if manifest.is_file() {
            let mut args = vec![
                "run",
                "--locked",
                "--manifest-path",
                "test/lab/runner/Cargo.toml",
                "--",
                "run",
                "--evidence",
                dir.to_str().ok_or("non-UTF8 evidence directory")?,
            ];
            if let Some(profile) = lab_profile(kind) {
                args.push(profile);
            }
            args.push("--require-full");
            let code = command::log(root, "cargo", &args, &dir.join("lab-owner.log"))?;
            let report = if dir.join("lab-report.json").is_file() {
                read_json(&dir.join("lab-report.json"))?
            } else {
                json!({"status":"failed","reason":"owner produced no lab report"})
            };
            write_json(&owner_report, &report)?;
            let manifest = if dir.join("source-manifest.json").is_file() {
                read_json(&dir.join("source-manifest.json"))?
            } else {
                Value::Null
            };
            let manifest_hash = if dir.join("source-manifest.json").is_file() {
                source::file(&dir.join("source-manifest.json"))?
            } else {
                String::new()
            };
            let raw_binding = report["public_receipts"].get("wargame").is_none()
                || source::file(&dir.join("wargame-report.raw.json"))
                    .is_ok_and(|hash| report["public_receipts"]["wargame"]["raw_sha256"] == hash);
            let receipt_valid =
                raw_binding && crate::lab_receipts::validate(&report, head, &manifest_hash).is_ok();
            let public_receipt = if raw_binding {
                crate::lab_receipts::public(&report, head, &manifest_hash)
            } else {
                json!({"status":"rejected","error_code":"invalid_lab_public_receipt"})
            };
            components.push(
                json!({"owner":"lab-runner","exit_code":code,"report":report,"source":manifest,"public_receipt":public_receipt,"public_receipt_valid":receipt_valid}),
            );
        } else {
            components.push(json!({"owner":"lab-runner","status":"blocked","reason":"owning Rust Docker orchestration not delivered"}));
        }
    }
    let after = source::snapshot(root)?;
    let changed = before["manifest_sha256"] != after["manifest_sha256"]
        || before["source_sha"] != after["source_sha"];
    write_json(&dir.join("delivery-source-after.json"), &after)?;
    if !owner_report.is_file() {
        write_json(
            &owner_report,
            &json!({"status":"blocked","reason":"required owning execution/report was not available","components":components}),
        )?;
    }
    let owner = components
        .iter()
        .find(|component| component.get("report").is_some())
        .map(|component| &component["report"])
        .unwrap_or(&Value::Null);
    let cases:Vec<_>=ids.iter().map(|id| {
        let actual=owner["cases"].as_array().into_iter().flatten().find(|case|case["id"]==*id);
        match actual { Some(case)=>json!({"id":id,"status":case["status"],"evidence":"owner-report.json","detail":case["detail"]}),None=>json!({"id":id,"status":"blocked","evidence":"owner-report.json","detail":"complete product case is not yet qualified by the owning runner"}) }
    }).collect();
    let errors = usize::from(changed)
        + components
            .iter()
            .filter(|component| {
                component["status"] == "failed"
                    || component["report"]["status"] == "failed"
                    || crate::lab_diagnostics::validate(&component["report"]).is_err()
                    || component["public_receipt_valid"] == false
                    || component["exit_code"]
                        .as_i64()
                        .is_some_and(|code| code != 0 && code != 2)
            })
            .count();
    let counters: Vec<_> = components
        .iter()
        .filter(|component| component.get("report").is_some())
        .map(|component| component["report"]["skips"].as_u64())
        .collect();
    let skips = if counters.is_empty() {
        None
    } else {
        counters.into_iter().try_fold(0u64, |sum, value| {
            value.and_then(|value| sum.checked_add(value))
        })
    };
    let full = !changed
        && errors == 0
        && components
            .iter()
            .filter(|component| component.get("report").is_some())
            .all(|component| component["exit_code"] == 0)
        && owner["qualification"] == "full-product"
        && owner["errors"].as_u64() == Some(0)
        && owner["skips"].as_u64() == Some(0)
        && owner["full_product_qualified"] == true
        && owner["source_sha"] == head
        && owner["documentation_review_sha256"] == review
        && cases.iter().all(|case| case["status"] == "passed");
    let mut images = Vec::new();
    for filename in [
        "runtime-image.json",
        "collector-image.json",
        "browser-product-image.json",
        "controls-image.json",
    ] {
        let path = dir.join(filename);
        if path.is_file() {
            let image = read_json(&path)?;
            if let Some(id) = image["Id"].as_str() {
                images.push(id.to_owned());
            }
        }
    }
    let report = json!({"schema":1,"kind":kind,"source_sha":head,"source":before,"documentation_review_sha256":review,"status":if full{"passed"}else if errors>0{"failed"}else{"blocked"},"qualification":if full{"full-product"}else{"component-only"},"full_product_qualified":full,"release_ready":full,"errors":errors,"skips":skips,"cases":cases,"components":components,"source_changed_during_execution":changed,"isolation":if kind=="native"{"native"}else{"docker"},"runtime":"rust","images":images,"private_evidence":dir.strip_prefix(root).map_err(|e|e.to_string())?.to_string_lossy().replace('\\',"/")});
    write_json(&dir.join("delivery-pilot.json"), &report)?;
    evidence::register(&dir, "Source-bound delivery pilot")?;
    let destination = root
        .join("target/delivery-evidence")
        .join(format!("{kind}-{}.json", std::env::consts::OS));
    let mut summary = public_summary(&report);
    summary["private_report_sha256"] = json!(source::file(&dir.join("delivery-pilot.json"))?);
    write_json(&destination, &summary)?;
    // Public CI gets only the same allowlisted summary, never the private bundle.
    evidence::emit_public_summary(&format!("Source-bound {kind} pilot"), &summary)?;
    println!("pilot evidence: {}", destination.display());
    validate(&report, kind, head, &review)
}
fn lab_profile(kind: &str) -> Option<&'static str> {
    match kind {
        "browser" => Some("--collector"),
        "browser-product" => Some("--browser-product"),
        _ => None,
    }
}
fn public_summary(report: &Value) -> Value {
    let mut summary = json!({});
    for key in [
        "schema",
        "kind",
        "source_sha",
        "documentation_review_sha256",
        "status",
        "qualification",
        "full_product_qualified",
        "release_ready",
        "errors",
        "skips",
        "source_changed_during_execution",
        "isolation",
        "runtime",
        "images",
    ] {
        summary[key] = report[key].clone();
    }
    summary["source_manifest_sha256"] = report["source"]["manifest_sha256"].clone();
    summary["canonical_git_tree_sha256"] = report["source"]["canonical_git_tree_sha256"].clone();
    summary["cases"]=json!(report["cases"].as_array().into_iter().flatten().map(|case|json!({"id":case["id"],"status":case["status"],"evidence":"private source-bound owning report"})).collect::<Vec<_>>());
    summary["components"] = json!(
        report["components"]
            .as_array()
            .into_iter()
            .flatten()
            .map(component_summary)
            .collect::<Vec<_>>()
    );
    summary
}
fn component_summary(component: &Value) -> Value {
    let report = &component["report"];
    let counts = report["cases"].as_array().map(|cases| json!({
        "total":cases.len(),
        "passed":cases.iter().filter(|case| case["status"] == "passed").count(),
        "failed":cases.iter().filter(|case| case["status"] == "failed").count(),
        "other":cases.iter().filter(|case| case["status"] != "passed" && case["status"] != "failed").count()
    }));
    // Assertions are the owner's case denominator, never a Rust test count.
    json!({"owner":component["owner"],"exit_code":component["exit_code"],
        "status":report["status"],"qualification":report["qualification"],
        "schema":report["schema"],"full_product_qualified":report["full_product_qualified"],
        "assertions":counts,"required_unavailable_count":report["required_unavailable"].as_array().map(Vec::len),
        "safe_diagnostics":crate::lab_diagnostics::public(report),
        "public_receipt":component.get("public_receipt").cloned().unwrap_or(Value::Null)})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn product_browser_and_component_profiles_are_separate() {
        assert_eq!(lab_profile("browser"), Some("--collector"));
        assert_eq!(lab_profile("browser-product"), Some("--browser-product"));
        assert_eq!(lab_profile("docker"), None);
        assert_eq!(
            required("browser").unwrap(),
            required("browser-product").unwrap()
        );
    }
    #[test]
    fn component_and_stale_reports_cannot_pass() {
        for qualification in ["component-only", "unsupported", ""] {
            let value = json!({"schema":1,"kind":"native","source_sha":"a","documentation_review_sha256":"b","qualification":qualification,"status":"passed","errors":0,"skips":0});
            assert!(validate(&value, "native", "a", "b").is_err());
        }
    }
    #[test]
    fn empty_case_success_is_not_success() {
        let value = json!({"schema":1,"kind":"native","source_sha":"a","documentation_review_sha256":"b","qualification":"full-product","status":"passed","errors":0,"skips":0,"cases":[]});
        assert!(validate(&value, "native", "a", "b").is_err());
    }
    #[test]
    fn errors_are_integer_zero_not_boolean() {
        let value = json!({"schema":1,"kind":"native","source_sha":"a","documentation_review_sha256":"b","qualification":"full-product","status":"passed","errors":false,"skips":false});
        assert!(validate(&value, "native", "a", "b").is_err());
    }
    #[test]
    fn uploaded_summary_does_not_copy_actor_or_raw_trace_fields() {
        let raw = json!({"source_sha":"commit","source":{"manifest_sha256":"digest"},"cases":[{"id":"startup","status":"passed","detail":{"cookie":"private-canary"}}],"components":[{"owner":"native","exit_code":2,"report":{"schema":1,"status":"blocked","actor":{"sid":"private-canary"},"raw_body":"private-canary"}}]});
        let summary = public_summary(&raw);
        assert!(!summary.to_string().contains("private-canary"));
        assert_eq!(summary["source_sha"], "commit");
        assert_eq!(summary["components"][0]["exit_code"], 2);
    }
    #[test]
    fn owner_assertion_counts_are_explicit_and_unknown_is_not_zero() {
        let absent = component_summary(&json!({"report":{}}));
        assert!(absent["assertions"].is_null());
        let actual = component_summary(
            &json!({"exit_code":2,"report":{"schema":"humanymous.product-lab/2","cases":[{"status":"passed","detail":"private-canary"},{"status":"failed"},{"status":"blocked"}]}}),
        );
        assert_eq!(
            actual["assertions"],
            json!({"total":3,"passed":1,"failed":1,"other":1})
        );
        assert!(!actual.to_string().contains("private-canary"));
        assert_eq!(actual["exit_code"], 2);
    }
}
