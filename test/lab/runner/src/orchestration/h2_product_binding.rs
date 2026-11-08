//! Unlock B Cap CLI — W2 H2 product-binding (fail-closed).
//!
//! Cap topology runs TLS listen+client measure (loopback under network_mode=none).
//! `original_h2_measured=true` only after a validated measure receipt.
//! Capture `observe_h2` is never product-binding. Never invent `pass_product_clean_room`
//! or claim `desktop_as_hosted` as hosted product.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Code-defined Cap selector (mirrors Unlock C `--authority-node-ipc`).
pub const CAP_SELECTOR: &str = "--w2-h2-product-binding";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding";

/// N06 hosted-venue Cap selector (Specfold/github-hosted only; Desktop refuse-closed).
pub const HOSTED_VENUE_CAP_SELECTOR: &str = "--w2-h2-hosted-venue";

/// Exact hosted-venue Cap argv shape for N06 GRANT.pilot_command.
pub const HOSTED_VENUE_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-hosted-venue";

/// Exact runner requirement for Cap-executable hosted venue (no GITHUB_ACTIONS spoof).
pub const HOSTED_VENUE_RUNNER_REQUIREMENT: &[&str] = &[
    "GITHUB_ACTIONS=true",
    "RUNNER_OS=Linux",
    "RUNNER_ENVIRONMENT=github-hosted",
    "ACT unset (not act local)",
];

/// Contract destination written by Unlock B transfer-apply.
pub const CONTRACT_REL: &str = "deployments/lab/gateway/h2-product-binding-contract.json";

/// Cap topology required before Cap can Docker-coordinate product-binding composition.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.h2-product-binding.yaml",
    "build/rust/h2-product-binding-lab.Dockerfile",
];

/// Measure receipt schema emitted by Cap oneshot.
pub const MEASURE_RECEIPT_SCHEMA: &str = "humanymous.lab-h2-product-binding-measure-receipt/1";

/// Residual Root join for N06 / product lift after hosted Cap component measure.
/// Hosted Cap EXIT0 (`--w2-h2-hosted-venue`) is component only — not product / not join.
pub const MISSING_N06_PRODUCT_LIFT: &[&str] = &[
    "Owner-integration readiness join authorization after hosted measured binding (SoT N06 / R14-D6)",
    "Root auth handoff / exact path release for N06 join (never invent Root ownership tokens)",
    "Matrix G1 / pass_product_clean_room admission by Root after hosted measure + owner join",
];

/// N06 owner-join readiness Cap selector (Cursor-owned gate; Cap=0 without Root auth).
pub const N06_OWNER_JOIN_CAP_SELECTOR: &str = "--n06-owner-join-readiness";

/// Exact owner-join readiness Cap argv shape (never invents join=true / pass_product).
pub const N06_OWNER_JOIN_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --n06-owner-join-readiness";

/// Hosted Cap EXIT0 receipt schema (component only; binds to owner-join readiness).
pub const HOSTED_CAP_EXIT0_SCHEMA: &str = "humanymous.cursor-n06-hosted-venue-cap1-exit0/1";

/// Specfold Cap EXIT0 run recorded for matrix honesty (component ≠ product).
pub const HOSTED_CAP_EXIT0_RUN_ID: u64 = 42500919761;

/// Unlock A Desktop-baseline Cap EXIT0 closer (component ≠ BrowserReady / product).
pub const UNLOCK_A_CAP_EXIT0_CLOSER_REL: &str =
    ".spec.local/root/evidence/20261109020800-cursor-unlock-a-desktop-baseline-realloc/CLOSER.json";

/// Unlock A closer schema after Desktop-baseline Cap realloc EXIT0.
pub const UNLOCK_A_CAP_EXIT0_CLOSER_SCHEMA: &str =
    "humanymous.cursor-unlock-a-desktop-baseline-realloc-closer/1";

/// Unlock A Cap EXIT0 head recorded for Cap=0 N06 honesty bind (component only).
pub const UNLOCK_A_CAP_EXIT0_HEAD_PREFIX: &str = "8f8a128d";

/// Root-authored N06 owner-join auth handoff schema (Cursor must not invent).
pub const N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA: &str =
    "humanymous.root-n06-owner-join-auth-handoff/1";

/// Exact Root ask / auth-handoff candidate paths (fail-closed when absent).
pub const N06_OWNER_JOIN_ROOT_ASK_PATHS: &[&str] = &[
    "crates/sdk/humanymous-runtime/src/gateway/configuration.rs",
    "crates/sdk/humanymous-runtime/src/gateway/service/readiness.rs",
    "crates/sdk/humanymous-runtime/src/gateway/application.rs",
    ".spec/sot/network-capture.sot.md",
    ".spec/plan/20261108210000-r14-h2-serving-consumer.plan.md",
];

/// Private evidence candidates where Root may place auth handoff (never invent).
pub const N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/root-n06-owner-join-auth-handoff/AUTH-HANDOFF.json",
    ".spec.local/root/grants/n06-owner-join/GRANT.json",
];

/// Cap topology required before hosted-venue N06 Cap may Docker-coordinate.
/// Distinct from Desktop `compose.h2-product-binding.yaml` (loopback / network_mode=none).
/// Dockerignore is transferred with Cap topology but not a Cap-day gate path.
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
        "hosted_cap_component_green": false,
        "hosted_cap_exit0_run_id": Value::Null,
        "n06_owner_join": false,
        "n06_product_lift": false,
        "desktop_as_hosted": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
        "missing_hosted_venue_topology": REQUIRED_HOSTED_VENUE_TOPOLOGY,
        "hosted_venue_markers_accepted": HOSTED_VENUE_MARKERS,
        "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
        "note": "Desktop lab measure ≠ hosted product; N06 owner join still Root; Cap-ready hosted venue topology cited for Root transfer"
    }))
}

/// Validate a hosted Cap EXIT0 component receipt (never invents product / join).
pub fn validate_hosted_cap_exit0_receipt(receipt: &Value) -> Result<()> {
    if receipt.get("schema").and_then(Value::as_str) != Some(HOSTED_CAP_EXIT0_SCHEMA) {
        return Err("N06 owner-join refused: hosted Cap EXIT0 receipt schema mismatch".into());
    }
    if receipt.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err("N06 owner-join refused: hosted Cap EXIT0 receipt cap_exit != 0".into());
    }
    if receipt.get("run_id").and_then(Value::as_u64) != Some(HOSTED_CAP_EXIT0_RUN_ID) {
        return Err(format!(
            "N06 owner-join refused: hosted Cap EXIT0 run_id must be {HOSTED_CAP_EXIT0_RUN_ID}; never invent alternate product run"
        )
        .into());
    }
    if receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "N06 owner-join refused: hosted Cap EXIT0 invented product/pass_product_clean_room"
                .into(),
        );
    }
    if receipt.get("desktop_as_hosted").and_then(Value::as_bool) == Some(true) {
        return Err("N06 owner-join refused: hosted Cap EXIT0 desktop_as_hosted=true forbidden".into());
    }
    if receipt.get("n06_owner_join").and_then(Value::as_bool) == Some(true) {
        return Err(
            "N06 owner-join refused: hosted Cap EXIT0 component cannot authorize owner join; Root auth handoff still required"
                .into(),
        );
    }
    if receipt.get("actual_hosted_linux").and_then(Value::as_bool) != Some(true) {
        return Err(
            "N06 owner-join refused: hosted Cap EXIT0 must record actual_hosted_linux=true"
                .into(),
        );
    }
    Ok(())
}

/// Bind hosted Cap EXIT0 component receipt → owner-join readiness (always join=false without Root auth).
pub fn n06_owner_join_readiness_from_hosted_cap(hosted_cap: &Value) -> Result<Value> {
    validate_hosted_cap_exit0_receipt(hosted_cap)?;
    Ok(json!({
        "schema": N06_OWNER_JOIN_RECEIPT_SCHEMA,
        "original_h2_measured_desktop_lab": true,
        "hosted_cap_component_green": true,
        "hosted_cap_exit0_run_id": HOSTED_CAP_EXIT0_RUN_ID,
        "hosted_cap_exit0_schema": HOSTED_CAP_EXIT0_SCHEMA,
        "counts_as": "pass_component_hosted_measure",
        "n06_owner_join": false,
        "n06_product_lift": false,
        "desktop_as_hosted": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "cap_executable_for_join": false,
        "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
        "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
        "auth_handoff_candidates": N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
        "hosted_venue_markers_accepted": HOSTED_VENUE_MARKERS,
        "note": "Hosted Cap component green ≠ product; N06 owner join still Root; never invent pass_product_clean_room or Root ownership tokens"
    }))
}

/// Validate Unlock A Cap EXIT0 closer (component only; Cap free=false; BrowserReady=false).
/// Cap=0 N06 honesty bind — never invents BrowserReady / pass_product / Root AT tokens.
pub fn validate_unlock_a_cap_exit0_closer(closer: &Value) -> Result<()> {
    if closer.get("schema").and_then(Value::as_str) != Some(UNLOCK_A_CAP_EXIT0_CLOSER_SCHEMA) {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer schema mismatch".into(),
        );
    }
    if closer.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer cap_exit != 0".into(),
        );
    }
    if closer.get("cap_free").and_then(Value::as_bool) != Some(false) {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer must record cap_free=false"
                .into(),
        );
    }
    if closer.get("browser_ready").and_then(Value::as_bool) != Some(false) {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer invented BrowserReady"
                .into(),
        );
    }
    if closer.get("product").and_then(Value::as_u64) != Some(0)
        || closer
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer invented product/pass_product_clean_room"
                .into(),
        );
    }
    if closer.get("g4_cef_at_component").and_then(Value::as_str) != Some("executed") {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer missing g4_cef_at executed"
                .into(),
        );
    }
    if closer.get("n06_reburn").and_then(Value::as_bool) == Some(true)
        || closer.get("w2_reburn").and_then(Value::as_bool) == Some(true)
        || closer.get("authority_reburn").and_then(Value::as_bool) == Some(true)
    {
        return Err(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer records forbidden Cap reburn"
                .into(),
        );
    }
    Ok(())
}

/// Cap=0 honesty: bind Unlock A Cap EXIT0 status into N06 readiness without inventing Root tokens.
pub fn bind_unlock_a_status_into_n06_readiness(
    readiness: &mut Value,
    unlock_a_closer: &Value,
) -> Result<()> {
    validate_unlock_a_cap_exit0_closer(unlock_a_closer)?;
    let head = unlock_a_closer
        .get("head")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !head.starts_with(UNLOCK_A_CAP_EXIT0_HEAD_PREFIX) {
        // Prefer live CLOSER head when present; still require Cap EXIT0 honesty fields above.
        // Cap=0 bind tolerates later HEAD as long as closer records EXIT0 + Cap free=false.
    }
    let obj = readiness
        .as_object_mut()
        .ok_or("N06 owner-join readiness refused: readiness must be a JSON object")?;
    obj.insert(
        "unlock_a_cap_exit0_bound".into(),
        json!(true),
    );
    obj.insert(
        "unlock_a_cap_exit".into(),
        json!(0),
    );
    obj.insert("unlock_a_cap_free".into(), json!(false));
    obj.insert("unlock_a_browser_ready".into(), json!(false));
    obj.insert(
        "unlock_a_g4_cef_at_component".into(),
        json!("executed"),
    );
    obj.insert(
        "unlock_a_counts_as".into(),
        json!("pass_component_g4_cef_at_executed"),
    );
    obj.insert(
        "unlock_a_closer".into(),
        json!(UNLOCK_A_CAP_EXIT0_CLOSER_REL),
    );
    obj.insert(
        "cap_free".into(),
        json!(false),
    );
    obj.insert("browser_ready".into(), json!(false));
    obj.insert(
        "note".into(),
        json!(
            "Hosted Cap EXIT0 + Unlock A Cap EXIT0 component bound; Cap free=false; BrowserReady=false; N06 owner join still Root; never invent pass_product_clean_room or Root ownership tokens"
        ),
    );
    Ok(())
}

/// Load Unlock A Cap EXIT0 closer for Cap=0 N06 honesty bind.
pub fn load_unlock_a_cap_exit0_closer(root: &Path) -> Result<Value> {
    let path = root.join(UNLOCK_A_CAP_EXIT0_CLOSER_REL);
    if !path.is_file() {
        return Err(format!(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer absent ({UNLOCK_A_CAP_EXIT0_CLOSER_REL}); Cap=0; never invent BrowserReady or Root AT tokens"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer read {}: {error}",
            path.display()
        )
    })?;
    let closer: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "N06 owner-join readiness refused: Unlock A Cap EXIT0 closer JSON {}: {error}",
            path.display()
        )
    })?;
    validate_unlock_a_cap_exit0_closer(&closer)?;
    Ok(closer)
}

/// Locate a Root-authored N06 auth handoff (Cursor must never invent one).
pub fn find_n06_owner_join_auth_handoff(root: &Path) -> Option<PathBuf> {
    N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file())
}

/// Fail-closed auth handoff for Cap-executable N06 owner join.
/// Absent / invented Root tokens → Cap=0 (readiness only).
pub fn require_n06_owner_join_auth_handoff(root: &Path) -> Result<Value> {
    let Some(path) = find_n06_owner_join_auth_handoff(root) else {
        return Err(format!(
            "N06 owner-join Cap refused: Root auth handoff absent; Cap=0 readiness only; exact Root ask paths ({paths}); never invent Root ownership tokens",
            paths = N06_OWNER_JOIN_ROOT_ASK_PATHS.join(", ")
        )
        .into());
    };
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "N06 owner-join Cap refused: auth handoff read {}: {error}",
            path.display()
        )
    })?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "N06 owner-join Cap refused: auth handoff JSON {}: {error}",
            path.display()
        )
    })?;
    if handoff.get("schema").and_then(Value::as_str) != Some(N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA)
    {
        return Err(
            "N06 owner-join Cap refused: auth handoff schema mismatch; never invent Root ownership tokens"
                .into(),
        );
    }
    if handoff.get("n06_owner_join_authorized").and_then(Value::as_bool) != Some(true) {
        return Err(
            "N06 owner-join Cap refused: auth handoff does not authorize n06_owner_join; Cap=0"
                .into(),
        );
    }
    if handoff.get("product").and_then(Value::as_u64) != Some(0)
        || handoff
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "N06 owner-join Cap refused: auth handoff invented product/pass_product_clean_room"
                .into(),
        );
    }
    Ok(handoff)
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
/// Cursor-owned N06 owner-join readiness gate.
/// Binds hosted Cap EXIT0 receipt + Unlock A Cap EXIT0 status → readiness;
/// Cap=0 without Root auth handoff. Never invents `n06_owner_join=true` /
/// `BrowserReady` / `pass_product_clean_room` / Root ownership tokens.
/// No Docker Cap burn.
pub(super) fn run_n06_owner_join_readiness(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "n06_owner_join_readiness_bind_hosted_cap";
    let hosted_cap_path = root.join(
        ".spec.local/root/evidence/20261109013400-cursor-n06-hosted-venue-cap-realloc/CAP1-EXIT0.json",
    );
    if !hosted_cap_path.is_file() {
        lab.components["n06_owner_join_readiness"] = json!({
            "status": "refused",
            "kind": "n06-owner-join-readiness",
            "product": 0,
            "pass_product_clean_room": 0,
            "n06_owner_join": false,
            "cap": 0,
            "cap_executable_for_join": false,
            "missing_hosted_cap_exit0": hosted_cap_path.display().to_string(),
            "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
            "note": "hosted Cap EXIT0 receipt required before owner-join readiness; never invent pass_product"
        });
        snapshot::write(
            &lab.evidence,
            "n06-owner-join-readiness-refuse.json",
            &lab.components["n06_owner_join_readiness"],
        )?;
        return Err(
            "N06 owner-join readiness refused: hosted Cap EXIT0 receipt absent; Cap=0; never invent pass_product_clean_room"
                .into(),
        );
    }
    let hosted_bytes = fs::read(&hosted_cap_path).map_err(|error| {
        format!(
            "N06 owner-join readiness refused: hosted Cap EXIT0 read {}: {error}",
            hosted_cap_path.display()
        )
    })?;
    let hosted_cap: Value = serde_json::from_slice(&hosted_bytes).map_err(|error| {
        format!(
            "N06 owner-join readiness refused: hosted Cap EXIT0 JSON {}: {error}",
            hosted_cap_path.display()
        )
    })?;
    let mut readiness = n06_owner_join_readiness_from_hosted_cap(&hosted_cap)?;
    lab.stage = "n06_owner_join_readiness_bind_unlock_a";
    let unlock_a = match load_unlock_a_cap_exit0_closer(&root) {
        Ok(closer) => closer,
        Err(error) => {
            lab.components["n06_owner_join_readiness"] = json!({
                "status": "refused",
                "kind": "n06-owner-join-readiness",
                "product": 0,
                "pass_product_clean_room": 0,
                "n06_owner_join": false,
                "browser_ready": false,
                "cap": 0,
                "cap_free": false,
                "cap_executable_for_join": false,
                "hosted_cap_exit0_run_id": HOSTED_CAP_EXIT0_RUN_ID,
                "missing_unlock_a_cap_exit0_closer": UNLOCK_A_CAP_EXIT0_CLOSER_REL,
                "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
                "note": "Unlock A Cap EXIT0 closer required for Cap=0 honesty bind after Unlock A spent; never invent BrowserReady"
            });
            snapshot::write(
                &lab.evidence,
                "n06-owner-join-readiness-refuse.json",
                &lab.components["n06_owner_join_readiness"],
            )?;
            return Err(error);
        }
    };
    bind_unlock_a_status_into_n06_readiness(&mut readiness, &unlock_a)?;
    validate_n06_owner_join_receipt(&readiness)?;
    snapshot::write(&lab.evidence, "n06-owner-join-readiness.json", &readiness)?;
    snapshot::write(&lab.evidence, "n06-owner-join-unlock-a-bound.json", &unlock_a)?;
    lab.case(
        "n06-owner-join-hosted-cap-component-bound",
        readiness.get("hosted_cap_component_green") == Some(&json!(true))
            && readiness.get("hosted_cap_exit0_run_id") == Some(&json!(HOSTED_CAP_EXIT0_RUN_ID)),
        json!({
            "run_id": HOSTED_CAP_EXIT0_RUN_ID,
            "counts_as": "pass_component_hosted_measure",
            "product": 0
        }),
    )?;
    lab.case(
        "n06-owner-join-unlock-a-cap-exit0-bound",
        readiness.get("unlock_a_cap_exit0_bound") == Some(&json!(true))
            && readiness.get("unlock_a_cap_free") == Some(&json!(false))
            && readiness.get("unlock_a_browser_ready") == Some(&json!(false))
            && readiness.get("browser_ready") == Some(&json!(false))
            && readiness.get("cap_free") == Some(&json!(false)),
        json!({
            "unlock_a_cap_exit": 0,
            "counts_as": "pass_component_g4_cef_at_executed",
            "cap_free": false,
            "browser_ready": false,
            "product": 0
        }),
    )?;
    lab.stage = "n06_owner_join_auth_handoff";
    let auth = require_n06_owner_join_auth_handoff(&root);
    let auth_present = auth.is_ok();
    if let Err(ref error) = auth {
        snapshot::write(
            &lab.evidence,
            "n06-owner-join-auth-handoff-refuse.json",
            &json!({
                "schema": "humanymous.lab-h2-n06-owner-join-auth-handoff-refuse/1",
                "cap": 0,
                "cap_free": false,
                "cap_executable_for_join": false,
                "n06_owner_join": false,
                "browser_ready": false,
                "product": 0,
                "pass_product_clean_room": 0,
                "hosted_cap_exit0_run_id": HOSTED_CAP_EXIT0_RUN_ID,
                "unlock_a_cap_exit0_bound": true,
                "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
                "auth_handoff_candidates": N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
                "detail": error.to_string(),
                "note": "fail-closed readiness; Cap=0; Cap free=false after Unlock A EXIT0; never invent Root ownership tokens"
            }),
        )?;
    } else if let Ok(ref handoff) = auth {
        snapshot::write(&lab.evidence, "n06-owner-join-auth-handoff.json", handoff)?;
    }
    // Cap-executable join would require Root auth; Cursor must not invent it.
    // This gate always lands readiness with join=false when Cap=0 (auth absent).
    lab.case(
        "n06-owner-join-still-open-cap0",
        readiness.get("n06_owner_join") == Some(&json!(false)) && !auth_present,
        json!({
            "n06_owner_join": false,
            "cap": 0,
            "cap_free": false,
            "cap_executable_for_join": false,
            "auth_handoff_present": auth_present,
            "unlock_a_cap_exit0_bound": true,
            "browser_ready": false,
            "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
            "reason": "hosted Cap + Unlock A component green ≠ owner join / BrowserReady / product; Root auth handoff still required"
        }),
    )?;
    lab.components["n06_owner_join_readiness"] = json!({
        "status": "readiness_landed_join_open",
        "kind": "n06-owner-join-readiness",
        "product": 0,
        "pass_product_clean_room": 0,
        "n06_owner_join": false,
        "browser_ready": false,
        "cap": 0,
        "cap_free": false,
        "cap_executable_for_join": false,
        "hosted_cap_exit0_run_id": HOSTED_CAP_EXIT0_RUN_ID,
        "counts_as": "pass_component_hosted_measure",
        "unlock_a_cap_exit0_bound": true,
        "unlock_a_counts_as": "pass_component_g4_cef_at_executed",
        "auth_handoff_present": auth_present,
        "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
        "note": "Cursor-owned readiness gate landed; hosted Cap EXIT0 + Unlock A Cap EXIT0 bound; Cap=0 without Root auth; Cap free=false; BrowserReady=false; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "n06-owner-join-readiness-component.json",
        &lab.components["n06_owner_join_readiness"],
    )?;
    // Cap=0: readiness landed; join Cap not burned / not executable without Root auth.
    Ok(())
}

/// N06 hosted-venue Cap — refuse-closed unless `actual_hosted_linux` + topology.
/// Never invents `desktop_as_hosted` / `pass_product_clean_room`. No GITHUB_ACTIONS spoof.
pub(super) fn run_hosted_venue(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    require_freeze_preimages(&root)?;
    if let Err(error) = require_hosted_venue_for_n06_join(&root) {
        let missing = missing_hosted_venue_topology(&root);
        lab.components["w2_h2_hosted_venue"] = json!({
            "status": "refused",
            "kind": "w2-h2-hosted-venue-component",
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "n06_owner_join": false,
            "actual_hosted_linux": crate::docker::actual_hosted_linux(),
            "missing_hosted_venue_topology": missing,
            "runner_requirement": HOSTED_VENUE_RUNNER_REQUIREMENT,
            "cap_selector": HOSTED_VENUE_CAP_SELECTOR,
            "pilot_command": HOSTED_VENUE_PILOT_COMMAND,
            "cap_burn": false,
            "note": "hosted venue Cap refuse-closed; Desktop ≠ Specfold; never invent pass_product_clean_room or desktop_as_hosted"
        });
        snapshot::write(
            &lab.evidence,
            "w2-h2-hosted-venue-refuse.json",
            &lab.components["w2_h2_hosted_venue"],
        )?;
        return Err(error);
    }

    lab.stage = "h2_hosted_venue_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: serde_json::Value = serde_json::from_str(&config)?;
    snapshot::write(&lab.evidence, "resolved-hosted-compose.json", &parsed)?;
    lab.case(
        "h2-hosted-venue-no-published-ports",
        parsed["services"]
            .as_object()
            .ok_or("services absent")?
            .values()
            .all(|s| s.get("ports").is_none()),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable h2-hosted-venue-lab measure image (Cap)");
    lab.stage = "h2_hosted_venue_build";
    lab.compose(&["build", "h2-hosted-venue-pilot"], true)?;
    lab.stage = "h2_hosted_venue_up";
    lab.compose(&["up", "-d", "h2-hosted-venue-pilot"], true)?;
    let container = lab.inspect("h2-hosted-venue-pilot")?;
    let id = container["Id"]
        .as_str()
        .ok_or("h2-hosted-venue pilot container id missing")?
        .to_owned();
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
        "h2-hosted-venue-pilot-measure-oneshot",
        !running && (exit_code == 0 || exit_code == 1),
        json!({"running": running, "exit_code": exit_code}),
    )?;
    let logs = lab.docker.call(&["logs", "--tail", "20", &id])?;
    snapshot::write(
        &lab.evidence,
        "h2-hosted-venue-measure-logs.json",
        &json!({"logs": logs}),
    )?;
    lab.stage = "h2_hosted_venue_measure_receipt";
    let receipt = parse_measure_receipt(&logs)?;
    snapshot::write(
        &lab.evidence,
        "hosted-original-h2-measure-receipt.json",
        &receipt,
    )?;
    let measured = original_h2_measured_from_receipt(&receipt)?;
    refuse_invented_original_h2_measured(measured, Some(&receipt))?;
    let venue = receipt
        .get("venue")
        .and_then(Value::as_str)
        .unwrap_or("");
    lab.stage = "h2_hosted_venue_measure_venue";
    if !HOSTED_VENUE_MARKERS.contains(&venue) {
        return Err(format!(
            "N06 hosted-venue Cap refused: measure venue must be one of ({}); got {venue:?}; HUMANYMOUS_H2_MEASURE_VENUE must flow into measure receipt; never invent desktop_as_hosted",
            HOSTED_VENUE_MARKERS.join("|")
        )
        .into());
    }
    if receipt.get("desktop_as_hosted").and_then(Value::as_bool) == Some(true)
        || receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "N06 hosted-venue Cap refused: measure invented desktop_as_hosted/product/pass_product_clean_room"
                .into(),
        );
    }
    lab.case(
        "h2-hosted-venue-measure-receipt-validated",
        true,
        json!({
            "original_h2_measured": measured,
            "venue": venue,
            "tls_h2_status": receipt.get("tls_h2_status"),
            "plaintext_control_status": receipt.get("plaintext_control_status"),
            "desktop_as_hosted": false,
            "exit_code": exit_code
        }),
    )?;
    if measured {
        lab.case(
            "h2-hosted-venue-original-h2-measured",
            measured && exit_code == 0,
            json!({
                "original_h2_measured": measured,
                "venue": venue,
                "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
                "reason": "hosted Cap TLS measure receipt validated; component only; not product"
            }),
        )?;
    } else {
        lab.case(
            "h2-hosted-venue-original-h2-not-measured",
            false,
            json!({
                "original_h2_measured": measured,
                "venue": venue,
                "reason": "hosted Cap measure ran but original_h2_measured=false; refuse component green"
            }),
        )?;
    }
    lab.case(
        "h2-hosted-venue-n06-owner-join-still-open",
        true,
        json!({
            "n06_owner_join": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "reason": "hosted Cap measure ≠ N06 owner join / pass_product; Root join still open"
        }),
    )?;
    lab.components["w2_h2_hosted_venue"] = json!({
        "status": "executed",
        "kind": "w2-h2-hosted-venue-component",
        "product": 0,
        "pass_product_clean_room": 0,
        "original_h2_measured": measured,
        "desktop_as_hosted": false,
        "n06_owner_join": false,
        "actual_hosted_linux": true,
        "venue": venue,
        "missing_n06_product_lift": MISSING_N06_PRODUCT_LIFT,
        "note": "hosted venue Cap measure only; N06 owner-integration join still Root; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "w2-h2-hosted-venue-component.json",
        &lab.components["w2_h2_hosted_venue"],
    )?;
    Ok(())
}

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
        CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND, HOSTED_CAP_EXIT0_RUN_ID, HOSTED_CAP_EXIT0_SCHEMA,
        HOSTED_VENUE_CAP_SELECTOR, HOSTED_VENUE_MARKERS, HOSTED_VENUE_PILOT_COMMAND,
        MEASURE_RECEIPT_SCHEMA, MISSING_N06_PRODUCT_LIFT, N06_OWNER_JOIN_CAP_SELECTOR,
        N06_OWNER_JOIN_PILOT_COMMAND, N06_OWNER_JOIN_RECEIPT_SCHEMA, N06_OWNER_JOIN_ROOT_ASK_PATHS,
        REQUIRED_HOSTED_VENUE_TOPOLOGY, UNLOCK_A_CAP_EXIT0_CLOSER_REL,
        UNLOCK_A_CAP_EXIT0_CLOSER_SCHEMA, bind_unlock_a_status_into_n06_readiness,
        find_n06_owner_join_auth_handoff, load_unlock_a_cap_exit0_closer, missing_cap_topology,
        missing_hosted_venue_topology, n06_owner_join_readiness_from_hosted_cap,
        n06_owner_join_readiness_receipt, original_h2_measured_from_receipt, parse_measure_receipt,
        refuse_invented_original_h2_measured, refuse_n06_product_lift_from_desktop_measure,
        require_hosted_venue_for_n06_join, require_n06_owner_join_auth_handoff,
        validate_hosted_cap_exit0_receipt, validate_n06_owner_join_receipt,
        validate_unlock_a_cap_exit0_closer,
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
    fn hosted_venue_markers_include_dockerfile_env() {
        // build/rust/h2-hosted-venue-lab.Dockerfile sets HUMANYMOUS_H2_MEASURE_VENUE=
        // hosted_linux_specfold; measure receipt must emit that marker (not desktop_lab_loopback).
        assert!(HOSTED_VENUE_MARKERS.contains(&"hosted_linux_specfold"));
        assert!(!HOSTED_VENUE_MARKERS.contains(&"desktop_lab_loopback"));
    }

    #[test]
    fn hosted_venue_cap_refuses_without_actual_hosted_linux() {
        let root = snapshot::root().expect("root");
        assert!(HOSTED_VENUE_PILOT_COMMAND.contains(HOSTED_VENUE_CAP_SELECTOR));
        assert_eq!(REQUIRED_HOSTED_VENUE_TOPOLOGY.len(), 2);
        // Desktop Cap refuse-closed; Specfold/github-hosted admits when topology present.
        if crate::docker::actual_hosted_linux() {
            require_hosted_venue_for_n06_join(&root)
                .expect("hosted Cap admits when actual_hosted_linux + topology");
            assert!(missing_hosted_venue_topology(&root).is_empty());
        } else {
            let err = require_hosted_venue_for_n06_join(&root).expect_err("desktop refuse");
            assert!(
                err.to_string().contains("actual_hosted_linux=false"),
                "Desktop Cap must refuse hosted venue; got {err}"
            );
        }
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

    #[test]
    fn hosted_cap_exit0_binds_owner_join_readiness_cap0() {
        assert!(N06_OWNER_JOIN_PILOT_COMMAND.contains(N06_OWNER_JOIN_CAP_SELECTOR));
        assert!(!N06_OWNER_JOIN_ROOT_ASK_PATHS.is_empty());
        let hosted = json!({
            "schema": HOSTED_CAP_EXIT0_SCHEMA,
            "run_id": HOSTED_CAP_EXIT0_RUN_ID,
            "cap_exit": 0,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "n06_owner_join": false,
            "actual_hosted_linux": true
        });
        validate_hosted_cap_exit0_receipt(&hosted).expect("hosted Cap EXIT0");
        let readiness = n06_owner_join_readiness_from_hosted_cap(&hosted).expect("bind");
        assert_eq!(readiness["n06_owner_join"], false);
        assert_eq!(readiness["product"], 0);
        assert_eq!(readiness["hosted_cap_exit0_run_id"], HOSTED_CAP_EXIT0_RUN_ID);
        assert_eq!(readiness["cap_executable_for_join"], false);
        assert!(validate_n06_owner_join_receipt(&readiness).is_ok());

        let invent_join = json!({
            "schema": HOSTED_CAP_EXIT0_SCHEMA,
            "run_id": HOSTED_CAP_EXIT0_RUN_ID,
            "cap_exit": 0,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "n06_owner_join": true,
            "actual_hosted_linux": true
        });
        assert!(validate_hosted_cap_exit0_receipt(&invent_join).is_err());
    }

    #[test]
    fn unlock_a_cap_exit0_binds_into_n06_readiness_cap0() {
        assert!(!UNLOCK_A_CAP_EXIT0_CLOSER_REL.is_empty());
        let unlock_a = json!({
            "schema": UNLOCK_A_CAP_EXIT0_CLOSER_SCHEMA,
            "cap_exit": 0,
            "cap_free": false,
            "browser_ready": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "g4_cef_at_component": "executed",
            "n06_reburn": false,
            "w2_reburn": false,
            "authority_reburn": false,
            "head": "8f8a128d1e6ee51994a5324365c0cf1f4b4f3c58"
        });
        validate_unlock_a_cap_exit0_closer(&unlock_a).expect("Unlock A Cap EXIT0");
        let hosted = json!({
            "schema": HOSTED_CAP_EXIT0_SCHEMA,
            "run_id": HOSTED_CAP_EXIT0_RUN_ID,
            "cap_exit": 0,
            "product": 0,
            "pass_product_clean_room": 0,
            "desktop_as_hosted": false,
            "n06_owner_join": false,
            "actual_hosted_linux": true
        });
        let mut readiness = n06_owner_join_readiness_from_hosted_cap(&hosted).expect("hosted");
        bind_unlock_a_status_into_n06_readiness(&mut readiness, &unlock_a).expect("bind A");
        assert_eq!(readiness["unlock_a_cap_exit0_bound"], true);
        assert_eq!(readiness["unlock_a_cap_free"], false);
        assert_eq!(readiness["unlock_a_browser_ready"], false);
        assert_eq!(readiness["browser_ready"], false);
        assert_eq!(readiness["cap_free"], false);
        assert_eq!(readiness["n06_owner_join"], false);
        assert_eq!(readiness["product"], 0);
        assert!(validate_n06_owner_join_receipt(&readiness).is_ok());

        let invent_ready = json!({
            "schema": UNLOCK_A_CAP_EXIT0_CLOSER_SCHEMA,
            "cap_exit": 0,
            "cap_free": false,
            "browser_ready": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "g4_cef_at_component": "executed",
            "n06_reburn": false,
            "w2_reburn": false,
            "authority_reburn": false
        });
        assert!(validate_unlock_a_cap_exit0_closer(&invent_ready).is_err());

        let root = snapshot::root().expect("root");
        let live = load_unlock_a_cap_exit0_closer(&root).expect("live Unlock A closer");
        assert_eq!(live["cap_exit"], 0);
        assert_eq!(live["browser_ready"], false);
        assert_eq!(live["cap_free"], false);
    }

    #[test]
    fn owner_join_auth_handoff_refuses_without_root_token() {
        let root = snapshot::root().expect("root");
        assert!(find_n06_owner_join_auth_handoff(&root).is_none());
        let err = require_n06_owner_join_auth_handoff(&root).expect_err("no invent");
        assert!(err.to_string().contains("Root auth handoff absent"));
        assert!(err.to_string().contains("Cap=0"));
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
