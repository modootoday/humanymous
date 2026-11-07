//! Trusted-source producer wiring and a receiver bound to separately captured
//! official API metadata. Supplied metadata is not itself a signed attestation.
use super::{
    Bundle, actions_log,
    admission::{self, Expected, Profile},
    private_io,
};
use crate::{Result, command, require, source, write_json};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Component, Path},
};

fn read(root: &Path, relative: &str, cap: usize) -> Result<Vec<u8>> {
    require(
        source::safe_relative(relative),
        "header member path refused",
    )?;
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        require(
            matches!(component, Component::Normal(_)),
            "header member component refused",
        )?;
        path.push(component);
        let meta = fs::symlink_metadata(&path).map_err(|_| "header member missing")?;
        require(!meta.file_type().is_symlink(), "header member link refused")?;
    }
    bounded(&path, cap)
}
fn bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).map_err(|_| "header evidence file missing")?;
    require(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= cap as u64,
        "header evidence file bound refused",
    )?;
    let mut out = Vec::new();
    fs::File::open(path)
        .map_err(|_| "header evidence open failed")?
        .take(cap as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| "header evidence read failed")?;
    require(out.len() <= cap, "header evidence read bound refused")?;
    Ok(out)
}
fn strict(bytes: &[u8], cap: usize) -> Result<Value> {
    humanymous_test_support::json::decode_strict_json(bytes, cap)
        .map_err(|_| "header metadata JSON refused".into())
}
fn lines(bytes: &[u8], count: usize) -> Result<Vec<&str>> {
    require(
        bytes.ends_with(b"\n") && bytes.iter().all(|b| *b == b'\n' || b.is_ascii_graphic()),
        "header private sidecar encoding refused",
    )?;
    let rows: Vec<_> = std::str::from_utf8(bytes)
        .map_err(|_| "header private sidecar UTF-8 refused")?
        .lines()
        .collect();
    require(
        rows.len() == count,
        "header private sidecar line count refused",
    )?;
    Ok(rows)
}
fn identity(dir: &Path, p: &Value) -> Result<Value> {
    let bytes = read(dir, "header-compiler.identity.txt", 8192)?;
    let rows = lines(&bytes, 7)?;
    require(
        rows[0] == "schema=humanymous.header-compiler-identity/1",
        "header private identity schema refused",
    )?;
    let mut out = json!({});
    for (language, base) in [("c", 1), ("cpp", 4)] {
        let path = rows[base]
            .strip_prefix(&format!("{language}.path="))
            .ok_or("header compiler path key refused")?;
        let before = rows[base + 1]
            .strip_prefix(&format!("{language}.sha256.before="))
            .ok_or("header compiler before key refused")?;
        let after = rows[base + 2]
            .strip_prefix(&format!("{language}.sha256.after="))
            .ok_or("header compiler after key refused")?;
        require(
            p["compiler"]["resolvedPath"] == path
                && p["compiler"]["sha256"] == before
                && before == after,
            "header actual compiler profile refused",
        )?;
        let canonical =
            fs::canonicalize(path).map_err(|_| "header actual compiler path unavailable")?;
        require(
            canonical == Path::new(path) && source::file(&canonical)? == before,
            "header independent compiler bytes refused",
        )?;
        out[language] = json!({"resolved_path":path,"before_sha256":before,"after_sha256":after});
    }
    Ok(out)
}
fn argv(dir: &Path, p: &Value) -> Result<(Value, Value)> {
    let mut hashes = json!({});
    let mut templates = json!({});
    let header = fs::canonicalize(dir.join("native-abi-v1.h"))
        .map_err(|_| "header actual argv input unavailable")?;
    let header = header
        .to_str()
        .ok_or("header actual argv input path refused")?;
    for (language, kind, standard, command) in [
        ("c", "c", "-std=c11", "cCommand"),
        ("cpp", "c++", "-std=c++11", "cppCommand"),
    ] {
        let bytes = read(dir, &format!("header-compiler.{language}-argv.txt"), 16384)?;
        let rows = lines(&bytes, 9)?;
        require(
            rows[0] == "schema=humanymous.header-compiler-argv/1",
            "header actual argv schema refused",
        )?;
        let program = rows[1]
            .strip_prefix("program=")
            .ok_or("header actual argv program absent")?;
        let path = Path::new(program);
        require(
            path.is_absolute()
                && path.file_name().and_then(|s| s.to_str()) == p["compiler"][command].as_str()
                && fs::canonicalize(path).ok().as_deref()
                    == p["compiler"]["resolvedPath"].as_str().map(Path::new),
            "header actual argv compiler refused",
        )?;
        let expected = [
            "-x",
            kind,
            "-fsyntax-only",
            "-Xclang",
            "-fdump-record-layouts-complete",
            standard,
            header,
        ];
        for (id, arg) in expected.iter().enumerate() {
            require(
                rows[id + 2].strip_prefix(&format!("arg{id:02}=")) == Some(*arg),
                "header actual argv source/header coupling refused",
            )?;
        }
        hashes[language] = json!(source::sha(&bytes));
        templates[language] = json!([
            "$COMPILER_INVOCATION",
            "-x",
            kind,
            "-fsyntax-only",
            "-Xclang",
            "-fdump-record-layouts-complete",
            standard,
            "$HEADER_FILE_ID_0"
        ]);
    }
    Ok((hashes, templates))
}
fn tool(root: &Path, dir: &Path, name: &str, program: &str) -> Result<String> {
    let output = dir.join(format!("header-{name}-identity.log"));
    let code = command::json_log(
        root,
        "timeout",
        &["--signal=TERM", "--kill-after=15s", "15s", program, "-vV"],
        &output,
    )?;
    require(
        code == 0 && bounded(&output.with_extension("stderr.log"), 65536)?.is_empty(),
        "header tool identity command failed",
    )?;
    let bytes = bounded(&output, 65536)?;
    require(!bytes.is_empty(), "header tool identity output missing")?;
    Ok(source::sha(&bytes))
}
fn config(root: &Path, dir: &Path) -> Result<String> {
    require(
        std::env::var("CARGO_BUILD_JOBS").as_deref() == Ok("1")
            && std::env::var("CARGO_INCREMENTAL").as_deref() == Ok("0"),
        "header fixed build environment refused",
    )?;
    let forbidden = [
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_BUILD_DIR",
        "CARGO_BUILD_RUSTFLAGS",
    ];
    require(
        forbidden
            .iter()
            .all(|name| std::env::var_os(name).is_none()),
        "header unknown build override refused",
    )?;
    let mut paths = BTreeSet::new();
    let mut current = Some(root);
    while let Some(path) = current {
        paths.insert(path.join(".cargo/config"));
        paths.insert(path.join(".cargo/config.toml"));
        current = path.parent();
    }
    if let Some(home) = std::env::var_os("CARGO_HOME") {
        paths.insert(Path::new(&home).join("config"));
        paths.insert(Path::new(&home).join("config.toml"));
    } else if let Some(home) = std::env::var_os("HOME") {
        paths.insert(Path::new(&home).join(".cargo/config"));
        paths.insert(Path::new(&home).join(".cargo/config.toml"));
    }
    let mut rows = Vec::new();
    for path in paths {
        let text = path.to_str().ok_or("header config path refused")?;
        let hash = if path.exists() {
            Some(source::sha(&bounded(&path, 1024 * 1024)?))
        } else {
            None
        };
        rows.push(json!({"path":text,"sha256":hash}));
    }
    let record = json!({"schema":"humanymous.header-private-config/1","jobs":1,"incremental":0,"unknown_overrides_present":false,"files":rows});
    write_json(&dir.join("header-private-config.json"), &record)?;
    Ok(source::sha(
        &serde_json::to_vec(&record).map_err(|_| "header config encoding failed")?,
    ))
}

pub(crate) struct Prepared {
    pub frames: String,
    pub summary: Value,
}
pub(crate) fn parent(path: &Path) -> Result<()> {
    private_io::check_parent(path)?;
    println!(
        "{}",
        json!({"schema":"humanymous.header-private-parent-observation/1","verified_now":true,"held_during_check_only":true,"future_lease_claimed":false,"trusted_ancestors_required":true,"native_producer_qualified":false})
    );
    Ok(())
}
pub(crate) fn plan(root: &Path, sha: &str) -> Result<()> {
    require(
        source::head(root)? == sha,
        "header source plan compile-time checkout mismatch",
    )?;
    let expected = Expected::source(root, sha)?;
    let snapshot = source::snapshot(root)?;
    let bytes =
        serde_json::to_vec_pretty(&snapshot).map_err(|_| "header source plan encoding failed")?;
    let profile = Profile::compiled()?;
    admission::snapshot(&bytes, &expected, profile.limits()?.caps[14])?;
    println!(
        "{}",
        json!({"schema":"humanymous.header-candidate-source-plan/1","source_sha":sha,"profile_sha256":profile.sha256(),"canonical_paths":expected.inputs.len(),"original_snapshot_bytes":bytes.len(),"original_snapshot_sha256":source::sha(&bytes),"working_bytes_equal_canonical_git":true,"hosted_context_generated":false,"dispatch_approved":false})
    );
    Ok(())
}
fn generation_exit(report: &Value) -> Result<i64> {
    let code = report["commands"]["header-generation"]["exit_code"]
        .as_i64()
        .ok_or("header generator execution status absent")?;
    require(code == 0, "header generator execution failed")?;
    Ok(code)
}
pub(crate) fn observe(root: &Path, dir: &Path) -> Result<Value> {
    let profile = Profile::compiled()?;
    let p = profile.value();
    let (hashes, templates) = argv(dir, p)?;
    Ok(
        json!({"compiler_identity":identity(dir,p)?,"original_argv_sha256":hashes,"argv_template":templates,
        "rustc_identity_sha256":tool(root,dir,"rustc","rustc")?,"cargo_identity_sha256":tool(root,dir,"cargo","cargo")?,
        "config_manifest_sha256":config(root,dir)?}),
    )
}
pub(crate) fn prepare(root: &Path, dir: &Path, report: &Value) -> Result<Prepared> {
    let profile = Profile::compiled()?;
    let p = profile.value();
    let limits = profile.limits()?;
    require(
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var("GITHUB_JOB").as_deref() == Ok("header")
            && std::env::var_os("ACT").is_none(),
        "header proof producer venue refused",
    )?;
    let positive = |name: &str| -> Result<u64> {
        let text = std::env::var(name).map_err(|_| "header hosted context missing")?;
        let n = text
            .parse::<u64>()
            .map_err(|_| "header hosted context refused")?;
        require(
            n > 0 && n.to_string() == text,
            "header hosted context noncanonical",
        )?;
        Ok(n)
    };
    let sha = source::head(root)?;
    let run = positive("GITHUB_RUN_ID")?;
    let attempt = positive("GITHUB_RUN_ATTEMPT")?;
    require(
        std::env::var("GITHUB_SHA").as_deref() == Ok(sha.as_str()),
        "header proof source context refused",
    )?;
    let expected = Expected::git(root, &sha, run, attempt, "header")?;
    let mut files = Vec::new();
    for (id, path) in profile.paths().iter().enumerate() {
        files.push(read(dir, path, limits.caps[id])?);
    }
    let observed = &report["header"]["proof_observation"];
    let context = json!({"schema":"humanymous.header-proof-context/1","source_sha":sha,"run_id":run,"run_attempt":attempt,"job_name":"header",
        "profile_sha256":profile.sha256(),"declared_image":p["image"],"image_identity_meaning":"declared-workflow-digest-not-attestation",
        "os":"linux","arch":"x86_64","compiler_identity":observed["compiler_identity"],"generator_version":report["header"]["generator_version"].as_str().map(|s|format!("cbindgen {s}")),
        "generator_sha256":report["header"]["generator_sha256"],"generator_exit_code":generation_exit(report)?,
        "owner_exit_code":report["commands"]["header-conformance"]["exit_code"],"owner_success_marker":String::from_utf8(read(dir,"header-conformance.log",65536)?).map_err(|_|"header owner marker encoding refused")?.trim_end_matches('\n'),
        "rustc_identity_sha256":observed["rustc_identity_sha256"],"cargo_identity_sha256":observed["cargo_identity_sha256"],
        "config_manifest_sha256":observed["config_manifest_sha256"],"environment_profile":"jobs1-incremental0","recipe_sha256":expected.inputs.get(p["header"]["owningVerifier"].as_str().ok_or("header owning recipe absent")?).ok_or("header canonical recipe absent")?.sha256,
        "cargo_lock_sha256":expected.inputs.get("Cargo.lock").ok_or("header canonical lock absent")?.sha256,
        "original_argv_sha256":observed["original_argv_sha256"],"argv_template":observed["argv_template"],"argv_template_meaning":"source-bound-template-not-original-argv"});
    let bundle = Bundle { context, files };
    let frames = super::encode(&bundle, &limits, |b| {
        admission::validate(b, &profile, &expected)
    })?;
    write_json(&dir.join("header-proof-context.json"), &bundle.context)?;
    fs::write(dir.join("header-proof-frames.log"), frames.as_bytes())
        .map_err(|_| "header prepared transport write failed")?;
    Ok(Prepared {
        summary: json!({"state":"prepared","profile_sha256":profile.sha256(),"canonical_path_count":expected.inputs.len(),"source_before_bytes":bundle.files[14].len(),"source_after_bytes":bundle.files[15].len(),"original_files":16,"prepared_transport_sha256":source::sha(frames.as_bytes()),"requires_official_receiver_verification":true}),
        frames,
    })
}

fn binding(run: &Value, job: &Value) -> Result<(String, u64, u64, u64)> {
    let sha = run["head_sha"]
        .as_str()
        .ok_or("header official source absent")?;
    let id = run["id"].as_u64().ok_or("header official run absent")?;
    let attempt = run["run_attempt"]
        .as_u64()
        .ok_or("header official attempt absent")?;
    let job_id = job["id"].as_u64().ok_or("header official job absent")?;
    require(
        crate::lab_receipts::digest(&json!(sha), 40)
            && id > 0
            && attempt > 0
            && job_id > 0
            && run["repository"]["full_name"] == "modootoday/humanymous"
            && run["status"] == "completed"
            && run["conclusion"] == "success"
            && job["run_id"] == id
            && job["run_attempt"] == attempt
            && job["head_sha"] == sha
            && job["name"] == "header"
            && job["status"] == "completed"
            && job["conclusion"] == "success"
            && job["labels"] == json!(["ubuntu-24.04"])
            && job["runner_group_name"] == "GitHub Actions",
        "header official run/job binding refused",
    )?;
    Ok((sha.into(), id, attempt, job_id))
}
pub(crate) fn receive(
    root: &Path,
    log: &Path,
    run_metadata: &Path,
    job_metadata: &Path,
    parent: &Path,
) -> Result<()> {
    let profile = Profile::compiled()?;
    let limits = profile.limits()?;
    let raw_run = bounded(run_metadata, 1024 * 1024)?;
    let raw_job = bounded(job_metadata, 1024 * 1024)?;
    let (sha, run, attempt, job) = binding(
        &strict(&raw_run, 1024 * 1024)?,
        &strict(&raw_job, 1024 * 1024)?,
    )?;
    let expected = Expected::git(root, &sha, run, attempt, "header")?;
    let raw_log = bounded(log, limits.log)?;
    let protocol = actions_log::extract(&raw_log, &limits)?;
    let bundle = super::decode(&protocol, &limits, |b| {
        admission::validate(b, &profile, &expected)
    })?;
    let manifest: Vec<_> = profile
        .paths()
        .iter()
        .zip(&bundle.files)
        .map(|(path, bytes)| json!({"path":path,"bytes":bytes.len(),"sha256":source::sha(bytes)}))
        .collect();
    let receipt = json!({"schema":"humanymous.reconstructed-header-proof/1","complete":true,"source_sha":sha,"run_id":run,"run_attempt":attempt,"official_job_id":job,
        "profile_sha256":profile.sha256(),"original_files":manifest,"retained_api_log_sha256":source::sha(&raw_log),
        "official_run_metadata_sha256":source::sha(&raw_run),"official_job_metadata_sha256":source::sha(&raw_job),"metadata_requires_trusted_out_of_band_official_capture":true,
        "signed_attestation":false,"declared_image_attestation":false,"trusted_ancestors_and_same_owner_nonreplacement_required":true,"crash_durability_after_completion_promotion_claimed":false,
        "requires_successful_receiver_exit_and_verified_marker":true,"full_product_qualified":false});
    let name = format!("header-proof-{run}-{job}-{attempt}");
    let output = private_io::reconstruct(parent, &name, profile.paths(), &bundle.files, &receipt)?;
    println!("header proof reconstructed: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generator_status_uses_actual_generation_not_the_version_probe() {
        let mut report = json!({"commands":{"cbindgen-version":{"exit_code":0},"header-generation":{"exit_code":101}}});
        assert!(generation_exit(&report).is_err());
        report["commands"]["header-generation"]["exit_code"] = json!(0);
        report["commands"]["cbindgen-version"]["exit_code"] = json!(101);
        assert_eq!(generation_exit(&report).unwrap(), 0);
        report["commands"]["header-generation"] = Value::Null;
        assert!(generation_exit(&report).is_err());
    }
    #[test]
    fn supplied_official_metadata_must_bind_successful_source_job_attempt_and_hosted_tuple() {
        let run = json!({"id":1,"head_sha":"a".repeat(40),"run_attempt":2,"repository":{"full_name":"modootoday/humanymous"},"status":"completed","conclusion":"success"});
        let job = json!({"id":3,"run_id":1,"head_sha":"a".repeat(40),"run_attempt":2,"name":"header","status":"completed","conclusion":"success","labels":["ubuntu-24.04"],"runner_group_name":"GitHub Actions"});
        assert!(binding(&run, &job).is_ok());
        for key in [
            "id",
            "run_id",
            "head_sha",
            "run_attempt",
            "name",
            "status",
            "conclusion",
            "labels",
            "runner_group_name",
        ] {
            let mut bad = job.clone();
            bad[key] = Value::Null;
            assert!(binding(&run, &bad).is_err());
        }
        let mut failed = run.clone();
        failed["conclusion"] = json!("failure");
        assert!(binding(&failed, &job).is_err());
    }
}
