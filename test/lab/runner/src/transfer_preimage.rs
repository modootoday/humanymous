//! TRANSFER apply / preimage checks for Cap-ready unlock→Cap shortening.
//!
//! Fail-closed: Cap must abort when live workspace bytes disagree with the
//! reviewed transfer-manifest preimages or (after apply) candidate digests.
//! Never invents `pass_product_clean_room`. Does not apply transfers.
use crate::docker::Result;
use crate::snapshot;
use serde_json::{Map, Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-transfer-preimage-verify/1";

/// Verify a Unlock-C style `transfer-manifest.json` against the live workspace.
pub fn verify(args: &[String]) -> Result<i32> {
    let mut manifest_path = None;
    let mut root_path = None;
    let mut emit_path = None;
    let mut expect_applied = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                index += 1;
                manifest_path = Some(args.get(index).ok_or("--manifest path missing")?.clone());
            }
            "--root" => {
                index += 1;
                root_path = Some(args.get(index).ok_or("--root path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--expect-applied" => expect_applied = true,
            _ => {
                return Err(
                    "usage: transfer-verify --manifest PATH [--root PATH] [--emit PATH] [--expect-applied]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let manifest_path = manifest_path.ok_or("--manifest required")?;
    let root = match root_path {
        Some(path) => Path::new(&path).canonicalize()?,
        None => snapshot::root()?,
    };
    let report = verify_manifest(&root, Path::new(&manifest_path))?;
    let applied = report["applied"]
        .as_bool()
        .ok_or("transfer verify missing applied")?;
    let preimages_ok = report["preimages_match"]
        .as_bool()
        .ok_or("transfer verify missing preimages_match")?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("transfer verify encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    } else {
        println!("{rendered}");
    }
    if expect_applied {
        if !applied {
            return Err(
                "transfer-verify --expect-applied refused: live destinations do not match candidate digests; Cap abort; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(0);
    }
    if !preimages_ok && !applied {
        return Err(
            "transfer-verify refused: live preimage mismatch and transfer not applied; Cap abort; never invent pass_product_clean_room"
                .into(),
        );
    }
    Ok(0)
}

/// Pure verify against an already-parsed manifest object and workspace root.
pub fn verify_manifest(root: &Path, manifest_path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(manifest_path)
        .map_err(|error| format!("manifest read failed: {error}"))?;
    let manifest: Value =
        serde_json::from_str(&raw).map_err(|error| format!("manifest json: {error}"))?;
    verify_manifest_value(root, &manifest, manifest_path)
}

fn verify_manifest_value(root: &Path, manifest: &Value, manifest_path: &Path) -> Result<Value> {
    let entries = manifest
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("manifest.entries array required")?;
    let mut checked = Vec::new();
    let mut preimage_ok = 0u64;
    let mut preimage_mismatch = 0u64;
    let mut candidate_ok = 0u64;
    let mut candidate_mismatch = 0u64;
    let mut missing_new = 0u64;
    for entry in entries {
        let destination = entry
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("entry.destination required")?;
        let candidate_sha = entry
            .get("candidateSha256")
            .and_then(Value::as_str)
            .ok_or("entry.candidateSha256 required")?;
        let preimage_sha = entry.get("preimageSha256").and_then(Value::as_str);
        let live_path = root.join(destination);
        let live_sha = match fs::read(&live_path) {
            Ok(bytes) => Some(snapshot::digest(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!("live read {destination}: {error}").into());
            }
        };
        let mut row = Map::new();
        row.insert("destination".into(), json!(destination));
        row.insert("candidateSha256".into(), json!(candidate_sha));
        row.insert(
            "preimageSha256".into(),
            match preimage_sha {
                Some(sha) => json!(sha),
                None => Value::Null,
            },
        );
        row.insert(
            "liveSha256".into(),
            match live_sha.as_deref() {
                Some(sha) => json!(sha),
                None => Value::Null,
            },
        );
        let preimage_match = match (preimage_sha, live_sha.as_deref()) {
            (Some(expected), Some(live)) => expected.eq_ignore_ascii_case(live),
            (Some(_), None) => false,
            (None, None) => true,
            (None, Some(_)) => false,
        };
        let candidate_match = live_sha
            .as_deref()
            .is_some_and(|live| live.eq_ignore_ascii_case(candidate_sha));
        if preimage_match {
            preimage_ok += 1;
        } else if preimage_sha.is_some() {
            preimage_mismatch += 1;
        } else if live_sha.is_none() {
            missing_new += 1;
        } else {
            preimage_mismatch += 1;
        }
        if candidate_match {
            candidate_ok += 1;
        } else {
            candidate_mismatch += 1;
        }
        row.insert("preimage_match".into(), json!(preimage_match));
        row.insert("candidate_match".into(), json!(candidate_match));
        checked.push(Value::Object(row));
    }
    let applied = candidate_mismatch == 0 && !entries.is_empty();
    let preimages_match = preimage_mismatch == 0;
    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-runner-transfer-preimage-verify",
        "manifest": manifest_path.to_string_lossy(),
        "root": root.to_string_lossy(),
        "cap_burn": false,
        "pass_product": 0,
        "pass_product_clean_room": 0,
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "abort_cap_on_preimage_mismatch": true,
            "transfer_apply_is_root_owned": true,
            "detection_freeze": true,
        },
        "summary": {
            "entries": entries.len(),
            "preimage_ok": preimage_ok,
            "preimage_mismatch": preimage_mismatch,
            "candidate_ok": candidate_ok,
            "candidate_mismatch": candidate_mismatch,
            "missing_new_destinations": missing_new,
        },
        "preimages_match": preimages_match,
        "applied": applied,
        "cap_ready_for_unlock_c": applied,
        "entries": checked,
    }))
}

/// True when every destination in the manifest matches its candidate digest.
pub fn transfer_applied(root: &Path, manifest_path: &Path) -> Result<bool> {
    let report = verify_manifest(root, manifest_path)?;
    Ok(report["applied"].as_bool().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::verify_manifest_value;
    use crate::snapshot;
    use serde_json::json;
    use std::fs;

    #[test]
    fn preimage_and_candidate_mismatch_keeps_applied_false() {
        let directory = tempfile_dir("hm-transfer-preimage-");
        let dest = "deployments/lab/INTEGRATION.md";
        let path = directory.join(dest);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, b"live-bytes").expect("write");
        let live = snapshot::digest(b"live-bytes");
        let manifest = json!({
            "entries": [{
                "destination": dest,
                "preimageSha256": live,
                "candidateSha256": snapshot::digest(b"candidate-bytes"),
            }]
        });
        let report = verify_manifest_value(
            &directory,
            &manifest,
            directory.join("manifest.json").as_path(),
        )
        .expect("verify");
        assert_eq!(report["preimages_match"], true);
        assert_eq!(report["applied"], false);
        assert_eq!(report["pass_product_clean_room"], 0);
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn candidate_digest_match_marks_applied() {
        let directory = tempfile_dir("hm-transfer-applied-");
        let dest = "deployments/lab/authority/CAP-PILOT.md";
        let path = directory.join(dest);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        let bytes = b"cap-pilot-notes";
        fs::write(&path, bytes).expect("write");
        let digest = snapshot::digest(bytes);
        let manifest = json!({
            "entries": [{
                "destination": dest,
                "preimageSha256": null,
                "candidateSha256": digest,
            }]
        });
        let report = verify_manifest_value(
            &directory,
            &manifest,
            directory.join("manifest.json").as_path(),
        )
        .expect("verify");
        assert_eq!(report["applied"], true);
        assert_eq!(report["cap_ready_for_unlock_c"], true);
        let _ = fs::remove_dir_all(&directory);
    }

    fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
        let mut random = [0u8; 4];
        getrandom::fill(&mut random).expect("entropy");
        let path = std::env::temp_dir().join(format!(
            "{prefix}{:02x}{:02x}{:02x}{:02x}",
            random[0], random[1], random[2], random[3]
        ));
        fs::create_dir(&path).expect("tempdir");
        path
    }
}
