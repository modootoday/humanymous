//! Cap=0 Cap1 lander readiness serializer.
//!
//! When Cap free=false: HOLD packet with Cap1 serial landers prepared (no Cap1 burn).
//! When Cap free=true: arm Cap1 serial command preimages without executing Cap1.
//! Never invents Cap free / BrowserReady / AUTH / AT / pass_product / Cap burn.
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::cap_free_after_unlock_a;
use crate::orchestration::clean_room_product_matrix::{
    CODE_DEFINED_PILOT_COMMAND as PM_PILOT, PRODUCT_ONESHOT as PM_ONESHOT,
};
use crate::orchestration::red_blue_product::{
    CODE_DEFINED_PILOT_COMMAND as RB_PILOT, PRODUCT_ONESHOT as RB_ONESHOT,
};
use crate::snapshot;
use serde_json::{json, Value};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cap1-lander-prep/1";
const PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- cap1-lander-prep [--workspace PATH] [--emit PATH]";

/// Cap=0 Cap1 lander readiness (serialize only; never Cap1 burn).
///
/// Usage: `cap1-lander-prep [--workspace PATH] [--emit PATH]`
pub fn run(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            _ => {
                return Err(
                    "usage: cap1-lander-prep [--workspace PATH] [--emit PATH]; Cap0 only; never Cap1 burn / invent Cap free / pass_product"
                        .into(),
                );
            }
        }
        index += 1;
    }

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cap1-lander-prep refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };

    let report = prep(&root)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cap1-lander-prep encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");
    // Latency cutter: diagnostics always exit 0; readiness is in JSON.
    Ok(0)
}

fn landers(cap_free: bool) -> Value {
    let armed = if cap_free {
        "cap_free_true_armed_no_burn"
    } else {
        "HOLD_prepare_only_no_Cap1_burn"
    };
    json!([
        {
            "rank": 14,
            "id": "U-CAP1-CR-ONCE",
            "selector": "--clean-room-product-matrix --require-full",
            "pilot_command": PM_PILOT,
            "product_oneshot_when_cap_free": PM_ONESHOT,
            "serial_order": 2,
            "twin_forbidden": ["CR", "PM"],
            "status": armed,
            "note": "Cap1 step2 product-matrix feeder after Cap free; Cap0 prep only until Cap free=true; never invent pass_product_clean_room"
        },
        {
            "rank": 15,
            "id": "U-CAP1-W2H2-ALT",
            "selector": "--w2-h2-product-binding",
            "pilot_command": "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding --require-full",
            "status": armed,
            "note": "Unlock B alt; Decision B without AUTH/AT cannot close product; Cap burn=0 while Cap free=false"
        },
        {
            "rank": 16,
            "id": "U-RB-SCENARIO-LAND",
            "selector": "--red-blue-product --require-full",
            "pilot_command": RB_PILOT,
            "product_oneshot_when_cap_free": RB_ONESHOT,
            "serial_order": 3,
            "twin_forbidden": ["RB"],
            "status": armed,
            "note": "Cap1 step3 RB feeder then Docker e2e oneshot; twin reburn forbidden; Cap0 land-prep only"
        },
        {
            "rank": 17,
            "id": "U-CR-CLASS-CLEANROOM",
            "selector": "--clean-room-product → pass_product_clean_room",
            "pilot_command": "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --clean-room-product --require-full",
            "serial_order": 1,
            "twin_forbidden": ["CR"],
            "status": armed,
            "note": "Cap1 step1 sole product authority class; Cap twin hold ≠ product; Cap0 Cap-ready lander only"
        },
        {
            "rank": 48,
            "id": "U-DOCKER-E2E-VALIDATE",
            "selector": "Docker-only e2e / red-blue-validate product (e2e-docker-launch.ps1)",
            "pilot_command": "pwsh -File scripts/e2e-docker-launch.ps1",
            "product_oneshot_when_cap_free": [
                "pwsh -File scripts/e2e-docker-launch.ps1",
                "bash scripts/e2e-docker.sh",
                "make e2e",
                "pwsh -File scripts/e2e-docker.ps1"
            ],
            "serial_order": 3,
            "pressure_guard": true,
            "single_flight_mutex": true,
            "affinity": "desktop-linux",
            "status": armed,
            "note": "Docker-only completion authority; mutex+pressure via e2e-docker-launch.ps1; host/loopback ≠ product; Cap1 after Cap free; Cap0 AffinityOnly/PressureOnly probes allowed without inventing docker_e2e_validate_product"
        },
        {
            "rank": 58,
            "id": "U-M3",
            "selector": "M3 composed product e2e (clean-room apex)",
            "pilot_command": null,
            "status": if cap_free { "await_product_cells_then_M3" } else { "G1_G9_product_0_hold" },
            "note": "Apex after product cells + Docker e2e green; Cap0 lander only; never invent M3"
        }
    ])
}

pub(crate) fn prep(root: &Path) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let status = if cap_free {
        "cap_free_true_cap1_serial_armed"
    } else {
        "HOLD_prepare_only_no_Cap1_burn"
    };
    let next = if cap_free {
        "Cap free measured true; Cap1 serial once: --clean-room-product → --clean-room-product-matrix --require-full → Docker e2e (pwsh -File scripts/e2e-docker-launch.ps1) / --red-blue-product --require-full; never Cap twin 73-80 reburn; never invent pass_product / docker_e2e / BrowserReady"
    } else {
        "Cap free=false; Cap1 HOLD (clean-room+matrix+Docker e2e+red-blue); Cap burn=0; re-run after Cap0 cap0-chain restores Cap free via NEW non-twin GRANT→TRANSFER; Cap0 AffinityOnly/PressureOnly launcher probes ok; never invent Cap free / pass_product / docker_e2e"
    };

    let launcher_ps1 = root.join("scripts/e2e-docker-launch.ps1").is_file();
    let launcher_sh = root.join("scripts/e2e-docker.sh").is_file();
    let topology = json!({
        "compose_red_blue_product": root
            .join("deployments/lab/compose.red-blue-product.yaml")
            .is_file(),
        "dockerfile_red_blue_product": root
            .join("build/rust/red-blue-product-lab.Dockerfile")
            .is_file(),
        "compose_clean_room_product": root
            .join("deployments/lab/compose.clean-room-product.yaml")
            .is_file(),
        "compose_product": root.join("deployments/compose.yaml").is_file(),
        "e2e_docker_launch_ps1": launcher_ps1,
        "e2e_docker_sh": launcher_sh,
        "cap0_topology_ne_product": true,
        "host_loopback_ne_product": true
    });

    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-cap1-lander-prep",
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "docker_e2e_validate_product": false,
        "cap1_executed": false,
        "cap1_burn_authorized": false,
        "status": status,
        "landers": landers(cap_free),
        "cap1_serial_preferred_order": [
            {"order": 1, "id": "U-CR-CLASS-CLEANROOM", "selector": "--clean-room-product"},
            {"order": 2, "id": "U-CAP1-CR-ONCE", "selector": "--clean-room-product-matrix --require-full"},
            {"order": 3, "id": "U-DOCKER-E2E-VALIDATE", "selector": "pwsh -File scripts/e2e-docker-launch.ps1 / --red-blue-product --require-full"}
        ],
        "preferred_when_cap_free": {
            "rank": 17,
            "id": "U-CR-CLASS-CLEANROOM",
            "pilot_command": "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --clean-room-product --require-full",
            "then": [
                PM_PILOT,
                "pwsh -File scripts/e2e-docker-launch.ps1",
                RB_PILOT
            ],
            "product_oneshot": PM_ONESHOT
        },
        "docker_e2e_topology_honesty": topology,
        "pilot_command": PILOT_COMMAND,
        "next": next,
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "never_invent_browser_ready": true,
            "never_invent_cap_free": true,
            "never_invent_docker_e2e_validate_product": true,
            "host_loopback_ne_product": true,
            "never_forge_auth_at": true,
            "detection_freeze": true,
            "no_cap1_this_wave": true,
            "no_cap_twin_reburn_73_80": true,
            "cap0_only": true,
            "serialize_ne_execute": true,
            "component_ne_product": true
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::cap0_product_honesty::{UNLOCK_A_CLOSER_REL, UNLOCK_A_CLOSER_SCHEMA};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cap1-lander-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    fn write_unlock_a(root: &Path, cap_free: bool) {
        let path = root.join(UNLOCK_A_CLOSER_REL);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "schema": UNLOCK_A_CLOSER_SCHEMA,
                "cap_exit": 0,
                "cap_free": cap_free,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write");
    }

    #[test]
    fn cap_free_false_holds_without_cap1_burn() {
        let root = temp_root("hold");
        write_unlock_a(&root, false);
        let report = prep(&root).expect("prep");
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["cap1_executed"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["status"], "HOLD_prepare_only_no_Cap1_burn");
        assert_eq!(
            report["landers"][0]["status"],
            "HOLD_prepare_only_no_Cap1_burn"
        );
        assert!(report["landers"]
            .as_array()
            .expect("landers")
            .iter()
            .all(|lander| lander.get("pilot_command").is_some()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cap_free_true_arms_serial_without_executing() {
        let root = temp_root("armed");
        write_unlock_a(&root, true);
        let report = prep(&root).expect("prep");
        assert_eq!(report["cap_free"], true);
        assert_eq!(report["cap1_executed"], false);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["status"], "cap_free_true_cap1_serial_armed");
        assert_eq!(
            report["landers"][0]["status"],
            "cap_free_true_armed_no_burn"
        );
        assert_eq!(
            report["preferred_when_cap_free"]["pilot_command"],
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --clean-room-product --require-full"
        );
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(
            report["cap1_serial_preferred_order"][2]["id"],
            "U-DOCKER-E2E-VALIDATE"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
