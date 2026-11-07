//! Optional, privacy-limited Cargo wall-time observations, never reuse authority.
use crate::source;
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::OnceLock,
    time::Instant,
};

fn kind(program: &str, args: &[String]) -> Option<&'static str> {
    if !matches!(program, "cargo" | "cargo.exe") {
        return None;
    }
    match args.first()?.as_str() {
        "build" => Some("cargo-build"),
        "test" => Some("cargo-test"),
        "check" => Some("cargo-check"),
        "clippy" => Some("cargo-clippy"),
        "run" => Some("cargo-run"),
        "metadata" => Some("cargo-metadata"),
        "package" => Some("cargo-package"),
        "audit" => Some("cargo-audit"),
        "install" => Some("cargo-install"),
        _ => None,
    }
}
fn timed_args(program: &str, args: &[&str], enabled: bool) -> Vec<String> {
    let mut result: Vec<_> = args.iter().map(|value| (*value).to_owned()).collect();
    let boundary = args
        .iter()
        .position(|value| *value == "--")
        .unwrap_or(args.len());
    if enabled
        && matches!(program, "cargo" | "cargo.exe")
        && args
            .first()
            .is_some_and(|value| matches!(*value, "build" | "test" | "check" | "clippy" | "run"))
        && !args[..boundary]
            .iter()
            .any(|value| value.starts_with("--timings"))
    {
        result.insert(boundary, "--timings".into());
    }
    result
}
pub fn arguments(program: &str, args: &[&str]) -> Vec<String> {
    timed_args(
        program,
        args,
        std::env::var("HUMANYMOUS_CARGO_TIMINGS").as_deref() == Ok("1"),
    )
}
/// Explicit Cargo descriptor for the single fixed GNU timeout route.
pub struct CargoTimeout {
    pub inner: Vec<String>,
    pub outer: Vec<String>,
}
fn timeout_arguments(args: &[&str], budget: &str, enabled: bool) -> CargoTimeout {
    let inner = timed_args("cargo", args, enabled);
    let mut outer: Vec<String> = ["--signal=TERM", "--kill-after=15s", budget, "cargo"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    outer.extend(inner.iter().cloned());
    CargoTimeout { inner, outer }
}
pub fn cargo_timeout(args: &[&str], budget: &str) -> CargoTimeout {
    timeout_arguments(
        args,
        budget,
        std::env::var("HUMANYMOUS_CARGO_TIMINGS").as_deref() == Ok("1"),
    )
}
fn option(args: &[String], name: &str) -> Option<String> {
    let boundary = args
        .iter()
        .position(|value| value == "--")
        .unwrap_or(args.len());
    args[..boundary]
        .iter()
        .enumerate()
        .find_map(|(index, value)| {
            if value == name {
                args.get(index + 1)
                    .filter(|_| index + 1 < boundary)
                    .cloned()
            } else {
                value.strip_prefix(&format!("{name}=")).map(str::to_owned)
            }
        })
}
fn candidate(value: Option<&OsStr>) -> Value {
    json!({"present":value.is_some(),"explicitly_empty":value.is_some_and(OsStr::is_empty),
        "sha256":value.map(|value|source::sha(value.as_encoded_bytes()))})
}
fn directories(args: &[String], target: Option<&OsStr>, build: Option<&OsStr>) -> Value {
    let cli = option(args, "--target-dir");
    json!({"cli_target_dir":candidate(cli.as_deref().map(OsStr::new)),
        "inherited_cargo_target_dir":candidate(target),
        "inherited_cargo_build_build_dir":candidate(build),
        "effective_target_dir":null,"effective_build_dir":null,
        "cargo_configuration_resolved":false})
}
fn requested(args: &[String]) -> Value {
    let boundary = args
        .iter()
        .position(|value| value == "--")
        .unwrap_or(args.len());
    let outer = &args[..boundary];
    let profile = option(args, "--profile").unwrap_or_else(|| {
        if outer.iter().any(|arg| arg == "--release") {
            "release".into()
        } else {
            "default".into()
        }
    });
    let features: Vec<_> = outer
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if value == "--features" {
                outer.get(index + 1).cloned()
            } else {
                value.strip_prefix("--features=").map(str::to_owned)
            }
        })
        .collect();
    json!({"profile_sha256":source::sha(profile.as_bytes()),
        "argv_sha256":source::sha(&serde_json::to_vec(args).unwrap_or_default()),
        "features_sha256":source::sha(&serde_json::to_vec(&features).unwrap_or_default()),
        "target_sha256":option(args,"--target").map(|target| source::sha(target.as_bytes())),
        "all_features":outer.iter().any(|arg| arg == "--all-features"),
        "default_features":!outer.iter().any(|arg| arg == "--no-default-features"),
        "timings_requested":args.iter().take_while(|arg| arg.as_str() != "--").any(|arg| arg.starts_with("--timings"))})
}
fn rustc_identity(root: &Path, allow_probe: bool) -> Option<String> {
    static IDENTITY: OnceLock<Option<String>> = OnceLock::new();
    if !allow_probe {
        return IDENTITY.get().cloned().flatten();
    }
    IDENTITY
        .get_or_init(|| {
            Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .arg("-vV")
                .current_dir(root)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| source::sha(&output.stdout))
        })
        .clone()
}
#[derive(Clone, Copy)]
enum Observation {
    Direct,
    CargoTimeout,
}
impl Observation {
    fn may_probe_rustc(self) -> bool {
        matches!(self, Self::Direct)
    }
    fn elapsed_scope(self) -> &'static str {
        match self {
            Self::Direct => "direct_command_including_startup_wait",
            Self::CargoTimeout => "wrapper_including_startup_wait",
        }
    }
    fn process_scope(self) -> &'static str {
        match self {
            Self::Direct => "direct_command",
            Self::CargoTimeout => "gnu_timeout_wrapper",
        }
    }
}
pub struct Measurement {
    kind: Option<&'static str>,
    started: Instant,
    inputs: Value,
    lock: Option<String>,
    observation: Observation,
}
impl Measurement {
    pub fn start(root: &Path, program: &str, args: &[String]) -> Self {
        Self::with_scope(root, program, args, Observation::Direct)
    }
    pub fn wrapped_cargo(root: &Path, args: &[String]) -> Self {
        Self::with_scope(root, "cargo", args, Observation::CargoTimeout)
    }
    fn with_scope(root: &Path, program: &str, args: &[String], observation: Observation) -> Self {
        let kind = kind(program, args);
        let lock = kind.and_then(|_| source::file(&root.join("Cargo.lock")).ok());
        let inputs = if kind.is_some() {
            json!({"host_os":std::env::consts::OS,"host_arch":std::env::consts::ARCH,
                "rustc_vv_sha256":rustc_identity(root,observation.may_probe_rustc()),"root_context_lock_sha256":lock,
                "requested":requested(args),
                "rustflags_sha256":source::sha(std::env::var("CARGO_ENCODED_RUSTFLAGS").or_else(|_|std::env::var("RUSTFLAGS")).unwrap_or_default().as_bytes()),
                "target_dir_sha256":null,
                "directory_requests":directories(args,std::env::var_os("CARGO_TARGET_DIR").as_deref(),std::env::var_os("CARGO_BUILD_BUILD_DIR").as_deref()),
                "inherited_flag_requests":{"rustflags":candidate(std::env::var_os("RUSTFLAGS").as_deref()),
                    "encoded_rustflags":candidate(std::env::var_os("CARGO_ENCODED_RUSTFLAGS").as_deref())},
                "reuse_key_complete":false,"linked_artifact_reuse_qualified":false,
                "unknown_dimensions":["resolved feature graph","effective Cargo configuration and linker","native SDK ABI/tool identities","command-specific isolated dependency lock"]})
        } else {
            Value::Null
        };
        Self {
            kind,
            started: Instant::now(),
            inputs,
            lock,
            observation,
        }
    }
    pub fn finish(self, root: &Path, status: Option<ExitStatus>) {
        let elapsed_ms = self.started.elapsed().as_millis();
        let Some(kind) = self.kind else {
            return;
        };
        let record = json!({"schema":"humanymous.command-metric/1","command_kind":kind,
            "elapsed_scope":self.observation.elapsed_scope(),
            "observed_process_scope":self.observation.process_scope(),
            "elapsed_ms":elapsed_ms,"spawned":status.is_some(),"success":status.is_some_and(|status|status.success()),
            "exit_code":status.and_then(|status|status.code()),"inputs":self.inputs,
            "root_lock_changed":self.lock != source::file(&root.join("Cargo.lock")).ok(),
            "timing_html_scope":"private target/cargo-timings; not uploaded or parsed as machine timing"});
        // Keep stdout machine-readable for existing Cargo metadata/package callers.
        eprintln!("{record}");
        let directory = root.join("target/delivery-evidence");
        let persist = || -> std::io::Result<()> {
            std::fs::create_dir_all(&directory)?;
            if !directory.canonicalize()?.starts_with(root.canonicalize()?) {
                return Err(std::io::Error::other("metric path escaped checkout"));
            }
            let path = directory.join("command-metrics.jsonl");
            if path
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Err(std::io::Error::other("metric symlink refused"));
            }
            writeln!(
                OpenOptions::new().create(true).append(true).open(path)?,
                "{record}"
            )
        };
        if persist().is_err() {
            eprintln!(
                "{{\"schema\":\"humanymous.command-metric-storage/1\",\"status\":\"unavailable\"}}"
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_wrapped_cargo_preserves_stop_flags_and_forwarding() {
        let invocation = timeout_arguments(&["test", "--", "PRIVATE_OWNER_ARGUMENT"], "180s", true);
        assert_eq!(
            invocation.outer,
            [
                "--signal=TERM",
                "--kill-after=15s",
                "180s",
                "cargo",
                "test",
                "--timings",
                "--",
                "PRIVATE_OWNER_ARGUMENT"
            ]
        );
        assert_eq!(kind("timeout", &invocation.outer), None);
        assert_eq!(kind("cargo", &invocation.inner), Some("cargo-test"));
        assert_eq!(
            Observation::CargoTimeout.process_scope(),
            "gnu_timeout_wrapper"
        );
        assert!(!Observation::CargoTimeout.may_probe_rustc());
        assert!(Observation::Direct.may_probe_rustc());
        assert_eq!(
            Observation::CargoTimeout.elapsed_scope(),
            "wrapper_including_startup_wait"
        );
        assert_ne!(
            Observation::Direct.process_scope(),
            Observation::CargoTimeout.process_scope()
        );
        let disabled = timeout_arguments(&["build"], "1500s", false);
        assert_eq!(
            disabled.outer,
            [
                "--signal=TERM",
                "--kill-after=15s",
                "1500s",
                "cargo",
                "build"
            ]
        );
        assert!(
            !timeout_arguments(&["install", "cbindgen"], "180s", true)
                .inner
                .iter()
                .any(|arg| arg == "--timings")
        );
    }
    #[test]
    fn directory_candidates_preserve_cli_environment_and_unknown_effective_identity() {
        let args = [
            "install",
            "--target-dir=PRIVATE_CLI",
            "--",
            "--target-dir=FORWARDED",
        ]
        .map(str::to_owned);
        let value = directories(
            &args,
            Some(OsStr::new("PRIVATE_INHERITED")),
            Some(OsStr::new("PRIVATE_BUILD")),
        );
        assert_eq!(
            value["cli_target_dir"]["sha256"],
            source::sha(b"PRIVATE_CLI")
        );
        assert_eq!(
            value["inherited_cargo_target_dir"]["sha256"],
            source::sha(b"PRIVATE_INHERITED")
        );
        assert_eq!(
            value["inherited_cargo_build_build_dir"]["sha256"],
            source::sha(b"PRIVATE_BUILD")
        );
        assert!(value["effective_target_dir"].is_null());
        assert!(value["effective_build_dir"].is_null());
        assert_eq!(value["cargo_configuration_resolved"], false);
        assert!(!value.to_string().contains("PRIVATE"));
        let separated = ["build", "--target-dir", "PRIVATE_CLI"].map(str::to_owned);
        assert_eq!(
            directories(&separated, None, None)["cli_target_dir"],
            value["cli_target_dir"]
        );
        let forwarded = ["run", "--", "--target-dir", "PRIVATE_CLI"].map(str::to_owned);
        assert_eq!(
            directories(&forwarded, None, None)["cli_target_dir"]["present"],
            false
        );
    }
    #[test]
    fn unset_and_empty_environment_candidates_are_distinct_without_plaintext() {
        let unset = candidate(None);
        let empty = candidate(Some(OsStr::new("")));
        assert_eq!(unset["present"], false);
        assert!(unset["sha256"].is_null());
        assert_eq!(empty["present"], true);
        assert_eq!(empty["explicitly_empty"], true);
        assert_eq!(empty["sha256"], source::sha(b""));
        assert_ne!(unset, empty);
    }
    #[test]
    fn timing_flags_stay_outside_forwarded_owner_arguments() {
        assert_eq!(
            timed_args("cargo", &["run", "--", "--timings"], true),
            vec!["run", "--timings", "--", "--timings"]
        );
        assert_eq!(
            timed_args("cargo", &["metadata", "--format-version=1"], true),
            vec!["metadata", "--format-version=1"]
        );
        assert_eq!(
            timed_args("cargo", &["build", "--timings"], true),
            vec!["build", "--timings"]
        );
    }
    #[test]
    fn arbitrary_arguments_are_not_metric_payloads_or_reuse_authority() {
        let args = vec!["build".into(), "--features".into(), "private-canary".into()];
        assert!(!requested(&args).to_string().contains("private-canary"));
        assert_eq!(kind("cargo", &args), Some("cargo-build"));
        assert_eq!(kind("unknown-private-command", &args), None);
    }
}
