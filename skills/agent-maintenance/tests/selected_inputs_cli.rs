//! Actual command boundary: bounded JSON in, metadata/hashes out, no contents.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hmn-selected-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn request(&self, selected: &str, complete: bool) -> PathBuf {
        let path = self.0.join("request.json");
        fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "schema_version":1,"roots":[{"id":"fixture","path":self.0}],
                "files":[{"root":"fixture","path":selected}],"input_selection_complete":complete
            }))
            .unwrap(),
        )
        .unwrap();
        path
    }
    fn run(&self, path: &std::path::Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"))
            .arg("selected-inputs")
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
fn complete_binary_selection_emits_hashes_without_contents_or_authority() {
    let f = Fixture::new();
    let contents = b"PRIVATE_FILE_CONTENT\r\n\0\xff";
    fs::write(f.0.join("binary"), contents).unwrap();
    let output = f.run(&f.request("binary", true));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_FILE_CONTENT"));
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["schema"], "humanymous.selected-inputs-cli.v1");
    assert_eq!(receipt["observation"]["content_read_complete"], true);
    assert_eq!(receipt["observation"]["provider_identity_qualified"], false);
    assert_eq!(receipt["observation"]["qualified_complete"], false);
    assert_eq!(receipt["observation"]["cache_authorized"], false);
    assert_eq!(receipt["observation"]["execution_authorized"], false);
    assert!(
        receipt["observation"]["files"][0]["content"]["sha256"]
            .as_str()
            .is_some()
    );
    let request_bytes = receipt["request_read"]["actual_read_bytes"]
        .as_u64()
        .unwrap();
    assert_eq!(
        receipt["command_actual_read_bytes"],
        request_bytes + contents.len() as u64
    );
    assert_eq!(fs::read(f.0.join("binary")).unwrap(), contents);
}

#[test]
fn invalid_request_diagnostics_do_not_echo_values() {
    let f = Fixture::new();
    let path = f.0.join("invalid.json");
    fs::write(&path, br#"{"schema_version":"PRIVATE_REQUEST_VALUE"}"#).unwrap();
    let output = f.run(&path);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_REQUEST_VALUE"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("InvalidJson"));
}

#[test]
fn missing_selected_file_emits_incomplete_metadata_with_exit_one() {
    let f = Fixture::new();
    let output = f.run(&f.request("absent", true));
    assert_eq!(output.status.code(), Some(1));
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["observation"]["content_read_complete"], false);
    assert_eq!(
        receipt["observation"]["files"][0]["content"]["status"],
        "provider_refused"
    );
    assert_eq!(
        receipt["observation"]["files"][0]["content"]["sha256"],
        serde_json::Value::Null
    );
}

#[test]
fn unknown_selection_emits_read_success_but_exit_one() {
    let f = Fixture::new();
    fs::write(f.0.join("a"), b"a").unwrap();
    let output = f.run(&f.request("a", false));
    assert_eq!(output.status.code(), Some(1));
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["observation"]["content_read_complete"], true);
    assert_eq!(receipt["observation"]["input_selection_complete"], false);
    assert_eq!(receipt["observation"]["qualified_complete"], false);
}

#[test]
fn request_directory_or_missing_file_is_rejected() {
    let f = Fixture::new();
    for path in [&f.0, &f.0.join("missing.json")] {
        let output = f.run(path);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn oversized_request_is_rejected_before_selection() {
    let f = Fixture::new();
    let path = f.0.join("large.json");
    fs::write(&path, vec![b' '; 2 * 1024 * 1024 + 1]).unwrap();
    let output = f.run(&path);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TooLarge"));
}
