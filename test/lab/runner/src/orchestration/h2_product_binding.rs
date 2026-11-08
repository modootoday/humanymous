//! Unlock B Cap CLI — W2 H2 product-binding (fail-closed).
//!
//! Cap topology (compose + Dockerfile for gateway serve + `required.original_h2`)
//! is Cap-ready under claimed Cursor surface after Unlock B topology transfer.
//! Composition Cap with `network_mode=none` never measures hosted original_h2.
//! Capture `observe_h2` is never product-binding. Never invent `pass_product_clean_room`.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::json;
use std::{fs, path::Path};

/// Code-defined Cap selector (mirrors Unlock C `--authority-node-ipc`).
pub const CAP_SELECTOR: &str = "--w2-h2-product-binding";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding";

/// Contract destination written by Unlock B transfer-apply.
pub const CONTRACT_REL: &str = "deployments/lab/gateway/h2-product-binding-contract.json";

/// Cap topology required before Cap can Docker-coordinate product-binding composition.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.h2-product-binding.yaml",
    "build/rust/h2-product-binding-lab.Dockerfile",
];

/// Exact Root-hosted join still missing for `original_h2_measured=true` (N06).
///
/// Composition Cap EXIT 0 with `network_mode=none` does **not** satisfy these.
/// Desktop Cap must not be treated as hosted (`desktop_as_hosted` forbidden).
pub const MISSING_ORIGINAL_H2_MEASURE_JOIN: &[&str] = &[
    "Root-allocated hosted gateway H2 product-binding measure topology (TLS listen + client path; not composition network_mode=none alone)",
    "Cap pilot that serves humanymous-runtime gateway with route required.original_h2=true",
    "Same-owner live Connection + successful observe_h2 for that request (not capture native-component observe_h2 folklore)",
    "Owner-integration readiness join authorization after measured binding (SoT N06 / R14-D6)",
    "Honest measure receipt artifact Cap may consume before flipping original_h2_measured",
];

/// Freeze pins from Cap-ready contract — abort Cap on drift.
const FREEZE_PREIMAGES: &[(&str, &str)] = &[
    (
        "crates/sdk/humanymous-runtime/src/gateway/tls/h2.rs",
        "90a44014ac814c01e7e37194b3a903ddee8cf030d6fb8855b69b14f851199843",
    ),
    (
        "crates/sdk/humanymous-runtime/src/gateway/tls.rs",
        "7008fd3e94173c1fbd9e2d23d72635f10f15a7f623fb34625c10b67148390365",
    ),
    (
        "crates/sdk/humanymous-runtime/src/gateway/application/tls.rs",
        "2db651b3eea25ccf8f3eb917369738ee0f3e8658712787bb1c8f80e7621bb03f",
    ),
    (
        "crates/sdk/humanymous-runtime/src/gateway/application.rs",
        "1b8e741bc18e2aeccd6b87464a9f93af63c9b260b34dd286ea23477a228e6d42",
    ),
];

/// Composition Cap cannot claim measured original_h2; cite exact missing join.
pub fn original_h2_measured_from_composition_cap() -> bool {
    false
}

/// Fail-closed: refuse invented `original_h2_measured=true` without hosted join.
pub fn refuse_invented_original_h2_measured(measured: bool) -> Result<()> {
    if measured {
        return Err(format!(
            "w2-h2-product-binding Cap refused: original_h2_measured=true without Root-hosted measure join ({missing}); composition Cap / capture observe_h2 never lift N06; never invent pass_product_clean_room",
            missing = MISSING_ORIGINAL_H2_MEASURE_JOIN.join("; ")
        )
        .into());
    }
    Ok(())
}
/// Missing Cap topology paths.
pub fn missing_cap_topology(root: &Path) -> Vec<&'static str> {
    REQUIRED_CAP_TOPOLOGY
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect()
}

/// Fail-closed Cap topology gate — cite missing paths; no Docker burn.
pub fn require_cap_topology(root: &Path) -> Result<()> {
    let missing = missing_cap_topology(root);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "w2-h2-product-binding Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; gateway freeze pins are observation surfaces not Cap pilot topology; never invent pass_product_clean_room from capture observe_h2",
        missing = missing.join(", ")
    )
    .into())
}

/// Verify Cap-ready freeze pins match live workspace bytes.
pub fn require_freeze_preimages(root: &Path) -> Result<()> {
    for (relative, expected) in FREEZE_PREIMAGES {
        let path = root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "w2-h2-product-binding Cap refused: freeze surface missing ({relative})"
            )
            .into());
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("w2-h2-product-binding freeze read {relative}: {error}"))?;
        let actual = snapshot::digest(&bytes);
        if actual != *expected {
            return Err(format!(
                "w2-h2-product-binding Cap refused: freeze preimage drift for {relative}; abort Cap without Root delta (expected {expected}, got {actual})"
            )
            .into());
        }
    }
    Ok(())
}

/// Cap=1 entry: refuse-closed when Cap topology / freeze prerequisites fail;
/// otherwise Docker-coordinate Cap composition (component only).
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    require_freeze_preimages(&root)?;
    if let Err(error) = require_cap_topology(&root) {
        lab.components["w2_h2_product_binding"] = json!({
            "status": "refused",
            "kind": "w2-h2-product-binding-component",
            "product": 0,
            "pass_product_clean_room": 0,
            "missing_cap_topology": missing_cap_topology(&root),
            "contract": CONTRACT_REL,
            "note": "component refuse only; capture observe_h2 ≠ product-binding; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "w2-h2-product-binding-refuse.json",
            &lab.components["w2_h2_product_binding"],
        )?;
        return Err(error);
    }

    lab.stage = "h2_product_binding_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: serde_json::Value = serde_json::from_str(&config)?;
    snapshot::write(&lab.evidence, "resolved-compose.json", &parsed)?;
    lab.case(
        "h2-product-binding-no-published-ports",
        parsed["services"]
            .as_object()
            .ok_or("services absent")?
            .values()
            .all(|s| s.get("ports").is_none()),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable h2-product-binding-lab image (Cap)");
    lab.stage = "h2_product_binding_build";
    lab.compose(&["build", "h2-product-binding-pilot"], true)?;
    // Cap oneshot (exits 0). Do not use compose --wait.
    lab.stage = "h2_product_binding_up";
    lab.compose(&["up", "-d", "h2-product-binding-pilot"], true)?;
    let container = lab.inspect("h2-product-binding-pilot")?;
    lab.case(
        "h2-product-binding-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("compose.h2-product-binding.yaml network_mode=none"),
    )?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "h2-product-binding-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    // Composition Cap honesty: network_mode=none oneshot never measures hosted H2.
    let measured = original_h2_measured_from_composition_cap();
    refuse_invented_original_h2_measured(measured)?;
    lab.case(
        "h2-product-binding-original-h2-not-measured",
        !measured,
        json!({
            "original_h2_measured": measured,
            "missing_measure_join": MISSING_ORIGINAL_H2_MEASURE_JOIN,
            "reason": "composition Cap network_mode=none; Root-hosted measure join absent; capture observe_h2 ≠ product-binding"
        }),
    )?;
    lab.components["w2_h2_product_binding"] = json!({
        "status": "executed",
        "kind": "w2-h2-product-binding-component",
        "product": 0,
        "pass_product_clean_room": 0,
        "original_h2_measured": measured,
        "missing_measure_join": MISSING_ORIGINAL_H2_MEASURE_JOIN,
        "n06_join": "blocked",
        "note": "component Cap composition only; original_h2_measured stays false until Root-hosted measure join; never invent readiness original_h2 or pass_product_clean_room; capture observe_h2 ≠ product-binding"
    });
    snapshot::write(
        &lab.evidence,
        "w2-h2-product-binding-component.json",
        &lab.components["w2_h2_product_binding"],
    )?;
    snapshot::write(
        &lab.evidence,
        "original-h2-measure-join-missing.json",
        &json!({
            "schema": "humanymous.lab-h2-original-h2-measure-join/1",
            "original_h2_measured": measured,
            "missing_measure_join": MISSING_ORIGINAL_H2_MEASURE_JOIN,
            "composition_cap_network_mode": "none",
            "desktop_as_hosted": "forbidden",
            "product": 0,
            "pass_product_clean_room": 0,
            "n06_join": "blocked"
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND, MISSING_ORIGINAL_H2_MEASURE_JOIN,
        missing_cap_topology, original_h2_measured_from_composition_cap,
        refuse_invented_original_h2_measured,
    };
    use crate::snapshot;
    use std::fs;

    #[test]
    fn code_defined_pilot_embeds_cap_selector() {
        assert!(CODE_DEFINED_PILOT_COMMAND.contains(CAP_SELECTOR));
        assert!(CODE_DEFINED_PILOT_COMMAND.contains("humanymous-lab-runner"));
    }

    #[test]
    fn missing_topology_cites_compose_and_dockerfile() {
        let directory = tempfile_dir("hm-h2-binding-");
        let missing = missing_cap_topology(&directory);
        assert!(missing.contains(&"deployments/lab/compose.h2-product-binding.yaml"));
        assert!(missing.contains(&"build/rust/h2-product-binding-lab.Dockerfile"));
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn live_workspace_cap_topology_present_after_unlock_b() {
        let root = snapshot::root().expect("root");
        let missing = missing_cap_topology(&root);
        assert!(
            missing.is_empty(),
            "Cap topology must be present after Unlock B topology transfer; missing={missing:?}"
        );
    }

    #[test]
    fn composition_cap_never_claims_original_h2_measured() {
        assert!(!original_h2_measured_from_composition_cap());
        assert!(refuse_invented_original_h2_measured(false).is_ok());
        let err = refuse_invented_original_h2_measured(true).expect_err("invented measured");
        let message = err.to_string();
        assert!(message.contains("original_h2_measured=true"));
        assert!(message.contains("Root-hosted"));
        assert!(MISSING_ORIGINAL_H2_MEASURE_JOIN.len() >= 4);
        assert!(
            MISSING_ORIGINAL_H2_MEASURE_JOIN
                .iter()
                .any(|item| item.contains("required.original_h2=true"))
        );
        assert!(
            MISSING_ORIGINAL_H2_MEASURE_JOIN
                .iter()
                .any(|item| item.contains("observe_h2"))
        );
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
