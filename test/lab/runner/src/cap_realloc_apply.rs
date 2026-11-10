//! Cap=0 Cap realloc / Cap-free-apply helper.
//!
//! When Root lands a NEW non-twin Cap-free-restore GRANT + TRANSFER manifest
//! whose `preimageSha256` matches LIVE destination bytes, this Cap0 CLI may
//! flip Unlock A authority CLOSER `cap_free=true`. Refuse-closed otherwise.
//! Never invents Cap free / product / AUTH / AT. Never Cap1 Docker-burns.
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::{
    cap_free_after_unlock_a, UNLOCK_A_CLOSER_REL, UNLOCK_A_CLOSER_SCHEMA,
};
use crate::snapshot;
use crate::transfer_preimage;
use serde_json::{json, Value};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cap-realloc-apply/1";
const GRANT_SCHEMA: &str = "humanymous.root-cap-free-restore-grant/1";
const SPENT_FORBIDDEN_TWINS: &[&str] = &["G2", "G5", "CR", "G6", "RB", "PM"];

/// Cap=0 Cap realloc / Cap-free-apply.
///
/// Usage: `cap-realloc-apply --grant PATH --manifest PATH [--workspace PATH] [--dry-run] [--emit PATH]`
pub fn run(args: &[String]) -> Result<i32> {
    let mut grant_path = None;
    let mut manifest_path = None;
    let mut workspace = None;
    let mut emit_path = None;
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--grant" => {
                index += 1;
                grant_path = Some(args.get(index).ok_or("--grant path missing")?.clone());
            }
            "--manifest" => {
                index += 1;
                manifest_path = Some(args.get(index).ok_or("--manifest path missing")?.clone());
            }
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--dry-run" => dry_run = true,
            _ => {
                return Err(
                    "usage: cap-realloc-apply --grant PATH --manifest PATH [--workspace PATH] [--dry-run] [--emit PATH]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let grant_path = grant_path.ok_or(
        "cap-realloc-apply requires --grant PATH (Root Cap-free-restore GRANT); never invent Cap free",
    )?;
    let manifest_path = manifest_path.ok_or(
        "cap-realloc-apply requires --manifest PATH (TRANSFER preimages must match LIVE); never invent Cap free",
    )?;
    refuse_grant_template_path(Path::new(&grant_path))?;

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cap-realloc-apply refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };
    let report = apply_cap_free_restore(
        &root,
        Path::new(&grant_path),
        Path::new(&manifest_path),
        dry_run,
    )?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cap-realloc-apply encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");
    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("refuse_closed");
    if status.starts_with("refuse_closed") || status == "refused" {
        return Err(format!(
            "cap-realloc-apply refused: status={status}; Cap free stays false until Root GRANT→TRANSFER matches LIVE; never invent Cap free"
        )
        .into());
    }
    Ok(0)
}

/// Cap0 fail-closed: Root must copy GRANT.TEMPLATE.json → GRANT.json.
/// Never treat a `*.TEMPLATE.json` path as an authorized Cap-free-restore grant.
fn refuse_grant_template_path(grant_path: &Path) -> Result<()> {
    let name = grant_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let lowered = name.to_ascii_lowercase();
    if lowered.contains(".template.") || lowered.ends_with(".template.json") {
        return Err(
            "cap-realloc-apply refused: grant path is GRANT.TEMPLATE (or *.TEMPLATE.json); Root must copy to GRANT.json with LIVE-matching TRANSFER; never invent Cap free"
                .into(),
        );
    }
    Ok(())
}

fn load_json(path: &Path, label: &str) -> Result<Value> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "cap-realloc-apply refused: {label} read {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cap-realloc-apply refused: {label} JSON {}: {error}",
            path.display()
        )
        .into()
    })
}

fn feeder_is_spent_twin(grant: &Value) -> bool {
    let feeder = grant
        .get("feeder")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let unlock = grant
        .get("unlock")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    SPENT_FORBIDDEN_TWINS
        .iter()
        .any(|twin| feeder.eq_ignore_ascii_case(twin) || unlock.eq_ignore_ascii_case(twin))
}

pub(crate) fn validate_grant(grant: &Value) -> Result<()> {
    if grant.get("schema").and_then(Value::as_str) != Some(GRANT_SCHEMA) {
        return Err(format!(
            "cap-realloc-apply refused: grant.schema must be {GRANT_SCHEMA}; never invent Cap free"
        )
        .into());
    }
    if grant.get("root_authorized").and_then(Value::as_bool) != Some(true) {
        return Err(
            "cap-realloc-apply refused: grant.root_authorized must be true; never invent Cap free"
                .into(),
        );
    }
    if grant.get("allow_apply").and_then(Value::as_bool) != Some(true) {
        return Err(
            "cap-realloc-apply refused: grant.allow_apply must be true; never invent Cap free"
                .into(),
        );
    }
    if grant
        .get("authorize_cap_free_restore")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "cap-realloc-apply refused: grant.authorize_cap_free_restore must be true; never invent Cap free"
                .into(),
        );
    }
    if grant.get("new_non_twin").and_then(Value::as_bool) != Some(true) {
        return Err(
            "cap-realloc-apply refused: grant.new_non_twin must be true; spent G2/G5/CR/G6/RB/PM twin reburn FORBIDDEN"
                .into(),
        );
    }
    if feeder_is_spent_twin(grant) {
        return Err(
            "cap-realloc-apply refused: grant.feeder/unlock names spent twin G2/G5/CR/G6/RB/PM; NEW non-twin only; Cap free stays false"
                .into(),
        );
    }
    if grant.get("cap_burn_authorized").and_then(Value::as_bool) == Some(true)
        || grant.get("cap1_inside_apply").and_then(Value::as_bool) == Some(true)
    {
        return Err(
            "cap-realloc-apply refused: Cap=0 Cap-free-apply forbids Cap1 inside apply; set cap_burn_authorized=false and cap1_inside_apply=false"
                .into(),
        );
    }
    if grant.get("product").and_then(Value::as_u64) != Some(0)
        || grant.get("pass_product_clean_room").and_then(Value::as_u64) != Some(0)
    {
        return Err(
            "cap-realloc-apply refused: grant invented product/pass_product_clean_room; Cap0 only"
                .into(),
        );
    }
    let closer = grant
        .get("unlock_a_closer")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(
            "cap-realloc-apply refused: grant.unlock_a_closer must name Unlock A authority CLOSER; never invent Cap free",
        )?;
    if closer != UNLOCK_A_CLOSER_REL {
        return Err(format!(
            "cap-realloc-apply refused: grant.unlock_a_closer must be {UNLOCK_A_CLOSER_REL}; probe authority only"
        )
        .into());
    }
    let destinations = grant
        .get("destinations")
        .and_then(Value::as_array)
        .ok_or("cap-realloc-apply refused: grant.destinations array required")?;
    if destinations.is_empty() {
        return Err(
            "cap-realloc-apply refused: grant.destinations must be non-empty LIVE topology paths"
                .into(),
        );
    }
    Ok(())
}

/// Build Root rematch packet from transfer-verify entries so Cap-free GRANT
/// authors can set `preimageSha256` to LIVE digests without another inspect loop.
pub(crate) fn preimage_rematch_packet(report: &Value) -> Value {
    let entries = report
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut mismatches = Vec::new();
    for entry in entries {
        if entry.get("preimage_match").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let destination = entry
            .get("destination")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        let expected = entry
            .get("preimageSha256")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        let live = entry
            .get("liveSha256")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        let kind = if live == "<missing>" {
            "destination_absent"
        } else {
            "preimage_mismatch"
        };
        mismatches.push(json!({
            "destination": destination,
            "kind": kind,
            "manifest_preimageSha256": expected,
            "liveSha256": live,
            "root_must_set_preimageSha256_to": if live == "<missing>" { Value::Null } else { json!(live) },
            "note": if live == "<missing>" {
                "LIVE destination absent; Root must land destination bytes before Cap-free TRANSFER"
            } else {
                "Root rematch transfer-manifest preimageSha256 to liveSha256; Cap free stays false until match"
            }
        }));
    }
    json!({
        "mismatch_count": mismatches.len(),
        "mismatches": mismatches,
        "never_invent_cap_free": true,
        "never_invent_pass_product_clean_room": true
    })
}

fn format_preimage_mismatch_error(rematch: &Value) -> String {
    let count = rematch
        .get("mismatch_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut lines = vec![format!(
        "cap-realloc-apply refused: TRANSFER preimageSha256 mismatch vs LIVE ({count} entries); Cap free stays false; never invent Cap free"
    )];
    if let Some(mismatches) = rematch.get("mismatches").and_then(Value::as_array) {
        for entry in mismatches {
            let destination = entry
                .get("destination")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let expected = entry
                .get("manifest_preimageSha256")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let live = entry
                .get("liveSha256")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let set_to = entry
                .get("root_must_set_preimageSha256_to")
                .and_then(Value::as_str)
                .unwrap_or("<absent>");
            lines.push(format!(
                "destination={destination} manifest_preimageSha256={expected} liveSha256={live} root_must_set_preimageSha256_to={set_to}"
            ));
        }
    }
    lines.join("; ")
}

fn read_unlock_a_closer(root: &Path) -> Result<Value> {
    let path = root.join(UNLOCK_A_CLOSER_REL);
    let closer = load_json(&path, "Unlock A CLOSER")?;
    if closer.get("schema").and_then(Value::as_str) != Some(UNLOCK_A_CLOSER_SCHEMA) {
        return Err(format!(
            "cap-realloc-apply refused: Unlock A CLOSER schema must be {UNLOCK_A_CLOSER_SCHEMA}"
        )
        .into());
    }
    if closer.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err(
            "cap-realloc-apply refused: Unlock A CLOSER.cap_exit must be 0 before Cap free restore"
                .into(),
        );
    }
    Ok(closer)
}

fn write_unlock_a_closer_cap_free(root: &Path, closer: &mut Value) -> Result<()> {
    if let Some(object) = closer.as_object_mut() {
        object.insert("cap_free".into(), json!(true));
        object.insert("cap_realloc".into(), json!(true));
        object.insert(
            "cap_free_restore".into(),
            json!({
                "schema": SCHEMA,
                "grant_schema": GRANT_SCHEMA,
                "note": "Cap0 Cap-free-apply flipped Unlock A CLOSER.cap_free after Root NEW non-twin GRANT + LIVE-matching TRANSFER; Cap1 remains a separate Cap burn"
            }),
        );
    } else {
        return Err("cap-realloc-apply refused: Unlock A CLOSER must be a JSON object".into());
    }
    let path = root.join(UNLOCK_A_CLOSER_REL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CLOSER mkdir: {error}"))?;
    }
    let rendered =
        serde_json::to_string_pretty(closer).map_err(|error| format!("CLOSER encode: {error}"))?;
    fs::write(&path, format!("{rendered}\n")).map_err(|error| format!("CLOSER write: {error}"))?;
    Ok(())
}

pub(crate) fn apply_cap_free_restore(
    root: &Path,
    grant_path: &Path,
    manifest_path: &Path,
    dry_run: bool,
) -> Result<Value> {
    let grant = load_json(grant_path, "grant")?;
    validate_grant(&grant)?;
    let preimage_report = transfer_preimage::verify_manifest(root, manifest_path)?;
    let summary = preimage_report
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let entry_count = summary.get("entries").and_then(Value::as_u64).unwrap_or(0);
    if entry_count == 0 {
        return Err(
            "cap-realloc-apply refused: TRANSFER manifest entries empty; never invent Cap free"
                .into(),
        );
    }
    if preimage_report
        .get("preimages_match")
        .and_then(Value::as_bool)
        != Some(true)
    {
        let rematch = preimage_rematch_packet(&preimage_report);
        let error = format_preimage_mismatch_error(&rematch);
        return Ok(json!({
            "schema": SCHEMA,
            "record_kind": "lab-cap-realloc-apply",
            "cap": 0,
            "cap_burn": 0,
            "cap_free": false,
            "cap_free_before": cap_free_after_unlock_a(root).unwrap_or(false),
            "cap_free_after": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "applied": false,
            "dry_run": dry_run,
            "status": "refuse_closed_preimage_mismatch",
            "error_code": "refuse_closed_preimage_mismatch",
            "apply_path_error": {
                "error_code": "refuse_closed_preimage_mismatch",
                "machine_code": "refuse_closed_preimage_mismatch",
                "root_must": "Rematch transfer-manifest preimageSha256 to liveSha256 in preimage_rematch.root_must_set_preimageSha256_to"
            },
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "grant_schema": GRANT_SCHEMA,
            "grant": grant_path.to_string_lossy(),
            "manifest": manifest_path.to_string_lossy(),
            "preimages_match": false,
            "preimage_summary": summary,
            "preimage_rematch": rematch,
            "error": error,
            "cap1_inside_apply": false,
            "note": "TRANSFER preimageSha256 mismatch vs LIVE; Root rematch digests in preimage_rematch; Cap free stays false; never invent Cap free",
            "honesty": honesty_block()
        }));
    }
    let mut closer = read_unlock_a_closer(root)?;
    let already_free = closer.get("cap_free").and_then(Value::as_bool) == Some(true);
    let cap_free_before = cap_free_after_unlock_a(root)?;

    if already_free {
        return Ok(json!({
            "schema": SCHEMA,
            "record_kind": "lab-cap-realloc-apply",
            "cap": 0,
            "cap_burn": 0,
            "cap_free": true,
            "cap_free_before": true,
            "cap_free_after": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "applied": false,
            "dry_run": dry_run,
            "status": "already_free",
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "grant_schema": GRANT_SCHEMA,
            "grant": grant_path.to_string_lossy(),
            "manifest": manifest_path.to_string_lossy(),
            "preimages_match": true,
            "preimage_summary": preimage_report.get("summary"),
            "cap1_inside_apply": false,
            "note": "Unlock A CLOSER already cap_free=true; Cap0 Cap-free-apply is idempotent; Cap1 remains separate",
            "honesty": honesty_block()
        }));
    }

    if dry_run {
        return Ok(json!({
            "schema": SCHEMA,
            "record_kind": "lab-cap-realloc-apply",
            "cap": 0,
            "cap_burn": 0,
            "cap_free": false,
            "cap_free_before": cap_free_before,
            "cap_free_after": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "applied": false,
            "dry_run": true,
            "status": "validated_dry_run",
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "grant_schema": GRANT_SCHEMA,
            "grant": grant_path.to_string_lossy(),
            "manifest": manifest_path.to_string_lossy(),
            "preimages_match": true,
            "preimage_summary": preimage_report.get("summary"),
            "would_set_cap_free": true,
            "cap1_inside_apply": false,
            "note": "Dry-run validated Root GRANT + LIVE-matching TRANSFER; CLOSER.cap_free not written",
            "honesty": honesty_block()
        }));
    }

    write_unlock_a_closer_cap_free(root, &mut closer)?;
    let cap_free_after = cap_free_after_unlock_a(root)?;
    if !cap_free_after {
        return Err(
            "cap-realloc-apply refused: CLOSER write did not make cap_free_after_unlock_a true; fail-closed"
                .into(),
        );
    }

    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-cap-realloc-apply",
        "cap": 0,
        "cap_burn": 0,
        "cap_free": true,
        "cap_free_before": false,
        "cap_free_after": true,
        "product": 0,
        "pass_product_clean_room": 0,
        "applied": true,
        "dry_run": false,
        "status": "cap_free_restored",
        "unlock_a_closer": UNLOCK_A_CLOSER_REL,
        "grant_schema": GRANT_SCHEMA,
        "grant": grant_path.to_string_lossy(),
        "manifest": manifest_path.to_string_lossy(),
        "preimages_match": true,
        "preimage_summary": preimage_report.get("summary"),
        "cap1_inside_apply": false,
        "note": "Cap0 Cap-free-apply restored Unlock A CLOSER.cap_free=true after Root NEW non-twin GRANT + LIVE-matching TRANSFER; Cap1 product path remains a separate Cap burn",
        "honesty": honesty_block()
    }))
}

fn honesty_block() -> Value {
    json!({
        "never_invent_pass_product_clean_room": true,
        "never_invent_cap_free_without_grant": true,
        "never_forge_auth_at": true,
        "never_cap1_inside_cap_free_apply": true,
        "refuse_spent_twin_reburn_g2_g5_cr_g6_rb_pm": true,
        "require_live_preimage_match": true,
        "detection_freeze": true,
        "cap0_only": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cap-free-apply-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    fn write_closer(root: &Path, cap_free: bool) {
        let path = root.join(UNLOCK_A_CLOSER_REL);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        let closer = json!({
            "schema": UNLOCK_A_CLOSER_SCHEMA,
            "cap_exit": 0,
            "cap_free": cap_free,
            "cap": 1,
            "product": 0,
            "pass_product_clean_room": 0
        });
        fs::write(&path, serde_json::to_vec_pretty(&closer).expect("encode")).expect("write");
    }

    fn write_dest(root: &Path, relative: &str, bytes: &[u8]) -> String {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, bytes).expect("write dest");
        snapshot::digest(bytes)
    }

    fn valid_grant(dest: &str) -> Value {
        json!({
            "schema": GRANT_SCHEMA,
            "root_authorized": true,
            "allow_apply": true,
            "authorize_cap_free_restore": true,
            "new_non_twin": true,
            "feeder": "product-cap-free-restore",
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "destinations": [dest],
            "cap_burn_authorized": false,
            "cap1_inside_apply": false,
            "product": 0,
            "pass_product_clean_room": 0
        })
    }

    fn write_grant_and_manifest(
        root: &Path,
        dest: &str,
        live_sha: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let grant_path = root.join("GRANT.json");
        let manifest_path = root.join("transfer-manifest.json");
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&valid_grant(dest)).expect("grant"),
        )
        .expect("write grant");
        let manifest = json!({
            "entries": [{
                "destination": dest,
                "preimageSha256": live_sha,
                "candidateSha256": live_sha
            }]
        });
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("manifest"),
        )
        .expect("write manifest");
        (grant_path, manifest_path)
    }

    #[test]
    fn missing_grant_file_refuses() {
        let root = temp_root("missing-grant");
        write_closer(&root, false);
        let err = apply_cap_free_restore(
            &root,
            &root.join("no-grant.json"),
            &root.join("no-manifest.json"),
            true,
        )
        .expect_err("must refuse");
        assert!(err.to_string().contains("grant read"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn mismatched_preimage_refuses_closed() {
        let root = temp_root("mismatch");
        write_closer(&root, false);
        let dest = "deployments/lab/product/cap-free-restore-contract.json";
        let live = write_dest(&root, dest, b"live-bytes");
        let (grant_path, manifest_path) = write_grant_and_manifest(&root, dest, &live);
        // Corrupt preimage in manifest.
        let bad = json!({
            "entries": [{
                "destination": dest,
                "preimageSha256": "0".repeat(64),
                "candidateSha256": live
            }]
        });
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&bad).expect("encode"),
        )
        .expect("rewrite");
        let report = apply_cap_free_restore(&root, &grant_path, &manifest_path, false)
            .expect("structured refuse");
        assert_eq!(report["status"], "refuse_closed_preimage_mismatch");
        assert_eq!(report["error_code"], "refuse_closed_preimage_mismatch");
        assert_eq!(
            report["apply_path_error"]["error_code"],
            "refuse_closed_preimage_mismatch"
        );
        assert_eq!(report["preimages_match"], false);
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["preimage_rematch"]["mismatch_count"], 1);
        assert_eq!(
            report["preimage_rematch"]["mismatches"][0]["root_must_set_preimageSha256_to"],
            live
        );
        assert!(report["error"]
            .as_str()
            .unwrap_or("")
            .contains("root_must_set_preimageSha256_to="));
        assert!(!cap_free_after_unlock_a(&root).expect("probe"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn spent_twin_feeder_refuses() {
        let root = temp_root("twin");
        write_closer(&root, false);
        let dest = "deployments/lab/product/cap-free-restore-contract.json";
        let live = write_dest(&root, dest, b"twin-bytes");
        let (grant_path, manifest_path) = write_grant_and_manifest(&root, dest, &live);
        let mut grant = valid_grant(dest);
        grant
            .as_object_mut()
            .expect("obj")
            .insert("feeder".into(), json!("PM"));
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&grant).expect("encode"),
        )
        .expect("rewrite");
        let err = apply_cap_free_restore(&root, &grant_path, &manifest_path, true)
            .expect_err("must refuse twin");
        assert!(err.to_string().contains("spent twin"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn happy_path_fixture_restores_cap_free_without_cap1() {
        let root = temp_root("happy");
        write_closer(&root, false);
        let dest = "deployments/lab/product/cap-free-restore-contract.json";
        let live = write_dest(&root, dest, b"fixture-cap-free-restore");
        let (grant_path, manifest_path) = write_grant_and_manifest(&root, dest, &live);
        let dry = apply_cap_free_restore(&root, &grant_path, &manifest_path, true).expect("dry");
        assert_eq!(dry["status"], "validated_dry_run");
        assert_eq!(dry["cap_free_after"], false);
        assert!(!cap_free_after_unlock_a(&root).expect("still false"));

        let report =
            apply_cap_free_restore(&root, &grant_path, &manifest_path, false).expect("apply");
        assert_eq!(report["status"], "cap_free_restored");
        assert_eq!(report["cap_free_after"], true);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["cap1_inside_apply"], false);
        assert_eq!(report["product"], 0);
        assert!(cap_free_after_unlock_a(&root).expect("now true"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn grant_without_authorize_flag_refuses() {
        let root = temp_root("noauth");
        write_closer(&root, false);
        let dest = "deployments/lab/product/cap-free-restore-contract.json";
        let live = write_dest(&root, dest, b"noauth");
        let (grant_path, manifest_path) = write_grant_and_manifest(&root, dest, &live);
        let mut grant = valid_grant(dest);
        grant
            .as_object_mut()
            .expect("obj")
            .insert("authorize_cap_free_restore".into(), json!(false));
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&grant).expect("encode"),
        )
        .expect("rewrite");
        let err = apply_cap_free_restore(&root, &grant_path, &manifest_path, true)
            .expect_err("must refuse");
        assert!(err.to_string().contains("authorize_cap_free_restore"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn grant_template_filename_refuses_closed() {
        let err = refuse_grant_template_path(Path::new(
            ".spec.local/root/evidence/demo/GRANT.TEMPLATE.json",
        ))
        .expect_err("must refuse template grant path");
        assert!(err.to_string().contains("GRANT.TEMPLATE"));
        assert!(refuse_grant_template_path(Path::new("GRANT.json")).is_ok());
    }
}
