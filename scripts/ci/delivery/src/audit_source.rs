//! Exact owner baseline and declared seams; never substitute the current tree.
use crate::{Result, command, require, source};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

const BASE: &str = "f9b0cda3680c3b6d612cac8ace22e237812ed3cb";
const MANIFEST: &str = "crates/audit/tests/fixtures/native-owner-core-f9b0cda3.json";
const CATALOG: &str = "crates/audit/tests/fixtures/native-owner-validation.json";
const CURRENT_MANIFEST: &str =
    "crates/sdk/humanymous-runtime/tests/fixtures/audit/native-owner-core-f9b0cda3.json";
const CURRENT_CATALOG: &str =
    "crates/sdk/humanymous-runtime/tests/fixtures/audit/native-owner-validation.json";
const WIRING: &str = "crates/audit/src/verification/scan/source/native/tests.rs";
const GATE: &str = "crates/audit/src/verification/scan/source/native/tests/prerequisite.rs";
const MANIFEST_SHA: &str = "966fef7d3690d6db5eaa43dee50edaf1a72b46e9d9189587558d09ed347e9cc2";
const WIRING_BASE: &str = "de8e379fcb85e560e1cfc425091d34e56b8a96a9efd5e18d40411b877dd97590";
const WIRING_SEAM: &str = "73e0a818fee54798caab91299414b56789190234704e7304726817e8b4651308";
const GATE_SHA: &str = "9f6639ca5be94e4a6a63506a3f5752ab46bc66f094846738dd590d04931b8629";

fn original(root: &Path, path: &str) -> Result<Vec<u8>> {
    require(source::safe_relative(path), "Audit source path refused")?;
    let mut current = root.to_path_buf();
    for part in Path::new(path).components() {
        current.push(part);
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|_| "Audit source input missing")?;
        require(
            !metadata.file_type().is_symlink(),
            "Audit source symlink refused",
        )?;
    }
    let metadata = std::fs::metadata(&current).map_err(|_| "Audit source metadata missing")?;
    require(
        metadata.is_file() && metadata.len() <= 16 * 1024 * 1024,
        "Audit source file bound refused",
    )?;
    std::fs::read(current).map_err(|_| "Audit source read failed".into())
}

fn strict(bytes: &[u8]) -> Result<Value> {
    humanymous_platform::testing::json::decode_strict_json(bytes, 512 * 1024)
        .map_err(|_| "Audit source JSON refused".into())
}

fn committed(root: &Path, path: &str, bytes: &[u8]) -> Result<String> {
    let object = format!("HEAD:{path}");
    let output = command::output(root, "git", &["show", &object])?;
    require(
        output.status.success() && output.stdout == bytes,
        "Audit owner input differs from candidate Git bytes",
    )?;
    let blob = command::text(root, "git", &["rev-parse", &object])?;
    let blob = blob.trim();
    require(
        blob.len() == 40 && blob.bytes().all(|b| b.is_ascii_hexdigit()),
        "Audit owner Git blob missing",
    )?;
    Ok(blob.into())
}

fn rows(manifest: &Value) -> Result<BTreeMap<String, (u64, String)>> {
    let mut result = BTreeMap::new();
    let entries = manifest.as_array().ok_or("Audit baseline list missing")?;
    require(entries.len() == 193, "Audit baseline denominator changed")?;
    for entry in entries {
        let object = entry.as_object().ok_or("Audit baseline entry missing")?;
        require(
            object.len() == 3
                && object
                    .keys()
                    .all(|key| matches!(key.as_str(), "path" | "bytes" | "sha256")),
            "Audit baseline fields refused",
        )?;
        let path = entry["path"]
            .as_str()
            .ok_or("Audit baseline path missing")?;
        require(source::safe_relative(path), "Audit baseline path refused")?;
        let size = entry["bytes"]
            .as_u64()
            .ok_or("Audit baseline size missing")?;
        require(
            crate::lab_receipts::digest(&entry["sha256"], 64),
            "Audit baseline digest missing",
        )?;
        let hash = entry["sha256"]
            .as_str()
            .ok_or("Audit baseline digest missing")?;
        require(
            result.insert(path.into(), (size, hash.into())).is_none(),
            "Audit baseline duplicate path",
        )?;
    }
    require(
        result
            .get(WIRING)
            .is_some_and(|(_, hash)| hash == WIRING_BASE),
        "Audit baseline wiring changed",
    )?;
    Ok(result)
}

fn compare(
    expected: &BTreeMap<String, (u64, String)>,
    actual: &BTreeMap<String, (u64, String)>,
) -> Result<()> {
    require(
        expected == actual,
        "Audit baseline bytes differ; current-tree substitution refused",
    )
}

/// Check only: this entrypoint neither constructs a source tree nor runs Cargo.
/// The later runner must call baseline before any overlay and prepared afterwards.
pub fn check(root: &Path, phase: &str) -> Result<Value> {
    require(
        matches!(phase, "baseline" | "prepared"),
        "Audit source phase refused",
    )?;
    let prepared = phase == "prepared";
    let checkout = root.join("target/audit-read-only-source");
    require(
        source::head(&checkout)? == BASE,
        "Audit checkout is not exact f9 baseline",
    )?;
    let manifest_bytes = original(root, CURRENT_MANIFEST)?;
    require(
        source::sha(&manifest_bytes) == MANIFEST_SHA,
        "Audit immutable baseline manifest changed",
    )?;
    let manifest_blob = committed(root, CURRENT_MANIFEST, &manifest_bytes)?;
    let catalog_bytes = original(root, CURRENT_CATALOG)?;
    let catalog_blob = committed(root, CURRENT_CATALOG, &catalog_bytes)?;
    let catalog = strict(&catalog_bytes)?;
    require(
        catalog["schema"] == 1
            && catalog["profile"] == "audit-existing-owner-closed-component-v1"
            && catalog["baseline_source"] == BASE
            && catalog["baseline_manifest"] == MANIFEST
            && catalog["baseline_inputs"] == 193
            && catalog["semantic_case_count"] == 13
            && catalog["probe_compiler_attempts"] == 12,
        "Audit owner catalog identity refused",
    )?;
    let modified = catalog["test_only_delta"]["modified"]
        .as_array()
        .ok_or("Audit modified seam absent")?;
    let added = catalog["test_only_delta"]["added"]
        .as_array()
        .ok_or("Audit added seams absent")?;
    require(
        modified.len() == 1
            && modified[0]["path"] == WIRING
            && modified[0]["baseline_sha256"] == WIRING_BASE
            && modified[0]["expected_sha256"] == WIRING_SEAM
            && added.len() == 3
            && added[0]["path"] == GATE
            && added[0]["sha256"] == GATE_SHA
            && added[1]["path"] == MANIFEST
            && added[1]["sha256"] == MANIFEST_SHA
            && added[2]["path"] == CATALOG,
        "Audit declared seam set changed",
    )?;
    let mut expected = rows(&strict(&manifest_bytes)?)?;
    if prepared {
        let wiring = original(root, WIRING)?;
        let gate = original(root, GATE)?;
        require(
            source::sha(&wiring) == WIRING_SEAM && source::sha(&gate) == GATE_SHA,
            "Audit candidate seams changed",
        )?;
        committed(root, WIRING, &wiring)?;
        committed(root, GATE, &gate)?;
        expected.insert(WIRING.into(), (wiring.len() as u64, source::sha(&wiring)));
        for (path, bytes) in [
            (GATE, gate),
            (MANIFEST, manifest_bytes),
            (CATALOG, catalog_bytes),
        ] {
            expected.insert(path.into(), (bytes.len() as u64, source::sha(&bytes)));
        }
    }
    let mut actual = BTreeMap::new();
    for path in expected.keys() {
        let bytes = original(&checkout, path)?;
        actual.insert(path.clone(), (bytes.len() as u64, source::sha(&bytes)));
    }
    compare(&expected, &actual)?;
    // An exact HEAD plus the selected hashes is insufficient if undeclared files
    // or tracked edits can change Cargo's inputs. Require the entire worktree delta.
    let status = command::text(
        &checkout,
        "git",
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    let mut changes: Vec<_> = status.lines().map(str::to_string).collect();
    changes.sort();
    let mut allowed = if prepared {
        vec![
            format!(" M {WIRING}"),
            format!("?? {GATE}"),
            format!("?? {MANIFEST}"),
            format!("?? {CATALOG}"),
        ]
    } else {
        vec![]
    };
    allowed.sort();
    require(
        changes == allowed,
        "Audit undeclared checkout delta refused",
    )?;
    Ok(
        json!({"schema":"humanymous.audit-source-check/1","phase":phase,
        "runner_source_sha":source::head(root)?,"baseline_source_sha":BASE,
        "baseline_inputs":193,"baseline_unchanged":if prepared {192} else {193},
        "declared_modified":if prepared {1} else {0},"declared_added":if prepared {3} else {0},
        "owner_catalog_git_blob":catalog_blob,"baseline_manifest_git_blob":manifest_blob,
        "current_manifest_path":CURRENT_MANIFEST,"current_catalog_path":CURRENT_CATALOG,
        "historical_manifest_path":MANIFEST,"historical_catalog_path":CATALOG,
        "projected_inputs":actual.len(),"projected_manifest_sha256":source::sha(&serde_json::to_vec(&actual).map_err(|e|e.to_string())?),
        "unmodified_f9_execution":false,"compiler_executed":false,"full_product_qualified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_test_support_baseline_drift_is_refused() {
        let mut baseline = BTreeMap::new();
        for path in [
            "crates/test-support/src/process.rs",
            "crates/test-support/src/process/controlled_barrier.rs",
        ] {
            baseline.insert(path.into(), (3, source::sha(b"old")));
        }
        assert!(compare(&baseline, &baseline).is_ok());
        for path in baseline.keys() {
            let mut current = baseline.clone();
            current.insert(path.clone(), (3, source::sha(b"new")));
            assert!(compare(&baseline, &current).is_err());
        }
        let mut extra = baseline.clone();
        extra.insert("crates/audit/build.rs".into(), (0, source::sha(b"")));
        assert!(compare(&baseline, &extra).is_err());
        let mut missing = baseline.clone();
        missing.remove(baseline.keys().next().unwrap());
        assert!(compare(&baseline, &missing).is_err());
    }

    #[test]
    fn owner_baseline_denominator_and_wiring_are_closed() {
        // Synthetic schema inputs, not a replica of the immutable owner corpus.
        // The repository-only check above verifies the actual retained manifest.
        let mut manifest = json!((0..192).map(|index| json!({"path":format!("synthetic/input-{index}.rs"),"bytes":3,"sha256":source::sha(b"old")})).collect::<Vec<_>>());
        manifest
            .as_array_mut()
            .unwrap()
            .push(json!({"path":WIRING,"bytes":3,"sha256":WIRING_BASE}));
        assert_eq!(rows(&manifest).unwrap().len(), 193);
        let mut duplicate = manifest.clone();
        duplicate[1] = duplicate[0].clone();
        assert!(rows(&duplicate).is_err());
        let mut missing = manifest.clone();
        missing.as_array_mut().unwrap().pop();
        assert!(rows(&missing).is_err());
        let mut drift = manifest;
        let entry = drift
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["path"] == WIRING)
            .unwrap();
        entry["sha256"] = json!("0".repeat(64));
        assert!(rows(&drift).is_err());
    }
}
