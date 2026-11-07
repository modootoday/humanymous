//! Allocated actual SDK/package orchestration; no Cargo recursion or new C# source.
#[path = "package/projection.rs"]
mod projection;
mod support;
use humanymous_test_support::process::capture_with_stderr;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    source_sha: String,
    cargo_lock_sha256: String,
    native_sha256: String,
    harness_sha256: String,
    rustc: String,
    target: String,
    native_profile: String,
    generic_sha256: Option<String>,
}
fn required(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var_os(name).unwrap_or_else(|| panic!("missing supervisor input: {name}")),
    )
    .canonicalize()
    .unwrap()
}
fn source_git(source: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg(format!("safe.directory={}", source.display()))
        .current_dir(source)
        .env("GIT_OPTIONAL_LOCKS", "0");
    command
}
fn run(command: &mut Command, logs: &Path, stage: &str, timeout: Duration) -> Vec<u8> {
    let started = std::time::Instant::now();
    let result = match capture_with_stderr(command, b"", timeout, 4_194_304) {
        Ok(result) => result,
        Err(error) => {
            std::fs::write(logs.join(format!("{stage}.result.json")),serde_json::to_vec_pretty(&json!({"success":false,"exit_code":null,"elapsed_ms":started.elapsed().as_millis(),"capture_error_kind":format!("{:?}",error.kind()),"complete_output_available":false,"descendant_cleanup_qualified":false})).unwrap()).unwrap();
            panic!("{stage} capture failed; inspect private stage result");
        }
    };
    std::fs::write(logs.join(format!("{stage}.stdout")), &result.stdout).unwrap();
    std::fs::write(logs.join(format!("{stage}.stderr")), &result.stderr).unwrap();
    std::fs::write(logs.join(format!("{stage}.result.json")),serde_json::to_vec_pretty(&json!({"pid":result.pid,"exit_code":result.status.code(),"success":result.status.success(),"elapsed_ms":started.elapsed().as_millis(),"stdout_bytes":result.stdout.len(),"stderr_bytes":result.stderr.len()})).unwrap()).unwrap();
    assert!(
        result.status.success(),
        "{stage} failed; inspect private stage logs"
    );
    result.stdout
}
fn digest(path: &Path, logs: &Path, stage: &str) -> String {
    #[cfg(windows)]
    let mut command = {
        let root = std::env::var_os("SystemRoot").expect("actual Windows system root");
        let mut c = Command::new(PathBuf::from(root).join("System32/certutil.exe"));
        c.arg("-hashfile").arg(path).arg("SHA256");
        c
    };
    #[cfg(unix)]
    let mut command = {
        let mut c = Command::new("/usr/bin/sha256sum");
        c.arg(path);
        c
    };
    let bytes = run(&mut command, logs, stage, Duration::from_secs(30));
    let text = String::from_utf8_lossy(&bytes);
    let hashes: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .collect();
    assert_eq!(hashes.len(), 1, "one exact SHA256 result required");
    hashes[0].to_ascii_lowercase()
}
fn private_child(parent: &Path, name: &str) -> PathBuf {
    #[cfg(windows)]
    {
        humanymous_native_security::storage::create_user_private_directory(parent, name)
            .expect("ordinary actor and verified private parent required")
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let path = parent.join(name);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        path
    }
}
struct Sdk<'a> {
    root: &'a Path,
    scratch: &'a Path,
    logs: &'a Path,
    feed: &'a Path,
}
struct Publish<'a> {
    project: &'a Path,
    output: &'a Path,
    build: &'a Path,
    packages: &'a Path,
    rid: &'a str,
    aot: bool,
    packed: bool,
    stage: &'a str,
}
impl Sdk<'_> {
    fn command(&self, packages: &Path) -> Command {
        let mut command = Command::new("dotnet");
        command
            .current_dir(self.root)
            .env("DOTNET_CLI_HOME", self.scratch.join("cli-home"))
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
            .env("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false")
            .env("DOTNET_ADD_GLOBAL_TOOLS_TO_PATH", "false")
            .env("DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE", "true")
            .env("DOTNET_CLI_UI_LANGUAGE", "en-US")
            .env("TEMP", self.scratch.join("tmp"))
            .env("TMP", self.scratch.join("tmp"))
            .env("TMPDIR", self.scratch.join("tmp"))
            .env("NUGET_PACKAGES", packages)
            .env("HUMANYMOUS_NUGET_FEED", self.feed);
        command
    }
    fn publish(&self, plan: Publish<'_>) {
        let Publish {
            project,
            output,
            build,
            packages,
            rid,
            aot,
            packed,
            stage,
        } = plan;
        let mut restore = self.command(packages);
        restore
            .arg("restore")
            .arg(project)
            .arg("-r")
            .arg(rid)
            .arg("-p:SelfContained=true")
            .arg(format!("-p:HumanymousBuildRoot={}", build.display()))
            .arg(format!("-p:UsePackedAdapter={packed}"))
            .arg(format!("-p:PublishAot={aot}"))
            .arg("-m:1")
            .arg("-nr:false")
            .arg("-p:UseSharedCompilation=false");
        if packed {
            restore
                .arg("--configfile")
                .arg(self.root.join("NuGet.Packed.Config"));
        }
        run(
            &mut restore,
            self.logs,
            &format!("{stage}-restore"),
            Duration::from_secs(600),
        );
        let mut publish = self.command(packages);
        publish
            .arg("publish")
            .arg(project)
            .arg("-c")
            .arg("Release")
            .arg("-r")
            .arg(rid)
            .arg("--self-contained")
            .arg("true")
            .arg("--no-restore")
            .arg("-o")
            .arg(output)
            .arg("-m:1")
            .arg("-nr:false")
            .arg("-p:UseSharedCompilation=false")
            .arg(format!("-p:HumanymousBuildRoot={}", build.display()))
            .arg(format!("-p:UsePackedAdapter={packed}"))
            .arg(format!("-p:PublishAot={aot}"));
        run(
            &mut publish,
            self.logs,
            &format!("{stage}-publish"),
            Duration::from_secs(1200),
        );
    }
}

#[test]
#[ignore = "actual allocated SDK, clean package install and JIT/NativeAOT process pilot"]
fn sdk_package_and_real_host_matrix() {
    let execution_environment = support::execution_environment();
    let (rid, target, native_name, host_name) = match (std::env::consts::OS, std::env::consts::ARCH)
    {
        ("windows", "x86_64") => (
            "win-x64",
            "x86_64-pc-windows-msvc",
            "humanymous_dotnet_adapter_export.dll",
            "Humanymous.HostPilot.exe",
        ),
        ("linux", "x86_64") => (
            "linux-x64",
            "x86_64-unknown-linux-gnu",
            "libhumanymous_dotnet_adapter_export.so",
            "Humanymous.HostPilot",
        ),
        _ => panic!("unqualified SDK/native tuple"),
    };
    let source = required("HUMANYMOUS_DOTNET_SOURCE_ROOT");
    let parent = required("HUMANYMOUS_DOTNET_FIXTURE_PARENT");
    let native = required("HUMANYMOUS_DOTNET_NATIVE_LIBRARY");
    let harness = required("HUMANYMOUS_DOTNET_HOST_HARNESS");
    assert_eq!(native.file_name(), Some(OsStr::new(native_name)));
    let scratch = private_child(&parent, "dotnet-package-run");
    private_child(&scratch, "tmp");
    let logs = private_child(&scratch, "logs");
    let feed = private_child(&scratch, "feed");
    let package_cache = private_child(&scratch, "pack-cache");
    let adapter = source.join("adapters/dotnet");
    let provenance: Provenance = serde_json::from_slice(
        &std::fs::read(required("HUMANYMOUS_DOTNET_NATIVE_PROVENANCE")).unwrap(),
    )
    .unwrap();
    let head = run(
        source_git(&source).args(["rev-parse", "HEAD"]),
        &logs,
        "source-head",
        Duration::from_secs(30),
    );
    assert_eq!(
        String::from_utf8(head).unwrap().trim(),
        provenance.source_sha
    );
    let dirty = run(
        source_git(&source).args(["status", "--porcelain", "--untracked-files=normal"]),
        &logs,
        "source-clean",
        Duration::from_secs(30),
    );
    assert!(dirty.is_empty(), "committed clean source required");
    assert_eq!(provenance.target, target);
    assert_eq!(provenance.native_profile, "release");
    assert!(provenance.rustc.starts_with("rustc 1.99.1 "));
    assert_eq!(
        digest(&source.join("Cargo.lock"), &logs, "lock-sha256"),
        provenance.cargo_lock_sha256
    );
    assert_eq!(
        digest(&native, &logs, "native-sha256"),
        provenance.native_sha256
    );
    assert_eq!(
        digest(&harness, &logs, "harness-sha256"),
        provenance.harness_sha256
    );
    let sdk = Sdk {
        root: &adapter,
        scratch: &scratch,
        logs: &logs,
        feed: &feed,
    };
    let version = run(
        sdk.command(&package_cache).arg("--version"),
        &logs,
        "sdk-version",
        Duration::from_secs(30),
    );
    assert_eq!(String::from_utf8(version).unwrap().trim(), "10.0.403");
    let pack_build = private_child(&scratch, "pack-build");
    let mut pack = sdk.command(&package_cache);
    pack.arg("pack")
        .arg(adapter.join("src/Humanymous.Native/Humanymous.Native.csproj"))
        .arg("-c")
        .arg("Release")
        .arg("-o")
        .arg(&feed)
        .arg("-m:1")
        .arg("-nr:false")
        .arg("-p:UseSharedCompilation=false")
        .arg(format!("-p:HumanymousBuildRoot={}", pack_build.display()))
        .arg(format!("-p:NativeArtifact={}", native.display()))
        .arg(format!("-p:NativeRid={rid}"));
    run(&mut pack, &logs, "pack", Duration::from_secs(600));
    let package = feed.join("Humanymous.Native.Experimental.0.1.0-experimental.1.nupkg");
    assert!(package.is_file());
    let package_sha = digest(&package, &logs, "package-sha256");
    let generic = std::env::var_os("HUMANYMOUS_DOTNET_GENERIC_LIBRARY")
        .map(|p| PathBuf::from(p).canonicalize().unwrap());
    if let Some(path) = &generic {
        assert_eq!(
            Some(digest(path, &logs, "generic-sha256")),
            provenance.generic_sha256
        );
    } else {
        assert!(provenance.generic_sha256.is_none());
    }
    let consumer = private_child(&scratch, "consumer");
    for (input, output) in [
        (
            adapter.join("pilot/Program.cs"),
            consumer.join("Program.cs"),
        ),
        (
            adapter.join("pilot/Humanymous.HostPilot.csproj"),
            consumer.join("Humanymous.HostPilot.csproj"),
        ),
        (
            adapter.join("Directory.Build.props"),
            consumer.join("Directory.Build.props"),
        ),
    ] {
        std::fs::copy(&input, &output).unwrap();
        assert_eq!(
            std::fs::read(input).unwrap(),
            std::fs::read(output).unwrap()
        );
    }
    assert!(
        !consumer
            .join("../src/Humanymous.Native/Humanymous.Native.csproj")
            .exists()
    );
    let mut receipts = Vec::new();
    for aot in [false, true] {
        for packed in [false, true] {
            let name = match (aot, packed) {
                (false, false) => "jit-source",
                (false, true) => "jit-packed",
                (true, false) => "aot-source",
                (true, true) => "aot-packed",
            };
            let run_root = private_child(&scratch, name);
            let build = private_child(&run_root, "build");
            let packages = private_child(&run_root, "packages");
            let output = run_root.join("published");
            let project = if packed {
                consumer.join("Humanymous.HostPilot.csproj")
            } else {
                adapter.join("pilot/Humanymous.HostPilot.csproj")
            };
            sdk.publish(Publish {
                project: &project,
                output: &output,
                build: &build,
                packages: &packages,
                rid,
                aot,
                packed,
                stage: name,
            });
            let installed_native = output.join(native_name);
            if !packed {
                std::fs::copy(&native, &installed_native).unwrap();
            }
            if let Some(path) = &generic {
                std::fs::copy(path, output.join(path.file_name().unwrap())).unwrap();
            }
            assert_eq!(
                std::fs::read(&native).unwrap(),
                std::fs::read(&installed_native)
                    .expect("packed native asset must be restored, not manually injected")
            );
            let app = output.join(host_name);
            assert!(app.is_file());
            let fixture_parent = private_child(&run_root, "audit-parent");
            let mut execute = Command::new(&harness);
            execute
                .arg("--ignored")
                .arg("--nocapture")
                .arg("--exact")
                .arg("actual_kestrel_engine_audit_body_gc_and_shutdown")
                .arg("--test-threads=1")
                .env("HUMANYMOUS_DOTNET_PILOT", &app)
                .env("HUMANYMOUS_DOTNET_AUDIT_DIR", &fixture_parent)
                .env(
                    "HUMANYMOUS_DOTNET_EXPECT_AOT",
                    if aot { "true" } else { "false" },
                );
            if generic.is_some() {
                execute.env("HUMANYMOUS_DOTNET_GENERIC_PROBE", "1");
            } else {
                execute.env_remove("HUMANYMOUS_DOTNET_GENERIC_PROBE");
            }
            let observations = run(
                &mut execute,
                &logs,
                &format!("{name}-actual-host"),
                Duration::from_secs(180),
            );
            let summaries: Vec<Value> = String::from_utf8_lossy(&observations)
                .lines()
                .filter_map(|line| {
                    line.find('{')
                        .and_then(|i| serde_json::from_str::<Value>(&line[i..]).ok())
                })
                .filter(|v| v["schema"] == "humanymous.dotnet-host-observation/1")
                .collect();
            assert_eq!(
                summaries.len(),
                2,
                "both real authority profiles must execute"
            );
            for summary in &summaries {
                assert_eq!(summary["failed"], 0);
                assert_eq!(summary["ignored"], 0);
                assert_eq!(summary["dynamic_code"], !aot);
            }
            receipts.push(json!({"mode":name,"aot":aot,"packed":packed,"host_sha256":digest(&app,&logs,&format!("{name}-host-sha256")),"native_sha256":digest(&installed_native,&logs,&format!("{name}-installed-sha256")),"profiles":summaries,"package_install_verified":packed,"all_contract_cases_qualified":false}));
        }
    }
    assert_eq!(digest(&package, &logs, "package-after-sha256"), package_sha);
    assert_eq!(
        digest(&native, &logs, "native-after-sha256"),
        provenance.native_sha256
    );
    let dirty = run(
        source_git(&source).args(["status", "--porcelain", "--untracked-files=normal"]),
        &logs,
        "source-clean-after",
        Duration::from_secs(30),
    );
    assert!(
        dirty.is_empty(),
        "SDK/package execution must preserve the committed source checkout"
    );
    let controller_sha256 = digest(
        &std::env::current_exe().unwrap(),
        &logs,
        "controller-sha256",
    );
    let receipt = json!({"schema":"humanymous.dotnet-package-observation/1","execution_environment":execution_environment,"source_sha":provenance.source_sha,"cargo_lock_sha256":provenance.cargo_lock_sha256,"harness_sha256":provenance.harness_sha256,"controller_sha256":controller_sha256,"rustc":provenance.rustc,"target":provenance.target,"native_profile":provenance.native_profile,"rid":rid,"sdk":"10.0.403","package_sha256":package_sha,"runs":receipts,"full_product":false,"all_contract_cases_qualified":false,"publication":false});
    std::fs::write(
        scratch.join("package-observation.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    let public = projection::project(&receipt, &provenance.source_sha, &provenance.native_sha256)
        .expect("strict bounded host observation projection");
    std::fs::write(
        scratch.join("public-projection.json"),
        serde_json::to_vec_pretty(&public).unwrap(),
    )
    .unwrap();
    println!("{public}");
    assert_eq!(
        public["host_contract_complete"], true,
        "required .NET host cases remain NOT_RUN; fixture completion is not qualification"
    );
}

#[test]
fn ordinary_actor_git_and_sdk_commands_do_not_require_global_mutation() {
    let source = Path::new("owned source");
    let git = source_git(source);
    let arguments: Vec<_> = git.get_args().collect();
    assert_eq!(
        arguments,
        [OsStr::new("-c"), OsStr::new("safe.directory=owned source")]
    );
    assert_eq!(git.get_current_dir(), Some(source));
    assert!(
        git.get_envs()
            .any(|(key, value)| key == "GIT_OPTIONAL_LOCKS" && value == Some(OsStr::new("0")))
    );
    let scratch = Path::new("private scratch");
    let sdk = Sdk {
        root: source,
        scratch,
        logs: scratch,
        feed: scratch,
    };
    let command = sdk.command(scratch);
    for (name, expected) in [
        ("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false"),
        ("DOTNET_ADD_GLOBAL_TOOLS_TO_PATH", "false"),
        ("DOTNET_CLI_TELEMETRY_OPTOUT", "1"),
    ] {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == name && value == Some(OsStr::new(expected)))
        );
    }
    let temporary = scratch.join("tmp");
    for name in ["TEMP", "TMP", "TMPDIR"] {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == name && value == Some(temporary.as_os_str()))
        );
    }
}
