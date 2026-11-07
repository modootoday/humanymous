//! Build candidates have real binary identities but never grant shipping parity.
use crate::{Result, evidence, read_json, require, source};
use serde_json::{Value, json};
use std::path::Path;

fn validate(inventory: &Value) -> Result<&Value> {
    require(
        inventory["schema"] == "humanymous.rust-image-candidates/1"
            && inventory["legacy_shipping_guard"]
                == json!(["build/core.Dockerfile", "build/gate.Dockerfile"]),
        "candidate inventory must preserve legacy shipping guard",
    )?;
    let candidates = inventory["candidates"]
        .as_array()
        .ok_or("candidate inventory absent")?;
    require(
        candidates.len() == 1,
        "unexpected Rust image candidate inventory",
    )?;
    let record = &candidates[0];
    require(
        record["role"] == "runtime-private-pipes"
            && record["package"] == "humanymous-runtime"
            && record["binary"] == "humanymous-runtime"
            && record["dockerfile"] == "build/rust/runtime-candidate.Dockerfile"
            && record["entrypoint"] == json!(["/usr/local/bin/humanymous-runtime"]),
        "candidate binary/role identity mismatch",
    )?;
    require(
        [
            "full_product_qualified",
            "source_build_verified",
            "nonroot_runtime_verified",
            "image_cve_scan_verified",
            "external_witness_binary_injection",
            "legacy_core_gate_replacement",
        ]
        .iter()
        .all(|key| record[key] == false)
            && record["qualified_platforms"]
                .as_array()
                .is_some_and(Vec::is_empty),
        "candidate inventory cannot assert unmeasured qualification",
    )?;
    Ok(record)
}
fn builder(dockerfile: &str, tool: &Value) -> Result<()> {
    require(
        tool["schema"] == "humanymous.runtime-build-tool/1"
            && tool["name"] == "cargo-auditable"
            && tool["version"] == "0.7.7"
            && tool["license"] == "MIT OR Apache-2.0"
            && tool["source_revision"] == "3d3bb574f97dc09a264cfc666ec5d40790f6b3ba"
            && tool["archive_sha256"]
                == "ccdd489bc94d1338cd04389a842e00661300331ec915fb376b4810ef7d480b5d"
            && tool["packaged_lock_sha256"]
                == "6a4241448ccc2fa05f7bb8e83c78081fd9f62c5311f44465027bae90d6700f2f"
            && tool["tool_install_executed"] == false
            && tool["runtime_auditable_build_verified"] == false,
        "auditable build tool identity or unmeasured qualification drift",
    )?;
    let packages = tool["registry_dependency_archives"]
        .as_array()
        .ok_or("tool closure absent")?;
    let mut identities = std::collections::BTreeSet::new();
    require(
        packages.len() == 47
            && packages.iter().all(|package| {
                package["name"]
                    .as_str()
                    .is_some_and(|name| !name.is_empty())
                    && package["version"]
                        .as_str()
                        .is_some_and(|version| !version.is_empty())
                    && package["sha256"].as_str().is_some_and(|hash| {
                        hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                    && identities.insert((package["name"].as_str(), package["version"].as_str()))
            }),
        "auditable tool dependency closure invalid",
    )?;
    require(
        dockerfile.contains(
                "cargo auditable build --release --locked -p humanymous-runtime --no-default-features --features gateway --bin humanymous-runtime",
            )
            && dockerfile.contains("cargo install --locked --version 0.7.7 --root /opt/cargo-auditable cargo-auditable")
            && dockerfile.contains("RUN PATH=\"/opt/cargo-auditable/bin:${PATH}\" cargo auditable build")
            && dockerfile.contains("USER 65532:65532")
            && dockerfile.contains("ENTRYPOINT [\"/usr/local/bin/humanymous-runtime\"]")
            && !crate::docker_source::has_go_build(dockerfile)
            && !dockerfile.contains("golang:"),
        "candidate source/binary build contract mismatch",
    )
}
pub fn check(root: &Path) -> Result<()> {
    let inventory = read_json(&root.join("build/rust/image-candidates.json"))?;
    let candidate = validate(&inventory)?;
    let tool = read_json(&root.join("build/rust/runtime-build-tool.json"))?;
    let dockerfile = std::fs::read_to_string(root.join("build/rust/runtime-candidate.Dockerfile"))
        .map_err(|e| e.to_string())?;
    require(
        root.join("crates/sdk/humanymous-runtime/src/gateway/main.rs")
            .is_file(),
        "runtime source absent",
    )?;
    builder(&dockerfile, &tool)?;
    crate::notices::image(root, &dockerfile)?;
    let source = source::snapshot(root)?;
    evidence::emit_public_summary(
        "Rust image candidate source inventory",
        &json!({
        "schema":"humanymous.rust-image-candidate-summary/1","source_sha":source["source_sha"],
        "source_manifest_sha256":source["manifest_sha256"],"dockerfile_sha256":source::file(&root.join("build/rust/runtime-candidate.Dockerfile"))?,
        "build_tool_manifest_sha256":source::file(&root.join("build/rust/runtime-build-tool.json"))?,
        "candidate":candidate,"scope":"source inventory only; actual build, CVE scan, actor and shipping parity are separate evidence"}),
    )
}
/// Preparation never grants publication. Promoting this candidate requires a
/// reviewed source change and real source-bound platform/product/scan receipts.
pub fn shipping(root: &Path) -> Result<()> {
    let inventory = read_json(&root.join("build/rust/image-candidates.json"))?;
    let candidate = validate(&inventory)?;
    for legacy in ["build/core.Dockerfile", "build/gate.Dockerfile"] {
        require(
            !root.join(legacy).exists(),
            format!("retired Go shipping input remains: {legacy}"),
        )?;
    }
    let dockerfile = std::fs::read_to_string(root.join("build/rust/runtime-candidate.Dockerfile"))
        .map_err(|e| e.to_string())?;
    builder(
        &dockerfile,
        &read_json(&root.join("build/rust/runtime-build-tool.json"))?,
    )?;
    crate::notices::image(root, &dockerfile)?;
    require(
        candidate["source_build_verified"] == true
            && candidate["nonroot_runtime_verified"] == true
            && candidate["image_cve_scan_verified"] == true
            && candidate["full_product_qualified"] == true
            && candidate["qualified_platforms"] == json!(["linux/amd64", "linux/arm64"]),
        "runtime candidate remains unqualified for shipping; reviewed receipts required",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_inventory_build_cannot_revert_to_plain_cargo_or_unpinned_tool() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let dockerfile =
            std::fs::read_to_string(root.join("build/rust/runtime-candidate.Dockerfile")).unwrap();
        let tool = read_json(&root.join("build/rust/runtime-build-tool.json")).unwrap();
        assert!(builder(&dockerfile, &tool).is_ok());
        assert!(
            builder(
                &dockerfile.replace("cargo auditable build", "cargo build"),
                &tool
            )
            .is_err()
        );
        assert!(
            builder(
                &dockerfile.replace("--locked --version 0.7.7", "--version 0.7.7"),
                &tool
            )
            .is_err()
        );
        let mut changed = tool.clone();
        changed["registry_dependency_archives"][0]["sha256"] = json!("unknown");
        assert!(builder(&dockerfile, &changed).is_err());
    }
    #[test]
    fn candidate_is_not_a_core_gate_alias_or_qualified_platform() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let inventory = read_json(&root.join("build/rust/image-candidates.json")).unwrap();
        assert!(validate(&inventory).is_ok());
        for mutation in 0..3 {
            let mut changed = inventory.clone();
            match mutation {
                0 => changed["candidates"][0]["legacy_core_gate_replacement"] = json!(true),
                1 => changed["candidates"][0]["qualified_platforms"] = json!(["linux/amd64"]),
                _ => changed["legacy_shipping_guard"] = json!([]),
            }
            assert!(validate(&changed).is_err());
        }
    }
}
