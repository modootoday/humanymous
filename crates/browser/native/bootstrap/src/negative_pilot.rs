//! Test-only refusal pilot; shared ownership bounds and reaps each direct child.
use humanymous_test_support::process::ManagedChild;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
enum ErrorLabel {
    None,
    NotFound,
    PermissionDenied,
    TimedOut,
    Interrupted,
    Other,
}
impl ErrorLabel {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NotFound => "not_found",
            Self::PermissionDenied => "permission_denied",
            Self::TimedOut => "timed_out",
            Self::Interrupted => "interrupted",
            Self::Other => "other",
        }
    }
}
struct Facts {
    spawn_ok: bool,
    wait_completed: bool,
    deadline_expired: Option<bool>,
    reaped: Option<bool>,
    exit_code: Option<i32>,
    cleanup_attempted: bool,
    spawn_error_kind: ErrorLabel,
    wait_error_kind: ErrorLabel,
    cleanup_error_kind: ErrorLabel,
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            spawn_ok: false,
            wait_completed: false,
            deadline_expired: None,
            reaped: None,
            exit_code: None,
            cleanup_attempted: false,
            spawn_error_kind: ErrorLabel::None,
            wait_error_kind: ErrorLabel::None,
            cleanup_error_kind: ErrorLabel::None,
        }
    }
}
fn config() -> Value {
    let contract: Value = serde_json::from_str(include_str!("../../desktop/pilot-contract.json"))
        .expect("owned bootstrap contract required");
    let value = contract["bootstrap_rejection"].clone();
    assert_eq!(value["observation_schema"], 1);
    assert_eq!(value["expected_exit_code"], 78);
    assert_eq!(value["expected_profile_changed"], false);
    assert_eq!(value["deadline_ms"], 10000);
    assert_eq!(value["max_record_bytes"], 1024);
    let cases = value["cases"]
        .as_array()
        .expect("fixed bootstrap cases required");
    assert_eq!(cases.len(), 4);
    assert_eq!(value["case_count"], 4);
    let mut ids = BTreeSet::new();
    let mut flags = BTreeSet::new();
    for case in cases {
        let id = case["id"].as_str().expect("fixed case ID required");
        assert!(
            !id.is_empty()
                && id.len() <= 32
                && id.bytes().all(|c| c.is_ascii_lowercase() || c == b'_')
        );
        assert!(ids.insert(id.to_owned()));
        let flag = case["flag"].as_str().expect("fixed flag required");
        assert!(flag.len() <= 64 && super::forbidden_switch(flag) && flags.insert(flag.to_owned()));
    }
    let prefix = value["marker_prefix"]
        .as_str()
        .expect("fixed marker prefix required");
    assert!(
        prefix.len() <= 64
            && prefix.ends_with(' ')
            && prefix
                .bytes()
                .all(|c| c.is_ascii_uppercase() || matches!(c, b'_' | b' '))
    );
    value
}
fn error_kind(error: &io::Error) -> ErrorLabel {
    match error.kind() {
        io::ErrorKind::NotFound => ErrorLabel::NotFound,
        io::ErrorKind::PermissionDenied => ErrorLabel::PermissionDenied,
        io::ErrorKind::TimedOut => ErrorLabel::TimedOut,
        io::ErrorKind::Interrupted => ErrorLabel::Interrupted,
        _ => ErrorLabel::Other,
    }
}
fn case_record(config: &Value, id: &str, facts: &Facts) -> Result<Value, &'static str> {
    if facts.reaped == Some(false) {
        return Err("unobserved reaping must remain null");
    }
    if !config["cases"]
        .as_array()
        .is_some_and(|cases| cases.iter().any(|case| case["id"].as_str() == Some(id)))
    {
        return Err("unknown bootstrap case ID");
    }
    Ok(
        json!({"schema":config["observation_schema"],"kind":"case","case_id":id,
        "spawn_ok":facts.spawn_ok,"wait_completed":facts.wait_completed,
        "deadline_expired":facts.deadline_expired,"reaped":facts.reaped,
        "exit_code":facts.exit_code,"cleanup_attempted":facts.cleanup_attempted,
        "spawn_error_kind":facts.spawn_error_kind.as_str(),"wait_error_kind":facts.wait_error_kind.as_str(),
        "cleanup_error_kind":facts.cleanup_error_kind.as_str()}),
    )
}
fn profile_record(
    config: &Value,
    phase: &str,
    observed: bool,
    changed: Option<bool>,
    kind: ErrorLabel,
) -> Value {
    assert!(matches!(phase, "before" | "after"));
    json!({"schema":config["observation_schema"],"kind":"profiles","phase":phase,
        "inventory_ok":observed,"changed":changed,"error_kind":kind.as_str()})
}
fn emit(config: &Value, record: &Value) {
    let bytes = serde_json::to_string(record).expect("closed observation serialization failed");
    let prefix = config["marker_prefix"].as_str().unwrap();
    assert!(
        prefix.len() + bytes.len() + 1 <= config["max_record_bytes"].as_u64().unwrap() as usize
    );
    println!("{prefix}{bytes}");
}
fn profiles(bundle: &Path) -> io::Result<BTreeSet<OsString>> {
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(bundle)? {
        let name = entry?.file_name();
        if name.to_string_lossy().starts_with("synthetic-profile-") {
            names.insert(name);
        }
    }
    Ok(names)
}

pub(super) fn run() {
    let config = config();
    let bundle = PathBuf::from(
        std::env::var_os("HUMANYMOUS_CEF_PILOT_BUNDLE").expect("private native bundle required"),
    );
    let before = profiles(&bundle);
    emit(
        &config,
        &profile_record(
            &config,
            "before",
            before.is_ok(),
            None,
            before.as_ref().err().map_or(ErrorLabel::None, error_kind),
        ),
    );
    let before = before.expect("profile baseline unavailable");
    for case in config["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let flag = case["flag"].as_str().unwrap();
        let mut command = Command::new(bundle.join("humanymous_cef_bootstrap.exe"));
        command.arg(flag);
        let mut facts = Facts::default();
        let mut child = match ManagedChild::spawn(&mut command, b"") {
            Ok(child) => {
                facts.spawn_ok = true;
                child
            }
            Err(error) => {
                facts.spawn_error_kind = error_kind(&error);
                emit(&config, &case_record(&config, id, &facts).unwrap());
                panic!("bootstrap child startup failed");
            }
        };
        // wait stops/reaps on its deadline; Drop retains ownership on errors or panic.
        // This does not attest descendant termination, sandbox policy or GUI behavior.
        let deadline = Duration::from_millis(config["deadline_ms"].as_u64().unwrap());
        let started = Instant::now();
        let waited = child.wait(deadline);
        facts.deadline_expired = Some(started.elapsed() >= deadline);
        match &waited {
            Ok(status) => {
                facts.wait_completed = true;
                facts.reaped = Some(true);
                facts.exit_code = status.code();
            }
            Err(error) => {
                facts.wait_error_kind = error_kind(error);
                facts.cleanup_attempted = true;
                match child.stop() {
                    Ok(status) => {
                        facts.reaped = Some(true);
                        facts.exit_code = status.code();
                    }
                    Err(error) => facts.cleanup_error_kind = error_kind(&error),
                }
            }
        }
        emit(&config, &case_record(&config, id, &facts).unwrap());
        let status = waited.expect("security override did not reject promptly");
        assert_eq!(status.code(), Some(78), "{flag}");
    }
    let after = profiles(&bundle);
    let changed = after.as_ref().ok().map(|names| names != &before);
    emit(
        &config,
        &profile_record(
            &config,
            "after",
            after.is_ok(),
            changed,
            after.as_ref().err().map_or(ErrorLabel::None, error_kind),
        ),
    );
    assert_eq!(
        after.expect("profile inventory unavailable"),
        before,
        "a rejected override created a profile"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn case_projection_is_closed_and_rejects_foreign_ids() {
        let config = config();
        let value = case_record(&config, "no_sandbox", &Facts::default()).unwrap();
        let expected: BTreeSet<_> = config["case_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let actual: BTreeSet<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(actual, expected);
        assert!(case_record(&config, "PRIVATE_PATH", &Facts::default()).is_err());
        assert!(serde_json::to_string(&value).unwrap().len() <= 1024);
    }
    #[test]
    fn unobserved_reaping_stays_null() {
        let config = config();
        let mut facts = Facts {
            spawn_ok: true,
            exit_code: Some(78),
            ..Facts::default()
        };
        let value = case_record(&config, "no_sandbox", &facts).unwrap();
        assert!(value["reaped"].is_null());
        assert_eq!(value["wait_completed"], false);
        facts.reaped = Some(true);
        facts.wait_completed = true;
        assert_eq!(
            case_record(&config, "no_sandbox", &facts).unwrap()["reaped"],
            true
        );
        facts.reaped = Some(false);
        assert!(case_record(&config, "no_sandbox", &facts).is_err());
    }
    #[test]
    fn profile_error_projection_excludes_private_details() {
        let config = config();
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "PRIVATE_PATH_MESSAGE");
        let value = profile_record(&config, "after", false, None, error_kind(&error));
        let bytes = serde_json::to_string(&value).unwrap();
        assert!(!bytes.contains("PRIVATE_PATH_MESSAGE"));
        assert_eq!(value["error_kind"], "permission_denied");
        assert!(value["changed"].is_null());
    }
}
