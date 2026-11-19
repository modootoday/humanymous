//! Bounded subprocess transport; never invokes a shell or arbitrary script.
use humanymous_platform::testing::process::{ManagedChild, capture_with_stderr};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy)]
enum CommandBudget {
    Control,
    Input,
    ImageBuild,
    Observation,
}
impl CommandBudget {
    fn seconds(self) -> u64 {
        match self {
            Self::Control => 600,
            Self::Input => 30,
            Self::ImageBuild => 1200,
            Self::Observation => 30,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailureKind {
    Deadline,
    Capture,
    Exit,
}
impl FailureKind {
    fn output(success: bool, valid_utf8: bool) -> Option<Self> {
        if !success {
            Some(Self::Exit)
        } else if !valid_utf8 {
            Some(Self::Capture)
        } else {
            None
        }
    }
    fn capture(error: &std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::TimedOut {
            Self::Deadline
        } else {
            Self::Capture
        }
    }
    pub fn go_build_stage(self, service: &str) -> Option<&'static str> {
        match (service, self) {
            ("go-gate", Self::Deadline) => Some("go_runtime_build_deadline"),
            ("go-gate", Self::Capture) => Some("go_runtime_build_capture"),
            ("go-gate", Self::Exit) => Some("go_runtime_build_exit"),
            ("go-egress-canary", Self::Deadline) => Some("go_canary_build_deadline"),
            ("go-egress-canary", Self::Capture) => Some("go_canary_build_capture"),
            ("go-egress-canary", Self::Exit) => Some("go_canary_build_exit"),
            _ => None,
        }
    }
}
mod affinity;
/// Local engine endpoint of an explicitly named cloud-container venue.
pub const CLOUD_CONTAINER_ENDPOINT: &str = "unix:///var/run/docker.sock";
/// Engine venue chosen by the operator. Never inferred from the daemon the CLI
/// currently selects: the cloud-container venue exists only when it is named
/// together with the engine ID the operator observed before the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Venue {
    /// Rule 96 affinity: genuine GitHub-hosted Linux, the Windows Desktop
    /// `desktop-linux` engine, or an allocated Desktop CLI proxy.
    Affinity,
    /// A cloud session container's own Linux engine on the local Unix socket,
    /// bound to an explicitly supplied engine ID. Never a qualifying venue.
    CloudContainer { engine_id: String },
}
impl Venue {
    /// Parse `--venue` and `--engine-id`. Both or neither; only one venue name.
    pub fn parse(venue: Option<&str>, engine_id: Option<&str>) -> Result<Self> {
        match (venue, engine_id) {
            (None, None) => Ok(Self::Affinity),
            (Some("cloud-container"), Some(id)) => {
                if id.is_empty()
                    || id.len() > 256
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-:".contains(&b))
                {
                    return Err("--engine-id must be the observed engine ID".into());
                }
                Ok(Self::CloudContainer {
                    engine_id: id.to_owned(),
                })
            }
            (Some("cloud-container"), None) => Err(
                "--venue cloud-container requires --engine-id ID observed before the run".into(),
            ),
            (Some(_), _) => Err("--venue accepts only cloud-container; hosted and Desktop venues follow rule 96 without a flag".into()),
            (None, Some(_)) => Err("--engine-id binds only an explicit --venue".into()),
        }
    }
    /// Only rule 96 affinity venues can qualify a product run.
    pub fn qualifying(&self) -> bool {
        matches!(self, Self::Affinity)
    }
    /// Receipt fields: the venue name, its bound engine ID and its limits.
    pub fn report(&self) -> Value {
        match self {
            Self::Affinity => {
                json!({"venue":"rule-96-affinity","qualifying_venue":true,"explicit":false})
            }
            Self::CloudContainer { engine_id } => {
                json!({"venue":"cloud-container","explicit":true,"endpoint":CLOUD_CONTAINER_ENDPOINT,
                "bound_engine_id":engine_id,"authenticated_provenance":false,"qualifying_venue":false,
                "github_hosted":false,"windows_desktop_affinity_claimed":false})
            }
        }
    }
}
/// The cloud-container venue runs only where it was named: not on Windows, not
/// on genuine GitHub-hosted Linux, and with no engine or Desktop override in
/// the environment that could point the CLI at another daemon.
fn cloud_container_environment(
    windows: bool,
    hosted: bool,
    variable: &dyn Fn(&str) -> Option<String>,
) -> Result<()> {
    if windows || hosted {
        return Err("cloud-container venue refused on Windows Desktop or GitHub-hosted Linux; those use rule 96 affinity".into());
    }
    for name in [
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "HUMANYMOUS_DOCKER_DESKTOP_PROXY",
        "HUMANYMOUS_DOCKER_DESKTOP_BASELINE",
    ] {
        let value = variable(name).unwrap_or_default();
        if !value.is_empty() && !(name == "DOCKER_HOST" && value == CLOUD_CONTAINER_ENDPOINT) {
            return Err(format!(
                "{name} refused for the cloud-container venue; its endpoint is explicit"
            )
            .into());
        }
    }
    Ok(())
}
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
    pub(crate) last_failure: Option<FailureKind>,
    desktop_baseline: Option<Value>,
    venue: Venue,
}
pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}
impl Docker {
    pub fn new(root: &Path) -> Self {
        Self::with_venue(root, Venue::Affinity)
    }
    pub fn with_venue(root: &Path, venue: Venue) -> Self {
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
            last_failure: None,
            desktop_baseline: None,
            venue,
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
        if matches!(self.venue, Venue::CloudContainer { .. }) {
            cloud_container_environment(cfg!(windows), actual_hosted_linux(), &|name| {
                std::env::var(name).ok()
            })?;
            command.args(["--host", CLOUD_CONTAINER_ENDPOINT]);
        } else if cfg!(windows) {
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
        self.invoke_budget(
            args,
            environment,
            required,
            input,
            if input.is_empty() {
                CommandBudget::Control
            } else {
                CommandBudget::Input
            },
        )
    }
    pub fn invoke_image_build(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
    ) -> Result<Output> {
        self.invoke_budget(args, environment, required, b"", CommandBudget::ImageBuild)
    }
    pub fn invoke_observation(&mut self, args: &[String]) -> Result<Output> {
        self.invoke_budget(args, &[], true, b"", CommandBudget::Observation)
    }
    fn invoke_budget(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
        input: &[u8],
        budget: CommandBudget,
    ) -> Result<Output> {
        self.last_failure = None;
        let mut command = self.command(args, environment)?;
        let captured = capture_with_stderr(
            &mut command,
            input,
            Duration::from_secs(budget.seconds()),
            4 * 1024 * 1024,
        );
        let captured = match captured {
            Ok(value) => value,
            Err(error) => {
                self.last_failure = Some(FailureKind::capture(&error));
                self.transcript.push(json!({"argv":args,"budget_seconds":budget.seconds(),"capture_error_kind":format!("{:?}",error.kind()),"detail":error.to_string(),"partial_output_unavailable":true}));
                return Err(error.into());
            }
        };
        let status = captured.status;
        let stdout_valid_utf8 = std::str::from_utf8(&captured.stdout).is_ok();
        let stdout = String::from_utf8_lossy(&captured.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&captured.stderr).into_owned();
        self.transcript
            .push(json!({"argv":args,"budget_seconds":budget.seconds(),"exit":status.code(),"stdout":stdout,"stdout_valid_utf8":stdout_valid_utf8,"stderr":stderr}));
        self.last_failure = FailureKind::output(status.success(), stdout_valid_utf8);
        if required && !status.success() {
            return Err(format!(
                "Docker {} failed, exit {:?}; see private transcript",
                args.first().map_or("", String::as_str),
                status.code()
            )
            .into());
        }
        if !stdout_valid_utf8 {
            return Err(
                "Docker stdout is not UTF-8; observed exit and lossy text retained privately"
                    .into(),
            );
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
    let (mut evidence, mut venue, mut engine_id) = (None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let slot = match flag.as_str() {
            "--evidence" => &mut evidence,
            "--venue" => &mut venue,
            "--engine-id" => &mut engine_id,
            _ => return Err("engine-preflight --evidence FRESH_PRIVATE_BUNDLE [--venue cloud-container --engine-id ID] required".into()),
        };
        if slot
            .replace(rest.next().ok_or("flag value missing")?)
            .is_some()
        {
            return Err("engine-preflight flag repeated".into());
        }
    }
    let venue = Venue::parse(venue.map(String::as_str), engine_id.map(String::as_str))?;
    let root = crate::snapshot::root()?;
    let evidence =
        PathBuf::from(evidence.ok_or("engine-preflight --evidence required")?).canonicalize()?;
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
    let mut docker = Docker::with_venue(&root, venue.clone());
    let result = docker.recheck_engine();
    let report = json!({"schema":"humanymous.lab-engine-preflight/1","status":if result.is_ok(){"observed"}else{"failed"},"venue":venue.report(),"observation":docker.engine_identity,"private_transcript":docker.transcript,"error":result.as_ref().err().map(|e|e.to_string()),"executable_sha256":crate::snapshot::digest(&std::fs::read(std::env::current_exe()?)?),"actual_sdk_connection_observed":false,"full_product_qualified":false});
    crate::snapshot::write(&evidence, "engine-preflight.private.json", &report)?;
    println!(
        "{}",
        json!({"schema":"humanymous.lab-engine-preflight-public/1","status":report["status"],"private_report_sha256":crate::snapshot::digest(&std::fs::read(path)?),"venue":venue.report(),"authenticated_provenance":false,"actual_sdk_connection_observed":false,"full_product_qualified":false})
    );
    result?;
    Ok(0)
}
#[cfg(test)]
mod venue_tests {
    use super::{CLOUD_CONTAINER_ENDPOINT, Venue, cloud_container_environment};
    #[test]
    fn cloud_container_venue_is_explicit_and_bound_to_an_engine_id() {
        assert_eq!(Venue::parse(None, None).unwrap(), Venue::Affinity);
        assert!(Venue::Affinity.qualifying());
        let venue = Venue::parse(Some("cloud-container"), Some("c7f5a196-202c")).unwrap();
        assert!(matches!(venue, Venue::CloudContainer { .. }) && !venue.qualifying());
        let report = venue.report();
        assert_eq!(report["bound_engine_id"], "c7f5a196-202c");
        assert_eq!(report["authenticated_provenance"], false);
        assert_eq!(report["qualifying_venue"], false);
        for (name, id) in [
            (Some("cloud-container"), None),
            (Some("cloud-container"), Some("")),
            (Some("cloud-container"), Some("id with space")),
            (Some("github-hosted"), Some("id")),
            (Some("windows-desktop"), None),
            (None, Some("id")),
        ] {
            assert!(Venue::parse(name, id).is_err(), "{name:?} {id:?}");
        }
    }
    #[test]
    fn cloud_container_venue_refuses_other_engines_and_hosted_runs() {
        let none = |_: &str| None;
        assert!(cloud_container_environment(false, false, &none).is_ok());
        assert!(cloud_container_environment(true, false, &none).is_err());
        assert!(cloud_container_environment(false, true, &none).is_err());
        let same = |name: &str| (name == "DOCKER_HOST").then(|| CLOUD_CONTAINER_ENDPOINT.into());
        assert!(cloud_container_environment(false, false, &same).is_ok());
        for name in [
            "DOCKER_HOST",
            "DOCKER_CONTEXT",
            "HUMANYMOUS_DOCKER_DESKTOP_PROXY",
            "HUMANYMOUS_DOCKER_DESKTOP_BASELINE",
        ] {
            let other = |n: &str| (n == name).then(|| "unix:///other.sock".to_owned());
            assert!(
                cloud_container_environment(false, false, &other).is_err(),
                "{name}"
            );
        }
    }
}
#[cfg(test)]
mod failure_tests {
    use super::FailureKind;
    #[test]
    fn build_failures_use_typed_errors_and_only_owned_static_targets() {
        assert_eq!(super::CommandBudget::Control.seconds(), 600);
        assert_eq!(super::CommandBudget::Input.seconds(), 30);
        assert_eq!(super::CommandBudget::ImageBuild.seconds(), 1200);
        assert_eq!(super::CommandBudget::Observation.seconds(), 30);
        assert_eq!(FailureKind::output(false, false), Some(FailureKind::Exit));
        assert_eq!(FailureKind::output(true, false), Some(FailureKind::Capture));
        assert_eq!(FailureKind::output(true, true), None);
        let deadline = FailureKind::capture(&std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "PRIVATE",
        ));
        assert_eq!(
            deadline.go_build_stage("go-gate"),
            Some("go_runtime_build_deadline")
        );
        let other = FailureKind::capture(&std::io::Error::other("deadline PRIVATE"));
        assert_eq!(
            other.go_build_stage("go-gate"),
            Some("go_runtime_build_capture")
        );
        assert_eq!(
            FailureKind::Exit.go_build_stage("go-gate"),
            Some("go_runtime_build_exit")
        );
        assert_eq!(
            deadline.go_build_stage("go-egress-canary"),
            Some("go_canary_build_deadline")
        );
        for kind in [deadline, other, FailureKind::Exit] {
            assert_eq!(kind.go_build_stage("PRIVATE_TARGET"), None);
        }
    }
}
