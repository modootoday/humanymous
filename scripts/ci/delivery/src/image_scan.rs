//! Observed binary inventory is mandatory; a successful empty scan is not clean.
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

const DIRECTORY: &str = "target/delivery-image-scan";
const IMAGE: &str = "humanymous-runtime-candidate:validation";
const BINARY: &str = "usr/local/bin/humanymous-runtime";

fn identity(inspected: &Value, sha: &str, manifest: &str) -> Result<String> {
    require(
        inspected.as_array().is_some_and(|rows| rows.len() == 1),
        "single candidate image required",
    )?;
    let image = &inspected[0];
    let id = image["Id"].as_str().ok_or("candidate image ID absent")?;
    require(
        crate::supply::digest(id)
            && image["Config"]["Labels"]["org.opencontainers.image.revision"] == sha
            && image["Config"]["Labels"]["org.humanymous.source-manifest-sha256"] == manifest
            && image["Config"]["User"] == "65532:65532"
            && image["Config"]["Entrypoint"] == json!(["/usr/local/bin/humanymous-runtime"]),
        "candidate image identity/source mismatch",
    )?;
    Ok(id.to_owned())
}
fn projection(scan: &Value, id: &str, sha: &str, manifest: &str) -> Result<Value> {
    require(
        scan["SchemaVersion"] == 2
            && scan["Trivy"]["Version"] == "0.70.0"
            && scan["ArtifactType"] == "container_image"
            && scan["Metadata"]["ImageID"] == id,
        "Trivy schema/version/image mismatch",
    )?;
    let labels = &scan["Metadata"]["ImageConfig"]["config"]["Labels"];
    require(
        labels["org.opencontainers.image.revision"] == sha
            && labels["org.humanymous.source-manifest-sha256"] == manifest,
        "scanned image source labels mismatch",
    )?;
    let rows = match scan.get("Results") {
        None => &[][..],
        Some(Value::Array(rows)) => rows.as_slice(),
        Some(_) => return Err("malformed image scan results".into()),
    };
    let mut inventories = 0;
    let mut packages = BTreeSet::new();
    let mut findings = 0u64;
    for row in rows {
        if let Some(modified) = row.get("ExperimentalModifiedFindings") {
            require(
                modified.as_array().is_some_and(Vec::is_empty),
                "modified findings cannot hide image vulnerabilities",
            )?;
        }
        if let Some(vulnerabilities) = row.get("Vulnerabilities") {
            let vulnerabilities = vulnerabilities
                .as_array()
                .ok_or("malformed vulnerability list")?;
            for vulnerability in vulnerabilities {
                require(
                    vulnerability["VulnerabilityID"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty())
                        && matches!(
                            vulnerability["Severity"].as_str(),
                            Some("UNKNOWN" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL")
                        ),
                    "malformed vulnerability record",
                )?;
            }
            findings += vulnerabilities.len() as u64;
        }
        if row["Class"] == "lang-pkgs" && row["Type"] == "rustbinary" {
            require(
                row["Target"]
                    .as_str()
                    .is_some_and(|name| name.strip_prefix('/').unwrap_or(name) == BINARY),
                "Rust inventory does not describe shipped binary",
            )?;
            inventories += 1;
            for package in row["Packages"]
                .as_array()
                .ok_or("Rust binary package inventory absent")?
            {
                let name = package["Name"].as_str().ok_or("Rust package name absent")?;
                let version = package["Version"]
                    .as_str()
                    .ok_or("Rust package version absent")?;
                require(
                    !name.is_empty()
                        && !version.is_empty()
                        && name.len() < 256
                        && version.len() < 256
                        && packages.insert((name.to_owned(), version.to_owned())),
                    "Rust package invalid or duplicated",
                )?;
            }
        }
    }
    let coverage = inventories == 1 && !packages.is_empty();
    let inventory_bytes = serde_json::to_vec(&packages).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    Ok(
        json!({"scan_status":if !coverage {"not_scanned"} else if findings > 0 {"vulnerable"} else {"clean_observed_inventory"},
        "rust_binary_inventory_observed":coverage,"rust_binary_inventory_count":inventories,
        "observed_rust_packages":if coverage {Some(packages.len())} else {None},
        "inventory_sha256":if coverage {Some(format!("{:x}",Sha256::digest(inventory_bytes)))} else {None},
        "vulnerability_count":findings,"inventory_completeness_verified":false,"full_product_qualified":false,
        "shipping_qualified":false}),
    )
}
fn hosted(root: &Path) -> Result<Value> {
    require(
        std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var_os("ACT").is_none(),
        "image scan requires standard hosted context",
    )?;
    let snapshot = source::snapshot(root)?;
    require(
        snapshot["dirty"] == false
            && snapshot["source_sha"] == std::env::var("GITHUB_SHA").unwrap_or_default(),
        "image scan requires clean exact workflow source",
    )?;
    Ok(snapshot)
}
pub fn run(root: &Path, mode: &str) -> Result<()> {
    let snapshot = hosted(root)?;
    let directory = root.join(DIRECTORY);
    if mode == "prepare" {
        for path in [
            ".trivyignore",
            ".trivyignore.yaml",
            "trivy.yaml",
            ".trivy.yaml",
        ] {
            require(
                !root.join(path).exists(),
                "implicit Trivy suppression/configuration forbidden",
            )?;
        }
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        write_json(&directory.join("source-before.json"), &snapshot)?;
        use std::io::Write;
        let output = std::env::var_os("GITHUB_OUTPUT").ok_or("GitHub output absent")?;
        writeln!(
            std::fs::OpenOptions::new()
                .append(true)
                .open(output)
                .map_err(|e| e.to_string())?,
            "source-manifest-sha256={}",
            snapshot["manifest_sha256"]
                .as_str()
                .ok_or("source manifest absent")?
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    require(mode == "verify", "unknown image scan mode")?;
    let bundle = evidence::bundle(root)?;
    let mut summary = json!({"schema":"humanymous.image-scan-summary/1","source_sha":snapshot["source_sha"],
        "source_manifest_sha256":snapshot["manifest_sha256"],"cargo_lock_sha256":source::file(&root.join("Cargo.lock"))?,
        "scan_status":"invalid_or_missing","shipping_qualified":false,"full_product_qualified":false});
    let result: Result<()> = (|| {
        let before = read_json(&directory.join("source-before.json"))?;
        require(before == snapshot, "image source changed after prepare")?;
        let sha = snapshot["source_sha"].as_str().ok_or("source SHA absent")?;
        let manifest = snapshot["manifest_sha256"]
            .as_str()
            .ok_or("source manifest absent")?;
        let inspected: Value = serde_json::from_str(&command::text(
            root,
            "docker",
            &["image", "inspect", IMAGE],
        )?)
        .map_err(|e| e.to_string())?;
        let id = identity(&inspected, sha, manifest)?;
        summary["image_id"] = json!(id);
        let path = directory.join("trivy.json");
        let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        require(
            metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= 64 * 1024 * 1024,
            "image scan output not a bounded regular file",
        )?;
        std::fs::copy(&path, bundle.join("trivy.raw.json")).map_err(|e| e.to_string())?;
        summary["scan_sha256"] = json!(source::file(&path)?);
        let scan = read_json(&bundle.join("trivy.raw.json"))?;
        let public = projection(&scan, &id, sha, manifest)?;
        for (key, value) in public.as_object().ok_or("scan projection absent")? {
            summary[key] = value.clone();
        }
        let scanner = std::env::var("HUMANYMOUS_TRIVY_OUTCOME").unwrap_or_default();
        summary["scanner_step_outcome"] = json!(if scanner == "success" {
            "success"
        } else {
            "not_success"
        });
        if scanner != "success" && summary["scan_status"] == "clean_observed_inventory" {
            summary["scan_status"] = json!("scanner_failed");
        }
        require(
            summary["rust_binary_inventory_observed"] == true,
            "Rust binary inventory not scanned",
        )?;
        require(
            summary["vulnerability_count"] == 0 && scanner == "success",
            "image vulnerability or scanner failure",
        )
    })();
    let after = source::snapshot(root)?;
    summary["source_changed"] = json!(snapshot != after);
    write_json(&bundle.join("image-scan-summary.json"), &summary)?;
    evidence::register(&bundle, "Source bound runtime image scan coverage")?;
    evidence::emit_public_summary("Observed runtime binary vulnerability coverage", &summary)?;
    result?;
    require(snapshot == after, "image scan source changed")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"SchemaVersion":2,"Trivy":{"Version":"0.70.0"},"ArtifactType":"container_image",
            "Metadata":{"ImageID":"sha256:identity","ImageConfig":{"config":{"Labels":{"org.opencontainers.image.revision":"source","org.humanymous.source-manifest-sha256":"manifest"}}}},
            "Results":[{"Target":BINARY,"Class":"lang-pkgs","Type":"rustbinary","Packages":[{"Name":"example","Version":"1.0.0"}]}]})
    }
    #[test]
    fn empty_or_lock_only_scan_is_not_clean_binary_coverage() {
        let mut omitted = fixture();
        omitted.as_object_mut().unwrap().remove("Results");
        assert_eq!(
            projection(&omitted, "sha256:identity", "source", "manifest").unwrap()["scan_status"],
            "not_scanned"
        );
        for rows in [
            json!([]),
            json!([{"Target":"Cargo.lock","Class":"lang-pkgs","Type":"cargo","Packages":[{"Name":"example","Version":"1"}]}]),
        ] {
            let mut scan = fixture();
            scan["Results"] = rows;
            assert_eq!(
                projection(&scan, "sha256:identity", "source", "manifest").unwrap()["scan_status"],
                "not_scanned"
            );
        }
        let mut scan = fixture();
        scan["Results"][0]["Packages"] = json!([]);
        assert_eq!(
            projection(&scan, "sha256:identity", "source", "manifest").unwrap()["scan_status"],
            "not_scanned"
        );
    }
    #[test]
    fn identity_and_findings_are_independent_from_nonempty_inventory() {
        let mut scan = fixture();
        assert_eq!(
            projection(&scan, "sha256:identity", "source", "manifest").unwrap()["scan_status"],
            "clean_observed_inventory"
        );
        assert!(projection(&scan, "sha256:other", "source", "manifest").is_err());
        scan["Results"][0]["Vulnerabilities"] = json!([{"VulnerabilityID":"CVE-example","Severity":"UNKNOWN","Description":"private-canary"}]);
        let public = projection(&scan, "sha256:identity", "source", "manifest").unwrap();
        assert_eq!(public["scan_status"], "vulnerable");
        assert!(!public.to_string().contains("private-canary"));
        scan["Results"][0]["ExperimentalModifiedFindings"] = json!([{}]);
        assert!(projection(&scan, "sha256:identity", "source", "manifest").is_err());
    }
}
