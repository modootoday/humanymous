//! Hosted-only immutable untouched Go fixture acquisition and compiler binding.
use super::Lab;
use crate::{
    docker::Result,
    snapshot::{self, Snapshot},
};
use humanymous_test_support::{json::decode_strict_json, process::capture_with_stderr};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command, time::Duration};

pub(super) struct Fixture {
    pub context: Snapshot,
    pub export: String,
    pub identity: Value,
}
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = capture_with_stderr(
        Command::new("git").args(args).current_dir(root),
        &[],
        Duration::from_secs(120),
        4 * 1024 * 1024,
    )?;
    if !output.status.success() {
        return Err("fixed upstream Git operation failed".into());
    }
    Ok(output.stdout)
}
pub(super) fn require_hosted() -> Result<()> {
    if !crate::docker::actual_hosted_linux() {
        return Err("actual Go fixture requires approved free hosted Linux job".into());
    }
    Ok(())
}
pub(super) fn prepare(lab: &mut Lab) -> Result<Fixture> {
    require_hosted()?;
    let source = lab.config_file("adapters/go/pilot-contract.json")?;
    let contract = decode_strict_json(&fs::read(source)?, 65536)?;
    let compiler = decode_strict_json(
        &fs::read(lab.config_file("adapters/go/compiler-input.json")?)?,
        65536,
    )?;
    let mut nonce = [0; 8];
    getrandom::fill(&mut nonce)?;
    let name = format!(
        "hm-rust-lab-{}",
        nonce.iter().map(|v| format!("{v:02x}")).collect::<String>()
    );
    let path = std::env::temp_dir().canonicalize()?.join(name);
    fs::create_dir(&path)?;
    let context = Snapshot(path);
    let checkout = context.0.join("upstream");
    fs::create_dir(&checkout)?;
    let revision = contract["fixture"]["revision"]
        .as_str()
        .ok_or("upstream revision")?;
    let repository = contract["fixture"]["repository"]
        .as_str()
        .ok_or("upstream repository")?;
    if repository != "https://github.com/mccutchen/go-httpbin"
        || revision.len() != 40
        || !revision.bytes().all(|v| v.is_ascii_hexdigit())
    {
        return Err("fixed untouched fixture identity rejected".into());
    }
    git(&checkout, &["init", "--quiet"])?;
    git(&checkout, &["fetch", "--depth=1", repository, revision])?;
    git(
        &checkout,
        &["checkout", "--detach", "--quiet", "FETCH_HEAD"],
    )?;
    if String::from_utf8(git(&checkout, &["rev-parse", "HEAD"])?)?.trim() != revision
        || !git(&checkout, &["status", "--porcelain"])?.is_empty()
    {
        return Err("untouched fixture checkout mismatch".into());
    }
    let inventory = git(&checkout, &["ls-files", "-z"])?;
    let build_context = context.0.join("tracked-source");
    fs::create_dir(&build_context)?;
    let mut files = Vec::new();
    for relative in std::str::from_utf8(&inventory)?
        .split('\0')
        .filter(|s| !s.is_empty())
    {
        if relative.split('/').any(|p| p == "..") || Path::new(relative).is_absolute() {
            return Err("upstream inventory path refused".into());
        }
        let path = checkout.join(relative);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 16 * 1024 * 1024
        {
            return Err("upstream fixture file type/size refused".into());
        }
        let bytes = fs::read(path)?;
        let destination = build_context.join(relative);
        fs::create_dir_all(destination.parent().ok_or("upstream file parent")?)?;
        fs::write(destination, &bytes)?;
        files.push(json!({"path":relative,"bytes":bytes.len(),"sha256":snapshot::digest(&bytes)}));
    }
    let entry = contract["fixture"]["entrypoint"]
        .as_str()
        .ok_or("upstream entrypoint")?;
    if snapshot::digest(&fs::read(checkout.join(entry))?)
        != contract["fixture"]["entrypoint_sha256"]
    {
        return Err("upstream entrypoint digest mismatch".into());
    }
    let fixture_sha = snapshot::digest(&serde_json::to_vec(&files)?);
    let image = compiler["compiler_image"]
        .as_str()
        .ok_or("compiler image")?;
    lab.docker
        .call(&["pull", "--platform", "linux/amd64", image])?;
    let manifest = lab
        .docker
        .call(&["buildx", "imagetools", "inspect", "--raw", image])?;
    let manifest = decode_strict_json(manifest.as_bytes(), 262144)?;
    if !manifest["manifests"].as_array().is_some_and(|v| {
        v.iter().any(|m| {
            m["platform"]["os"] == "linux"
                && m["platform"]["architecture"] == "amd64"
                && m["digest"] == compiler["linux_amd64_manifest_digest"]
        })
    }) {
        return Err("selected compiler child manifest mismatch".into());
    }
    let inspected: Value = serde_json::from_str(&lab.docker.call(&["image", "inspect", image])?)?;
    if inspected[0]["Architecture"] != "amd64"
        || inspected[0]["Os"] != "linux"
        || !inspected[0]["RepoDigests"].as_array().is_some_and(|v| {
            v.iter().any(|d| {
                d.as_str().is_some_and(|d| {
                    d.ends_with(
                        compiler["manifest_index_digest"]
                            .as_str()
                            .unwrap_or("missing"),
                    )
                })
            })
        })
    {
        return Err("actual compiler platform/index identity mismatch".into());
    }
    let version = lab.docker.call(&[
        "run",
        "--rm",
        "--network",
        "none",
        "--read-only",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges:true",
        image,
        "go",
        "version",
    ])?;
    if version.trim()
        != format!(
            "go version {} linux/amd64",
            compiler["expected_go_version"]
                .as_str()
                .ok_or("Go version")?
        )
    {
        return Err("actual compiler version mismatch".into());
    }
    let output = context.0.join("export");
    fs::create_dir(&output)?;
    let recipe = lab.config_file("adapters/go/Dockerfile.fixture")?;
    let export = format!("type=local,dest={}", output.display());
    lab.docker.call(&[
        "buildx",
        "build",
        "--platform",
        "linux/amd64",
        "--network",
        "none",
        "--file",
        &recipe,
        "--output",
        &export,
        build_context.to_str().ok_or("fixture context UTF8")?,
    ])?;
    let binary = output.join("go-httpbin");
    let metadata = fs::symlink_metadata(&binary)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 64 * 1024 * 1024
    {
        return Err("Go exported artifact refused".into());
    }
    let identity = json!({"schema":"humanymous.go-fixture-build/1","fixture_revision":revision,"full_fixture_sha256":fixture_sha,"files":files,"binary_sha256":snapshot::digest(&fs::read(binary)?),"compiler":compiler,"actual_go_version":version.trim(),"compiler_image":inspected[0],"compiler_manifest":manifest,"project_authored_go_source":false,"dependency_mode":"stdlib-only-mod-readonly-network-none","full_product_qualified":false});
    snapshot::write(&lab.evidence, "go-fixture-build.json", &identity)?;
    Ok(Fixture {
        context,
        export: output.to_string_lossy().into_owned(),
        identity,
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn local_act_and_unknown_runner_cannot_be_labelled_hosted() {
        use crate::docker::hosted_context;
        assert!(hosted_context(
            Some("true"),
            Some("Linux"),
            Some("github-hosted"),
            false
        ));
        assert!(!hosted_context(
            Some("true"),
            Some("Linux"),
            Some("github-hosted"),
            true
        ));
        for environment in [None, Some("self-hosted"), Some("unknown")] {
            assert!(!hosted_context(
                Some("true"),
                Some("Linux"),
                environment,
                false
            ));
        }
        assert!(!hosted_context(
            None,
            Some("Linux"),
            Some("github-hosted"),
            false
        ));
        assert!(!hosted_context(
            Some("true"),
            Some("Windows"),
            Some("github-hosted"),
            false
        ));
    }
}
