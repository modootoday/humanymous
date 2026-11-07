use crate::{Options, evidence, fixture::Directory, harness};
use humanymous_test_support::process::capture;
use serde_json::{Value, json};
use std::{fs, io, path::Path, process::Command, time::Duration};

pub fn binary(directory: &Path, name: &str) -> std::path::PathBuf {
    directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}
fn assert(condition: bool, message: &str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
fn names(path: &Path) -> io::Result<Vec<String>> {
    let mut files = fs::read_dir(path)?
        .map(|r| r.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<io::Result<Vec<_>>>()?;
    files.sort();
    Ok(files)
}
fn cli(options: &Options) -> io::Result<Value> {
    let fixture = Directory::new()?;
    let root = fixture.path();
    let artifact = root.join("artifact");
    fs::write(&artifact, b"owned nonexecuted native provisioning fixture")?;
    let digest = evidence::digest(&artifact)?;
    let before = names(root)?;
    let mut rows = Vec::new();
    for (action, dry, valid) in [
        ("doctor", false, true),
        ("install", true, true),
        ("doctor", false, false),
    ] {
        let mut command = Command::new(binary(&options.binaries, "humanymous"));
        command
            .arg(action)
            .arg("--artifact")
            .arg(&artifact)
            .arg("--destination")
            .arg(root.join("installed"))
            .args([
                "--sha256",
                if valid {
                    &digest
                } else {
                    "0000000000000000000000000000000000000000000000000000000000000000"
                },
                "--local-management-verified",
            ]);
        if dry {
            command.arg("--dry-run");
        }
        let output = capture(&mut command, b"", Duration::from_secs(15), 65536)?;
        assert(
            output.status.success() == valid,
            "CLI doctor/dry-run/digest prerequisite mismatch",
        )?;
        assert(
            names(root)? == before && evidence::digest(&artifact)? == digest,
            "read-only CLI mutated fixture",
        )?;
        rows.push(json!({"action":action,"dry_run":dry,"digest_valid":valid,"exit_code":output.status.code(),"snapshot_unchanged":true}));
    }
    Ok(json!(rows))
}
fn reject_invalid(options: &Options) -> io::Result<Value> {
    let mut rows = Vec::new();
    for name in ["humanymous-runtime", "humanymous-authority"] {
        let output = capture(
            &mut Command::new(binary(&options.binaries, name)),
            b"{}",
            Duration::from_secs(15),
            65536,
        )?;
        assert(
            !output.status.success() && output.stdout.is_empty(),
            "invalid configuration produced readiness/success",
        )?;
        rows.push(json!({"binary":name,"exit_code":output.status.code(),"readiness_bytes":output.stdout.len()}));
    }
    Ok(json!(rows))
}
pub fn run(options: &Options) -> io::Result<i32> {
    let identity = evidence::identity(options.expected_sid.as_deref())?;
    crate::fixture::actor_environment(options.expected_sid.as_deref())?;
    let source = match &options.source {
        Some(path) => serde_json::from_slice(&fs::read(path)?)?,
        None => evidence::source(false)?,
    };
    evidence::verify_snapshot(&source)?;
    let mut report = json!({"schema":1,"kind":"native","status":"blocked","qualification":"component-only","source_sha":source["source_sha"],"source":source,"documentation_review_sha256":evidence::digest(Path::new("docs/reviews.json"))?,"documentation_examples_executed":false,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"actor":identity,"errors":0,"skips":0,"cases":[],"missing_required_cases":["local-only-admin","documentation-examples"],"unsupported":["raw-tls-h2-positive-capture","real-browser-collector","OS-service-activation","remote-machine-peer-denial"],"full_runtime_qualified":false});
    report["missing_required_cases"]
        .as_array_mut()
        .unwrap()
        .extend(
            crate::service::catalog::CASES
                .iter()
                .map(|case| json!(case.id)),
        );
    let result = (|| -> io::Result<()> {
        let manifests = match &options.manifest {
            Some(path) => serde_json::from_slice(&fs::read(path)?)?,
            None => harness::prepare(
                &options.report,
                &options.binaries,
                harness::Scope::Native,
                &report["source"],
            )?,
        };
        for name in ["humanymous", "humanymous-runtime", "humanymous-authority"] {
            let path = binary(&options.binaries, name).canonicalize()?;
            let products = manifests["products"]
                .as_array()
                .ok_or_else(|| io::Error::other("compiled products absent"))?;
            let row = products
                .iter()
                .find(|r| r["target"] == name)
                .ok_or_else(|| io::Error::other("required compiled binary missing"))?;
            assert(
                row["executable"] == json!(path) && row["sha256"] == evidence::digest(&path)?,
                "product path/digest differs from Cargo build",
            )?;
        }
        report["manifest"] = manifests.clone();
        harness::execute(
            &manifests["harnesses"],
            &options.report,
            &mut report["coverage"],
            harness::Scope::Native,
        )?;
        report["skips"] = json!(
            report["coverage"]["harnesses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["ignored"].as_u64().unwrap_or(0))
                .sum::<u64>()
        );
        let cli = cli(options)?;
        let refusals = reject_invalid(options)?;
        report["cases"] = json!([
            {"id":"startup","status":"passed","evidence":"actual runtime native_process harness readiness assertion"},
            {"id":"configuration-invalid","status":"passed","evidence":refusals},
            {"id":"doctor","status":"passed","evidence":cli},
            {"id":"install-dry-run-no-mutation","status":"passed","evidence":"CLI process snapshots and artifact bytes unchanged"},
            {"id":"restart-replay","status":"passed","evidence":"runtime/authority native_process harness replay and restart assertions"},
            {"id":"storage-failure-closed","status":"passed","evidence":"authority concurrent_nonce_reservation_and_durable_failure actual-process assertion"},
            {"id":"capture-prerequisite-refusal","status":"passed","evidence":"authority real_ipc_auth_replay_body_scope_capture_and_restart_epoch actual-process assertion"}
        ]);
        evidence::verify_snapshot(&report["source"])?;
        Ok(())
    })();
    if let Err(error) = result {
        report["status"] = "failed".into();
        report["errors"] = 1.into();
        report["error"] = error.to_string().into();
    }
    evidence::write(&options.report, &report)?;
    Ok(if report["errors"] == 0 { 2 } else { 1 })
}
