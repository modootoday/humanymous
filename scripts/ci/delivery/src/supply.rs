//! Immutable image identity, per-platform SPDX/provenance and exact OIDC signer.
use crate::{Result, command, require, source, write_json};
use serde_json::{Value, json};
use std::path::Path;
pub fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn has_source(value: &Value, sha: &str) -> bool {
    match value {
        Value::Object(fields) => {
            let uri = fields.get("uri").and_then(Value::as_str).unwrap_or("");
            (matches!(
                uri.strip_prefix("git+")
                    .unwrap_or(uri)
                    .split(['#', '?'])
                    .next(),
                Some(
                    "https://github.com/modootoday/humanymous.git"
                        | "https://github.com/modootoday/humanymous"
                )
            ) && fields.get("digest").is_some_and(|digest| {
                digest
                    .as_object()
                    .is_some_and(|values| values.values().any(|value| value == sha))
            })) || fields.values().any(|v| has_source(v, sha))
        }
        Value::Array(values) => values.iter().any(|v| has_source(v, sha)),
        _ => false,
    }
}
pub fn platform_proof(
    manifest: &Value,
    configs: &Value,
    provenance: &Value,
    sbom: &Value,
    expected: &str,
    sha: &str,
) -> Result<()> {
    require(
        digest(expected) && manifest["digest"] == expected,
        "published index differs from this run's validated digest",
    )?;
    let children = manifest["manifests"]
        .as_array()
        .ok_or("multi-platform index missing")?;
    for arch in ["amd64", "arm64"] {
        require(
            children.iter().any(|child| {
                child["platform"]["os"] == "linux"
                    && child["platform"]["architecture"] == arch
                    && child["digest"].as_str().is_some_and(digest)
            }),
            format!("missing linux/{arch} image"),
        )?;
        let platform = format!("linux/{arch}");
        require(
            configs[&platform]["config"]["Labels"]["org.opencontainers.image.revision"] == sha,
            format!("{platform}: source revision label mismatch"),
        )?;
        require(
            has_source(&provenance[&platform], sha),
            format!("{platform}: exact Git source provenance missing"),
        )?;
        let spdx = &sbom[&platform]["SPDX"];
        require(
            spdx["spdxVersion"]
                .as_str()
                .is_some_and(|v| v.starts_with("SPDX-"))
                && spdx["packages"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
            format!("{platform}: nonempty SPDX SBOM missing"),
        )?;
    }
    Ok(())
}
fn inspect(root: &Path, reference: &str, template: &str) -> Result<Value> {
    serde_json::from_str(&command::text(
        root,
        "docker",
        &[
            "buildx",
            "imagetools",
            "inspect",
            reference,
            "--format",
            template,
        ],
    )?)
    .map_err(|e| e.to_string())
}
pub fn verify_images(root: &Path) -> Result<()> {
    let head = source::head(root)?;
    require(
        std::env::var("GITHUB_SHA").as_deref() == Ok(&head),
        "image verifier checkout differs from release source",
    )?;
    let reference = std::env::var("GITHUB_REF").map_err(|e| e.to_string())?;
    let tag = reference
        .strip_prefix("refs/tags/")
        .ok_or("image verification requires exact tag")?;
    let version = tag.strip_prefix('v').ok_or("version tag missing v")?;
    require(
        !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit() || b == b'.'),
        "invalid stable image tag",
    )?;
    let identity = format!(
        "https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/tags/{tag}"
    );
    let mut reports = Vec::new();
    for image in ["runtime"] {
        let digest =
            std::fs::read_to_string(root.join("digests").join(image)).map_err(|e| e.to_string())?;
        let digest = digest.trim();
        require(
            self::digest(digest),
            "missing/malformed image digest artifact",
        )?;
        let name = format!("ghcr.io/modootoday/humanymous-{image}");
        let pinned = format!("{name}@{digest}");
        let manifest = inspect(root, &pinned, "{{json .Manifest}}")?;
        let configs = inspect(root, &pinned, "{{json .Image}}")?;
        let provenance = inspect(root, &pinned, "{{json .Provenance}}")?;
        let sbom = inspect(root, &pinned, "{{json .SBOM}}")?;
        platform_proof(&manifest, &configs, &provenance, &sbom, digest, &head)?;
        for tag in [version, "latest"] {
            require(
                inspect(root, &format!("{name}:{tag}"), "{{json .Manifest}}")?["digest"] == digest,
                "published tag points to another digest",
            )?;
        }
        let signatures: Value = serde_json::from_str(&command::text(
            root,
            "cosign",
            &[
                "verify",
                "--certificate-identity",
                &identity,
                "--certificate-oidc-issuer",
                "https://token.actions.githubusercontent.com",
                &pinned,
            ],
        )?)
        .map_err(|e| e.to_string())?;
        require(
            signatures.as_array().is_some_and(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item["critical"]["image"]["docker-manifest-digest"] == digest)
            }),
            "verified cosign payload lacks exact manifest digest",
        )?;
        reports.push(json!({"image":name,"digest":digest,"source_sha":head,"manifest":manifest,"provenance":provenance,"sbom":sbom,"signature_identity":identity,"verified":true}));
    }
    write_json(
        &root.join("target/delivery-evidence/image-publication.json"),
        &json!({"schema":"humanymous.image-publication/1","source_sha":head,"images":reports}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn proofs() -> (Value, Value, Value, Value) {
        let digest = format!("sha256:{}", "a".repeat(64));
        let manifest = json!({"digest":digest,"manifests":[{"digest":digest,"platform":{"os":"linux","architecture":"amd64"}},{"digest":digest,"platform":{"os":"linux","architecture":"arm64"}}]});
        let mut configs = json!({});
        let mut provenance = json!({});
        let mut sbom = json!({});
        for platform in ["linux/amd64", "linux/arm64"] {
            configs[platform] =
                json!({"config":{"Labels":{"org.opencontainers.image.revision":"commit"}}});
            provenance[platform] = json!({"SLSA":{"materials":[{"uri":"git+https://github.com/modootoday/humanymous.git","digest":{"sha1":"commit"}}]}});
            sbom[platform] =
                json!({"SPDX":{"spdxVersion":"SPDX-2.3","packages":[{"name":"runtime"}]}});
        }
        (manifest, configs, provenance, sbom)
    }
    #[test]
    fn real_platform_proof_shape_accepts_both_architectures() {
        let (m, c, p, s) = proofs();
        assert!(
            platform_proof(
                &m,
                &c,
                &p,
                &s,
                &format!("sha256:{}", "a".repeat(64)),
                "commit"
            )
            .is_ok()
        );
    }
    #[test]
    fn stale_tag_missing_sbom_wrong_source_and_architecture_fail() {
        for mutation in 0..4 {
            let (mut m, c, mut p, mut s) = proofs();
            match mutation {
                0 => m["digest"] = json!(format!("sha256:{}", "b".repeat(64))),
                1 => s["linux/arm64"] = Value::Null,
                2 => p["linux/amd64"] = json!({"SLSA":{"materials":[]}}),
                _ => m["manifests"] = json!([]),
            };
            assert!(
                platform_proof(
                    &m,
                    &c,
                    &p,
                    &s,
                    &format!("sha256:{}", "a".repeat(64)),
                    "commit"
                )
                .is_err()
            );
        }
    }
}
