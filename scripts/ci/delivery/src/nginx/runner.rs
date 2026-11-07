//! Fixed Docker argv and owned cleanup; the contract is data, never executable.
use super::{contract, validate};
use crate::{Result, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::{
    fs::File,
    path::Path,
    process::{Command, Stdio},
};

fn capture(
    root: &Path,
    args: &[&str],
    environment: &[(String, String)],
    path: &Path,
) -> Result<i32> {
    let stdout = File::create(path).map_err(|e| e.to_string())?;
    let stderr = File::create(path.with_extension("stderr.log")).map_err(|e| e.to_string())?;
    let status = Command::new("docker")
        .args(args)
        .envs(environment.iter().cloned())
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}
fn image_id(path: &Path) -> Result<String> {
    let image = read_json(path)?;
    let id = image[0]["Id"]
        .as_str()
        .ok_or("nginx image identity absent")?;
    require(
        id.len() == 71
            && id.starts_with("sha256:")
            && id[7..].bytes().all(|c| c.is_ascii_hexdigit()),
        "invalid nginx image identity",
    )?;
    require(
        image[0]["Config"]["User"] == "65532:65532",
        "nginx image actor differs from contract",
    )?;
    Ok(id.to_owned())
}
pub fn run(root: &Path) -> Result<()> {
    let before = source::snapshot(root)?;
    let revision = before["source_sha"]
        .as_str()
        .ok_or("nginx source revision absent")?;
    require(
        revision.len() == 40 && revision.bytes().all(|c| c.is_ascii_hexdigit()),
        "exact nginx source SHA required",
    )?;
    let manifest = before["manifest_sha256"]
        .as_str()
        .ok_or("nginx source manifest absent")?;
    let lock = source::file(&root.join("Cargo.lock"))?;
    let directory = evidence::bundle(root)?;
    write_json(&directory.join("nginx-source-before.json"), &before)?;
    let stamp = directory
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.get(..14))
        .ok_or("owned evidence ID absent")?;
    require(
        stamp.bytes().all(|c| c.is_ascii_digit()),
        "owned nginx project ID invalid",
    )?;
    let project = format!("hm-ci-nginx-{stamp}");
    let image = format!("humanymous/nginx-native:ci-{}-{stamp}", &revision[..12]);
    let environment = vec![
        ("HUMANYMOUS_SOURCE_REV".into(), revision.into()),
        ("HUMANYMOUS_SOURCE_MANIFEST".into(), manifest.into()),
        ("HUMANYMOUS_NGINX_IMAGE".into(), image.clone()),
    ];
    let base = [
        "compose",
        "-p",
        &project,
        "-f",
        "adapters/nginx/compose.yaml",
    ];
    let mut stage = "compose_config";
    let mut observed_image = Value::Null;
    let result: Result<Value> = (|| {
        let mut args = base.to_vec();
        args.extend(["config", "--quiet"]);
        require(
            capture(
                root,
                &args,
                &environment,
                &directory.join("nginx-config.log"),
            )? == 0,
            "nginx compose config failed",
        )?;
        stage = "image_build";
        let mut args = base.to_vec();
        args.push("build");
        require(
            capture(
                root,
                &args,
                &environment,
                &directory.join("nginx-build.log"),
            )? == 0,
            "nginx image build failed",
        )?;
        stage = "image_identity";
        require(
            capture(
                root,
                &["image", "inspect", &image],
                &environment,
                &directory.join("nginx-image-before.json"),
            )? == 0,
            "nginx image inspection failed",
        )?;
        let id = image_id(&directory.join("nginx-image-before.json"))?;
        observed_image = json!(id);
        stage = "native_pilot";
        let mut args = base.to_vec();
        args.extend(["run", "--rm", "native-pilot"]);
        require(
            capture(
                root,
                &args,
                &environment,
                &directory.join("nginx-owner.json"),
            )? == 0,
            "nginx pilot failed",
        )?;
        stage = "report_validation";
        let summary = validate(
            &read_json(&directory.join("nginx-owner.json"))?,
            revision,
            manifest,
            &lock,
        )?;
        require(
            capture(
                root,
                &["image", "inspect", &image],
                &environment,
                &directory.join("nginx-image-after.json"),
            )? == 0
                && image_id(&directory.join("nginx-image-after.json"))? == id,
            "nginx image changed during execution",
        )?;
        Ok(summary)
    })();
    // Cleanup uses only the generated, reserved project, including on failures.
    let mut cleanup = base.to_vec();
    cleanup.extend(["down", "--remove-orphans"]);
    let cleanup_code = capture(
        root,
        &cleanup,
        &environment,
        &directory.join("nginx-cleanup.log"),
    )
    .ok();
    let after = source::snapshot(root).ok();
    if let Some(after) = &after {
        write_json(&directory.join("nginx-source-after.json"), after)?;
    }
    let changed = after.as_ref().is_none_or(|after| {
        before["source_sha"] != after["source_sha"]
            || before["manifest_sha256"] != after["manifest_sha256"]
    });
    let passed = result.is_ok() && cleanup_code == Some(0) && !changed;
    let summary = json!({"schema":"humanymous.nginx.execution-summary/1","status":if passed{"passed"}else{"failed"},
        "source_revision":revision,"source_manifest":manifest,"cargo_lock_sha256":lock,"image_id":observed_image,
        "required_scenario_groups":contract()?["required_groups"].as_array().map(Vec::len),
        "report":result.as_ref().ok(),"failure_stage":if result.is_err(){json!(stage)}else{Value::Null},
        "cleanup_exit_code":cleanup_code,"source_changed_during_execution":changed,"full_product_qualified":false});
    write_json(&directory.join("nginx-execution-summary.json"), &summary)?;
    write_json(
        &root.join("target/delivery-evidence/nginx-component.json"),
        &summary,
    )?;
    evidence::register(&directory, "Owned nginx component execution")?;
    evidence::emit_public_summary("Source-bound nginx scenario groups", &summary)?;
    require(passed, "nginx component execution/contract/cleanup failed")
}
