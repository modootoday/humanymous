//! Fail-closed Unlock A/B/C gate for Cap-ready preflight.
//!
//! Cap burn remains forbidden until any unlock is measured true. Unlock C is
//! detected from TRANSFER apply (candidate digests on live destinations) plus
//! Cap module presence. Unlock B is true only when Root-authored
//! `UNLOCK-B-APPLIED.json` exists (written by Unlock-B transfer-apply after a
//! grant with non-empty `pilot_command` naming the code-defined Cap CLI
//! `--w2-h2-product-binding`). Unlock A is true only when Root-authored
//! `UNLOCK-A-APPLIED.json` exists with Cap CLI `--g4-cef-at` pilot after
//! path-release destinations — never invent path release, CEF bundle, or
//! folklore pilot argv. Never invents product. Capture `observe_h2` ≠
//! product-binding. A11y/`--require-full` Cap spam forbidden.
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::{
    BROWSER_READY_CAP_SELECTOR, BROWSER_READY_PILOT_COMMAND, BROWSER_READY_ROOT_ASK_PATHS,
    G2_CAP_SELECTOR, G2_PILOT_COMMAND, G2_PRODUCT_BINDING_TOPOLOGY, G5_CAP_SELECTOR,
    G5_PILOT_COMMAND, G5_ROOT_ASK_PATHS, INTERACTIVE_AT_HANDOFF_CANDIDATES,
    N06_OWNER_AUTH_CAP_SELECTOR, N06_OWNER_AUTH_PILOT_COMMAND, cap_free_after_unlock_a,
};
use crate::orchestration::g2_collector_product_binding::{
    CAP_READY_PACK_REL as G2_CAP_READY_PACK, CAP_SELECTOR as G2_CAP_CLI_SELECTOR,
    CODE_DEFINED_PILOT_COMMAND as G2_CAP_CLI_PILOT, CONTRACT_REL as G2_CAP_CONTRACT,
    REQUIRED_CAP_TOPOLOGY as G2_REQUIRED_CAP_TOPOLOGY, missing_cap_topology as g2_missing_cap_topology,
};
use crate::orchestration::g4_cef_at::{
    CAP_SELECTOR as G4_CEF_CAP_SELECTOR, CEF_BUNDLE_ENV,
    CODE_DEFINED_PILOT_COMMAND as G4_CODE_DEFINED_PILOT, PATH_RELEASE_SURFACES,
    REQUIRED_CAP_MODULE,
};
use crate::orchestration::h2_product_binding::{
    CAP_SELECTOR as W2_H2_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND,
    N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES, N06_OWNER_JOIN_CAP_SELECTOR,
    N06_OWNER_JOIN_PILOT_COMMAND, N06_OWNER_JOIN_ROOT_ASK_PATHS,
};
use crate::snapshot;
use crate::transfer_preimage;
use serde_json::{Value, json};
use std::{env, fs, path::Path};

const SCHEMA: &str = "humanymous.lab-unlock-gate/1";

/// Default private Unlock-C transfer manifest under claimed Cap-ready evidence.
pub const DEFAULT_UNLOCK_C_MANIFEST: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/authority-node-ipc/candidate-pack/transfer-manifest.json";

/// Default private Unlock-B transfer manifest (one-grant-apply scaffolding).
pub const DEFAULT_UNLOCK_B_MANIFEST: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/w2-h2-product-binding/candidate-pack/transfer-manifest.json";

/// Default private Unlock-A transfer manifest (path-release scaffold; empty entries).
pub const DEFAULT_UNLOCK_A_MANIFEST: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/g4-cef-at/candidate-pack/transfer-manifest.json";

/// Root-authored apply receipts (fail-closed; absence ⇒ unlock false).
const UNLOCK_A_RECEIPT: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/g4-cef-at/UNLOCK-A-APPLIED.json";
const UNLOCK_B_RECEIPT: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/w2-h2-product-binding/UNLOCK-B-APPLIED.json";

/// Live Cap module destinations that must exist after Unlock C transfer.
const UNLOCK_C_REQUIRED_PATHS: &[&str] = &[
    "deployments/lab/authority/CAP-PILOT.md",
    "deployments/lab/authority/cap-pilot-contract.json",
    "test/lab/runner/src/orchestration/authority.rs",
];

/// Live Cap contract destination after Unlock B transfer (topology still Root-only).
const UNLOCK_B_REQUIRED_PATHS: &[&str] = &[
    "deployments/lab/gateway/h2-product-binding-contract.json",
    "deployments/lab/gateway/H2-PRODUCT-BINDING.md",
    "test/lab/runner/src/orchestration/h2_product_binding.rs",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockProbe {
    pub a_g4_cef_at: bool,
    pub b_w2_h2_product_binding: bool,
    pub c_authority_cap: bool,
    pub any: bool,
    pub authority_cap_cli_status: &'static str,
    pub reason_c: String,
    /// Cap CLI status for W2 H2 product-binding (bound only after Unlock B + Cap CLI pilot).
    pub w2_h2_cap_cli_status: &'static str,
    /// Cap argv echoed from UNLOCK-B-APPLIED when present; code-defined Cap CLI preferred.
    pub w2_h2_pilot_command: Option<String>,
    pub reason_b: String,
    /// Cap CLI status for G4 CEF/AT (bound only after Unlock A + Cap CLI pilot).
    pub g4_cef_at_cap_cli_status: &'static str,
    /// Cap argv echoed from UNLOCK-A-APPLIED when present; code-defined Cap CLI preferred.
    pub g4_cef_at_pilot_command: Option<String>,
    pub reason_a: String,
}

impl UnlockProbe {
    pub fn to_json(&self, root: &Path, manifest: Option<&Path>) -> Value {
        let unlock_a_manifest = root.join(DEFAULT_UNLOCK_A_MANIFEST);
        let unlock_b_manifest = root.join(DEFAULT_UNLOCK_B_MANIFEST);
        let cef_bundle_present = env::var_os(CEF_BUNDLE_ENV)
            .as_ref()
            .is_some_and(|value| !value.is_empty());
        json!({
            "schema": SCHEMA,
            "record_kind": "lab-runner-unlock-gate",
            "root": root.to_string_lossy(),
            "manifest": manifest.map(|path| path.to_string_lossy().into_owned()),
            "unlock_a_manifest": if unlock_a_manifest.is_file() {
                Some(unlock_a_manifest.to_string_lossy().into_owned())
            } else {
                None
            },
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
                "A": self.reason_a,
                "B": self.reason_b,
                "C": self.reason_c,
            },
            "authority_cap_cli": {
                "selector": "--authority-node-ipc",
                "status": self.authority_cap_cli_status,
                "reason": self.reason_c,
            },
            "w2_h2_cap_cli": {
                "selector": W2_H2_CAP_SELECTOR,
                "status": self.w2_h2_cap_cli_status,
                "pilot_command": self.w2_h2_pilot_command,
                "code_defined_pilot_command": CODE_DEFINED_PILOT_COMMAND,
                "reason": self.reason_b,
                "note": "Cap CLI --w2-h2-product-binding is the code-defined GRANT.pilot_command; refuse capture observe_h2 and feeder --require-full; Cap topology compose/Dockerfile remain Root-allocated"
            },
            "g4_cef_at_cap_cli": {
                "selector": G4_CEF_CAP_SELECTOR,
                "status": self.g4_cef_at_cap_cli_status,
                "pilot_command": self.g4_cef_at_pilot_command,
                "code_defined_pilot_command": G4_CODE_DEFINED_PILOT,
                "reason": self.reason_a,
                "path_release_surfaces": PATH_RELEASE_SURFACES,
                "required_cap_module": REQUIRED_CAP_MODULE,
                "cef_bundle_env": CEF_BUNDLE_ENV,
                "cef_bundle_present": cef_bundle_present,
                "note": "Cap CLI --g4-cef-at is the code-defined GRANT.pilot_command; refuse a11y/--require-full Cap spam; empty transfer entries + missing path release + missing CEF bundle keep blocked_claim"
            },
            "cap": {
                "burn_authorized_by_unlock_gate": self.any,
                "burn_this_turn": false,
                "free": cap_free_after_unlock_a(root).unwrap_or(false),
                "realloc_policy": "only_real_structural_next_feeder_after_exit0_honesty",
                "no_cap_spam_abc": true,
                "g2_g5_cap_cli_with_topology": g2_missing_cap_topology(root).is_empty(),
                "g2_cap_cli_scaffolded": true,
                "g2_cap_topology_live": g2_missing_cap_topology(root).is_empty(),
            },
            "g2_collector_product_binding_cap_cli": {
                "selector": G2_CAP_CLI_SELECTOR,
                "pilot_command": G2_CAP_CLI_PILOT,
                "code_defined_pilot_command": G2_CAP_CLI_PILOT,
                "status": if g2_missing_cap_topology(root).is_empty() {
                    "cap_cli_topology_live_cap1_requires_realloc"
                } else {
                    "cap0_scaffold_topology_cap_ready"
                },
                "required_cap_topology": G2_REQUIRED_CAP_TOPOLOGY,
                "missing_cap_topology": g2_missing_cap_topology(root),
                "contract": G2_CAP_CONTRACT,
                "cap_ready_pack": G2_CAP_READY_PACK,
                "note": "Cap CLI Cap=0 scaffolded; Cap topology Cap-ready until TRANSFER; Cap=1 only after Cap realloc + live topology; never invent pass_product_clean_room"
            },
            "g2_collector_product_binding_cap0": {
                "selector": G2_CAP_SELECTOR,
                "pilot_command": G2_PILOT_COMMAND,
                "status": "cap0_honesty_only",
                "topology_surfaces": G2_PRODUCT_BINDING_TOPOLOGY,
                "note": "Cap=0 readiness only; Desktop collector component ≠ G2 product; Cap CLI Cap=0 scaffolded separately; no Cap=1 without Cap realloc + live Cap CLI+topology"
            },
            "g5_witness_transport_cap0": {
                "selector": G5_CAP_SELECTOR,
                "pilot_command": G5_PILOT_COMMAND,
                "status": "cap0_honesty_only",
                "root_ask_paths": G5_ROOT_ASK_PATHS,
                "note": "Cap=0 honesty only; custody/witness component ≠ authenticated transport product; no Cap=1 without Cap free + claimable Cap CLI+topology"
            },
            "n06_owner_join_cap0": {
                "selector": N06_OWNER_JOIN_CAP_SELECTOR,
                "pilot_command": N06_OWNER_JOIN_PILOT_COMMAND,
                "status": "cap0_honesty_only",
                "n06_owner_join": false,
                "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
                "auth_handoff_candidates": N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
                "note": "Cap=0 readiness only; hosted Cap EXIT0 + Unlock A EXIT0 bound ≠ owner join; Root auth handoff still required; never invent Root tokens"
            },
            "n06_owner_auth_handoff_cap0": {
                "selector": N06_OWNER_AUTH_CAP_SELECTOR,
                "pilot_command": N06_OWNER_AUTH_PILOT_COMMAND,
                "status": "cap0_honesty_only",
                "n06_owner_join": false,
                "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
                "auth_handoff_candidates": N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
                "note": "Cap=0 honesty only; hosted Cap EXIT0 + Unlock A EXIT0 + N06 readiness ≠ owner join; Root owner-auth handoff still required; never invent Root tokens"
            },
            "browser_ready_interactive_at_cap0": {
                "selector": BROWSER_READY_CAP_SELECTOR,
                "pilot_command": BROWSER_READY_PILOT_COMMAND,
                "status": "cap0_honesty_only",
                "browser_ready": false,
                "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
                "handoff_candidates": INTERACTIVE_AT_HANDOFF_CANDIDATES,
                "note": "Cap=0 honesty only; Unlock A g4_cef_at executed ≠ BrowserReady; Root interactive AT Windows tuple still required; never invent AT tokens"
            },
            "product": 0,
            "pass_product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "cap_burn_only_after_unlock": true,
                "detection_freeze": true,
                "never_invent_unlock_b_pilot_command": true,
                "code_defined_w2_h2_cap_cli": true,
                "never_invent_unlock_a_path_release": true,
                "code_defined_g4_cef_at_cap_cli": true,
                "never_cap_spam_a11y_require_full": true,
                "capture_component_is_not_product_binding": true,
                "cap_realloc_only_structural_feeder": true,
                "g2_g5_cap0_not_product": true,
                "g2_cap_cli_scaffolded_not_product": true,
                "code_defined_g2_collector_product_binding_cap_cli": true,
                "g4_component_not_browser_ready": true,
                "never_invent_interactive_at_tokens": true,
                "hosted_unlock_a_n06_readiness_not_owner_join": true,
                "never_invent_n06_owner_auth_tokens": true,
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
                "w2_h2_cap_cli_status": status.w2_h2_cap_cli_status,
                "w2_h2_pilot_command": status.w2_h2_pilot_command,
                "g4_cef_at_cap_cli_status": status.g4_cef_at_cap_cli_status,
                "g4_cef_at_pilot_command": status.g4_cef_at_pilot_command,
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
    let (a, reason_a, g4_status, g4_pilot) = probe_unlock_a(root)?;
    let (b, reason_b, w2_status, w2_pilot) = probe_unlock_b(root)?;
    let (c, reason_c, cli_status) = probe_unlock_c(root, manifest)?;
    Ok(UnlockProbe {
        a_g4_cef_at: a,
        b_w2_h2_product_binding: b,
        c_authority_cap: c,
        any: a || b || c,
        authority_cap_cli_status: cli_status,
        reason_c,
        w2_h2_cap_cli_status: w2_status,
        w2_h2_pilot_command: w2_pilot,
        reason_b,
        g4_cef_at_cap_cli_status: g4_status,
        g4_cef_at_pilot_command: g4_pilot,
        reason_a,
    })
}

/// Unlock A Cap surface: bound only when UNLOCK-A-APPLIED carries Cap CLI pilot.
fn probe_unlock_a(root: &Path) -> Result<(bool, String, &'static str, Option<String>)> {
    let receipt_path = root.join(UNLOCK_A_RECEIPT);
    let manifest_path = root.join(DEFAULT_UNLOCK_A_MANIFEST);
    let empty_entries = manifest_entries_empty(&manifest_path);
    let cef_bundle_present = env::var_os(CEF_BUNDLE_ENV)
        .as_ref()
        .is_some_and(|value| !value.is_empty());
    if !receipt_path.is_file() {
        return Ok((
            false,
            format!(
                "no UNLOCK-A-APPLIED; Unlock A Cap CLI {G4_CEF_CAP_SELECTOR} exists but GRANT.pilot_command + path-release destinations + transfer-apply still required; exact missing: path release {} OR browser-ui execution admission; empty transfer-manifest entries={empty_entries}; {CEF_BUNDLE_ENV} present={cef_bundle_present}; never invent path release or Cap-spam a11y/--require-full",
                PATH_RELEASE_SURFACES.join(" and/or ")
            ),
            "blocked_claim",
            None,
        ));
    }
    let raw = fs::read_to_string(&receipt_path)
        .map_err(|error| format!("UNLOCK-A-APPLIED read failed: {error}"))?;
    let receipt: Value =
        serde_json::from_str(&raw).map_err(|error| format!("UNLOCK-A-APPLIED json: {error}"))?;
    let pilot = receipt
        .get("pilot_command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let Some(pilot) = pilot else {
        return Ok((
            false,
            format!(
                "UNLOCK-A-APPLIED present but pilot_command empty/absent; fail-closed blocked_claim; exact missing Cap CLI {G4_CEF_CAP_SELECTOR}; never invent pilot_command; {CEF_BUNDLE_ENV} present={cef_bundle_present}"
            ),
            "blocked_claim",
            None,
        ));
    };
    if !pilot.contains(G4_CEF_CAP_SELECTOR) {
        return Ok((
            false,
            format!(
                "UNLOCK-A-APPLIED pilot_command lacks Cap CLI {G4_CEF_CAP_SELECTOR}; refuse a11y/feeder folklore; code-defined pilot is {G4_CODE_DEFINED_PILOT}; {CEF_BUNDLE_ENV} present={cef_bundle_present}"
            ),
            "blocked_claim",
            Some(pilot),
        ));
    }
    let missing: Vec<&str> = REQUIRED_CAP_MODULE
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect();
    if !missing.is_empty() {
        return Ok((
            false,
            format!(
                "UNLOCK-A-APPLIED present but Cap module/freeze destinations missing ({}); transfer incomplete; Cap CLI stays blocked_claim; {CEF_BUNDLE_ENV} present={cef_bundle_present}",
                missing.join(", ")
            ),
            "blocked_claim",
            Some(pilot),
        ));
    }
    if !cef_bundle_present {
        // Unlock measured true for grant/apply, but Cap day still refuse-closed on bundle.
        return Ok((
            true,
            format!(
                "UNLOCK-A-APPLIED present with Cap CLI {G4_CEF_CAP_SELECTOR}; Cap=1 may attempt matching g4-cef-at pack but Cap day refuse-closed until {CEF_BUNDLE_ENV} is set; path release surfaces {}; component≠product",
                PATH_RELEASE_SURFACES.join(" and/or ")
            ),
            "bound",
            Some(pilot),
        ));
    }
    Ok((
        true,
        format!(
            "UNLOCK-A-APPLIED present with Cap CLI {G4_CEF_CAP_SELECTOR} and {CEF_BUNDLE_ENV}; Cap=1 may burn matching g4-cef-at pack once; component≠product; never Cap-spam a11y/--require-full"
        ),
        "bound",
        Some(pilot),
    ))
}

fn manifest_entries_empty(manifest_path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(manifest_path) else {
        return true;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&raw) else {
        return true;
    };
    manifest
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| entries.is_empty())
        .unwrap_or(true)
}

/// Unlock B Cap surface: bound only when UNLOCK-B-APPLIED carries Cap CLI pilot.
fn probe_unlock_b(root: &Path) -> Result<(bool, String, &'static str, Option<String>)> {
    let receipt_path = root.join(UNLOCK_B_RECEIPT);
    if !receipt_path.is_file() {
        return Ok((
            false,
            format!(
                "no UNLOCK-B-APPLIED; Unlock B Cap CLI {W2_H2_CAP_SELECTOR} exists but GRANT.pilot_command + transfer-apply still required; Cap topology compose/Dockerfile remain Root-only"
            ),
            "blocked_claim",
            None,
        ));
    }
    let raw = fs::read_to_string(&receipt_path)
        .map_err(|error| format!("UNLOCK-B-APPLIED read failed: {error}"))?;
    let receipt: Value =
        serde_json::from_str(&raw).map_err(|error| format!("UNLOCK-B-APPLIED json: {error}"))?;
    let pilot = receipt
        .get("pilot_command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let Some(pilot) = pilot else {
        return Ok((
            false,
            "UNLOCK-B-APPLIED present but pilot_command empty/absent; fail-closed blocked_claim; never invent pilot_command".into(),
            "blocked_claim",
            None,
        ));
    };
    if !pilot.contains(W2_H2_CAP_SELECTOR) {
        return Ok((
            false,
            format!(
                "UNLOCK-B-APPLIED pilot_command lacks Cap CLI {W2_H2_CAP_SELECTOR}; refuse capture/feeder folklore; code-defined pilot is {CODE_DEFINED_PILOT_COMMAND}"
            ),
            "blocked_claim",
            Some(pilot),
        ));
    }
    let missing: Vec<&str> = UNLOCK_B_REQUIRED_PATHS
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect();
    if !missing.is_empty() {
        return Ok((
            false,
            format!(
                "UNLOCK-B-APPLIED present but Cap contract/module destinations missing ({}); transfer incomplete; Cap CLI stays blocked_claim",
                missing.join(", ")
            ),
            "blocked_claim",
            Some(pilot),
        ));
    }
    Ok((
        true,
        format!(
            "UNLOCK-B-APPLIED present with Cap CLI {W2_H2_CAP_SELECTOR}; Cap=1 may attempt matching w2-h2-product-binding pack (refuse-closed if Cap topology absent); component≠product"
        ),
        "bound",
        Some(pilot),
    ))
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

/// Cap CLI status for `--w2-h2-product-binding` (bound only after Unlock B apply).
pub fn w2_h2_cap_cli_status(root: &Path) -> Result<&'static str> {
    Ok(probe_root(root, None)?.w2_h2_cap_cli_status)
}

/// Cap CLI status for `--g4-cef-at` (bound only after Unlock A apply).
pub fn g4_cef_at_cap_cli_status(root: &Path) -> Result<&'static str> {
    Ok(probe_root(root, None)?.g4_cef_at_cap_cli_status)
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

/// Fail-closed refuse for `--w2-h2-product-binding` until Unlock B is measured true.
pub fn require_w2_h2_product_binding_unlocked(root: &Path) -> Result<()> {
    let status = probe_root(root, None)?;
    if status.b_w2_h2_product_binding {
        return Ok(());
    }
    Err(format!(
        "w2-h2-product-binding Cap CLI is blocked_claim: {}; never invent pass_product_clean_room; never reburn capture observe_h2 as product-binding",
        status.reason_b
    )
    .into())
}

/// Fail-closed refuse for `--g4-cef-at` until Unlock A is measured true.
pub fn require_g4_cef_at_unlocked(root: &Path) -> Result<()> {
    let status = probe_root(root, None)?;
    if status.a_g4_cef_at {
        return Ok(());
    }
    Err(format!(
        "g4-cef-at Cap CLI is blocked_claim: {}; exact missing includes path release {} and/or env:{CEF_BUNDLE_ENV}; never invent pass_product_clean_room; never Cap-spam a11y/--require-full",
        status.reason_a,
        PATH_RELEASE_SURFACES.join(" and/or ")
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{UNLOCK_A_RECEIPT, UNLOCK_B_RECEIPT, UNLOCK_C_REQUIRED_PATHS, probe_root};
    use crate::orchestration::g4_cef_at::{
        CAP_SELECTOR as G4_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as G4_PILOT,
    };
    use crate::orchestration::h2_product_binding::{CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND};
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
        assert_eq!(status.w2_h2_cap_cli_status, "blocked_claim");
        assert_eq!(status.g4_cef_at_cap_cli_status, "blocked_claim");
        assert!(status.w2_h2_pilot_command.is_none());
        assert!(status.g4_cef_at_pilot_command.is_none());
        assert!(status.reason_a.contains("crates/browser/**"));
        assert!(status.reason_a.contains("HUMANYMOUS_CEF_PILOT_BUNDLE"));
        assert_eq!(
            status.to_json(&directory, None)["pass_product_clean_room"],
            0
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn unlock_a_receipt_with_cap_cli_pilot_binds_when_module_present() {
        let directory = tempfile_dir("hm-unlock-gate-a-");
        let receipt_path = directory.join(UNLOCK_A_RECEIPT);
        fs::create_dir_all(receipt_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&json!({
                "schema": "humanymous.cursor-unlock-a-applied/1",
                "unlock": "A",
                "applied": true,
                "pilot_command": G4_PILOT,
                "pass_product_clean_room": 0
            }))
            .expect("json"),
        )
        .expect("write");
        for relative in [
            "test/lab/runner/src/orchestration/g4_cef_at.rs",
            "crates/browser/native/desktop/pilot-contract.json",
        ] {
            let path = directory.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(&path, b"present\n").expect("write");
        }
        let status = probe_root(&directory, None).expect("probe");
        assert!(status.a_g4_cef_at);
        assert!(status.any);
        assert_eq!(status.g4_cef_at_cap_cli_status, "bound");
        assert_eq!(status.g4_cef_at_pilot_command.as_deref(), Some(G4_PILOT));
        assert_eq!(
            status.to_json(&directory, None)["g4_cef_at_cap_cli"]["selector"],
            G4_CAP_SELECTOR
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn unlock_a_receipt_without_cap_selector_stays_blocked() {
        let directory = tempfile_dir("hm-unlock-gate-a-bad-");
        let receipt_path = directory.join(UNLOCK_A_RECEIPT);
        fs::create_dir_all(receipt_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&json!({
                "schema": "humanymous.cursor-unlock-a-applied/1",
                "unlock": "A",
                "applied": true,
                "pilot_command": "cargo run --manifest-path scripts/ci/cef-pilot/Cargo.toml --locked",
                "pass_product_clean_room": 0
            }))
            .expect("json"),
        )
        .expect("write");
        let status = probe_root(&directory, None).expect("probe");
        assert!(!status.a_g4_cef_at);
        assert_eq!(status.g4_cef_at_cap_cli_status, "blocked_claim");
        assert!(status.reason_a.contains(G4_CAP_SELECTOR));
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn unlock_b_receipt_with_cap_cli_pilot_binds_when_destinations_present() {
        let directory = tempfile_dir("hm-unlock-gate-b-");
        let receipt_path = directory.join(UNLOCK_B_RECEIPT);
        fs::create_dir_all(receipt_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&json!({
                "schema": "humanymous.cursor-unlock-b-applied/1",
                "unlock": "B",
                "applied": true,
                "pilot_command": CODE_DEFINED_PILOT_COMMAND,
                "pass_product_clean_room": 0
            }))
            .expect("json"),
        )
        .expect("write");
        for relative in [
            "deployments/lab/gateway/h2-product-binding-contract.json",
            "deployments/lab/gateway/H2-PRODUCT-BINDING.md",
            "test/lab/runner/src/orchestration/h2_product_binding.rs",
        ] {
            let path = directory.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(&path, b"present\n").expect("write");
        }
        let status = probe_root(&directory, None).expect("probe");
        assert!(status.b_w2_h2_product_binding);
        assert!(status.any);
        assert_eq!(status.w2_h2_cap_cli_status, "bound");
        assert_eq!(
            status.w2_h2_pilot_command.as_deref(),
            Some(CODE_DEFINED_PILOT_COMMAND)
        );
        assert_eq!(
            status.to_json(&directory, None)["w2_h2_cap_cli"]["selector"],
            CAP_SELECTOR
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn unlock_b_receipt_without_cap_selector_stays_blocked() {
        let directory = tempfile_dir("hm-unlock-gate-b-bad-");
        let receipt_path = directory.join(UNLOCK_B_RECEIPT);
        fs::create_dir_all(receipt_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&json!({
                "schema": "humanymous.cursor-unlock-b-applied/1",
                "unlock": "B",
                "applied": true,
                "pilot_command": "root-named-hosted-gateway-h2-product-binding-pilot",
                "pass_product_clean_room": 0
            }))
            .expect("json"),
        )
        .expect("write");
        let status = probe_root(&directory, None).expect("probe");
        assert!(!status.b_w2_h2_product_binding);
        assert_eq!(status.w2_h2_cap_cli_status, "blocked_claim");
        assert!(status.reason_b.contains(CAP_SELECTOR));
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
