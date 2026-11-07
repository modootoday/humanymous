//! Optional, privacy-limited Cargo wall-time observations, never reuse authority.
use crate::source;
use serde_json::{Value, json};
use std::{
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
fn rustc_identity(root: &Path) -> Option<String> {
    static IDENTITY: OnceLock<Option<String>> = OnceLock::new();
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
pub struct Measurement {
    kind: Option<&'static str>,
    started: Instant,
    inputs: Value,
    lock: Option<String>,
}
impl Measurement {
    pub fn start(root: &Path, program: &str, args: &[String]) -> Self {
        let kind = kind(program, args);
        let lock = kind.and_then(|_| source::file(&root.join("Cargo.lock")).ok());
        let inputs = if kind.is_some() {
            json!({"host_os":std::env::consts::OS,"host_arch":std::env::consts::ARCH,
                "rustc_vv_sha256":rustc_identity(root),"root_context_lock_sha256":lock,
                "requested":requested(args),
                "rustflags_sha256":source::sha(std::env::var("CARGO_ENCODED_RUSTFLAGS").or_else(|_|std::env::var("RUSTFLAGS")).unwrap_or_default().as_bytes()),
                "target_dir_sha256":source::sha(std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_|"target".into()).as_bytes()),
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
        }
    }
    pub fn finish(self, root: &Path, status: Option<ExitStatus>) {
        let elapsed_ms = self.started.elapsed().as_millis();
        let Some(kind) = self.kind else {
            return;
        };
        let record = json!({"schema":"humanymous.command-metric/1","command_kind":kind,
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
