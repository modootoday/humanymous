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
pub fn check(root: &Path) -> Result<()> {
    let inventory = read_json(&root.join("build/rust/image-candidates.json"))?;
    let candidate = validate(&inventory)?;
    let dockerfile = std::fs::read_to_string(root.join("build/rust/runtime-candidate.Dockerfile"))
        .map_err(|e| e.to_string())?;
    require(
        root.join("crates/runtime/src/main.rs").is_file()
            && dockerfile.contains(
                "cargo build --release --locked -p humanymous-runtime --bin humanymous-runtime",
            )
            && dockerfile.contains("USER 65532:65532")
            && dockerfile.contains("ENTRYPOINT [\"/usr/local/bin/humanymous-runtime\"]")
            && !dockerfile.contains("go build")
            && !dockerfile.contains("golang:"),
        "candidate source/binary build contract mismatch",
    )?;
    let source = source::snapshot(root)?;
    evidence::emit_public_summary(
        "Rust image candidate source inventory",
        &json!({
        "schema":"humanymous.rust-image-candidate-summary/1","source_sha":source["source_sha"],
        "source_manifest_sha256":source["manifest_sha256"],"dockerfile_sha256":source::file(&root.join("build/rust/runtime-candidate.Dockerfile"))?,
        "candidate":candidate,"scope":"source inventory only; actual build, CVE scan, actor and shipping parity are separate evidence"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
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
