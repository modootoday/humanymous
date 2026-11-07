//! Separate generic ABI-only caller using the shared bounded child owner.
use humanymous_test_support::process::capture_with_stderr;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: PathBuf,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    source_sha: String,
    native_source_sha: String,
    sdk: Artifact,
    assembly: Artifact,
    native_library: Artifact,
    managed_source: Artifact,
    managed_support: Vec<Artifact>,
}

fn hexadecimal(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn save(report: &Path, name: &str, bytes: &[u8]) {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report.join(name))
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
fn regular(path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 67_108_864 {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}
fn verify_artifact(artifact: &Artifact, report: &Path, label: &str, until: Instant) {
    assert!(artifact.path.is_absolute() && regular(&artifact.path));
    #[cfg(windows)]
    {
        let canonical = artifact.path.canonicalize().unwrap();
        assert!(
            matches!(canonical.components().next(),Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(),std::path::Prefix::Disk(_)|std::path::Prefix::VerbatimDisk(_))),
            "local drive artifact required"
        );
    }
    assert!(hexadecimal(&artifact.sha256, 64));
    let budget = until
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(30));
    assert!(
        !budget.is_zero(),
        "generic artifact admission budget expired"
    );
    let result = capture_with_stderr(
        &mut super::support::artifacts::hash_command(&artifact.path),
        b"",
        budget,
        65_536,
    )
    .expect("bounded stock artifact digest");
    save(report, &format!("{label}-hash.stdout"), &result.stdout);
    save(report, &format!("{label}-hash.stderr"), &result.stderr);
    assert!(result.status.success(), "stock digest command failed");
    assert_eq!(
        super::support::artifacts::parse_sha256(&result.stdout).unwrap(),
        artifact.sha256.to_ascii_lowercase(),
        "generic caller input changed"
    );
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Starting {
    kind: String,
    pid: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    kind: String,
    passed: u8,
    case: String,
}

fn verify(bytes: &[u8], expected_pid: u32) -> Result<(), &'static str> {
    if bytes.len() > 4096 || !bytes.ends_with(b"\n") {
        return Err("frame_budget");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "frame_utf8")?;
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 2 {
        return Err("frame_denominator");
    }
    let starting: Starting = serde_json::from_str(lines[0]).map_err(|_| "starting_schema")?;
    let receipt: Receipt = serde_json::from_str(lines[1]).map_err(|_| "receipt_schema")?;
    if expected_pid == 0 || starting.kind != "starting" || starting.pid != expected_pid {
        return Err("pid_binding");
    }
    if receipt.kind != "generic-native-abi"
        || receipt.passed != 1
        || receipt.case != "pointer-free-roundtrip-and-negative-ownership"
    {
        return Err("probe_receipt");
    }
    Ok(())
}

#[test]
#[ignore = "explicitly allocated actual managed generic ABI-only process, not Kestrel"]
fn actual_generic_abi_only_process() {
    let venue = super::support::execution_environment();
    let input_path = PathBuf::from(
        std::env::var_os("HUMANYMOUS_DOTNET_GENERIC_INPUT")
            .expect("strict prebuilt generic caller inputs required"),
    );
    let report = PathBuf::from(
        std::env::var_os("HUMANYMOUS_DOTNET_GENERIC_REPORT_DIR")
            .expect("fresh private caller report directory required"),
    );
    assert!(
        input_path.is_absolute()
            && regular(&input_path)
            && std::fs::metadata(&input_path).unwrap().len() <= 16_384
    );
    assert!(report.is_absolute() && report.is_dir());
    let inputs: Inputs = serde_json::from_slice(&std::fs::read(&input_path).unwrap())
        .expect("strict generic caller input schema");
    assert!(hexadecimal(&inputs.source_sha, 40) && hexadecimal(&inputs.native_source_sha, 40));
    assert_eq!(
        inputs.sdk.path.file_name(),
        Some(std::ffi::OsStr::new(if cfg!(windows) {
            "dotnet.exe"
        } else {
            "dotnet"
        }))
    );
    assert_eq!(
        inputs.managed_source.path.file_name(),
        Some(std::ffi::OsStr::new("Program.cs"))
    );
    assert_eq!(
        inputs.assembly.path.file_name(),
        Some(std::ffi::OsStr::new("Humanymous.HostPilot.dll"))
    );
    let expected_native = if cfg!(windows) {
        "humanymous_native_abi_export.dll"
    } else {
        "libhumanymous_native_abi_export.so"
    };
    assert_eq!(
        inputs.native_library.path.file_name(),
        Some(std::ffi::OsStr::new(expected_native))
    );
    let expected_support: BTreeSet<_> = [
        "Humanymous.HostPilot.deps.json",
        "Humanymous.HostPilot.runtimeconfig.json",
        "Humanymous.Native.dll",
    ]
    .into_iter()
    .collect();
    assert_eq!(inputs.managed_support.len(), 3);
    let actual_support: BTreeSet<_> = inputs
        .managed_support
        .iter()
        .map(|a| a.path.file_name().and_then(|n| n.to_str()).unwrap())
        .collect();
    assert_eq!(actual_support, expected_support);
    let until = Instant::now() + Duration::from_secs(60);
    for (label, artifact) in [
        ("sdk", &inputs.sdk),
        ("assembly", &inputs.assembly),
        ("native", &inputs.native_library),
        ("managed-source", &inputs.managed_source),
    ] {
        verify_artifact(artifact, &report, label, until);
    }
    for (index, artifact) in inputs.managed_support.iter().enumerate() {
        verify_artifact(artifact, &report, &format!("support-{index}"), until);
    }
    let stage = report.join("caller");
    std::fs::create_dir(&stage).expect("new isolated caller directory required");
    for (index, artifact) in [&inputs.assembly, &inputs.native_library]
        .into_iter()
        .chain(inputs.managed_support.iter())
        .enumerate()
    {
        let staged = stage.join(artifact.path.file_name().unwrap());
        std::fs::copy(&artifact.path, &staged).unwrap();
        verify_artifact(
            &Artifact {
                path: staged,
                sha256: artifact.sha256.clone(),
            },
            &report,
            &format!("copied-{index}"),
            until,
        );
    }
    std::fs::create_dir(stage.join("cli-home")).unwrap();
    std::fs::create_dir(stage.join("tmp")).unwrap();
    let mut command = Command::new(&inputs.sdk.path);
    command
        .env_clear()
        .arg(stage.join("Humanymous.HostPilot.dll"))
        .current_dir(&stage)
        .env("DOTNET_ROOT", inputs.sdk.path.parent().unwrap())
        .env("DOTNET_CLI_HOME", stage.join("cli-home"))
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_EnableDiagnostics", "0")
        .env("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false")
        .env("DOTNET_ADD_GLOBAL_TOOLS_TO_PATH", "false")
        .env("TEMP", stage.join("tmp"))
        .env("TMP", stage.join("tmp"))
        .env("TMPDIR", stage.join("tmp"));
    #[cfg(windows)]
    {
        let system = std::env::var_os("SystemRoot").expect("actual Windows system root");
        command
            .env("SystemRoot", &system)
            .env("WINDIR", &system)
            .env("PATH", PathBuf::from(system).join("System32"));
    }
    let captured = capture_with_stderr(
        &mut command,
        b"{\"generic_probe\":true,\"generic_probe_only\":true}\n",
        Duration::from_secs(30),
        4096,
    );
    let stage_canonical = stage.canonicalize().unwrap();
    let report_canonical = report.canonicalize().unwrap();
    assert_eq!(stage_canonical.parent(), Some(report_canonical.as_path()));
    assert_eq!(
        stage_canonical.file_name(),
        Some(std::ffi::OsStr::new("caller"))
    );
    let removed = std::fs::remove_dir_all(&stage).is_ok();
    let result = match captured {
        Ok(result) => result,
        Err(error) => {
            save(&report,"capture-error.json",&serde_json::to_vec_pretty(&serde_json::json!({"kind":format!("{:?}",error.kind()),"fixture_removed":removed,"exit_code":null})).unwrap());
            panic!("bounded generic-only caller capture failed");
        }
    };
    save(&report, "stdout.log", &result.stdout);
    save(&report, "stderr.log", &result.stderr);
    save(&report,"process.json",&serde_json::to_vec_pretty(&serde_json::json!({"pid":result.pid,"exit_code":result.status.code(),"stdout_bytes":result.stdout.len(),"stderr_bytes":result.stderr.len(),"fixture_removed":removed})).unwrap());
    assert!(removed, "owned generic caller fixture cleanup failed");
    assert!(
        result.status.success(),
        "actual generic caller nonzero exit"
    );
    assert!(result.stderr.is_empty(), "unexpected generic caller stderr");
    verify(&result.stdout, result.pid).expect("exact generic-only caller frames and actual PID");
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.generic-abi-managed-caller/1","execution_environment":venue,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"source_sha":inputs.source_sha,"native_source_sha":inputs.native_source_sha,"managed_source_sha256":inputs.managed_source.sha256,"assembly_sha256":inputs.assembly.sha256,"native_library_sha256":inputs.native_library.sha256,"sdk_sha256":inputs.sdk.sha256,"caller":"approved-csharp-fdd-jit","passed":1,"failed":0,"ignored":0,"native_process_exit":result.status.code(),"actual_pid_bound":true,"frames":2,"fixture_removed":removed,"kestrel_started":false,"dotnet_adapter_qualified":false,"native_aot_qualified":false,"cpp_qualified":false,"full_product":false})
    );
}

#[test]
fn generic_receipt_requires_exact_pid_frames_and_no_host_events() {
    let valid = b"{\"kind\":\"starting\",\"pid\":42}\n{\"kind\":\"generic-native-abi\",\"passed\":1,\"case\":\"pointer-free-roundtrip-and-negative-ownership\"}\n";
    assert_eq!(verify(valid, 42), Ok(()));
    assert_eq!(verify(valid, 43), Err("pid_binding"));
    let extra = [valid.as_slice(), b"{\"kind\":\"ready\"}\n".as_slice()].concat();
    assert_eq!(verify(&extra, 42), Err("frame_denominator"));
    let duplicate = String::from_utf8(valid.to_vec())
        .unwrap()
        .replace("\"passed\":1", "\"passed\":1,\"passed\":1");
    assert_eq!(verify(duplicate.as_bytes(), 42), Err("receipt_schema"));
    assert!(verify(&valid[..valid.len() - 1], 42).is_err());
}
