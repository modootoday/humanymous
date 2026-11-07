//! Owned Docker I/O for diagnostic custody observations; never a product gate.
use super::Lab;
use crate::{docker::Result, snapshot};
use humanymous_test_support::json::decode_strict_json;
use serde_json::{Value, json};
use std::fs;
mod projection;
use projection::{contract, paths, project, role};
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let contract = contract()?;
    let source_raw = fs::read(lab.evidence.join("source-manifest.json"))?;
    let source = decode_strict_json(&source_raw, 16 * 1024 * 1024)?;
    let recipe = snapshot::digest(&fs::read(
        lab.config_file("deployments/lab/images/product.Dockerfile")?,
    )?);
    let mut ledger = json!({"schema":contract["private_schema"],"image_sha256":null,"observations":contract["cases"].as_array().ok_or("mount cases")?.iter().map(|v|json!({"role":v["role"],"surface":v["surface"],"observation":null})).collect::<Vec<_>>(),"full_product_qualified":false});
    snapshot::write(&lab.evidence, "mount-probe-ledger.json", &ledger)?;
    let result = execute(lab, &contract, &mut ledger);
    if let Err(error) = &result {
        ledger["execution_error"] = json!({"stage":lab.stage,"detail":error.to_string()});
    }
    snapshot::write(&lab.evidence, "mount-probe-ledger.json", &ledger)?;
    let raw = fs::read(lab.evidence.join("mount-probe-ledger.json"))?;
    let public = project(
        &ledger,
        &raw,
        &source,
        &snapshot::digest(&source_raw),
        &recipe,
        &super::go_public::run_context()?,
    )?;
    snapshot::write(&lab.evidence, "mount-probe-public.json", &public)?;
    lab.public_receipts["mount_probe"] = public;
    lab.components["mount_probe"] = ledger;
    result
}
fn execute(lab: &mut Lab, contract: &Value, ledger: &mut Value) -> Result<()> {
    lab.stage = "mount_probe_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let config = decode_strict_json(config.as_bytes(), 1048576)?;
    snapshot::write(&lab.evidence, "mount-probe-compose.private.json", &config)?;
    let previous_volumes = lab.docker.call(&[
        "volume",
        "ls",
        "-q",
        "--filter",
        &format!("label=com.docker.compose.project={}", lab.project),
    ])?;
    lab.case(
        "mount-probe-no-previous-volumes",
        previous_volumes.trim().is_empty(),
        json!("fresh project-owned storage required"),
    )?;
    lab.stage = "mount_probe_build";
    lab.compose(&["build", "mount-writer-image"], true)?;
    let image = config["services"]["mount-writer-image"]["image"]
        .as_str()
        .ok_or("mount image name")?;
    let inspected = decode_strict_json(
        lab.docker.call(&["image", "inspect", image])?.as_bytes(),
        1048576,
    )?;
    let image = inspected[0]["Id"]
        .as_str()
        .ok_or("mount image ID")?
        .to_owned();
    ledger["image_sha256"] = json!(image);
    snapshot::write(&lab.evidence, "mount-probe-image.private.json", &inspected)?;
    for (index, case) in contract["cases"]
        .as_array()
        .ok_or("mount case inventory")?
        .iter()
        .enumerate()
    {
        lab.stage = "mount_probe_observation";
        let service = case["service"].as_str().ok_or("mount service name")?;
        lab.compose(&["create", service], true)?;
        let container = lab.inspect(service)?;
        let networks = container["NetworkSettings"]["Networks"]
            .as_object()
            .is_some_and(|n| {
                n.len() <= 1
                    && n.iter().all(|(name, v)| {
                        name == "none"
                            && ["IPAddress", "Gateway", "GlobalIPv6Address", "IPv6Gateway"]
                                .iter()
                                .all(|field| v[*field].as_str() == Some(""))
                    })
            });
        lab.case(
            &format!("mount-probe-boundary-{service}"),
            container["Image"] == image
                && container["HostConfig"]["NetworkMode"] == "none"
                && networks,
            json!("exact built image; none network without addresses or gateways"),
        )?;
        if case["surface"] == "fresh-volume" {
            let mut volumes = Vec::new();
            for path in role(contract, &case["role"])?["paths"]
                .as_array()
                .ok_or("mount volume inventory")?
            {
                let mount = container["Mounts"]
                    .as_array()
                    .and_then(|m| m.iter().find(|m| m["Destination"] == path["path"]))
                    .ok_or("mount volume destination absent")?;
                let expected_name = format!(
                    "{}_{}",
                    lab.project,
                    path["volume"].as_str().ok_or("mount volume key")?
                );
                if mount["Type"] != "volume" || mount["Name"] != expected_name {
                    return Err("mount volume ownership name mismatch".into());
                }
                let inspected = decode_strict_json(
                    lab.docker
                        .call(&["volume", "inspect", &expected_name])?
                        .as_bytes(),
                    65536,
                )?;
                if inspected[0]["Labels"]["com.docker.compose.project"] != lab.project
                    || inspected[0]["Labels"]["org.humanymous.owner"] != super::OWNER
                {
                    return Err("mount volume ownership labels refused".into());
                }
                volumes.push(inspected[0].clone());
            }
            ledger["observations"][index]["volume_provenance"] = json!(volumes);
        }
        let id = container["Id"].as_str().ok_or("mount owned container ID")?;
        let output = lab
            .docker
            .invoke_observation(&["start".into(), "-a".into(), id.into()])?;
        snapshot::write(
            &lab.evidence,
            &format!("{service}-output.private.json"),
            &json!({"stdout":output.stdout,"stderr":output.stderr}),
        )?;
        let observation = decode_strict_json(output.stdout.as_bytes(), 32768)?;
        paths(&observation, role(contract, &case["role"])?, contract)?;
        let exited = decode_strict_json(lab.docker.call(&["inspect", id])?.as_bytes(), 1048576)?;
        if exited[0]["Image"] != image
            || exited[0]["State"]["Running"] != false
            || exited[0]["State"]["ExitCode"] != 0
        {
            return Err("mount observation did not complete on the bound image".into());
        }
        ledger["observations"][index]["container_provenance"] = container;
        ledger["observations"][index]["observation"] = observation;
        snapshot::write(&lab.evidence, "mount-probe-ledger.json", ledger)?;
        lab.case(
            &format!("mount-probe-observed-{service}"),
            true,
            json!("complete observation; refusal is not qualification"),
        )?;
    }
    Ok(())
}
