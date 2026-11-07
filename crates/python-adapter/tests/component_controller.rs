//! Hosted component orchestration; the actual cases remain in wsgi_server.rs.
#![cfg(all(unix, feature = "host-pilot"))]
#[path = "support/component_lifecycle.rs"]
mod component_lifecycle;
#[path = "support/component_process.rs"]
mod component_process;
#[path = "support/wsgi_receipt.rs"]
mod receipt;
use component_process::cleanup;
#[path = "support/component_files.rs"]
mod component_files;
use component_files::{
    directory_manifest, directory_manifest_excluding_venv, harness_path, source_manifest,
};
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

type Result<T> = std::result::Result<T, String>;
const STEPS: [&str; 9] = [
    "source",
    "dependencies",
    "check",
    "clippy",
    "pure",
    "module",
    "authority",
    "harness",
    "actual",
];

struct Controller {
    root: PathBuf,
    bundle: PathBuf,
    target: PathBuf,
    steps: Vec<Value>,
    source: String,
    dependencies: String,
    status: &'static str,
}
impl Controller {
    fn new() -> Result<Self> {
        if fs::metadata("/proc/self")
            .map_err(|_| "actor identity required")?
            .uid()
            == 0
            || std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
        {
            return Err("ordinary hosted actor required".into());
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let temp = PathBuf::from(
            std::env::var_os("RUNNER_TEMP").ok_or("runner temporary directory required")?,
        )
        .canonicalize()
        .map_err(|e| e.to_string())?;
        let bundle = temp.join("python-component-private");
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&bundle)
            .map_err(|_| "fresh private component bundle required")?;
        let target =
            PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").ok_or("exclusive target required")?)
                .canonicalize()
                .map_err(|e| e.to_string())?;
        if !target.starts_with(&temp) {
            return Err("target must be inside runner temporary directory".into());
        }
        Ok(Self {
            root,
            bundle,
            target,
            steps: STEPS
                .iter()
                .map(|step| json!({"step":step,"status":"NOT_RUN"}))
                .collect(),
            source: String::new(),
            dependencies: String::new(),
            status: "RUNNING",
        })
    }
    fn persist(&self) -> Result<()> {
        let safe = json!({"schema":"humanymous.python-component/1","status":self.status,"sourceSha":std::env::var("GITHUB_SHA").unwrap_or_default(),"steps":self.steps,"wholeProductQualified":false,"productionBrowserEvidence":false,"automaticUpload":false});
        fs::write(
            self.bundle.join("safe-summary.json"),
            serde_json::to_vec_pretty(&safe).unwrap(),
        )
        .map_err(|e| e.to_string())
    }
    fn run(&mut self, step: &str, program: &Path, args: &[&str], seconds: u64) -> Result<Vec<u8>> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&self.root)
            .env("PYTHONDONTWRITEBYTECODE", "1");
        let captured =
            capture_with_stderr(&mut command, b"", Duration::from_secs(seconds), 8 << 20);
        let entry = self
            .steps
            .iter_mut()
            .find(|entry| entry["step"] == step)
            .ok_or("unknown controller step")?;
        match captured {
            Ok(output) => {
                fs::write(
                    self.bundle.join(format!("{step}.stdout.private")),
                    &output.stdout,
                )
                .map_err(|e| e.to_string())?;
                fs::write(
                    self.bundle.join(format!("{step}.stderr.private")),
                    &output.stderr,
                )
                .map_err(|e| e.to_string())?;
                *entry = json!({"step":step,"status":if output.status.success(){"PASS"}else{"FAIL"},"exitCode":output.status.code(),"stdoutSha256":hex::encode(Sha256::digest(&output.stdout)),"stderrSha256":hex::encode(Sha256::digest(&output.stderr))});
                self.persist()?;
                if !output.status.success() {
                    return Err(format!("{step} failed; private output retained"));
                }
                Ok(output.stdout)
            }
            Err(_) => {
                *entry = json!({"step":step,"status":"FAIL","reason":"bounded process supervision failed"});
                self.persist()?;
                Err(format!("{step} process failed"))
            }
        }
    }
    fn cargo(&mut self, step: &str, args: &[&str]) -> Result<Vec<u8>> {
        self.run(step, Path::new("cargo"), args, 900)
    }
    fn execute(&mut self) -> Result<()> {
        let sha = self.run("source", Path::new("git"), &["rev-parse", "HEAD"], 15)?;
        if std::str::from_utf8(&sha)
            .map_err(|_| "source identity encoding")?
            .trim()
            != std::env::var("GITHUB_SHA").map_err(|_| "exact source SHA required")?
        {
            return Err("source SHA mismatch".into());
        }
        self.source = source_manifest(&self.root)?;
        let python = self.bundle.join("venv/bin/python");
        let venv = self.bundle.join("venv");
        self.run(
            "dependencies",
            Path::new("python3"),
            &["-m", "venv", venv.to_str().ok_or("venv path")?],
            120,
        )?;
        self.run(
            "dependencies",
            &python,
            &[
                "-m",
                "pip",
                "install",
                "--disable-pip-version-check",
                "--only-binary=:all:",
                "--require-hashes",
                "-r",
                "adapters/python/waitress-requirement.txt",
            ],
            180,
        )?;
        self.run(
            "dependencies",
            &python,
            &[
                "-m",
                "pip",
                "install",
                "--disable-pip-version-check",
                "--only-binary=:all:",
                "Flask==3.1.3",
                "Django==6.1.3",
            ],
            180,
        )?;
        self.dependencies = directory_manifest(&self.bundle.join("venv"))?;
        self.cargo(
            "check",
            &[
                "check",
                "--locked",
                "-p",
                "humanymous-python-adapter",
                "--features",
                "host-pilot",
                "--all-targets",
            ],
        )?;
        self.cargo(
            "clippy",
            &[
                "clippy",
                "--locked",
                "-p",
                "humanymous-python-adapter",
                "--features",
                "host-pilot",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?;
        let pure = self.cargo(
            "pure",
            &[
                "test",
                "--locked",
                "-p",
                "humanymous-python-adapter",
                "--features",
                "host-pilot",
                "--test",
                "wsgi_server",
                "--",
                "--test-threads=1",
            ],
        )?;
        if !String::from_utf8_lossy(&pure).contains("2 passed; 0 failed; 6 ignored;") {
            return Err("pure framing tests did not execute exactly".into());
        }
        self.cargo(
            "module",
            &[
                "build",
                "--locked",
                "-p",
                "humanymous-python-adapter",
                "--lib",
                "--features",
                "extension-module,host-pilot",
            ],
        )?;
        let module = self.bundle.join("module");
        fs::create_dir(&module).map_err(|e| e.to_string())?;
        fs::copy(
            self.target.join("debug/libhumanymous_python.so"),
            module.join("humanymous_python.so"),
        )
        .map_err(|e| e.to_string())?;
        self.cargo(
            "authority",
            &[
                "build",
                "--locked",
                "-p",
                "humanymous-authority",
                "--bin",
                "humanymous-authority",
            ],
        )?;
        let authority = self.bundle.join("humanymous-authority");
        fs::copy(self.target.join("debug/humanymous-authority"), &authority)
            .map_err(|e| e.to_string())?;
        let build = self.cargo(
            "harness",
            &[
                "test",
                "--locked",
                "-p",
                "humanymous-python-adapter",
                "--features",
                "host-pilot",
                "--test",
                "wsgi_server",
                "--no-run",
                "--message-format=json",
            ],
        )?;
        let executable = harness_path(&build)?;
        let harness = self.bundle.join("wsgi_server");
        fs::copy(executable, &harness).map_err(|e| e.to_string())?;
        for file in [&authority, &harness] {
            fs::set_permissions(file, fs::Permissions::from_mode(0o500))
                .map_err(|e| e.to_string())?;
        }
        fs::set_permissions(
            module.join("humanymous_python.so"),
            fs::Permissions::from_mode(0o400),
        )
        .map_err(|e| e.to_string())?;
        let artifacts = directory_manifest_excluding_venv(&self.bundle)?;
        let mut command = Command::new(&harness);
        command.args(["--list"]);
        let discovery = capture_with_stderr(&mut command, b"", Duration::from_secs(15), 1 << 20)
            .map_err(|_| "test discovery process failed")?;
        if !discovery.status.success() {
            return Err("test discovery failed".into());
        }
        receipt::discovery(
            std::str::from_utf8(&discovery.stdout).map_err(|_| "discovery encoding")?,
        )?;
        // This path is blocked by require_safe_lifecycle until the shared
        // owned-lifetime contract is reviewed; observations do not authorize signals.
        let mut command = Command::new(&harness);
        command
            .args(["--include-ignored", "--test-threads=1", "--nocapture"])
            .env("HUMANYMOUS_HOSTED_NATIVE_PILOT", "1")
            .env("HUMANYMOUS_PYTHON_MODULE_DIR", &module)
            .env(
                "HUMANYMOUS_WAITRESS_SERVE",
                self.bundle.join("venv/bin/waitress-serve"),
            )
            .env("HUMANYMOUS_AUTHORITY_BIN", &authority)
            .env("PYTHONDONTWRITEBYTECODE", "1");
        let captured = component_process::actual(&mut command, &self.bundle);
        let (status, stdout) = match captured {
            Ok(output) => output,
            Err(_) => {
                self.steps[8] = json!({"step":"actual","status":"FAIL","reason":"direct harness deadline or output failure"});
                self.persist()?;
                return Err("actual supervision failed".into());
            }
        };
        fs::write(self.bundle.join("actual.stdout.private"), &stdout).map_err(|e| e.to_string())?;
        let result = if status.success() {
            receipt::validate(std::str::from_utf8(&stdout).map_err(|_| "actual output encoding")?)
        } else {
            Err("actual harness failed".into())
        };
        self.steps[8] = json!({"step":"actual","status":if result.is_ok(){"PASS"}else{"FAIL"},"exitCode":status.code(),"stdoutSha256":hex::encode(Sha256::digest(&stdout)),"requiredTests":8,"requiredRows":14,"requiredCases":28,"stderr":"suppressed-by-shared-managed-child"});
        self.persist()?;
        result?;
        if self.source != source_manifest(&self.root)?
            || self.dependencies != directory_manifest(&self.bundle.join("venv"))?
            || artifacts != directory_manifest_excluding_venv(&self.bundle)?
        {
            return Err("source, dependencies or sealed artifact closure changed".into());
        }
        cleanup(&self.bundle)?;
        println!(
            "{}",
            json!({"schema":"humanymous.python-component/1","sourceSha":std::env::var("GITHUB_SHA").unwrap_or_default(),"tests":8,"rows":14,"actualCases":28,"failed":0,"ignored":0,"filtered":0,"sourceManifestSha256":self.source,"dependencyManifestSha256":self.dependencies,"artifactManifestSha256":artifacts,"wholeProductQualified":false,"productionBrowserEvidence":false})
        );
        Ok(())
    }
}

#[test]
#[ignore = "explicit standard hosted Python component coordinator"]
fn coordinate() -> Result<()> {
    component_lifecycle::require_safe_lifecycle()?;
    let mut controller = Controller::new().expect("fresh owned controller");
    controller.persist().unwrap();
    let result = controller.execute();
    let cleaned = cleanup(&controller.bundle);
    controller.status = if result.is_ok() && cleaned.is_ok() {
        "PASS"
    } else {
        "FAIL"
    };
    controller.persist().unwrap();
    println!(
        "{}",
        json!({"schema":"humanymous.python-component-status/1","status":controller.status,"steps":controller.steps,"ownedProcessScanEmpty":cleaned.is_ok(),"wholeProductQualified":false,"automaticUpload":false})
    );
    assert!(
        result.is_ok() && cleaned.is_ok(),
        "Python component failed; safe counts and private diagnostics retained"
    );
    Ok(())
}

#[test]
#[ignore = "always-run owned task cleanup observation"]
fn coordinate_cleanup() {
    let bundle = PathBuf::from(std::env::var_os("RUNNER_TEMP").expect("runner temp"))
        .join("python-component-private");
    if bundle.exists() {
        cleanup(&bundle).expect("owned process cleanup must complete");
    }
}
