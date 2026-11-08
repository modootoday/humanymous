//! Unlock B Cap CLI — W2 H2 product-binding (fail-closed).
//!
//! Cap topology runs TLS listen+client measure (loopback under network_mode=none).
//! `original_h2_measured=true` only after a validated measure receipt.
//! Capture `observe_h2` is never product-binding. Never invent `pass_product_clean_room`
//! or claim `desktop_as_hosted` as hosted product.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
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

/// Measure receipt schema emitted by Cap oneshot.
pub const MEASURE_RECEIPT_SCHEMA: &str = "humanymous.lab-h2-product-binding-measure-receipt/1";

/// Residual Root join for N06 / product lift after Desktop lab measure.
pub const MISSING_N06_PRODUCT_LIFT: &[&str] = &[
    "Owner-integration readiness join authorization after measured binding (SoT N06 / R14-D6)",
    "Root-hosted venue qualification (Desktop lab measure ≠ desktop_as_hosted / hosted product)",
    "Matrix G1 / pass_product_clean_room admission by Root after measured binding",
];

/// Cap topology required before hosted-venue N06 Cap may Docker-coordinate.
/// Distinct from Desktop `compose.h2-product-binding.yaml` (loopback / network_mode=none).
pub const REQUIRED_HOSTED_VENUE_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.h2-hosted-venue.yaml",
    "build/rust/h2-hosted-venue-lab.Dockerfile",
];

/// Owner-join readiness receipt schema (never invents product; Cap-ready for Root).
pub const N06_OWNER_JOIN_RECEIPT_SCHEMA: &str =
    "humanymous.lab-h2-n06-owner-join-readiness/1";

/// Allowed hosted venue markers on an owner-join / hosted measure receipt.
pub const HOSTED_VENUE_MARKERS: &[&str] = &[
    "github-hosted-linux",
    "root-hosted-linux",
    "hosted_linux_specfold",
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

/// Parse Cap oneshot stdout for a measure receipt JSON object.
pub fn parse_measure_receipt(logs: &str) -> Result<Value> {
    let mut last_err = "measure receipt JSON absent from Cap logs".to_string();
    for line in logs.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => {
                if value.get("schema").and_then(Value::as_str) == Some(MEASURE_RECEIPT_SCHEMA) {
                    return Ok(value);
                }
                last_err = format!(" Cap log JSON schema mismatch: {trimmed}");
            }
            Err(error) => last_err = format!("Cap log JSON parse: {error}"),
        }
    }
    Err(last_err.into())
}

/// Fail-closed: `original_h2_measured` only from validated measure receipt.
pub fn original_h2_measured_from_receipt(receipt: &Value) -> Result<bool> {
    if receipt.get("schema").and_then(Value::as_str) != Some(MEASURE_RECEIPT_SCHEMA) {
        return Err("w2-h2-product-binding Cap refused: measure receipt schema mismatch".into());
    }
    if receipt.get("desktop_as_hosted").and_then(Value::as_bool) == Some(true) {
        return Err(
            "w2-h2-product-binding Cap refused: desktop_as_hosted=true forbidden; Desktop lab ≠ hosted product"
                .into(),
        );
    }
    if receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "w2-h2-product-binding Cap refused: measure receipt invented product/pass_product_clean_room"
                .into(),
        );
    }
    let measured = receipt
        .get("original_h2_measured")
        .and_then(Value::as_bool)
        .ok_or("w2-h2-product-binding Cap refused: measure receipt missing original_h2_measured")?;
    if measured {
        let status = receipt
            .get("tls_h2_status")
            .and_then(Value::as_u64)
            .ok_or("w2-h2-product-binding Cap refused: measured=true without tls_h2_status")?;
        if status == 422 {
            return Err(
                "w2-h2-product-binding Cap refused: measured=true but tls_h2_status=422"
                    .into(),
            );
        }
        let control = receipt
            .get("plaintext_control_status")
            .and_then(Value::as_u64)
            .ok_or(
                "w2-h2-product-binding Cap refused: measured=true without plaintext_control_status",
            )?;
        if control != 422 {
            return Err(
                "w2-h2-product-binding Cap refused: measured=true without plaintext 422 control"
                    .into(),
            );
        }
    }
    Ok(measured)
}

/// Fail-closed: refuse invented `original_h2_measured=true` without receipt.
pub fn refuse_invented_original_h2_measured(
    measured: bool,
    receipt: Option<&Value>,
) -> Result<()> {
    if !measured {
        return Ok(());
    }
    let Some(receipt) = receipt else {
        return Err(
            "w2-h2-product-binding Cap refused: original_h2_measured=true without measure receipt; composition Cap / capture observe_h2 never lift N06; never invent pass_product_clean_room"
                .into(),
        );
    };
    let validated = original_h2_measured_from_receipt(receipt)?;
    if !validated {
        return Err(
            "w2-h2-product-binding Cap refused: original_h2_measured=true but receipt validates false"
                .into(),
        );
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

/// Missing hosted-venue Cap topology paths (N06 owner join Cap-ready).
pub fn missing_hosted_venue_topology(root: &Path) -> Vec<&'static str> {
    REQUIRED_HOSTED_VENUE_TOPOLOGY
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

/// Fail-closed: Desktop lab measure never lifts N06 product / pass_product.
pub fn refuse_n06_product_lift_from_desktop_measure(receipt: &Value) -> Result<()> {
    let venue = receipt.get("venue").and_then(Value::as_str).unwrap_or("");
    if venue == "desktop_lab_loopback" || venue.is_empty() {
        if receipt.get("product").and_then(Value::as_u64) != Some(0)
            || receipt
                .get("pass_product_clean_room")
                .and_then(Value::as_u64)
                != Some(0)
        {
            return Err(
                "N06 owner-join refused: Desktop lab measure invented product/pass_product_clean_room; measured Desktop ≠ hosted product"
                    .into(),
            );
        }
        if receipt.get("desktop_as_hosted").and_then(Value::as_bool) == Some(true) {
            return Err(
                "N06 owner-join refused: desktop_as_hosted=true forbidden; Desktop lab measure ≠ hosted venue"
                    .into(),
            );
        }
        if receipt.get("n06_owner_join").and_then(Value::as_bool) == Some(true)
            || receipt.get("n06_product_lift").and_then(Value::as_bool) == Some(true)
        {
            return Err(
                "N06 owner-join refused: Desktop lab measure cannot authorize owner join / product lift; Root hosted venue still required"
                    .into(),
            );
        }
    }
    Ok(())
}

/// Fail-closed hosted-venue gate for N06 Cap day (topology + actual hosted Linux).
pub fn require_hosted_venue_for_n06_join(root: &Path) -> Result<()> {
    if !crate::docker::actual_hosted_linux() {
        return Err(
            "N06 hosted-venue Cap refused: actual_hosted_linux=false; Desktop Cap cannot qualify hosted venue; never invent desktop_as_hosted; Cap CLI refuse-closed before Docker"
                .into(),
        );
    }
    let missing = missing_hosted_venue_topology(root);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "N06 hosted-venue Cap refused: missing hosted Cap topology ({missing}); Cap-ready pack cites exact destinations; never invent pass_product_clean_room from Desktop measure",
        missing = missing.join(", ")
    )
    .into())
}

/// Build Cap-ready N06 owner-join readiness receipt after Desktop measure.
/// Always `n06_owner_join=false` / product=0 until Root hosted join.
pub fn n06_owner_join_readiness_receipt(
    measured: bool,
    desktop_measure_receipt: &Value,
) -> Result<Value> {
    refuse_n06_product_lift_from_desktop_measure(desktop_measure_receipt)?;
    let measured_ok = original_h2_measured_from_receipt(desktop_measure_receipt)?;
    if measured != measured_ok {
        return Err(
            "N06 owner-join readiness refused: measured flag disagrees with validated Desktop receipt"
                .into(),
        );
    }
    Ok(json!({
        "schema": N06_OWNER_JOIN_RECEIPT_SCHEMA,
        "original_h2_measured_desktop_lab": measured,
        "n06_owner_join": false,
        "n06_product_lift": false,
        "desktop_as_hosted": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
        "missing_hosted_venue_topology": REQUIRED_HOSTED_VENUE_TOPOLOGY,
        "hosted_venue_markers_accepted": HOSTED_VENUE_MARKERS,
        "note": "Desktop lab measure ≠ hosted product; N06 owner join still Root; Cap-ready hosted venue topology cited for Root transfer"
    }))
}

/// Validate a Root-authored N06 owner-join / hosted measure receipt (fail-closed).
pub fn validate_n06_owner_join_receipt(receipt: &Value) -> Result<()> {
    if receipt.get("schema").and_then(Value::as_str) != Some(N06_OWNER_JOIN_RECEIPT_SCHEMA)
        && receipt.get("schema").and_then(Value::as_str) != Some(MEASURE_RECEIPT_SCHEMA)
    {
        return Err("N06 owner-join refused: receipt schema mismatch".into());
    }
    if receipt.get("desktop_as_hosted").and_then(Value::as_bool) == Some(true) {
        return Err("N06 owner-join refused: desktop_as_hosted=true forbidden".into());
    }
    if receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "N06 owner-join refused: receipt invented product/pass_product_clean_room".into(),
        );
    }
    let venue = receipt.get("venue").and_then(Value::as_str).unwrap_or("");
    if receipt.get("n06_owner_join").and_then(Value::as_bool) == Some(true) {
        if !HOSTED_VENUE_MARKERS.contains(&venue) {
            return Err(format!(
                "N06 owner-join refused: n06_owner_join=true requires hosted venue marker ({allowed}); got {venue:?}; Desktop lab never qualifies",
                allowed = HOSTED_VENUE_MARKERS.join(", ")
            )
            .into());
        }
        if !crate::docker::actual_hosted_linux() {
            return Err(
                "N06 owner-join refused: n06_owner_join=true without actual_hosted_linux; never invent desktop_as_hosted"
                    .into(),
            );
        }
    }
    Ok(())
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
/// otherwise Docker-coordinate Cap measure (component only).
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
    eprintln!("lab: building immutable h2-product-binding-lab measure image (Cap)");
    lab.stage = "h2_product_binding_build";
    lab.compose(&["build", "h2-product-binding-pilot"], true)?;
    // Cap measure oneshot (exits 0 on measured receipt). Do not use compose --wait.
    lab.stage = "h2_product_binding_up";
    lab.compose(&["up", "-d", "h2-product-binding-pilot"], true)?;
    let container = lab.inspect("h2-product-binding-pilot")?;
    lab.case(
        "h2-product-binding-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("compose.h2-product-binding.yaml network_mode=none loopback measure"),
    )?;
    let id = container["Id"]
        .as_str()
        .ok_or("h2-product-binding pilot container id missing")?
        .to_owned();
    // Wait briefly if measure oneshot is still in flight (no second inspect case).
    let mut exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let mut running = container["State"]["Running"] == true;
    if running {
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let state: Value = serde_json::from_str(&lab.docker.call(&["inspect", &id])?)?;
            exit_code = state[0]["State"]["ExitCode"].as_i64().unwrap_or(-1);
            running = state[0]["State"]["Running"] == true;
            if !running {
                break;
            }
        }
    }
    lab.case(
        "h2-product-binding-pilot-measure-oneshot",
        !running && (exit_code == 0 || exit_code == 1),
        json!({"running": running, "exit_code": exit_code}),
    )?;
    let logs = lab.docker.call(&["logs", "--tail", "20", &id])?;
    snapshot::write(
        &lab.evidence,
        "h2-product-binding-measure-logs.json",
        &json!({"logs": logs}),
    )?;
    let receipt = parse_measure_receipt(&logs)?;
    snapshot::write(&lab.evidence, "original-h2-measure-receipt.json", &receipt)?;
    let measured = original_h2_measured_from_receipt(&receipt)?;
    refuse_invented_original_h2_measured(measured, Some(&receipt))?;
    refuse_n06_product_lift_from_desktop_measure(&receipt)?;
    let n06_readiness = n06_owner_join_readiness_receipt(measured, &receipt)?;
    validate_n06_owner_join_receipt(&n06_readiness)?;
    snapshot::write(
        &lab.evidence,
        "n06-owner-join-readiness.json",
        &n06_readiness,
    )?;
    lab.case(
        "h2-product-binding-measure-receipt-validated",
        true,
        json!({
            "original_h2_measured": measured,
            "tls_h2_status": receipt.get("tls_h2_status"),
            "plaintext_control_status": receipt.get("plaintext_control_status"),
            "desktop_as_hosted": false,
            "exit_code": exit_code
        }),
    )?;
    if measured {
        lab.case(
            "h2-product-binding-original-h2-measured",
            measured && exit_code == 0,
            json!({
                "original_h2_measured": measured,
                "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
                "reason": "Desktop lab TLS measure receipt validated; not hosted product"
            }),
        )?;
    } else {
        lab.case(
            "h2-product-binding-original-h2-not-measured",
            !measured,
            json!({
                "original_h2_measured": measured,
                "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
                "reason": "measure receipt present but original_h2_measured=false; capture observe_h2 ≠ product-binding"
            }),
        )?;
    }
    lab.case(
        "h2-product-binding-n06-owner-join-still-open",
        n06_readiness.get("n06_owner_join") == Some(&json!(false)),
        json!({
            "n06_owner_join": false,
            "missing_hosted_venue_topology": missing_hosted_venue_topology(&root),
            "reason": "measured Desktop lab ≠ hosted product; Root owner join still open"
        }),
    )?;
    lab.components["w2_h2_product_binding"] = json!({
        "status": "executed",
        "kind": "w2-h2-product-binding-component",
        "product": 0,
        "pass_product_clean_room": 0,
        "original_h2_measured": measured,
        "desktop_as_hosted": false,
        "n06_owner_join": false,
        "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
        "missing_hosted_venue_topology": missing_hosted_venue_topology(&root),
        "n06_join": if measured { "measured_desktop_lab_awaiting_owner_hosted_join" } else { "blocked" },
        "note": "component Cap TLS measure; never invent pass_product_clean_room; capture observe_h2 ≠ product-binding; Desktop ≠ hosted product; N06 owner join Cap-ready cites hosted venue topology"
    });
    snapshot::write(
        &lab.evidence,
        "w2-h2-product-binding-component.json",
        &lab.components["w2_h2_product_binding"],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND, HOSTED_VENUE_MARKERS, MEASURE_RECEIPT_SCHEMA,
        MISSING_N06_PRODUCT_LIFT, N06_OWNER_JOIN_RECEIPT_SCHEMA, REQUIRED_HOSTED_VENUE_TOPOLOGY,
        missing_cap_topology, missing_hosted_venue_topology, n06_owner_join_readiness_receipt,
        original_h2_measured_from_receipt, parse_measure_receipt,
        refuse_invented_original_h2_measured, refuse_n06_product_lift_from_desktop_measure,
        require_hosted_venue_for_n06_join, validate_n06_owner_join_receipt,
    };
    use crate::snapshot;
    use serde_json::json;
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
    fn hosted_venue_topology_still_missing_on_desktop() {
        let root = snapshot::root().expect("root");
        let missing = missing_hosted_venue_topology(&root);
        assert_eq!(missing, REQUIRED_HOSTED_VENUE_TOPOLOGY.to_vec());
        let err = require_hosted_venue_for_n06_join(&root).expect_err("desktop refuse");
        assert!(err.to_string().contains("actual_hosted_linux=false"));
    }

    #[test]
    fn desktop_measure_cannot_lift_n06_product() {
        let receipt = json!({
            "schema": MEASURE_RECEIPT_SCHEMA,
            "original_h2_measured": true,
            "tls_h2_status": 403,
            "plaintext_control_status": 422,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "venue": "desktop_lab_loopback"
        });
        assert!(refuse_n06_product_lift_from_desktop_measure(&receipt).is_ok());
        let readiness = n06_owner_join_readiness_receipt(true, &receipt).expect("readiness");
        assert_eq!(readiness["schema"], N06_OWNER_JOIN_RECEIPT_SCHEMA);
        assert_eq!(readiness["n06_owner_join"], false);
        assert_eq!(readiness["product"], 0);
        assert!(validate_n06_owner_join_receipt(&readiness).is_ok());

        let invented = json!({
            "schema": MEASURE_RECEIPT_SCHEMA,
            "original_h2_measured": true,
            "tls_h2_status": 403,
            "plaintext_control_status": 422,
            "product": 1,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "venue": "desktop_lab_loopback"
        });
        assert!(refuse_n06_product_lift_from_desktop_measure(&invented).is_err());

        let fake_join = json!({
            "schema": N06_OWNER_JOIN_RECEIPT_SCHEMA,
            "n06_owner_join": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "venue": "desktop_lab_loopback"
        });
        let err = validate_n06_owner_join_receipt(&fake_join).expect_err("desktop join");
        assert!(err.to_string().contains("hosted venue marker"));
        assert!(HOSTED_VENUE_MARKERS.contains(&"github-hosted-linux"));
    }

    #[test]
    fn measure_receipt_required_before_original_h2_measured() {
        assert!(refuse_invented_original_h2_measured(false, None).is_ok());
        let err = refuse_invented_original_h2_measured(true, None).expect_err("invented measured");
        assert!(err.to_string().contains("without measure receipt"));
        let receipt = json!({
            "schema": MEASURE_RECEIPT_SCHEMA,
            "original_h2_measured": true,
            "tls_h2_status": 403,
            "plaintext_control_status": 422,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false
        });
        assert!(original_h2_measured_from_receipt(&receipt).unwrap());
        assert!(refuse_invented_original_h2_measured(true, Some(&receipt)).is_ok());
        let bad = json!({
            "schema": MEASURE_RECEIPT_SCHEMA,
            "original_h2_measured": true,
            "tls_h2_status": 422,
            "plaintext_control_status": 422,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false
        });
        assert!(original_h2_measured_from_receipt(&bad).is_err());
        assert!(MISSING_N06_PRODUCT_LIFT.len() >= 2);
    }

    #[test]
    fn parse_measure_receipt_takes_last_matching_schema() {
        let logs = "noise\n{\"schema\":\"other\"}\n{\"schema\":\"humanymous.lab-h2-product-binding-measure-receipt/1\",\"original_h2_measured\":false,\"product\":0,\"pass_product_clean_room\":0,\"desktop_as_hosted\":false}\n";
        let receipt = parse_measure_receipt(logs).expect("receipt");
        assert_eq!(receipt["original_h2_measured"], false);
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
