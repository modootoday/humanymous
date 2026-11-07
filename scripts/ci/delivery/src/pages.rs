//! Build-only Pages provenance and trusted step outcomes, without parsing logs.
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::path::Path;
const DIRECTORY: &str = "target/delivery-pages";
fn snapshot(root: &Path) -> Result<Value> {
    require(
        std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var_os("ACT").is_none(),
        "Pages evidence requires standard hosted context",
    )?;
    let sha = source::head(root)?;
    let rust_source = source::snapshot(root)?;
    require(
        std::env::var("GITHUB_SHA").as_deref() == Ok(&sha),
        "Pages checkout differs from workflow source",
    )?;
    let dirty = !command::text(
        root,
        "git",
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--",
            "docs",
        ],
    )?
    .trim()
    .is_empty();
    Ok(
        json!({"source_sha":sha,"docs_dirty":dirty,"rust_source_dirty":rust_source["dirty"],"rust_source_manifest_sha256":rust_source["manifest_sha256"],
        "docs_git_tree":command::text(root,"git",&["rev-parse","HEAD:docs"])?.trim(),
        "config_sha256":source::file(&root.join("docs/_config.yml"))?,
        "gemfile_lock_sha256":source::file(&root.join("docs/Gemfile.lock"))?,
        "cargo_lock_sha256":source::file(&root.join("Cargo.lock"))?}),
    )
}
fn outcome(value: &str) -> Result<Value> {
    require(
        matches!(value, "success" | "failure" | "cancelled" | "skipped"),
        "unknown Pages step outcome",
    )?;
    Ok(json!({"outcome":value,"exit_code":if value=="success" {Some(0)}else{None::<u8>}}))
}
fn package_projection(manifest: &Value, archive_sha: &str, baseurl: &str) -> Result<Value> {
    require(
        manifest["schema"] == "humanymous.public-pages-archive/1"
            && manifest["baseurl"] == baseurl
            && manifest["archive_sha256"] == archive_sha
            && ["archive_sha256", "source_input_sha256", "inventory_sha256"]
                .iter()
                .all(|key| crate::lab_receipts::digest(&manifest[*key], 64)),
        "public Pages archive binding mismatch",
    )?;
    let count = manifest["files"]
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or("public Pages inventory missing")?
        .len();
    Ok(
        json!({"status":"observed","archive_sha256":archive_sha,"manifest_source_input_sha256":manifest["source_input_sha256"],"inventory_sha256":manifest["inventory_sha256"],"public_files":count}),
    )
}
pub fn run(root: &Path, mode: &str) -> Result<()> {
    let current = snapshot(root)?;
    let directory = root.join(DIRECTORY);
    if mode == "prepare" {
        require(
            current["docs_dirty"] == false && current["rust_source_dirty"] == false,
            "Pages source and builder must be clean",
        )?;
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        for base in ["apex", "subpath"] {
            std::fs::create_dir(directory.join(base)).map_err(|e| e.to_string())?;
        }
        return write_json(&directory.join("source-before.json"), &current);
    }
    require(mode == "finish", "unknown Pages evidence mode")?;
    let before = read_json(&directory.join("source-before.json"))?;
    let mut stages = serde_json::Map::new();
    for (stage, variable) in [
        ("bundle_install", "HUMANYMOUS_PAGES_INSTALL"),
        ("bundle_audit", "HUMANYMOUS_PAGES_AUDIT"),
        ("apex", "HUMANYMOUS_PAGES_APEX"),
        ("subpath", "HUMANYMOUS_PAGES_SUBPATH"),
        ("package_apex", "HUMANYMOUS_PAGES_PACKAGE_APEX"),
        ("package_subpath", "HUMANYMOUS_PAGES_PACKAGE_SUBPATH"),
        ("archive_apex", "HUMANYMOUS_PAGES_ARCHIVE_APEX"),
        ("archive_subpath", "HUMANYMOUS_PAGES_ARCHIVE_SUBPATH"),
    ] {
        stages.insert(
            stage.into(),
            outcome(&std::env::var(variable).unwrap_or_default())?,
        );
    }
    let unchanged = before == current
        && current["docs_dirty"] == false
        && current["rust_source_dirty"] == false;
    let mut packages = serde_json::Map::new();
    let mut package_bindings_valid = true;
    for (base, baseurl) in [("apex", ""), ("subpath", "/humanymous")] {
        if stages[&format!("archive_{base}")]["outcome"] == "success" {
            let result: Result<Value> = (|| {
                let archive = directory.join(base).join("artifact.tar");
                let manifest = directory.join(base).join("manifest.json");
                for (path, max) in [(&archive, 72 * 1024 * 1024), (&manifest, 4 * 1024 * 1024)] {
                    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
                    require(
                        metadata.is_file()
                            && !metadata.file_type().is_symlink()
                            && metadata.len() <= max,
                        "public Pages package file invalid",
                    )?;
                }
                let mut value =
                    package_projection(&read_json(&manifest)?, &source::file(&archive)?, baseurl)?;
                value["manifest_sha256"] = json!(source::file(&manifest)?);
                value["owner_archive_check_outcome"] = json!("success");
                Ok(value)
            })();
            match result {
                Ok(value) => {
                    packages.insert(base.into(), value);
                }
                Err(_) => {
                    package_bindings_valid = false;
                    packages.insert(
                        base.into(),
                        json!({"status":"invalid","error_code":"package_binding_failed"}),
                    );
                }
            }
        } else {
            packages.insert(base.into(), Value::Null);
            package_bindings_valid = false;
        }
    }
    let transfer = std::env::var("HUMANYMOUS_PAGES_TRANSFER")
        .ok()
        .map(|state| outcome(&state))
        .transpose()?;
    let passed = unchanged
        && package_bindings_valid
        && stages.values().all(|value| value["outcome"] == "success")
        && transfer
            .as_ref()
            .is_none_or(|value| value["outcome"] == "success");
    let summary = json!({"schema":"humanymous.pages-build-summary/1","source":current,"source_unchanged":unchanged,
        "stages":stages,"packages":packages,"serving_transfer_roundtrip":transfer,"status":if passed{"passed"}else{"failed"},
        "validation_scope":"two base URLs; trusted step outcomes only, no mixed log parser",
        "browser_qualified":false,"assistive_technology_qualified":false,"full_product_qualified":false,"deployment_performed":false});
    write_json(&directory.join("summary.json"), &summary)?;
    use std::io::Write;
    let output = std::env::var_os("GITHUB_OUTPUT").ok_or("GitHub output absent")?;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(output)
            .map_err(|e| e.to_string())?,
        "source-unchanged={unchanged}"
    )
    .map_err(|e| e.to_string())?;
    evidence::emit_public_summary("Exact-source Pages build outcomes", &summary)?;
    require(passed, "Pages build or source integrity failed")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_or_unrun_steps_never_invent_zero_exit_success() {
        assert_eq!(outcome("success").unwrap()["exit_code"], 0);
        for state in ["failure", "cancelled", "skipped"] {
            assert!(outcome(state).unwrap()["exit_code"].is_null());
        }
        assert!(outcome("").is_err());
        assert!(outcome("passed").is_err());
    }
    #[test]
    fn package_counts_cannot_replace_exact_archive_byte_binding() {
        let value = json!({"schema":"humanymous.public-pages-archive/1","baseurl":"","archive_sha256":"a".repeat(64),"source_input_sha256":"b".repeat(64),"inventory_sha256":"c".repeat(64),"files":[{"path":".well-known/security.txt"}]});
        assert_eq!(
            package_projection(&value, &"a".repeat(64), "").unwrap()["public_files"],
            1
        );
        assert!(package_projection(&value, &"d".repeat(64), "").is_err());
        assert!(package_projection(&value, &"a".repeat(64), "/humanymous").is_err());
        let mut empty = value;
        empty["files"] = json!([]);
        assert!(package_projection(&empty, &"a".repeat(64), "").is_err());
    }
}
