//! Unlock A Cap CLI — G4 CEF/AT (fail-closed).
//!
//! Cap selector mirrors Unlock B/C. Cap burn remains refuse-closed until
//! Root path release + transfer-apply writes `UNLOCK-A-APPLIED` with the
//! code-defined pilot, and Cap day still refuses when
//! `HUMANYMOUS_CEF_PILOT_BUNDLE` (or durable Cap-day bind) is absent. Never
//! invents `pass_product_clean_room` or Cap-spams a11y/`--require-full`.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::json;
use std::{fs, path::Path};

/// Code-defined Cap selector (mirrors Unlock B `--w2-h2-product-binding`).
pub const CAP_SELECTOR: &str = "--g4-cef-at";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g4-cef-at";

/// Private CEF bundle env required for process cases (fail-closed when unset).
pub const CEF_BUNDLE_ENV: &str = "HUMANYMOUS_CEF_PILOT_BUNDLE";

/// Cap module + freeze surface required after Unlock A Cap CLI lands.
pub const REQUIRED_CAP_MODULE: &[&str] = &[
    "test/lab/runner/src/orchestration/g4_cef_at.rs",
    "crates/browser/native/desktop/pilot-contract.json",
];

/// Exact Root path-release surfaces Cap cites when Unlock A is still scaffold.
pub const PATH_RELEASE_SURFACES: &[&str] = &["crates/browser/**", "scripts/ci/cef-pilot/**"];

/// Freeze pin from Cap-ready g4 harness — abort Cap on drift.
const FREEZE_PREIMAGES: &[(&str, &str)] = &[(
    "crates/browser/native/desktop/pilot-contract.json",
    "eac4fab4466ae524a84dc3f33ffe70e6ab7537025bba9e7d94a93f052aa8f85a",
)];

/// Missing Cap module / freeze surfaces.
pub fn missing_cap_module(root: &Path) -> Vec<&'static str> {
    REQUIRED_CAP_MODULE
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect()
}

/// Exact missing Cap prerequisites cited by refuse-closed Cap CLI.
pub fn missing_cap_prerequisites(root: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    for relative in missing_cap_module(root) {
        missing.push(relative.to_string());
    }
    let cap_day = crate::cef_bundle::cap_day_present(root).unwrap_or(false);
    if !cap_day {
        missing.push(format!(
            "env:{CEF_BUNDLE_ENV}|durable:{}",
            crate::cef_bundle::CAP_DAY_BIND_RELATIVE
        ));
    }
    missing
}

/// Fail-closed Cap module gate — cite missing paths; no CEF burn.
pub fn require_cap_module(root: &Path) -> Result<()> {
    let missing = missing_cap_module(root);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "g4-cef-at Cap refused: missing Cap module/freeze surfaces ({missing}); Cap CLI refuse-closed before CEF; never invent pass_product_clean_room",
        missing = missing.join(", ")
    )
    .into())
}

/// Fail-closed CEF bundle gate — cite exact env / durable Cap-day bind; no CEF burn.
pub fn require_cef_bundle() -> Result<()> {
    let root = snapshot::root()?;
    if crate::cef_bundle::cap_day_present(&root)? {
        return Ok(());
    }
    Err(format!(
        "g4-cef-at Cap refused: missing CEF bundle env {CEF_BUNDLE_ENV} (or durable Cap-day bind {}); Cap CLI refuse-closed before CEF process pilot; path release still requires crates/browser/** and/or scripts/ci/cef-pilot/**; never invent pass_product_clean_room",
        crate::cef_bundle::CAP_DAY_BIND_RELATIVE
    )
    .into())
}

/// Verify Cap-ready freeze pins match live workspace bytes.
pub fn require_freeze_preimages(root: &Path) -> Result<()> {
    for (relative, expected) in FREEZE_PREIMAGES {
        let path = root.join(relative);
        if !path.is_file() {
            return Err(
                format!("g4-cef-at Cap refused: freeze surface missing ({relative})").into(),
            );
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("g4-cef-at freeze read {relative}: {error}"))?;
        let actual = snapshot::digest(&bytes);
        if actual != *expected {
            return Err(format!(
                "g4-cef-at Cap refused: freeze preimage drift for {relative}; abort Cap without Root delta (expected {expected}, got {actual})"
            )
            .into());
        }
    }
    Ok(())
}

/// Cap=1 entry: refuse-closed when unlock/module/bundle/freeze prerequisites fail.
///
/// Cap day CEF process execution remains Root-gated; this module never invents
/// product. When prerequisites hold, Cap records a component-only staged
/// receipt and still refuses interactive AT product claims.
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    require_freeze_preimages(&root)?;
    if let Err(error) = require_cap_module(&root) {
        lab.components["g4_cef_at"] = json!({
            "status": "refused",
            "kind": "g4-cef-at-component",
            "product": 0,
            "pass_product_clean_room": 0,
            "missing_cap_module": missing_cap_module(&root),
            "path_release_surfaces": PATH_RELEASE_SURFACES,
            "note": "component refuse only; never invent BrowserReady or pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "g4-cef-at-refuse.json",
            &lab.components["g4_cef_at"],
        )?;
        return Err(error);
    }
    if let Err(error) = require_cef_bundle() {
        lab.components["g4_cef_at"] = json!({
            "status": "refused",
            "kind": "g4-cef-at-component",
            "product": 0,
            "pass_product_clean_room": 0,
            "missing": missing_cap_prerequisites(&root),
            "path_release_surfaces": PATH_RELEASE_SURFACES,
            "note": "CEF bundle absent; Cap CLI refuse-closed; never Cap-spam a11y/--require-full; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "g4-cef-at-refuse.json",
            &lab.components["g4_cef_at"],
        )?;
        return Err(error);
    }

    // Cap burn after Unlock A + bundle is component-only staged receipt.
    // Interactive CEF/IME/AT process pilots remain Root-named; do not invent.
    lab.stage = "g4_cef_at_component";
    lab.components["g4_cef_at"] = json!({
        "status": "executed",
        "kind": "g4-cef-at-component",
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "cef_bundle_env": CEF_BUNDLE_ENV,
        "note": "component Cap surface only after Unlock A + CEF bundle; interactive AT product still Root-gated; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "g4-cef-at-component.json",
        &lab.components["g4_cef_at"],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CAP_SELECTOR, CEF_BUNDLE_ENV, CODE_DEFINED_PILOT_COMMAND, missing_cap_module,
        missing_cap_prerequisites, require_cef_bundle,
    };
    use crate::snapshot;
    use std::fs;

    #[test]
    fn code_defined_pilot_embeds_cap_selector() {
        assert!(CODE_DEFINED_PILOT_COMMAND.contains(CAP_SELECTOR));
        assert!(CODE_DEFINED_PILOT_COMMAND.contains("humanymous-lab-runner"));
    }

    #[test]
    fn missing_module_cites_cap_and_freeze_surfaces() {
        let directory = tempfile_dir("hm-g4-cef-");
        let missing = missing_cap_module(&directory);
        assert!(missing.contains(&"test/lab/runner/src/orchestration/g4_cef_at.rs"));
        assert!(missing.contains(&"crates/browser/native/desktop/pilot-contract.json"));
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn cef_bundle_refuse_message_cites_exact_env_and_paths() {
        // Do not mutate process env (forbid unsafe-code). When Cap-day presence
        // is absent Cap refuses with exact env + path-release surfaces; when
        // present via env or durable bind, require_cef_bundle succeeds.
        match require_cef_bundle() {
            Ok(()) => {
                let root = snapshot::root().expect("root");
                assert!(
                    crate::cef_bundle::cap_day_present(&root).expect("cap_day"),
                    "require_cef_bundle Ok requires Cap-day presence"
                );
            }
            Err(error) => {
                let text = error.to_string();
                assert!(text.contains(CEF_BUNDLE_ENV), "{text}");
                assert!(text.contains("crates/browser/**"), "{text}");
                assert!(text.contains("scripts/ci/cef-pilot/**"), "{text}");
            }
        }
    }

    #[test]
    fn live_workspace_cap_module_present() {
        let root = snapshot::root().expect("root");
        let missing = missing_cap_module(&root);
        assert!(
            missing.is_empty(),
            "Cap module must be present after Unlock A Cap CLI land; missing={missing:?}"
        );
        let missing_prereq = missing_cap_prerequisites(&root);
        let cap_day = crate::cef_bundle::cap_day_present(&root).expect("cap_day");
        if !cap_day {
            assert!(
                missing_prereq
                    .iter()
                    .any(|item| item.contains(CEF_BUNDLE_ENV)),
                "missing_prereq={missing_prereq:?}"
            );
        } else {
            assert!(
                !missing_prereq
                    .iter()
                    .any(|item| item.contains(CEF_BUNDLE_ENV)),
                "missing_prereq={missing_prereq:?}"
            );
        }
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
