//! The only tolerated optimization failures in required validation are this pilot.
use crate::{Result, require};
use serde_json::{Value, json};
pub const RESTORE: &str = "actions/cache/restore@9255dc7a253b0ccc959486e2bca901246202afeb";
pub const SAVE: &str = "actions/cache/save@9255dc7a253b0ccc959486e2bca901246202afeb";
pub const KEY: &str = "public-crates-v1-${{ runner.os }}-${{ runner.arch }}-rust1.99.1-${{ hashFiles('Cargo.lock') }}";
pub const PATH: &str = "~/.cargo/registry/cache/index.crates.io-*/*.crate";
pub const SAVE_PATH: &str = "${{ steps.public_crates_plan.outputs.archive_paths }}";
pub const RESTORE_IF: &str =
    "${{ github.repository == 'modootoday/humanymous' && hashFiles('Cargo.lock') != '' }}";
pub const SAVE_IF: &str = "${{ success() && github.repository == 'modootoday/humanymous' && github.event_name == 'push' && (github.ref == 'refs/heads/codex/rust' || github.ref == 'refs/heads/main') && steps.public_crates_plan.outputs.save_allowed == 'true' && steps.public_crates_restore.outputs.cache-hit != 'true' }}";
pub fn approved_step(file: &str, job: &str, step: &Value) -> bool {
    if file != "rust-validation.yml"
        || job != "policy"
        || step["continue-on-error"] != true
        || step["with"]["key"] != KEY
        || step["with"]["enableCrossOsArchive"] != false
    {
        return false;
    }
    (step["id"] == "public_crates_restore"
        && step["uses"] == RESTORE
        && step["if"] == RESTORE_IF
        && step["with"]["path"] == PATH
        && step["with"].get("restore-keys").is_none()
        && step["with"]["fail-on-cache-miss"] == false
        && step["with"]["lookup-only"] == false)
        || (step["id"] == "public_crates_save"
            && step["uses"] == SAVE
            && step["if"] == SAVE_IF
            && step["with"]["path"] == SAVE_PATH)
}
pub fn validate(catalog: &Value, workflow: &Value) -> Result<()> {
    require(
        catalog["schema"] == "humanymous.public-crate-cache-policy/1"
            && catalog["key"] == KEY
            && catalog["restore_path"] == PATH
            && catalog["max_archive_bytes"] == super::scan::MAX_BYTES
            && catalog["max_repository_cache_gb"] == 10
            && catalog["max_retention_days"] == 7
            && catalog["compiled_artifacts_allowed"] == false,
        "public archive cache catalog drift",
    )?;
    let job = &workflow["jobs"]["policy"];
    require(
        job["permissions"] == json!({"contents":"read","actions":"read"}),
        "archive quota readback must remain read-only",
    )?;
    let steps = job["steps"].as_array().ok_or("policy steps missing")?;
    for id in ["public_crates_restore", "public_crates_save"] {
        let matches: Vec<_> = steps.iter().filter(|step| step["id"] == id).collect();
        require(
            matches.len() == 1 && approved_step("rust-validation.yml", "policy", matches[0]),
            "public cache transport guard drift",
        )?;
    }
    let plan = steps
        .iter()
        .find(|step| step["id"] == "public_crates_plan")
        .ok_or("public archive preflight missing")?;
    require(
        plan["run"]
            == "cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- archive-cache"
            && plan.get("if").is_none()
            && plan.get("continue-on-error").is_none()
            && plan["env"]["GH_TOKEN"] == "${{ github.token }}",
        "public archive preflight bypass",
    )
}
