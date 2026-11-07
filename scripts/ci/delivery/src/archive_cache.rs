//! Optional public archive transport; failure never skips a validation gate.
use crate::{Result, command, evidence, require, source};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub mod policy;
mod scan;

fn read_lock(path: &Path) -> Result<toml::Value> {
    toml::from_str(&std::fs::read_to_string(path).map_err(|_| "lock_unavailable")?)
        .map_err(|_| "lock_invalid".into())
}

fn quota(storage: &Value, retention: &Value, usage: &Value, bytes: u64) -> Result<()> {
    let gb = storage["max_cache_size_gb"]
        .as_u64()
        .filter(|gb| (1..=10).contains(gb))
        .ok_or("quota_unverified")?;
    require(
        retention["max_cache_retention_days"]
            .as_u64()
            .is_some_and(|days| (1..=7).contains(&days)),
        "retention_unverified",
    )?;
    let used = usage["active_caches_size_in_bytes"]
        .as_u64()
        .ok_or("usage_unverified")?;
    require(
        bytes <= scan::MAX_BYTES
            && used
                .checked_add(bytes)
                .is_some_and(|total| total <= gb * 1_000_000_000),
        "free_quota_insufficient",
    )
}
fn trusted_save(repository: &str, event: &str, reference: &str) -> bool {
    repository == "modootoday/humanymous"
        && event == "push"
        && matches!(reference, "refs/heads/codex/rust" | "refs/heads/main")
}
fn get(endpoint: &str, root: &Path) -> Result<Value> {
    serde_json::from_str(&command::text(root, "gh", &["api", endpoint])?)
        .map_err(|_| "cache_api_invalid".into())
}
fn plan(root: &Path) -> Result<(Value, Vec<PathBuf>)> {
    require(
        std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && trusted_save(
                &std::env::var("GITHUB_REPOSITORY").unwrap_or_default(),
                &std::env::var("GITHUB_EVENT_NAME").unwrap_or_default(),
                &std::env::var("GITHUB_REF").unwrap_or_default(),
            ),
        "restore_only_context",
    )?;
    require(
        get("repos/modootoday/humanymous", root)?["private"] == false,
        "public_repository_unverified",
    )?;
    require(
        command::text(
            root,
            "git",
            &["status", "--porcelain=v1", "--", "Cargo.lock"],
        )?
        .trim()
        .is_empty(),
        "lock_not_committed",
    )?;
    let storage = get(
        "repos/modootoday/humanymous/actions/cache/storage-limit",
        root,
    )?;
    let retention = get(
        "repos/modootoday/humanymous/actions/cache/retention-limit",
        root,
    )?;
    let usage = get("repos/modootoday/humanymous/actions/cache/usage", root)?;
    quota(&storage, &retention, &usage, 0)?;
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or("home_unavailable")?;
    let cargo = home.join(".cargo");
    if let Some(configured) = std::env::var_os("CARGO_HOME") {
        require(
            PathBuf::from(configured).canonicalize().ok() == cargo.canonicalize().ok(),
            "alternate_cargo_home_refused",
        )?;
    }
    require(
        cargo
            .canonicalize()
            .map_err(|_| "cargo_home_unavailable")?
            .starts_with(home.canonicalize().map_err(|_| "home_unavailable")?),
        "cargo_home_symlink_refused",
    )?;
    let lock_before = source::file(&root.join("Cargo.lock"))?;
    let lock = read_lock(&root.join("Cargo.lock"))?;
    let archives = scan::collect(&cargo.join("registry/cache"), &lock)?;
    quota(&storage, &retention, &usage, archives.bytes)?;
    require(
        source::file(&root.join("Cargo.lock"))? == lock_before,
        "lock_changed",
    )?;
    let hit = match std::env::var("HUMANYMOUS_ARCHIVE_CACHE_HIT").as_deref() {
        Ok("true") => Some(true),
        Ok("false") => Some(false),
        _ => None,
    };
    Ok((
        json!({"schema":"humanymous.public-crate-cache-plan/1","save_allowed":true,"source_sha":source::head(root)?,
        "lock_sha256":lock_before,"verified_archive_count":archives.paths.len(),"archive_bytes":archives.bytes,
        "quota_gb":storage["max_cache_size_gb"],"retention_days":retention["max_cache_retention_days"],
        "usage_bytes":usage["active_caches_size_in_bytes"],"restore_hit":hit,
        "scope":"verified public crates.io .crate archives only; no compiled artifact or trust reuse"}),
        archives.paths,
    ))
}
fn outputs(summary: &Value, paths: &[PathBuf]) -> Result<()> {
    let Some(destination) = std::env::var_os("GITHUB_OUTPUT") else {
        return Ok(());
    };
    let mut output = std::fs::OpenOptions::new()
        .append(true)
        .open(destination)
        .map_err(|_| "cache_outputs_unavailable")?;
    writeln!(output, "save_allowed=false").map_err(|_| "cache_outputs_unavailable")?;
    if summary["save_allowed"] == true {
        writeln!(output, "archive_paths<<HUMANYMOUS_PUBLIC_ARCHIVES")
            .map_err(|_| "cache_outputs_unavailable")?;
        for path in paths {
            writeln!(output, "{}", path.to_str().ok_or("archive_path_invalid")?)
                .map_err(|_| "cache_outputs_unavailable")?;
        }
        writeln!(output, "HUMANYMOUS_PUBLIC_ARCHIVES").map_err(|_| "cache_outputs_unavailable")?;
        writeln!(output, "save_allowed=true").map_err(|_| "cache_outputs_unavailable")?;
    }
    Ok(())
}
pub fn run(root: &Path) -> Result<()> {
    let (mut summary, paths) = match plan(root) {
        Ok(value) => value,
        Err(_) => (
            json!({"schema":"humanymous.public-crate-cache-plan/1","save_allowed":false,
            "status":"no-cache-fallback","verified_archive_count":null,"archive_bytes":null,
            "reason":"context, quota, inputs or public checksums were not all verified"}),
            Vec::new(),
        ),
    };
    if outputs(&summary, &paths).is_err() {
        summary["save_allowed"] = json!(false);
        summary["status"] = json!("no-cache-fallback");
    }
    // Optimization failure cannot alter actual test/registry validation results.
    if evidence::emit_public_summary("Optional public crate archive cache", &summary).is_err() {
        eprintln!("public archive cache summary unavailable; no cache is required for validation");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_cache_reads_the_actual_lock_document_without_network() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let lock = read_lock(&root.join("Cargo.lock")).unwrap();
        assert!(
            lock["package"]
                .as_array()
                .is_some_and(|packages| !packages.is_empty())
        );
        assert!(read_lock(&root.join("Cargo.lock/not-a-document")).is_err());
    }
    #[test]
    fn unknown_oversized_paid_or_long_retention_cache_is_not_allowed() {
        let storage = json!({"max_cache_size_gb":10});
        let retention = json!({"max_cache_retention_days":7});
        let usage = json!({"active_caches_size_in_bytes":100});
        assert!(quota(&storage, &retention, &usage, 1000).is_ok());
        assert!(quota(&json!({"max_cache_size_gb":11}), &retention, &usage, 1).is_err());
        assert!(quota(&storage, &json!({"max_cache_retention_days":8}), &usage, 1).is_err());
        assert!(quota(&storage, &retention, &json!({}), 1).is_err());
        assert!(quota(&storage, &retention, &usage, scan::MAX_BYTES + 1).is_err());
    }
    #[test]
    fn pr_and_tag_contexts_never_save_public_cache() {
        assert!(trusted_save(
            "modootoday/humanymous",
            "push",
            "refs/heads/codex/rust"
        ));
        for (repo, event, reference) in [
            ("fork/repo", "push", "refs/heads/main"),
            ("modootoday/humanymous", "pull_request", "refs/heads/main"),
            ("modootoday/humanymous", "push", "refs/tags/v1.0.0"),
        ] {
            assert!(!trusted_save(repo, event, reference));
        }
    }
}
