//! Unlock C Cap candidate sketch — authority-node-ipc Docker Cap path.
//! Reviewable only. Do not copy to live runner until Root allocates + transfers.
//! Lab-runner coordinates Docker only; never host-executes pilot.mjs.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::json;

/// Cap=1 authority composition: build/run compose.authority.yaml from freeze.
pub fn run(lab: &mut Lab) -> Result<()> {
    lab.stage = "authority_compose_config";
    let config = lab
        .compose(&["config", "--format", "json"], true)?
        .stdout;
    let parsed: serde_json::Value = serde_json::from_str(&config)?;
    snapshot::write(&lab.evidence, "resolved-compose.json", &parsed)?;
    lab.case(
        "authority-no-published-ports",
        parsed["services"]
            .as_object()
            .ok_or("services absent")?
            .values()
            .all(|s| s.get("ports").is_none()),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable authority-lab image (Cap)");
    lab.stage = "authority_build";
    lab.compose(&["build", "authority-pilot"], true)?;
    // Pilot.mjs is a one-shot Cap measurement (exits 0). Do not use compose --wait
    // (that expects a long-running healthy service). Start detached, then inspect.
    lab.stage = "authority_up";
    lab.compose(&["up", "-d", "authority-pilot"], true)?;
    let container = lab.inspect("authority-pilot")?;
    lab.case(
        "authority-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("compose.authority.yaml network_mode=none"),
    )?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "authority-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    lab.components["authority_node_ipc"] = json!({
        "status": "executed",
        "kind": "authority-node-ipc-component",
        "product": 0,
        "pass_product_clean_room": 0,
        "note": "component composition only; never invent product"
    });
    Ok(())
}