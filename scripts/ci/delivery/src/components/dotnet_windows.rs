//! Windows backend of the `standard-user-dotnet-prebuilt` custody provider
//! (SoT dotnet-protected-host-launch 4.1, owner decision D5(a), issue 75
//! comment 6998447599). The elevated hosted parent is the windows-native-runner;
//! this component only invokes it and checks its typed report.
use super::dotnet_host::{self, sdk};
use crate::{Result, command, read_json, require, source};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const RUNNER_MANIFEST: &str = "scripts/ci/windows-native-runner/Cargo.toml";
const FIXTURE_SCHEMA: &str = "humanymous.dotnet-n1-fixture/1";

/// Every fact the N1 fixture must report before anything else runs on Windows.
const N1_FACTS: [(&str, bool); 12] = [
    ("profile_absent_before", true),
    ("frame_written", true),
    ("natural_exit", true),
    ("job_empty_observed", true),
    ("stdout_joined", true),
    ("stderr_joined", true),
    ("child_line_exact", true),
    ("stderr_marker_exact", true),
    ("profile_absent_after", true),
    ("account_deleted", true),
    ("scratch_deleted", true),
    ("retained_contained", false),
];

/// The N1 report is accepted only with every fact observed and no failure.
pub fn n1_report_ok(report: &Value) -> bool {
    let facts = &report["facts"];
    report["schema"] == FIXTURE_SCHEMA
        && report["status"] == "passed"
        && report["failures"].as_array().is_some_and(Vec::is_empty)
        && facts["start"] == "resumed-in-job"
        && facts["child_exit_code"] == 0
        && N1_FACTS.iter().all(|(key, required)| {
            let value = &facts[*key];
            if *required {
                *value == true
            } else {
                value.is_null()
            }
        })
}

/// The N3 creator-channel rule (SoT 3.1.6) lives in a Windows-only module, so
/// this Windows step is where its unit test actually executes.
pub const N3_UNIT: &str = "native_security::storage::actor_context::tests::creator_channel_binding_requires_one_piped_frame_and_exact_bytes";

/// D5-N4 option (a) (issue #75 comment 6999924070): the elevated parent sets
/// owner Administrators at creation and a foreign owner is refused.
pub const OWNER_AT_CREATION_UNIT: &str = "windows_object::create_file::native_tests::actual_elevated_administrators_owner_at_creation_and_foreign_owner_refused";

/// Run one Windows unit test alone and require its exact single-test footer.
fn unit(
    root: &Path,
    dir: &Path,
    report: &mut Value,
    (stage, package, features, name): (&str, &str, &str, &str),
    ignored: bool,
) -> Result<()> {
    report["stage"] = json!(stage);
    let log = dir.join(format!("dotnet-{stage}.log"));
    let mut args = vec![
        "test",
        "--locked",
        "-p",
        package,
        "--features",
        features,
        "--lib",
        "--",
        "--exact",
        name,
    ];
    if ignored {
        args.push("--ignored");
    }
    args.push("--test-threads=1");
    let code = command::log(root, "cargo", &args, &log)?;
    let text = std::fs::read_to_string(&log).map_err(|error| error.to_string())?;
    let passed = code == 0 && super::pilots::passed(&text, name);
    report["commands"][stage] = json!({"exit_code":code,"log_sha256":source::file(&log)?});
    if !passed {
        report["commands"][stage]["failure_excerpt"] = json!(failure_excerpt(&text));
    }
    report[format!("{}_passed", stage.replace('-', "_"))] = json!(passed);
    require(passed, "Windows unit did not pass alone")
}

/// Bounded panic/compiler lines of a failed unit run, for diagnosis only.
fn failure_excerpt(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut after = 0;
    for line in text.lines() {
        if line.contains("panicked at") || line.starts_with("error") || line.contains("FAILED") {
            after = 4;
        }
        if after > 0 {
            after -= 1;
            lines.push(line.chars().take(300).collect());
        }
        if lines.len() >= 24 {
            break;
        }
    }
    lines
}

/// Run the N3 unit test alone and require its exact single-test footer.
fn n3_unit(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    unit(
        root,
        dir,
        report,
        ("n3-unit", "humanymous-runtime", "native-security", N3_UNIT),
        false,
    )
}

/// SoT 4.1 N1 fixture: the secondary-logon service delivers the std handles
/// of the NoProfile actor launch. A failure stops the Windows provider; the
/// `DuplicateHandle` fallback is not approved.
pub fn stdio_fixture(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    n3_unit(root, dir, report)?;
    report["stage"] = json!("n1-fixture");
    let raw = dir.join("dotnet-n1-fixture.json");
    let log = dir.join("dotnet-n1-fixture.log");
    let raw_text = raw.to_str().ok_or("non-UTF-8 fixture report path")?;
    let code = command::log(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "--manifest-path",
            RUNNER_MANIFEST,
            "--",
            "dotnet-stdio-fixture",
            "--report",
            raw_text,
        ],
        &log,
    )?;
    report["commands"]["n1-fixture"] = json!({"exit_code":code,"log_sha256":source::file(&log)?});
    let value = read_json(&raw).unwrap_or(Value::Null);
    report["n1_fixture"] = json!({
        "status": value["status"],
        "facts": value["facts"],
        "failures": value["failures"],
    });
    require(code == 0, "N1 fixture runner failed")?;
    require(
        n1_report_ok(&value),
        "N1 std-handle delivery not observed: Windows provider stops",
    )
}

/// Schema of the runner's `dotnet-prebuilt` report.
const PREBUILT_SCHEMA: &str = "humanymous.dotnet-prebuilt-run/1";
/// App closure roles and names the runner's `/2` input requires, in order.
const APP_FILES: [(&str, &str); 7] = [
    ("apphost", "Humanymous.HostPilot.exe"),
    ("assembly", "Humanymous.HostPilot.dll"),
    ("deps", "Humanymous.HostPilot.deps.json"),
    ("runtimeconfig", "Humanymous.HostPilot.runtimeconfig.json"),
    ("binding", "Humanymous.Native.dll"),
    ("facade", "humanymous_dotnet_adapter_export.dll"),
    ("generic", "humanymous_native_abi_export.dll"),
];

/// The runner report is accepted only as a clean pass of the exact selector.
pub fn prebuilt_report_ok(report: &Value, selector: &str, tests: &[&str]) -> bool {
    let rows = report["tests"].as_array();
    report["schema"] == PREBUILT_SCHEMA
        && report["status"] == "passed"
        && report["selector"] == selector
        && report["failures"].as_array().is_some_and(Vec::is_empty)
        && report["budget_ms"]["renewed"] == false
        && report["facts"]["start"] == "resumed-in-job"
        && report["facts"]["actor_exit_code"] == 0
        && report["facts"]["natural_exit"] == true
        && report["facts"]["job_empty_observed"] == true
        && report["facts"]["readers_joined"] == true
        && report["facts"]["profile_absent_before"] == true
        && report["facts"]["profile_absent_after"] == true
        && report["facts"]["private_absent"] == true
        && report["facts"]["account_deleted"] == true
        && report["facts"]["scratch_deleted"] == true
        && report["facts"]["artifact_root_removed"] == true
        && report["facts"]["carrier_finished"] == true
        && report["child"]["producer_bound"] == true
        && rows.is_some_and(|rows| {
            rows.len() == tests.len()
                && rows
                    .iter()
                    .zip(tests)
                    .all(|(row, name)| row["name"] == *name && row["passed"] == true)
        })
}

fn text(path: &Path) -> Result<String> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "non-UTF-8 path".to_owned())
}
fn step(root: &Path, dir: &Path, report: &mut Value, stage: &str, args: &[&str]) -> Result<()> {
    report["stage"] = json!(stage);
    let log = dir.join(format!("{stage}.log"));
    let code = command::json_log(root, "cargo", args, &log)?;
    report["commands"][stage] = json!({"exit_code":code,"stdout_sha256":source::file(&log)?,"stderr_sha256":source::file(&log.with_extension("stderr.log"))?});
    require(code == 0, "owned component command failed")
}

/// The declared SDK download pilot alone on Windows (no coreutils timeout).
fn sdk_pilot(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    report["stage"] = json!("dotnet-sdk-pilot");
    let log = dir.join("dotnet-sdk-pilot.log");
    let code = command::log(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "humanymous-delivery",
            "--bin",
            "humanymous-delivery",
            "--",
            "--exact",
            dotnet_host::SDK_PILOT,
            "--ignored",
            "--test-threads=1",
        ],
        &log,
    )?;
    let text = std::fs::read_to_string(&log).map_err(|e| e.to_string())?;
    let passed = code == 0 && super::pilots::passed(&text, dotnet_host::SDK_PILOT);
    report["commands"]["dotnet-sdk-pilot"] =
        json!({"exit_code":code,"log_sha256":source::file(&log)?});
    report["dotnet_sdk_pilot_passed"] = json!(passed);
    require(passed, "dotnet SDK download pilot did not pass alone")
}

/// A fresh provider directory under the runner temp root.
fn provider_dir(prefix: &str) -> Result<PathBuf> {
    let base = std::env::var_os("RUNNER_TEMP")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = base.join(format!("{prefix}-{}", &dotnet_host::nonce()?[..12]));
    std::fs::create_dir(&path).map_err(|e| format!("provider dir: {e}"))?;
    Ok(path)
}

/// Build phase: pinned SDK, native exports, hosted harness and the win-x64
/// FDD app. Returns the runner's closed `/2` input.
fn prepare(
    root: &Path,
    dir: &Path,
    report: &mut Value,
    work: &Path,
    sdk_private: &Path,
) -> Result<Value> {
    let sdk = sdk::acquire(sdk_private, report)?;
    sdk.check(root, dir, report)?;
    step(
        root,
        dir,
        report,
        "dotnet-native-build",
        &[
            "build",
            "--locked",
            "-p",
            "humanymous-dotnet-adapter-export",
            "-p",
            "humanymous-native-abi-export",
        ],
    )?;
    step(
        root,
        dir,
        report,
        "dotnet-harness-build",
        &[
            "test",
            "--locked",
            "-p",
            "humanymous-dotnet-adapter",
            "--test",
            "hosted",
            "--no-run",
            "--message-format=json",
        ],
    )?;
    let harness = dotnet_host::harness_executable(
        &std::fs::read_to_string(dir.join("dotnet-harness-build.log"))
            .map_err(|e| e.to_string())?,
        "hosted",
    )?;
    let app_root = work.join("app");
    let build = work.join("dotnet-build");
    sdk.invoke(
        root,
        dir,
        report,
        "dotnet-publish",
        &[
            "publish",
            "pilot/Humanymous.HostPilot.csproj",
            "-c",
            "Release",
            "-r",
            "win-x64",
            "--self-contained",
            "false",
            "-o",
            app_root.to_str().ok_or("non-UTF-8 app root")?,
            "-m:1",
            "-nr:false",
            "-p:UseSharedCompilation=false",
            &format!(
                "-p:HumanymousBuildRoot={}",
                build.to_str().ok_or("non-UTF-8 build root")?
            ),
        ],
        "900s",
    )?;
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("debug");
    for name in [
        "humanymous_dotnet_adapter_export.dll",
        "humanymous_native_abi_export.dll",
    ] {
        std::fs::copy(target.join(name), app_root.join(name)).map_err(|e| e.to_string())?;
    }
    let mut files = Vec::new();
    for (role, name) in APP_FILES {
        let path = app_root.join(name);
        let bytes = std::fs::metadata(&path)
            .map_err(|e| format!("{name}: {e}"))?
            .len();
        files.push(json!({"role":role,"path":name,"bytes":bytes,"sha256":source::file(&path)?}));
    }
    let artifact = |path: &Path| -> Result<Value> {
        Ok(json!({
            "path": text(path)?,
            "bytes": std::fs::metadata(path).map_err(|e| e.to_string())?.len(),
            "sha256": source::file(path)?,
        }))
    };
    let source_sha = report["source_sha"]
        .as_str()
        .ok_or("source absent")?
        .to_owned();
    let lock = source::file(&root.join("Cargo.lock"))?;
    let harness_sha = source::file(&harness)?;
    let lineage = |receipt: &Value| json!({"source_sha":source_sha,"cargo_lock_sha256":lock,"receipt_sha256":receipt});
    report["dotnet_sdk_muxer_sha256"] = json!(source::file(&sdk.muxer)?);
    Ok(json!({
        "schema": "humanymous.dotnet-prebuilt-actor-input/2",
        "profile": "standard-user-dotnet-prebuilt",
        "selector": "hosted",
        "source": {
            "sha": source_sha,
            "manifest_sha256": report["source_manifest_sha256"],
            "cargo_lock_sha256": lock,
        },
        "harness": artifact(&harness)?,
        "managed_source": artifact(&root.join("adapters/dotnet/pilot/Program.cs"))?,
        "app": {"root": text(&app_root)?, "files": files},
        "runtime": {"root": text(&sdk.root)?},
        "provenance": {
            "managed": {"source_sha":source_sha,"required_source_sha":source_sha,"receipt_sha256":files[1]["sha256"]},
            "native": lineage(&files[5]["sha256"]),
            "harness": lineage(&json!(harness_sha)),
        },
    }))
}

/// Windows `component dotnet-host` (row 24 on Windows): the elevated runner
/// stages the closures, provisions the NoProfile standard-user actor and
/// runs the Kestrel and generic cases through the actor runner child.
pub fn hosted(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    unit(
        root,
        dir,
        report,
        (
            "owner-at-creation-unit",
            "humanymous-platform",
            "windows-file-create",
            OWNER_AT_CREATION_UNIT,
        ),
        true,
    )?;
    sdk_pilot(root, dir, report)?;
    let work = provider_dir("hm-dotnet-work")?;
    let sdk_private = match provider_dir("hm-dotnet-sdk") {
        Ok(path) => path,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&work);
            return Err(error);
        }
    };
    let outcome = (|| -> Result<Value> {
        let input = prepare(root, dir, report, &work, &sdk_private)?;
        let input_path = work.join("prebuilt-input.json");
        std::fs::write(
            &input_path,
            serde_json::to_vec(&input).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let raw = dir.join("dotnet-prebuilt.json");
        let log = dir.join("dotnet-prebuilt-runner.log");
        report["stage"] = json!("dotnet-prebuilt");
        let code = command::log(
            root,
            "cargo",
            &[
                "run",
                "--locked",
                "--manifest-path",
                RUNNER_MANIFEST,
                "--",
                "dotnet-prebuilt",
                "--input",
                input_path.to_str().ok_or("non-UTF-8 input path")?,
                "--report",
                raw.to_str().ok_or("non-UTF-8 report path")?,
            ],
            &log,
        )?;
        report["commands"]["dotnet-prebuilt"] =
            json!({"exit_code":code,"log_sha256":source::file(&log)?});
        let value = read_json(&raw).unwrap_or(Value::Null);
        require(
            code == 0 || !value.is_null(),
            "dotnet-prebuilt runner failed",
        )?;
        Ok(value)
    })();
    let work_removed = std::fs::remove_dir_all(&work).is_ok();
    let sdk_removed = std::fs::remove_dir_all(&sdk_private).is_ok();
    report["dotnet_host_work_removed"] = json!(work_removed);
    report["dotnet_sdk_private_removed"] = json!(sdk_removed);
    report["full_native_qualified"] = json!(false);
    report["package_qualification_receipt_created"] = json!(false);
    let value = outcome?;
    let kestrel = std::fs::read_to_string(dir.join("dotnet-kestrel.log")).unwrap_or_default();
    report["dotnet_host"] = json!({
        "provider": "standard-user-dotnet-prebuilt",
        "backend": "windows-job-noprofile-actor",
        "venue": "github-hosted",
        "status": value["status"],
        "facts": value["facts"],
        "child": value["child"],
        "tests": value["tests"],
        "failures": value["failures"],
        "observations": dotnet_host::host_observations(&kestrel).unwrap_or(Value::Null),
        "full_product": false,
    });
    require(
        prebuilt_report_ok(&value, "hosted", &dotnet_host::TESTS),
        "Windows dotnet provider terminal facts incomplete",
    )?;
    require(
        report["dotnet_host"]["observations"].is_array(),
        "Windows Kestrel observations incomplete",
    )?;
    require(work_removed, "dotnet provider work not removed")?;
    require(sdk_removed, "dotnet provider private SDK not removed")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn good() -> Value {
        json!({"schema":FIXTURE_SCHEMA,"status":"passed","failures":[],"facts":{
            "start":"resumed-in-job","child_exit_code":0,"profile_absent_before":true,
            "frame_written":true,"natural_exit":true,"job_empty_observed":true,
            "stdout_joined":true,"stderr_joined":true,"child_line_exact":true,
            "stderr_marker_exact":true,"profile_absent_after":true,
            "account_deleted":true,"scratch_deleted":true,"frame_bytes":98304}})
    }
    fn prebuilt() -> Value {
        json!({"schema":PREBUILT_SCHEMA,"status":"passed","selector":"hosted","failures":[],
            "budget_ms":{"run":180000,"cleanup":60000,"renewed":false},
            "facts":{"start":"resumed-in-job","actor_exit_code":0,"natural_exit":true,
                "job_empty_observed":true,"readers_joined":true,"profile_absent_before":true,
                "profile_absent_after":true,"private_absent":true,"account_deleted":true,
                "scratch_deleted":true,"artifact_root_removed":true,"carrier_finished":true},
            "child":{"producer_bound":true},
            "tests":[{"name":dotnet_host::TESTS[0],"passed":true},{"name":dotnet_host::TESTS[1],"passed":true}]})
    }
    #[test]
    fn prebuilt_report_requires_a_clean_pass_of_both_hosted_cases() {
        let tests = &dotnet_host::TESTS;
        assert!(prebuilt_report_ok(&prebuilt(), "hosted", tests));
        assert!(!prebuilt_report_ok(&prebuilt(), "package", tests));
        for pointer in [
            "/facts/natural_exit",
            "/facts/job_empty_observed",
            "/facts/profile_absent_after",
            "/facts/private_absent",
            "/facts/account_deleted",
            "/facts/carrier_finished",
            "/child/producer_bound",
            "/tests/1/passed",
            "/budget_ms/renewed",
        ] {
            let mut value = prebuilt();
            let slot = value.pointer_mut(pointer).unwrap();
            *slot = json!(!slot.as_bool().unwrap());
            assert!(!prebuilt_report_ok(&value, "hosted", tests), "{pointer}");
        }
        let mut value = prebuilt();
        value["tests"].as_array_mut().unwrap().pop();
        assert!(!prebuilt_report_ok(&value, "hosted", tests));
        let mut value = prebuilt();
        value["failures"] = json!(["x"]);
        assert!(!prebuilt_report_ok(&value, "hosted", tests));
    }
    #[test]
    fn app_closure_matches_the_runner_contract_order() {
        let source = include_str!("../../../windows-native-runner/src/actor/prebuilt.rs");
        for (role, name) in APP_FILES {
            assert!(
                source.contains(&format!("(\"{role}\", \"{name}\")")),
                "{name}"
            );
        }
    }
    #[test]
    fn failure_excerpt_is_bounded_to_panic_context() {
        let text = "noise\nthread 'x' panicked at src/a.rs:1:2:\nstep dir-create: OsFailure\nnote\nmore\nlater\n";
        let lines = failure_excerpt(text);
        assert_eq!(
            lines.first().unwrap(),
            "thread 'x' panicked at src/a.rs:1:2:"
        );
        assert_eq!(lines.len(), 4);
        assert!(failure_excerpt(&"FAILED\n".repeat(100)).len() <= 24);
    }
    #[test]
    fn windows_units_name_existing_tests() {
        let owner = include_str!(
            "../../../../../crates/integrations/humanymous-platform/src/windows_object/create_file/native_tests.rs"
        );
        let name = OWNER_AT_CREATION_UNIT.rsplit("::").next().unwrap();
        assert!(owner.contains(&format!("fn {name}()")));
        let n3 = include_str!(
            "../../../../../crates/sdk/humanymous-runtime/src/native_security/storage/actor_context.rs"
        );
        let name = N3_UNIT.rsplit("::").next().unwrap();
        assert!(n3.contains(&format!("fn {name}()")));
    }
    #[test]
    fn n1_report_requires_every_fact() {
        assert!(n1_report_ok(&good()));
        for (key, _) in N1_FACTS.iter().filter(|(_, required)| *required) {
            let mut value = good();
            value["facts"][*key] = json!(false);
            assert!(!n1_report_ok(&value), "{key}");
            value["facts"].as_object_mut().unwrap().remove(*key);
            assert!(!n1_report_ok(&value), "{key}");
        }
        let mut value = good();
        value["facts"]["retained_contained"] = json!(true);
        assert!(!n1_report_ok(&value));
        for (path, wrong) in [
            ("status", json!("failed")),
            ("schema", json!("other")),
            ("failures", json!(["x"])),
        ] {
            let mut value = good();
            value[path] = wrong;
            assert!(!n1_report_ok(&value), "{path}");
        }
        let mut value = good();
        value["facts"]["child_exit_code"] = json!(3);
        assert!(!n1_report_ok(&value));
        value["facts"]["child_exit_code"] = json!(0);
        value["facts"]["start"] = json!("not-started");
        assert!(!n1_report_ok(&value));
    }
}
