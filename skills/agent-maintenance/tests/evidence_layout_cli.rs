//! Actual command contract: supplied metadata cannot authorize or perform retirement.
#[path = "support/layout_request.rs"]
mod fixture;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hmn-layout-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn request(&self, value: &serde_json::Value) -> PathBuf {
        let path = self.0.join("request.json");
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
    fn run(&self, path: &std::path::Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"))
            .arg("evidence-layout-plan")
            .arg(path)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn complete_claims_emit_proposal_without_touching_candidate_or_verifying_archives() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("old-copy")).unwrap();
    fs::write(f.0.join("old-copy/source.bin"), b"PRIVATE_SENTINEL_CONTENT").unwrap();
    let value = fixture::request(&f.0);
    let path = f.request(&value);
    let input = fs::read(&path).unwrap();
    let output = f.run(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["plan"]["state"], "proposed");
    for field in [
        "retirement_authorized",
        "execution_authorized",
        "candidate_filesystem_verified",
        "owner_authority_verified",
        "terminal_handles_independently_verified",
        "archive_bytes_verified",
        "readback_independently_verified",
    ] {
        assert_eq!(result["plan"][field], false);
    }
    assert_eq!(
        result["request_read"]["actual_read_bytes"],
        input.len() as u64
    );
    assert_eq!(fs::read(&path).unwrap(), input);
    assert_eq!(
        fs::read(f.0.join("old-copy/source.bin")).unwrap(),
        b"PRIVATE_SENTINEL_CONTENT"
    );
    assert!(!f.0.join("fresh-scratch").exists());
    assert!(!f.0.join(".spec.local").exists());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_SENTINEL_CONTENT"));
}
#[test]
fn active_handle_emits_refused_metadata_with_exit_one() {
    let f = Fixture::new();
    let mut value = fixture::request(&f.0);
    value["consumers"]["handles"][0]["state"] = "active".into();
    let output = f.run(&f.request(&value));
    assert_eq!(output.status.code(), Some(1));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["plan"]["state"], "refused");
    assert_eq!(result["plan"]["retirement_authorized"], false);
}
#[test]
fn unknown_json_fields_are_sanitized_and_exit_two() {
    let f = Fixture::new();
    let mut value = fixture::request(&f.0);
    value["owner"]["PRIVATE_UNKNOWN_FIELD"] = "PRIVATE_SENTINEL".into();
    let output = f.run(&f.request(&value));
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_"));
}
#[test]
fn oversized_or_missing_request_does_not_start_a_plan() {
    let f = Fixture::new();
    let path = f.0.join("large.json");
    fs::write(&path, vec![b' '; 2 * 1024 * 1024 + 1]).unwrap();
    for path in [path, f.0.join("missing.json")] {
        let output = f.run(&path);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
