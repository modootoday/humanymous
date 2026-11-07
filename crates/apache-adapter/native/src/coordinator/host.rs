//! Owned Linux Docker lifecycle. No shell, publication, credential exchange or
//! arbitrary caller command is accepted. All diagnostics remain in the private
//! bundle; only the bounded typed summary may be projected publicly.
use super::engine;
use super::{Counts, Expected, contract, inspect, parse, validate};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, &'static str>;
fn ensure(value: bool, label: &'static str) -> Result<()> {
    if value { Ok(()) } else { Err(label) }
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "private-output-create")?;
    file.write_all(bytes).map_err(|_| "private-output-write")?;
    file.sync_all().map_err(|_| "private-output-sync")
}
fn read_bound(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|_| "artifact-read")?;
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "artifact-read")?;
    ensure(bytes.len() as u64 <= maximum, "artifact-size-bound")?;
    Ok(bytes)
}
fn file_hash(path: &Path) -> Result<String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "artifact-metadata")?;
    ensure(
        metadata.is_file() && metadata.len() <= 268_435_456,
        "artifact-file-bound",
    )?;
    let mut file = File::open(path).map_err(|_| "artifact-open")?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let n = file.read(&mut bytes).map_err(|_| "artifact-read")?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
struct Operations {
    bundle: PathBuf,
    deadline: Instant,
    next: usize,
    prefix: &'static str,
    last: Option<Value>,
    timings: BTreeMap<String, u64>,
    diagnostics: Value,
    archive: Option<PathBuf>,
    docker_program: String,
    docker_prefix: Vec<String>,
    engine_expected: engine::Expected,
    identity: Option<Value>,
}
fn operations(
    bundle: &Path,
    prefix: &'static str,
    seconds: u64,
    hosted: bool,
) -> Result<Operations> {
    let (program, flags, expected) = if hosted {
        (
            "docker".to_owned(),
            Vec::new(),
            engine::Expected {
                profile: engine::Profile::HostedLinux,
                server_id: None,
                version: None,
            },
        )
    } else {
        ensure(
            std::env::var_os("WSL_DISTRO_NAME").is_some(),
            "explicit-wsl-to-windows-client-required",
        )?;
        let program =
            std::env::var("HUMANYMOUS_APACHE_WINDOWS_DOCKER_WSL_PATH").unwrap_or_else(|_| {
                "/mnt/c/Program Files/Docker/Docker/resources/bin/docker.exe".into()
            });
        ensure(
            Path::new(&program).is_absolute() && Path::new(&program).is_file(),
            "windows-desktop-client-missing-no-fallback",
        )?;
        let id = std::env::var("HUMANYMOUS_APACHE_DOCKER_SERVER_ID")
            .map_err(|_| "provisioned-desktop-server-id-required")?;
        let version = std::env::var("HUMANYMOUS_APACHE_DOCKER_SERVER_VERSION")
            .map_err(|_| "provisioned-desktop-version-required")?;
        (
            program,
            vec!["--context".into(), "desktop-linux".into()],
            engine::Expected {
                profile: engine::Profile::WindowsDesktop,
                server_id: Some(id),
                version: Some(version),
            },
        )
    };
    Ok(Operations {
        bundle: bundle.to_owned(),
        deadline: Instant::now() + Duration::from_secs(seconds),
        next: 0,
        prefix,
        last: None,
        timings: BTreeMap::new(),
        diagnostics: super::diagnostics(&[]),
        archive: None,
        docker_program: program,
        docker_prefix: flags,
        engine_expected: expected,
        identity: None,
    })
}
impl Operations {
    fn command(
        &mut self,
        program: &str,
        args: &[String],
        limit: Duration,
    ) -> Result<(i32, PathBuf)> {
        ensure(Instant::now() < self.deadline, "coordinator-deadline")?;
        let started = Instant::now();
        let docker = program == self.docker_program;
        let verb = args
            .get(if docker { self.docker_prefix.len() } else { 0 })
            .map(String::as_str);
        let category = match (program, docker, verb) {
            ("git", _, _) => "git",
            (_, true, Some("build")) => "build",
            (_, true, Some("create")) => "create",
            (_, true, Some("start")) => "run",
            (_, true, Some("cp")) => "artifact-copy",
            _ => "inspect-cleanup",
        };
        self.next += 1;
        let stdout = self
            .bundle
            .join(format!("{}-{}-stdout.raw", self.prefix, self.next));
        let stderr = self
            .bundle
            .join(format!("{}-{}-stderr.raw", self.prefix, self.next));
        let output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&stdout)
            .map_err(|_| "command-output-create")?;
        let error = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&stderr)
            .map_err(|_| "command-output-create")?;
        let input = if category == "build" {
            Stdio::from(
                File::open(self.archive.as_ref().ok_or("source-archive-missing")?)
                    .map_err(|_| "source-archive-open")?,
            )
        } else {
            Stdio::null()
        };
        let mut child = Command::new(program)
            .args(args)
            .stdin(input)
            .stdout(output)
            .stderr(error)
            .spawn()
            .map_err(|_| "command-start")?;
        let until = (Instant::now() + limit).min(self.deadline);
        let result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code().ok_or("owned-command-signal"),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(50)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("owned-command-timeout");
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("owned-command-observation");
                }
            }
        };
        let elapsed = started.elapsed().as_millis() as u64;
        if category == "build" && result != Ok(0) {
            let mut bytes = read_bound(&stdout, 8_388_608).unwrap_or_default();
            bytes.extend(read_bound(&stderr, 8_388_608).unwrap_or_default());
            self.diagnostics = super::diagnostics(&bytes);
        }
        *self.timings.entry(category.to_owned()).or_default() += elapsed;
        let outcome = match result {
            Ok(code) => {
                json!({"category":category,"kind":"exited","exit":code,"elapsedMs":elapsed})
            }
            Err(reason) => json!({"category":category,"kind":reason,"elapsedMs":elapsed}),
        };
        write_new(
            &self
                .bundle
                .join(format!("{}-{}-outcome.json", self.prefix, self.next)),
            &serde_json::to_vec(&outcome).map_err(|_| "command-outcome-json")?,
        )?;
        self.last = Some(outcome);
        result.map(|code| (code, stdout))
    }
    fn success(&mut self, program: &str, args: &[String], limit: Duration) -> Result<PathBuf> {
        let (code, path) = self.command(program, args, limit)?;
        ensure(code == 0, "owned-command-failed")?;
        Ok(path)
    }
    fn docker_status(&mut self, args: &[String], limit: Duration) -> Result<(i32, PathBuf)> {
        let mut full = self.docker_prefix.clone();
        full.extend_from_slice(args);
        let program = self.docker_program.clone();
        self.command(&program, &full, limit)
    }
    fn docker(&mut self, args: &[String], limit: Duration) -> Result<PathBuf> {
        let (code, path) = self.docker_status(args, limit)?;
        ensure(code == 0, "owned-docker-command-failed")?;
        Ok(path)
    }
    fn preflight(&mut self) -> Result<Value> {
        if matches!(
            self.engine_expected.profile,
            engine::Profile::WindowsDesktop
        ) {
            let program = self.docker_program.clone();
            let path = self.success(
                "/usr/bin/wslpath",
                &argv(&["-w", &program]),
                Duration::from_secs(10),
            )?;
            ensure(
                text(&path)?
                    .trim()
                    .replace('\\', "/")
                    .eq_ignore_ascii_case(engine::WINDOWS_CLIENT),
                "windows-desktop-client-path-mismatch",
            )?;
            // The Windows child resolves credentials through its own PATH;
            // successful WSL lookup or Docker info does not establish this.
            let helper = self.success(
                "/mnt/c/Windows/System32/where.exe",
                &argv(&["docker-credential-desktop.exe"]),
                Duration::from_secs(10),
            )?;
            ensure(
                text(&helper)?.lines().next().is_some_and(|path| {
                    path.trim().replace('\\', "/").eq_ignore_ascii_case(
                        "C:/Program Files/Docker/Docker/resources/bin/docker-credential-desktop.exe",
                    )
                }),
                "windows-desktop-credential-helper-path-mismatch",
            )?;
        }
        let context_args = if matches!(
            self.engine_expected.profile,
            engine::Profile::WindowsDesktop
        ) {
            argv(&["context", "inspect", "desktop-linux"])
        } else {
            argv(&["context", "inspect"])
        };
        let context = self.docker(&context_args, Duration::from_secs(20))?;
        let context: Value = serde_json::from_slice(&read_bound(&context, 1_048_576)?)
            .map_err(|_| "docker-context-json")?;
        let contexts = context.as_array().ok_or("docker-context-shape")?;
        ensure(contexts.len() == 1, "docker-context-cardinality")?;
        let info = self.docker(
            &argv(&["info", "--format", "{{json .}}"]),
            Duration::from_secs(20),
        )?;
        let info: Value = serde_json::from_slice(&read_bound(&info, 1_048_576)?)
            .map_err(|_| "docker-info-json")?;
        let identity = engine::verify(&self.engine_expected, &contexts[0], &info)?;
        if let Some(previous) = &self.identity {
            ensure(previous == &identity, "docker-engine-changed-during-run")?;
        } else {
            self.identity = Some(identity.clone());
        }
        Ok(identity)
    }
    fn destination(&mut self, path: &Path) -> Result<String> {
        if matches!(self.engine_expected.profile, engine::Profile::HostedLinux) {
            return Ok(path.display().to_string());
        }
        let converted = self.success(
            "/usr/bin/wslpath",
            &argv(&["-w", &path.display().to_string()]),
            Duration::from_secs(10),
        )?;
        let value = text(&converted)?.trim().to_owned();
        ensure(
            !value.contains(['\n', '\r']) && !value.is_empty(),
            "windows-artifact-destination",
        )?;
        Ok(value)
    }
    fn inspect(&mut self, kind: &str, name: &str) -> Result<Value> {
        let path = self.docker(
            &[kind.into(), "inspect".into(), name.into()],
            Duration::from_secs(20),
        )?;
        let value: Value = serde_json::from_slice(&read_bound(&path, 1_048_576)?)
            .map_err(|_| "docker-inspect-json")?;
        let values = value.as_array().ok_or("docker-inspect-shape")?;
        ensure(values.len() == 1, "docker-inspect-cardinality")?;
        Ok(values[0].clone())
    }
}
fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
fn text(path: &Path) -> Result<String> {
    String::from_utf8(read_bound(path, 8_388_608)?).map_err(|_| "command-output-encoding")
}
fn snapshot(operations: &mut Operations, name: &str) -> Result<(String, String)> {
    let revision = operations.success(
        "git",
        &argv(&["rev-parse", "HEAD"]),
        Duration::from_secs(10),
    )?;
    let revision = text(&revision)?.trim().to_owned();
    ensure(
        revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
        "source-revision",
    )?;
    let scopes = [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        ".cargo",
        "crates",
        "adapters/apache",
    ];
    let mut args = argv(&["status", "--porcelain", "--untracked-files=all", "--"]);
    args.extend(scopes.iter().map(|value| (*value).to_owned()));
    let status = operations.success("git", &args, Duration::from_secs(10))?;
    ensure(
        read_bound(&status, 1_048_576)?.is_empty(),
        "source-checkout-must-be-committed-clean",
    )?;
    let mut args = argv(&["ls-files", "-z", "--"]);
    args.extend(scopes.iter().map(|value| (*value).to_owned()));
    let paths = operations.success("git", &args, Duration::from_secs(10))?;
    let paths = read_bound(&paths, 8_388_608)?;
    let root = std::env::current_dir()
        .map_err(|_| "source-root")?
        .canonicalize()
        .map_err(|_| "source-root")?;
    let mut sources = BTreeMap::new();
    for path in paths
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = std::str::from_utf8(path).map_err(|_| "source-path-encoding")?;
        ensure(
            Path::new(path)
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
            "source-path-scope",
        )?;
        let absolute = root.join(path);
        ensure(
            absolute
                .canonicalize()
                .map_err(|_| "source-path")?
                .starts_with(&root),
            "source-path-escape",
        )?;
        sources.insert(path.to_owned(), file_hash(&absolute)?);
    }
    ensure(!sources.is_empty(), "empty-source-manifest")?;
    let archive = operations.success(
        "git",
        &argv(&["archive", "--format=tar", "HEAD"]),
        Duration::from_secs(60),
    )?;
    let archive_hash = file_hash(&archive)?;
    operations.archive = Some(archive);
    let bytes=serde_json::to_vec(&json!({"schema":"humanymous.apache.source/0.1","sourceCommit":revision,"gitArchiveSha256":archive_hash,"observedWorktreeFiles":sources})).map_err(|_|"source-manifest-json")?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    write_new(&operations.bundle.join(name), &bytes)?;
    Ok((revision, hash))
}
struct Owned {
    nonce: String,
    name: String,
    tag: String,
    image: Option<String>,
    cleaned: bool,
}
impl Owned {
    fn cleanup(&mut self, operations: &mut Operations) -> Result<()> {
        operations.preflight()?;
        let names = operations.docker(
            &argv(&[
                "container",
                "ls",
                "--all",
                "--quiet",
                "--no-trunc",
                "--filter",
                &format!("name=^/{}$", self.name),
            ]),
            Duration::from_secs(20),
        )?;
        if !text(&names)?.trim().is_empty() {
            let value = operations.inspect("container", &self.name)?;
            ensure(
                value["Config"]["Labels"]["humanymous.apache.owner"] == self.nonce,
                "foreign-container-cleanup-refused",
            )?;
            operations.docker(
                &argv(&["container", "rm", "--force", &self.name]),
                Duration::from_secs(30),
            )?;
        }
        let names = operations.docker(
            &argv(&[
                "container",
                "ls",
                "--all",
                "--quiet",
                "--filter",
                &format!("name=^/{}$", self.name),
            ]),
            Duration::from_secs(20),
        )?;
        ensure(
            text(&names)?.trim().is_empty(),
            "container-cleanup-not-observed",
        )?;
        let images = operations.docker(
            &argv(&[
                "image",
                "ls",
                "--quiet",
                "--no-trunc",
                "--filter",
                &format!("reference={}", self.tag),
            ]),
            Duration::from_secs(20),
        )?;
        if !text(&images)?.trim().is_empty() {
            let value = operations.inspect("image", &self.tag)?;
            ensure(
                value["Config"]["Labels"]["humanymous.apache.owner"] == self.nonce,
                "foreign-image-cleanup-refused",
            )?;
            operations.docker(&argv(&["image", "rm", &self.tag]), Duration::from_secs(30))?;
        }
        let images = operations.docker(
            &argv(&[
                "image",
                "ls",
                "--quiet",
                "--filter",
                &format!("reference={}", self.tag),
            ]),
            Duration::from_secs(20),
        )?;
        ensure(
            text(&images)?.trim().is_empty(),
            "image-cleanup-not-observed",
        )?;
        self.cleaned = true;
        Ok(())
    }
}
fn execute(
    operations: &mut Operations,
    owned: &mut Owned,
    stage: &mut &'static str,
) -> Result<(Expected, Value, Counts, Result<()>)> {
    *stage = "engine";
    let identity = operations.preflight()?;
    write_new(
        &operations.bundle.join("apache-owned.json"),
        &serde_json::to_vec(
            &json!({"nonce":owned.nonce,"name":owned.name,"tag":owned.tag,"engine":identity}),
        )
        .map_err(|_| "owned-ledger-json")?,
    )?;
    *stage = "source";
    let (source, manifest) = snapshot(operations, "apache-source-manifest.json")?;
    if matches!(
        operations.engine_expected.profile,
        engine::Profile::HostedLinux
    ) {
        ensure(
            std::env::var("GITHUB_SHA").as_deref() == Ok(source.as_str()),
            "hosted-source-sha-mismatch",
        )?;
    }
    *stage = "build";
    operations.docker(
        &argv(&[
            "build",
            "--file",
            "adapters/apache/Dockerfile.native",
            "--target",
            "pilot",
            "--build-arg",
            &format!("HUMANYMOUS_SOURCE_REV={source}"),
            "--build-arg",
            &format!("HUMANYMOUS_SOURCE_MANIFEST_SHA256={manifest}"),
            "--label",
            &format!("humanymous.apache.owner={}", owned.nonce),
            "--tag",
            &owned.tag,
            "-",
        ]),
        Duration::from_secs(1800),
    )?;
    let after = snapshot(operations, "apache-source-after-build.json")?;
    ensure(
        after.0 == source && after.1 == manifest,
        "source-changed-during-build",
    )?;
    operations.preflight()?;
    let image = operations.inspect("image", &owned.tag)?;
    let image = image["Id"].as_str().ok_or("image-id-missing")?.to_owned();
    ensure(
        image
            .strip_prefix("sha256:")
            .is_some_and(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())),
        "image-id-shape",
    )?;
    owned.image = Some(image.clone());
    *stage = "create";
    operations.docker(
        &argv(&[
            "create",
            "--name",
            &owned.name,
            "--label",
            &format!("humanymous.apache.owner={}", owned.nonce),
            "--network",
            "none",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--user",
            "65532:65532",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,size=268435456",
            "--pids-limit",
            "256",
            "--memory",
            "1073741824",
            "--cpus",
            "2",
            "--env",
            &format!("HUMANYMOUS_APACHE_IMAGE={image}"),
            &image,
        ]),
        Duration::from_secs(30),
    )?;
    let created = operations.inspect("container", &owned.name)?;
    ensure(
        created["Config"]["User"] == "65532:65532"
            && created["HostConfig"]["NetworkMode"] == "none"
            && created["HostConfig"]["ReadonlyRootfs"] == true
            && created["HostConfig"]["Privileged"] == false
            && created["HostConfig"]["Memory"] == 1_073_741_824u64
            && created["HostConfig"]["PidsLimit"] == 256
            && created["HostConfig"]["CapDrop"]
                .as_array()
                .is_some_and(|values| values.iter().any(|value| value == "ALL"))
            && created["HostConfig"]["SecurityOpt"]
                .as_array()
                .is_some_and(|values| {
                    values.iter().any(|value| {
                        value == "no-new-privileges" || value == "no-new-privileges=true"
                    })
                }),
        "container-isolation-mismatch",
    )?;
    *stage = "run";
    let (start_exit, output) = operations.docker_status(
        &argv(&["start", "--attach", &owned.name]),
        Duration::from_secs(300),
    )?;
    operations.preflight()?;
    let state = operations.inspect("container", &owned.name)?;
    ensure(
        state["State"]["Status"] == "exited"
            && state["State"]["OOMKilled"] == false
            && state["State"]["Error"] == "",
        "container-not-cleanly-exited",
    )?;
    let exit = i32::try_from(
        state["State"]["ExitCode"]
            .as_i64()
            .ok_or("container-exit-missing")?,
    )
    .map_err(|_| "container-exit-bound")?;
    *stage = "artifacts";
    let contract = contract()?;
    let mut artifacts = BTreeMap::new();
    for key in [
        "moduleSha256",
        "probeModuleSha256",
        "httpdSha256",
        "abiReceiptSha256",
        "lockSha256",
    ] {
        let remote = contract["artifact_bindings"][key]["path"]
            .as_str()
            .ok_or("contract-artifact-path")?;
        ensure(
            remote.starts_with("/opt/httpd/") && !remote.contains(".."),
            "contract-artifact-scope",
        )?;
        let local = operations.bundle.join(format!("{key}.artifact"));
        let destination = operations.destination(&local)?;
        operations.docker(
            &["cp".into(), format!("{}:{remote}", owned.name), destination],
            Duration::from_secs(30),
        )?;
        artifacts.insert(key.to_owned(), file_hash(&local)?);
    }
    let expected = Expected {
        source,
        manifest,
        image,
        artifacts,
    };
    *stage = "report";
    let bytes = read_bound(&output, 1_048_576)?;
    let report = parse(&bytes)?;
    write_new(&operations.bundle.join("apache-pilot-private.json"), &bytes)?;
    let counts = inspect(&report, &expected)?;
    // Keep a valid-but-failed pilot as failed, including its exact observed
    // counts. The caller will still perform and verify container/image cleanup.
    let outcome = if start_exit != 0 {
        Err("attach-command-failed")
    } else {
        validate(&report, &expected, exit).map(|_| ())
    };
    Ok((expected, report, counts, outcome))
}
fn cleanup_only(bundle: &Path, hosted: bool) -> Result<()> {
    let value = parse(&read_bound(&bundle.join("apache-owned.json"), 4096)?)?;
    ensure(
        value.as_object().is_some_and(|object| object.len() == 4) && value["engine"].is_object(),
        "owned-ledger-shape",
    )?;
    let nonce = value["nonce"]
        .as_str()
        .ok_or("owned-ledger-nonce")?
        .to_owned();
    ensure(
        nonce.len() == 36
            && nonce.bytes().enumerate().all(|(index, byte)| {
                if [8, 13, 18, 23].contains(&index) {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                }
            }),
        "owned-ledger-nonce",
    )?;
    let name = format!("h-apache-{nonce}");
    let tag = format!("humanymous-apache-pilot:{nonce}");
    ensure(
        value["name"] == name && value["tag"] == tag,
        "owned-ledger-scope",
    )?;
    let mut owned = Owned {
        nonce,
        name,
        tag,
        image: None,
        cleaned: false,
    };
    let mut operations = operations(bundle, "cleanup-command", 120, hosted)?;
    operations.identity = Some(value["engine"].clone());
    let result = owned.cleanup(&mut operations);
    let summary = json!({"schema":"humanymous.apache.cleanup/0.1","status":if result.is_ok(){"passed"}else{"failed"},"containerImageCleanupObserved":result.is_ok()&&owned.cleaned,"fullProductSuccess":false});
    write_new(
        &bundle.join("apache-cleanup.json"),
        &serde_json::to_vec(&summary).map_err(|_| "cleanup-summary-json")?,
    )?;
    println!(
        "{}",
        serde_json::to_string(&summary).map_err(|_| "cleanup-summary-json")?
    );
    result
}
pub fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure(
        args.len() == 3
            && (args[0] == "coordinate" || args[0] == "coordinate-cleanup")
            && args[1] == "--bundle",
        "coordinate-arguments",
    )?;
    let local = std::env::var("HUMANYMOUS_APACHE_OWNED_LOCAL").as_deref() == Ok("1");
    let hosted = !local
        && std::env::var("ACT").as_deref() != Ok("true")
        && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("RUNNER_OS").as_deref() == Ok("Linux")
        && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted");
    ensure(hosted || local, "explicit-owned-execution-context-required")?;
    let bundle = PathBuf::from(&args[2]);
    if hosted {
        ensure(
            rustix::process::geteuid().as_raw() != 0,
            "hosted-ordinary-actor-required",
        )?;
        let temporary =
            PathBuf::from(std::env::var_os("RUNNER_TEMP").ok_or("hosted-temp-required")?);
        ensure(
            temporary.is_absolute()
                && temporary.canonicalize().map_err(|_| "hosted-temp-path")? == temporary
                && bundle == temporary.join("apache-native"),
            "hosted-private-bundle-scope",
        )?;
        if args[0] == "coordinate" {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&bundle)
                .map_err(|_| "fresh-hosted-private-bundle-required")?;
        }
    }
    ensure(
        bundle.is_absolute() && bundle.canonicalize().map_err(|_| "private-bundle-path")? == bundle,
        "private-bundle-canonical",
    )?;
    let metadata = std::fs::metadata(&bundle).map_err(|_| "private-bundle-metadata")?;
    ensure(
        metadata.is_dir()
            && metadata.uid() == rustix::process::geteuid().as_raw()
            && metadata.permissions().mode() & 0o777 == 0o700,
        "private-bundle-owner-mode",
    )?;
    if args[0] == "coordinate-cleanup" {
        return cleanup_only(&bundle, hosted).map_err(Into::into);
    }
    for entry in std::fs::read_dir(&bundle).map_err(|_| "private-bundle-inventory")? {
        ensure(
            entry.map_err(|_| "private-bundle-inventory")?.file_name() == "index.evidence.md",
            "fresh-private-bundle-required",
        )?;
    }
    write_new(
        &bundle.join("apache-coordinator.marker"),
        b"owned Apache run\n",
    )?;
    let nonce = humanymous_framework_adapter::uuid()?;
    let mut owned = Owned {
        nonce: nonce.clone(),
        name: format!("h-apache-{nonce}"),
        tag: format!("humanymous-apache-pilot:{nonce}"),
        image: None,
        cleaned: false,
    };
    let mut operations = operations(&bundle, "command", 2280, hosted)?;
    let mut stage = "source";
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(&mut operations, &mut owned, &mut stage)
    }))
    .unwrap_or(Err("coordinator-panic"));
    let execution_last = operations.last.clone();
    // Reserve a separate final cleanup budget even after execution timeout.
    operations.deadline = Instant::now() + Duration::from_secs(120);
    let cleanup = owned.cleanup(&mut operations);
    let passed = result
        .as_ref()
        .is_ok_and(|(_, _, _, outcome)| outcome.is_ok());
    let failure = result
        .as_ref()
        .err()
        .copied()
        .or_else(|| {
            result
                .as_ref()
                .ok()
                .and_then(|(_, _, _, outcome)| outcome.as_ref().err().copied())
        })
        .or_else(|| cleanup.as_ref().err().copied());
    let last = if result.is_err() || !passed {
        execution_last
    } else {
        operations.last.clone()
    };
    let reported_stage = if passed && cleanup.is_err() {
        "cleanup"
    } else {
        stage
    };
    let mut summary = json!({"schema":"humanymous.apache.coordinator/0.1","executionContext":if hosted{"github-hosted"}else{"owned-local"},"engine":operations.identity,"status":"failed","stage":reported_stage,"failure":failure,"lastCommand":last,"timingsMs":operations.timings,"compilerDiagnostics":operations.diagnostics,"requiredScenarioCount":45,"observedCounts":null,"sourceCommit":null,"sourceManifestSha256":null,"imageDigest":owned.image,"artifactHashes":null,"pilotCleanupObserved":false,"containerImageCleanupObserved":cleanup.is_ok()&&owned.cleaned,"fullProductSuccess":false});
    if let Ok((expected, report, counts, _)) = &result {
        summary["observedCounts"] =
            json!({"passed":counts.passed,"failed":counts.failed,"not_run":counts.not_run});
        summary["sourceCommit"] = expected.source.clone().into();
        summary["sourceManifestSha256"] = expected.manifest.clone().into();
        summary["artifactHashes"] = json!(expected.artifacts);
        summary["pilotCleanupObserved"] = report["cleanupObserved"].clone();
        if passed && cleanup.is_ok() {
            summary["status"] = "passed".into();
            summary["stage"] = "complete".into();
        }
    }
    write_new(
        &bundle.join("apache-public.json"),
        &serde_json::to_vec(&summary)?,
    )?;
    println!("{}", serde_json::to_string(&summary)?);
    let (_, _, _, outcome) = result.map_err(Box::<dyn std::error::Error>::from)?;
    cleanup.map_err(Box::<dyn std::error::Error>::from)?;
    outcome.map_err(Box::<dyn std::error::Error>::from)?;
    ensure(passed, "pilot-not-passed")?;
    Ok(())
}
