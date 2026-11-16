//! Phase 2 merge gate (common automation rules C2.10, C7.2, C7.3).
//!
//! Lists the changed paths of a pull request that fall in the human-gated set
//! (`scripts/ci/human-gated-paths.json`). With `--enforce` on a pull request,
//! a gated change has auto-merge turned off and the `needs-human` label added,
//! so Phase 2 auto-merge refuses these paths by a path check, not by the label
//! alone. The job itself never merges anything.
use crate::{Result, command};
use serde_json::Value;
use std::path::Path;

fn list(value: &Value, key: &str) -> Result<Vec<String>> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("human-gated paths: {key} absent"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("human-gated paths: {key} entry is not a string"))
        })
        .collect()
}

/// Returns the changed paths that are human-gated.
fn gated(config: &Value, changed: &[&str]) -> Result<Vec<String>> {
    if config["schema"] != "humanymous.human-gated-paths/1" {
        return Err("human-gated paths schema mismatch".into());
    }
    let (prefixes, files, suffixes) = (
        list(config, "prefixes")?,
        list(config, "files")?,
        list(config, "suffixes")?,
    );
    Ok(changed
        .iter()
        .filter(|path| {
            prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix.as_str()))
                || files.iter().any(|file| file == *path)
                || suffixes
                    .iter()
                    .any(|suffix| path.ends_with(suffix.as_str()))
        })
        .map(|path| (*path).to_owned())
        .collect())
}

fn revision(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.starts_with('-')
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'~' | b'^'))
    {
        return Err("merge-gate needs plain base and head revisions".into());
    }
    Ok(value)
}

/// `delivery merge-gate <base> <head> [--enforce]`.
pub fn check(root: &Path, base: &str, head: &str, enforce: bool) -> Result<()> {
    let range = format!("{}...{}", revision(base)?, revision(head)?);
    let names = command::text(
        root,
        "git",
        &["diff", "--name-only", "--no-renames", &range],
    )?;
    let changed: Vec<&str> = names.lines().filter(|line| !line.is_empty()).collect();
    let config = crate::read_json(&root.join("scripts/ci/human-gated-paths.json"))?;
    let hits = gated(&config, &changed)?;
    if hits.is_empty() {
        println!(
            "merge gate: {} changed path(s), none human-gated; auto-merge allowed",
            changed.len()
        );
        return Ok(());
    }
    println!("merge gate: needs-human ({} gated path(s))", hits.len());
    for path in &hits {
        println!("  {path}");
    }
    if enforce {
        let number = std::env::var("PR_NUMBER").map_err(|_| "PR_NUMBER is not set")?;
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err("PR_NUMBER must be a pull request number".into());
        }
        let repo =
            std::env::var("GITHUB_REPOSITORY").map_err(|_| "GITHUB_REPOSITORY is not set")?;
        // Turning auto-merge off fails harmlessly when it was never on.
        let disabled = command::output(
            root,
            "gh",
            &["pr", "merge", &number, "--repo", &repo, "--disable-auto"],
        )?;
        println!(
            "merge gate: auto-merge {}",
            if disabled.status.success() {
                "turned off"
            } else {
                "was not on"
            }
        );
        command::text(
            root,
            "gh",
            &[
                "pr",
                "edit",
                &number,
                "--repo",
                &repo,
                "--add-label",
                "needs-human",
            ],
        )?;
        println!("merge gate: labelled needs-human; the owner merges");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn committed_list_gates_workflows_scoring_go_and_itself() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let config = crate::read_json(&root.join("scripts/ci/human-gated-paths.json")).unwrap();
        let changed = [
            ".github/workflows/ci.yml",
            "crates/sdk/humanymous-core/src/scoring/policy.rs",
            "internal/foo/bar.go",
            "scripts/ci/human-gated-paths.json",
            "Cargo.lock",
            "docs/how-to/x.md",
            "crates/cli/src/main.rs",
            ".github/workflowsx",
        ];
        assert_eq!(
            gated(&config, &changed).unwrap(),
            [
                ".github/workflows/ci.yml",
                "crates/sdk/humanymous-core/src/scoring/policy.rs",
                "internal/foo/bar.go",
                "scripts/ci/human-gated-paths.json",
                "Cargo.lock",
            ]
        );
    }
    #[test]
    fn schema_mismatch_and_bad_revisions_are_refused() {
        assert!(gated(&serde_json::json!({"schema":"x"}), &[]).is_err());
        assert!(revision("--output=x").is_err());
        assert!(revision("abc").is_ok());
    }
}
