//! Registry publication is unreachable before exact-source unprivileged gates.
use crate::{Result, command, package, read_json, require, source};
use serde_json::Value;
use std::{fs, io::Write, path::Path};
const BASELINE: &str = "981d25cf71a5a54855d7e2b4359e832a9d5998e5";
pub fn migration(root: &Path) -> Result<()> {
    let inventory = read_json(
        &root.join(".spec/evidence/20261106194112-go-migration-source-inventory/go-inventory.json"),
    )?;
    require(inventory["baseline"] == BASELINE, "oracle baseline changed")?;
    let rows = inventory["files"]
        .as_array()
        .ok_or("missing source inventory")?;
    let expected: std::collections::BTreeSet<_> = command::text(
        root,
        "git",
        &[
            "ls-tree",
            "-r",
            "--name-only",
            BASELINE,
            "--",
            "cmd",
            "internal",
            "pkg",
        ],
    )?
    .lines()
    .filter(|p| p.ends_with(".go"))
    .map(str::to_owned)
    .collect();
    let actual: std::collections::BTreeSet<_> = rows
        .iter()
        .filter_map(|r| r["path"].as_str())
        .map(str::to_owned)
        .collect();
    require(
        !expected.is_empty() && actual == expected && rows.len() == actual.len(),
        "migration inventory omitted/duplicated baseline responsibility",
    )?;
    for row in rows {
        require(
            matches!(
                row["parity_status"].as_str(),
                Some("verified_runtime_parity" | "retired_with_review")
            ),
            format!(
                "incomplete runtime parity: {} ({})",
                row["path"], row["parity_status"]
            ),
        )?;
        let evidence = row["parity_evidence"]
            .as_array()
            .ok_or("missing canonical parity evidence")?;
        require(!evidence.is_empty(), "empty canonical parity evidence")?;
        for path in evidence {
            let path = path.as_str().ok_or("bad parity evidence path")?;
            require(
                source::safe_relative(path)
                    && path.starts_with(".spec/evidence/")
                    && root.join(path).is_file(),
                format!("missing/unsafe canonical parity evidence: {path}"),
            )?;
        }
        if row["parity_status"] == "retired_with_review" {
            require(
                row["disposition"]
                    .as_str()
                    .is_some_and(|s| s.trim().len() >= 30),
                "unreviewed responsibility retirement",
            )?;
        } else {
            let paths = row["rust_paths"]
                .as_array()
                .ok_or("missing Rust responsibility mapping")?;
            require(
                !paths.is_empty()
                    && paths.iter().all(|p| {
                        p.as_str().is_some_and(|p| {
                            source::safe_relative(p)
                                && p.starts_with("crates/")
                                && root.join(p).exists()
                        })
                    }),
                "invalid Rust responsibility mapping",
            )?;
        }
    }
    require(
        command::text(root, "git", &["ls-files", "--", "*.go", "go.mod", "go.sum"])?
            .trim()
            .is_empty(),
        "Go product/migration source remains in final shipping tree",
    )
}
fn trust_binding(registry: &str, record: &Value) -> Result<()> {
    let environment = match registry {
        "crates_io" => "crates",
        "npm" => "npm",
        "rubygems" => "rubygems",
        "pypi" => "pypi",
        _ => return Err("unknown registry trust binding".into()),
    };
    require(
        record["workflow"] == "release.yml"
            && record["environment"] == environment
            && record["repository"] == "modootoday/humanymous",
        "trusted publisher workflow/environment/repository binding mismatch",
    )
}
fn trust(root: &Path, registry: &str, record: &Value) -> Result<()> {
    require(
        record["verified"] == true,
        format!("external {registry} ownership/bootstrap/trust unverified"),
    )?;
    trust_binding(registry, record)?;
    let evidence = record["evidence"].as_str().ok_or("trust proof missing")?;
    require(
        source::safe_relative(evidence)
            && evidence.starts_with(".spec/evidence/")
            && root.join(evidence).is_file(),
        "external trust proof must be existing canonical evidence",
    )?;
    require(
        record["evidence_sha256"] == source::file(&root.join(evidence))?,
        "external trust proof digest stale",
    )?;
    if matches!(registry, "npm" | "rubygems" | "pypi") {
        require(
            record["direct_publish_allowed"] == true,
            "registry trust must explicitly permit direct publish",
        )?;
    }
    Ok(())
}
pub fn validate_preparations(records: &Value, planned: &Value, release: &Value) -> Result<()> {
    require(
        records["schema"] == 1,
        "registry prerequisite schema mismatch",
    )?;
    let entries = planned
        .as_object()
        .ok_or("planned publisher catalog missing")?;
    require(entries.len() == 2, "unexpected planned publisher inventory")?;
    for registry in ["rubygems", "pypi"] {
        let record = &records[registry];
        trust_binding(registry, record)?;
        require(
            record["verified"] == false
                && record["direct_publish_allowed"] == false
                && record["package_ready"] == false
                && record["publisher_configured"] == false
                && record["publisher_mode"] == "pending"
                && record["package_name"].is_null(),
            "preparation cannot claim a package, publisher or publication readiness",
        )?;
        let entry = &planned[registry];
        require(
            entry["workflow"] == record["workflow"]
                && entry["environment"] == record["environment"]
                && entry["state"] == "preparation-only"
                && entry["job_present"] == false,
            "planned publisher binding drift",
        )?;
        require(
            release["jobs"]
                .as_object()
                .is_some_and(|jobs| !jobs.contains_key(registry)),
            "preparation-only registry must not have a publishing job",
        )?;
    }
    Ok(())
}
pub fn readiness(root: &Path) -> Result<()> {
    let data = package::metadata(root)?;
    let mut failures = Vec::new();
    match package::graph(&data) {
        Err(error) => failures.push(error),
        Ok(graph) => {
            for package in graph.into_iter().flatten() {
                for key in ["description", "license", "repository", "readme"] {
                    if package[key].as_str().is_none_or(str::is_empty) {
                        failures.push(format!("{}: missing {key}", package["name"]));
                    }
                }
            }
        }
    }
    for check in [migration(root), package::image_sources(root)] {
        if let Err(error) = check {
            failures.push(error);
        }
    }
    if let Err(error) = package::node_receipt(root) {
        failures.push(error);
    }
    let prerequisite = read_json(&root.join("scripts/ci/registry-prerequisites.json"))?;
    require(
        prerequisite["schema"] == 1,
        "registry prerequisite schema mismatch",
    )?;
    for registry in ["crates_io", "npm", "rubygems", "pypi"] {
        if let Err(error) = trust(root, registry, &prerequisite[registry]) {
            failures.push(error);
        }
    }
    require(
        failures.is_empty(),
        format!("shipping prerequisites:\n{}", failures.join("\n")),
    )
}
fn version(root: &Path) -> Result<String> {
    let manifest: toml::Value = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| e.to_string())?
        .parse::<toml::Value>()
        .map_err(|e| e.to_string())?;
    manifest["workspace"]["package"]["version"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "missing workspace version".into())
}
fn semver(tag: &str) -> bool {
    tag.strip_prefix('v').is_some_and(|v| {
        let parts: Vec<_> = v.split('.').collect();
        parts.len() == 3
            && parts.iter().all(|part| {
                !part.is_empty()
                    && part.bytes().all(|b| b.is_ascii_digit())
                    && (part.len() == 1 || !part.starts_with('0'))
            })
    })
}
pub fn release(root: &Path) -> Result<()> {
    require(
        std::env::var("GITHUB_EVENT_NAME").as_deref() == Ok("push"),
        "publication requires tag push",
    )?;
    require(
        std::env::var("GITHUB_REPOSITORY").as_deref() == Ok("modootoday/humanymous"),
        "publication repository mismatch",
    )?;
    let reference = std::env::var("GITHUB_REF").map_err(|e| e.to_string())?;
    let tag = reference
        .strip_prefix("refs/tags/")
        .ok_or("publication requires tag ref")?;
    require(semver(tag), "strict stable SemVer tag required")?;
    let head = source::head(root)?;
    require(
        std::env::var("GITHUB_SHA").as_deref() == Ok(&head),
        "checkout differs from requested source SHA",
    )?;
    require(
        command::text(
            root,
            "git",
            &["rev-parse", &format!("refs/tags/{tag}^{{commit}}")],
        )?
        .trim()
            == head,
        "tag commit differs from checkout",
    )?;
    command::status(
        root,
        "git",
        &["merge-base", "--is-ancestor", &head, "origin/main"],
    )?;
    require(
        source::snapshot(root)?["dirty"] == false,
        "publication requires clean governed source bytes",
    )?;
    let version = version(root)?;
    require(
        tag == format!("v{version}"),
        "tag/workspace version mismatch",
    )?;
    for package in package::graph(&package::metadata(root)?)?
        .into_iter()
        .flatten()
    {
        require(package["version"] == version, "crate/tag version mismatch")?;
    }
    require(
        read_json(&root.join("adapters/node/package.json"))?["version"] == version,
        "npm/tag version mismatch",
    )?;
    readiness(root)
}
pub fn wave_output(root: &Path) -> Result<()> {
    let waves = package::graph(&package::metadata(root)?)?;
    let output = std::env::var("GITHUB_OUTPUT").map_err(|e| e.to_string())?;
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(output)
            .map_err(|e| e.to_string())?,
        "count={}",
        waves.len()
    )
    .map_err(|e| e.to_string())
}
pub fn publish_wave(root: &Path, index: usize) -> Result<()> {
    release(root)?;
    require(
        std::env::var("CARGO_REGISTRY_TOKEN").is_ok_and(|s| !s.is_empty()),
        "official short-lived crates OIDC token required",
    )?;
    let graph = package::graph(&package::metadata(root)?)?;
    let wave = graph.get(index).ok_or("wave outside reviewed graph")?;
    for package in wave {
        let name = package["name"].as_str().ok_or("package name missing")?;
        let version = package["version"]
            .as_str()
            .ok_or("package version missing")?;
        let (archive, expected) = package::validated_crate(root, name, version)?;
        match ureq::get(&format!("https://crates.io/api/v1/crates/{name}/{version}"))
            .header("User-Agent", "humanymous-delivery")
            .call()
        {
            Err(ureq::Error::StatusCode(404)) => {}
            Ok(mut response) => {
                let metadata: Value = serde_json::from_str(
                    &response
                        .body_mut()
                        .read_to_string()
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                published_identity(&metadata, name, version, &expected)?;
                println!(
                    "{}",
                    serde_json::json!({"crate":name,"version":version,"status":"already-published-verified","archive_sha256":expected})
                );
                continue;
            }
            Err(error) => return Err(format!("registry lookup failed: {error}")),
        }
        command::status(root, "cargo", &["package", "--locked", "-p", name])?;
        require(
            source::file(&archive)? == expected,
            "Cargo repack differs from validated same-run artifact; refusing upload",
        )?;
        command::status(root, "cargo", &["publish", "--locked", "-p", name])?;
    }
    Ok(())
}
fn published_identity(metadata: &Value, name: &str, version: &str, checksum: &str) -> Result<()> {
    require(
        metadata["version"]["crate"] == name
            && metadata["version"]["num"] == version
            && metadata["version"]["checksum"] == checksum,
        "immutable registry version differs from validated source/archive; fix forward, never overwrite",
    )
}
pub fn publish_npm(root: &Path) -> Result<()> {
    release(root)?;
    require(
        ["NODE_AUTH_TOKEN", "NPM_TOKEN"]
            .iter()
            .all(|name| std::env::var(name).map_or(true, |s| s.is_empty())),
        "npm OIDC only; token fallback forbidden",
    )?;
    package::node_receipt(root)?;
    let proof = read_json(&root.join("target/delivery-node/validated-archive.json"))?;
    require(
        proof["schema"] == "humanymous.node-archive/1"
            && proof["source_sha"] == source::head(root)?
            && proof["package_manifest_sha256"]
                == source::file(&root.join("adapters/node/package.json"))?
            && proof["provenance_sha256"]
                == source::file(&root.join("adapters/node/native/provenance.json"))?
            && proof["qualification_receipt_sha256"]
                == source::file(&root.join("target/node-native/qualification.json"))?,
        "validated npm archive identity/provenance differs from same-run receipt",
    )?;
    let filename = proof["filename"]
        .as_str()
        .ok_or("validated npm filename missing")?;
    require(
        source::safe_relative(filename) && !filename.contains('/'),
        "unsafe validated npm filename",
    )?;
    let archive = root.join("target/delivery-node").join(filename);
    require(
        proof["archive_sha256"] == source::file(&archive)?,
        "validated npm archive digest differs",
    )?;
    command::status(
        &root.join("adapters/node"),
        "npm",
        &[
            "publish",
            archive.to_str().ok_or("non-UTF8 npm archive")?,
            "--access",
            "public",
            "--provenance",
            "--ignore-scripts",
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_semver_only() {
        assert!(semver("v0.1.0"));
        for value in [
            "v01.0.0",
            "1.0.0",
            "v1.2",
            "v1.0.0-rc.1",
            "v1.0.0+meta",
            "v1.0.0/evil",
        ] {
            assert!(!semver(value));
        }
    }
    #[test]
    fn unverified_registry_never_passes() {
        assert!(
            trust(
                Path::new("."),
                "crates_io",
                &serde_json::json!({"verified":false})
            )
            .is_err()
        );
    }
    #[test]
    fn registry_products_require_distinct_exact_environments() {
        for (registry, environment) in [
            ("crates_io", "crates"),
            ("npm", "npm"),
            ("rubygems", "rubygems"),
            ("pypi", "pypi"),
        ] {
            let record = serde_json::json!({"workflow":"release.yml","repository":"modootoday/humanymous","environment":environment});
            assert!(trust_binding(registry, &record).is_ok());
            for wrong in ["release", "other", ""] {
                let mut changed = record.clone();
                changed["environment"] = serde_json::json!(wrong);
                assert!(trust_binding(registry, &changed).is_err());
            }
        }
    }
    #[test]
    fn new_publishers_are_typed_preparation_without_jobs_or_readiness() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let records = read_json(&root.join("scripts/ci/registry-prerequisites.json")).unwrap();
        let catalog = read_json(&root.join("scripts/ci/workflows.json")).unwrap();
        let planned = &catalog["planned_publishers"];
        let release = serde_json::json!({"jobs":{}});
        assert!(validate_preparations(&records, planned, &release).is_ok());
        for registry in ["rubygems", "pypi"] {
            for field in [
                "verified",
                "package_ready",
                "publisher_configured",
                "direct_publish_allowed",
            ] {
                let mut changed = records.clone();
                changed[registry][field] = serde_json::json!(true);
                assert!(validate_preparations(&changed, planned, &release).is_err());
            }
            let mut wrong_environment = records.clone();
            wrong_environment[registry]["environment"] = serde_json::json!("release");
            assert!(validate_preparations(&wrong_environment, planned, &release).is_err());
            let mut actual_job = release.clone();
            actual_job["jobs"][registry] = serde_json::json!({"permissions":{"id-token":"write"}});
            assert!(validate_preparations(&records, planned, &actual_job).is_err());
        }
    }
    #[test]
    fn immutable_resume_requires_matching_name_version_and_archive_checksum() {
        let metadata =
            serde_json::json!({"version":{"crate":"defender","num":"1.0.0","checksum":"expected"}});
        assert!(published_identity(&metadata, "defender", "1.0.0", "expected").is_ok());
        for (name, version, hash) in [
            ("attacker", "1.0.0", "expected"),
            ("defender", "1.0.1", "expected"),
            ("defender", "1.0.0", "other"),
        ] {
            assert!(published_identity(&metadata, name, version, hash).is_err());
        }
        assert!(
            published_identity(
                &serde_json::json!({"error":"registry outage"}),
                "defender",
                "1.0.0",
                "expected"
            )
            .is_err()
        );
    }
}
