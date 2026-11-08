//! Fail-closed Unlock A/B/C gate for Cap-ready preflight.
//!
//! Cap burn remains forbidden until any unlock is measured true. Unlock C is
//! detected from TRANSFER apply (candidate digests on live destinations) plus
//! Cap module presence. Unlock B is true only when Root-authored
//! `UNLOCK-B-APPLIED.json` exists (written by Unlock-B transfer-apply after a
//! grant with non-empty `pilot_command`). Unlock A stays false unless
//! Root-authored `UNLOCK-A-APPLIED.json` exists — never invent path release.
//! Never invents product.
use crate::docker::Result;
use crate::snapshot;
use crate::transfer_preimage;
use serde_json::{Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-unlock-gate/1";

/// Default private Unlock-C transfer manifest under claimed Cap-ready evidence.
pub const DEFAULT_UNLOCK_C_MANIFEST: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/authority-node-ipc/candidate-pack/transfer-manifest.json";

/// Default private Unlock-B transfer manifest (one-grant-apply scaffolding).
pub const DEFAULT_UNLOCK_B_MANIFEST: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/w2-h2-product-binding/candidate-pack/transfer-manifest.json";

/// Root-authored apply receipts (fail-closed; absence ⇒ unlock false).
const UNLOCK_A_RECEIPT: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/g4-cef-at/UNLOCK-A-APPLIED.json";
const UNLOCK_B_RECEIPT: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/w2-h2-product-binding/UNLOCK-B-APPLIED.json";

/// Live Cap module destinations that must exist after Unlock C transfer.
const UNLOCK_C_REQUIRED_PATHS: &[&str] = &[
    "deployments/lab/authority/CAP-PILOT.md",
    "deployments/lab/authority/cap-pilot-contract.json",
    "test/lab/runner/src/orchestration/authority.rs",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockProbe {
    pub a_g4_cef_at: bool,
    pub b_w2_h2_product_binding: bool,
    pub c_authority_cap: bool,
    pub any: bool,
    pub authority_cap_cli_status: &'static str,
    pub reason_c: String,
}

impl UnlockProbe {
    pub fn to_json(&self, root: &Path, manifest: Option<&Path>) -> Value {
        let unlock_b_manifest = root.join(DEFAULT_UNLOCK_B_MANIFEST);
        json!({
            "schema": SCHEMA,
            "record_kind": "lab-runner-unlock-gate",
            "root": root.to_string_lossy(),
            "manifest": manifest.map(|path| path.to_string_lossy().into_owned()),
            "unlock_b_manifest": if unlock_b_manifest.is_file() {
                Some(unlock_b_manifest.to_string_lossy().into_owned())
            } else {
                None
            },
            "unlock": {
                "A_g4_cef_at": self.a_g4_cef_at,
                "B_w2_h2_product_binding": self.b_w2_h2_product_binding,
                "C_authority_cap": self.c_authority_cap,
                "any": self.any,
            },
            "reasons": {
                "A": if self.a_g4_cef_at {
                    "UNLOCK-A-APPLIED present"
                } else {
                    "no UNLOCK-A-APPLIED; G4 CEF/AT path release absent"
                },
                "B": if self.b_w2_h2_product_binding {
                    "UNLOCK-B-APPLIED present (Root GRANT with pilot_command + transfer-apply)"
                } else {
                    "no UNLOCK-B-APPLIED; Unlock B candidate-pack ready but GRANT.pilot_command still Root-only"
                },
                "C": self.reason_c,
            },
            "authority_cap_cli": {
                "selector": "--authority-node-ipc",
                "status": self.authority_cap_cli_status,
                "reason": self.reason_c,
            },
            "cap": {
                "burn_authorized_by_unlock_gate": self.any,
                "burn_this_turn": false,
            },
            "product": 0,
            "pass_product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "cap_burn_only_after_unlock": true,
                "detection_freeze": true,
                "never_invent_unlock_b_pilot_command": true,
            },
        })
    }
}

/// CLI: probe Unlock A/B/C without burning Cap.
pub fn probe(args: &[String]) -> Result<i32> {
    let mut manifest_path = None;
    let mut root_path = None;
    let mut emit_path = None;
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
            _ => {
                return Err(
                    "usage: unlock-probe [--manifest PATH] [--root PATH] [--emit PATH]".into(),
                );
            }
        }
        index += 1;
    }
    let root = match root_path {
        Some(path) => Path::new(&path).canonicalize()?,
        None => snapshot::root()?,
    };
    let manifest = resolve_manifest(&root, manifest_path.as_deref())?;
    let status = probe_root(&root, manifest.as_deref())?;
    let report = status.to_json(&root, manifest.as_deref());
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("unlock-probe encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
        println!(
            "{}",
            json!({
                "schema": SCHEMA,
                "emit": path,
                "unlock_any": status.any,
                "authority_cap_cli_status": status.authority_cap_cli_status,
                "cap_burn": false,
                "pass_product_clean_room": 0,
            })
        );
    } else {
        println!("{rendered}");
    }
    Ok(0)
}

/// Probe unlock state for Cap-ready / fail-closed Cap CLI.
pub fn probe_root(root: &Path, manifest: Option<&Path>) -> Result<UnlockProbe> {
    let a = root.join(UNLOCK_A_RECEIPT).is_file();
    let b = root.join(UNLOCK_B_RECEIPT).is_file();
    let (c, reason_c, cli_status) = probe_unlock_c(root, manifest)?;
    Ok(UnlockProbe {
        a_g4_cef_at: a,
        b_w2_h2_product_binding: b,
        c_authority_cap: c,
        any: a || b || c,
        authority_cap_cli_status: cli_status,
        reason_c,
    })
}

fn probe_unlock_c(root: &Path, manifest: Option<&Path>) -> Result<(bool, String, &'static str)> {
    let missing: Vec<&str> = UNLOCK_C_REQUIRED_PATHS
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect();
    if !missing.is_empty() {
        return Ok((
            false,
            format!(
                "Unlock C false: required Cap destinations missing ({}); TRANSFER not applied; Cap CLI stays blocked_claim",
                missing.join(", ")
            ),
            "blocked_claim",
        ));
    }
    let Some(manifest_path) = manifest else {
        return Ok((
            false,
            "Unlock C markers present but transfer-manifest absent; fail-closed blocked_claim until manifest verify".into(),
            "blocked_claim",
        ));
    };
    let applied = transfer_preimage::transfer_applied(root, manifest_path)?;
    if !applied {
        return Ok((
            false,
            "Unlock C false: live destinations do not match candidate digests in transfer-manifest; Cap CLI stays blocked_claim".into(),
            "blocked_claim",
        ));
    }
    Ok((
        true,
        "Unlock C true: TRANSFER candidate digests match live destinations and Cap module files exist; Cap=1 may burn matching authority pack with honest component≠product receipts".into(),
        "bound",
    ))
}

fn resolve_manifest(root: &Path, explicit: Option<&str>) -> Result<Option<std::path::PathBuf>> {
    if let Some(path) = explicit {
        return Ok(Some(Path::new(path).canonicalize()?));
    }
    let default = root.join(DEFAULT_UNLOCK_C_MANIFEST);
    if default.is_file() {
        Ok(Some(default))
    } else {
        Ok(None)
    }
}

/// Cap CLI status for `authority-node-ipc` (bound only after Unlock C apply).
pub fn authority_cap_cli_status(root: &Path) -> Result<&'static str> {
    let manifest = resolve_manifest(root, None)?;
    Ok(probe_root(root, manifest.as_deref())?.authority_cap_cli_status)
}

/// Fail-closed refuse for `--authority-node-ipc` until Unlock C is measured true.
///
/// After Unlock C TRANSFER apply, Cap orchestration may burn once. Until then
/// Cap stays `blocked_claim` (no Docker burn, no invented product).
pub fn require_authority_cap_unlocked(root: &Path) -> Result<()> {
    let manifest = resolve_manifest(root, None)?;
    let status = probe_root(root, manifest.as_deref())?;
    if status.c_authority_cap {
        return Ok(());
    }
    Err(format!(
        "authority-node-ipc Cap CLI is blocked_claim: {}; deployments/lab/INTEGRATION.md forbids host-exec of pilot.mjs until Cap Docker path is transferred; never invent pass_product_clean_room",
        status.reason_c
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{UNLOCK_C_REQUIRED_PATHS, probe_root};
    use crate::snapshot;
    use serde_json::json;
    use std::fs;

    #[test]
    fn missing_transfer_keeps_unlock_c_false_and_blocked_claim() {
        let directory = tempfile_dir("hm-unlock-gate-");
        let status = probe_root(&directory, None).expect("probe");
        assert!(!status.c_authority_cap);
        assert!(!status.any);
        assert_eq!(status.authority_cap_cli_status, "blocked_claim");
        assert_eq!(
            status.to_json(&directory, None)["pass_product_clean_room"],
            0
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn applied_manifest_and_required_files_bind_authority_cap() {
        let directory = tempfile_dir("hm-unlock-gate-applied-");
        let mut entries = Vec::new();
        for relative in UNLOCK_C_REQUIRED_PATHS {
            let path = directory.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            let bytes = format!("candidate:{relative}").into_bytes();
            fs::write(&path, &bytes).expect("write");
            entries.push(json!({
                "destination": relative,
                "preimageSha256": null,
                "candidateSha256": snapshot::digest(&bytes),
            }));
        }
        let manifest_path = directory.join("transfer-manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&json!({ "entries": entries })).expect("json"),
        )
        .expect("manifest");
        let status = probe_root(&directory, Some(&manifest_path)).expect("probe");
        assert!(status.c_authority_cap);
        assert!(status.any);
        assert_eq!(status.authority_cap_cli_status, "bound");
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
