//! Bounded subprocess transport; never invokes a shell or arbitrary script.
use humanymous_test_support::process::{ManagedChild, capture_with_stderr};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
mod affinity;
pub(crate) fn hosted_context(
    actions: Option<&str>,
    os: Option<&str>,
    environment: Option<&str>,
    act_present: bool,
) -> bool {
    actions == Some("true")
        && os == Some("Linux")
        && environment == Some("github-hosted")
        && !act_present
}
pub(crate) fn actual_hosted_linux() -> bool {
    hosted_context(
        std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        std::env::var("RUNNER_OS").ok().as_deref(),
        std::env::var("RUNNER_ENVIRONMENT").ok().as_deref(),
        std::env::var_os("ACT").is_some(),
    )
}
pub struct Docker {
    pub root: PathBuf,
    executable: PathBuf,
    pub transcript: Vec<Value>,
    pub engine_identity: Option<Value>,
    desktop_baseline: Option<Value>,
}
pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}
impl Docker {
    pub fn new(root: &Path) -> Self {
        let windows = PathBuf::from("C:/Program Files/Docker/Docker/resources/bin/docker.exe");
        Self {
            root: root.into(),
            executable: if cfg!(windows) {
                windows
            } else {
                "docker".into()
            },
            transcript: Vec::new(),
            engine_identity: None,
            desktop_baseline: None,
        }
    }
    pub fn invoke(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
    ) -> Result<Output> {
        self.invoke_input(args, environment, required, b"")
    }
    fn raw_command(&self, args: &[String], environment: &[(String, String)]) -> Result<Command> {
        let mut command = Command::new(&self.executable);
        if cfg!(windows) {
            if !self.executable.is_file() {
                return Err("installed Windows Docker Desktop CLI missing; no WSL fallback".into());
            }
            command.args(["--context", "desktop-linux"]);
        } else if !actual_hosted_linux() {
            let proxy=std::env::var("HUMANYMOUS_DOCKER_DESKTOP_PROXY").map_err(|_|"local Linux Docker requires explicitly allocated Desktop proxy; no WSL fallback")?;
            if !proxy.starts_with("unix:///")
                || proxy.contains("..")
                || proxy.len() > 2048
                || std::env::var("DOCKER_HOST").ok().as_deref() != Some(proxy.as_str())
            {
                return Err("explicit Desktop proxy and configured DOCKER_HOST must match".into());
            }
            command.args(["--host", &proxy]);
        }
        command
            .args(args)
            .current_dir(&self.root)
            .envs(environment.iter().cloned());
        if let Some(parent) = self
            .executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            let mut paths = vec![parent.to_path_buf()];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            command.env("PATH", std::env::join_paths(paths)?);
        }
        Ok(command)
    }
    pub fn command(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
    ) -> Result<Command> {
        if environment.iter().any(|(name, _)| {
            matches!(
                name.as_str(),
                "DOCKER_HOST"
                    | "DOCKER_CONTEXT"
                    | "DOCKER_CONFIG"
                    | "HUMANYMOUS_DOCKER_DESKTOP_PROXY"
                    | "HUMANYMOUS_DOCKER_DESKTOP_BASELINE"
                    | "GITHUB_ACTIONS"
                    | "RUNNER_OS"
                    | "RUNNER_ENVIRONMENT"
                    | "ACT"
            )
        }) {
            return Err("engine/provenance overrides require parent allocation and preflight, not per-command substitution".into());
        }
        let mutation_boundary = !actual_hosted_linux()
            && args.iter().any(|arg| {
                matches!(
                    arg.as_str(),
                    "build" | "create" | "run" | "up" | "start" | "down"
                )
            });
        self.preflight(mutation_boundary)?;
        self.raw_command(args, environment)
    }
    pub fn recheck_engine(&mut self) -> Result<()> {
        self.preflight(true)
    }
    pub fn invoke_input(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
        input: &[u8],
    ) -> Result<Output> {
        let mut command = self.command(args, environment)?;
        let captured = capture_with_stderr(
            &mut command,
            input,
            Duration::from_secs(if input.is_empty() { 600 } else { 30 }),
            4 * 1024 * 1024,
        )?;
        let status = captured.status;
        let stdout = String::from_utf8(captured.stdout)?;
        let stderr = String::from_utf8_lossy(&captured.stderr).into_owned();
        self.transcript
            .push(json!({"argv":args,"exit":status.code(),"stdout":stdout,"stderr":stderr}));
        if required && !status.success() {
            return Err(format!(
                "Docker {} failed, exit {:?}; see private transcript",
                args.first().map_or("", String::as_str),
                status.code()
            )
            .into());
        }
        Ok(Output {
            success: status.success(),
            stdout,
            stderr,
        })
    }
    pub fn duplex(
        &mut self,
        args: &[String],
    ) -> Result<(
        ManagedChild,
        std::process::ChildStdin,
        std::process::ChildStdout,
    )> {
        self.transcript
            .push(json!({"argv":args,"transport":"private-duplex","input_recorded":false}));
        Ok(ManagedChild::spawn_duplex(&mut self.command(args, &[])?)?)
    }
    pub fn call(&mut self, args: &[&str]) -> Result<String> {
        Ok(self
            .invoke(
                &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                &[],
                true,
            )?
            .stdout)
    }
}
pub fn engine_preflight(args: &[String]) -> Result<i32> {
    if args.len() != 2 || args[0] != "--evidence" {
        return Err("engine-preflight --evidence FRESH_PRIVATE_BUNDLE required".into());
    }
    let root = crate::snapshot::root()?;
    let evidence = PathBuf::from(&args[1]).canonicalize()?;
    if !evidence.starts_with(root.join(".spec.local/root/evidence"))
        || !evidence.join("index.evidence.md").is_file()
    {
        return Err("existing private Specfold evidence bundle required".into());
    }
    let path = evidence.join("engine-preflight.private.json");
    if path.exists()
        || evidence.join("lab-report.json").exists()
        || evidence.join("source-manifest.json").exists()
    {
        return Err("fresh preflight evidence bundle required; preserve prior runs".into());
    }
    let mut docker = Docker::new(&root);
    let result = docker.recheck_engine();
    let report = json!({"schema":"humanymous.lab-engine-preflight/1","status":if result.is_ok(){"observed"}else{"failed"},"observation":docker.engine_identity,"private_transcript":docker.transcript,"error":result.as_ref().err().map(|e|e.to_string()),"executable_sha256":crate::snapshot::digest(&std::fs::read(std::env::current_exe()?)?),"actual_sdk_connection_observed":false,"full_product_qualified":false});
    crate::snapshot::write(&evidence, "engine-preflight.private.json", &report)?;
    println!(
        "{}",
        json!({"schema":"humanymous.lab-engine-preflight-public/1","status":report["status"],"private_report_sha256":crate::snapshot::digest(&std::fs::read(path)?),"authenticated_provenance":false,"actual_sdk_connection_observed":false,"full_product_qualified":false})
    );
    result?;
    Ok(0)
}
