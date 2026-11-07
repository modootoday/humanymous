//! Actual CLI boundary; discovery must not execute candidate files.
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "hmn-doctor-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"));
        command
            .arg("windows-doctor")
            .current_dir(&self.0)
            .env("PATH", "")
            .env("LOCALAPPDATA", &self.0)
            .env("USERPROFILE", &self.0)
            .env("ProgramFiles", &self.0)
            .env("ProgramFiles(x86)", &self.0);
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn discovery_does_not_execute_candidates_or_promote_sdk_presence() {
    let fixture = Fixture::new();
    let python = fixture.0.join("Programs/Python/Python312");
    fs::create_dir_all(python.join("include")).unwrap();
    fs::create_dir(python.join("libs")).unwrap();
    for file in ["python.exe", "include/Python.h", "libs/python312.lib"] {
        fs::write(python.join(file), b"NONEXECUTABLE_SENTINEL").unwrap();
    }
    let output = fixture.command().output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "humanymous.windows-doctor.v1");
    assert_eq!(report["complete"], false);
    assert_eq!(report["ruby_extension_abi_qualified"], false);
    assert_eq!(report["child_environment"], serde_json::json!([]));
    let tools = report["tools"].as_array().unwrap();
    assert!(
        tools
            .iter()
            .all(|tool| tool["version"].is_null() && tool["runtime_validated"] == false)
    );
    assert!(
        tools
            .iter()
            .find(|tool| tool["name"] == "python312")
            .unwrap()["path"]
            .is_string()
    );
    assert!(
        report["python_sdk"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["path"].is_string() && file["link_validated"] == false)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("NONEXECUTABLE_SENTINEL"));
}

#[test]
fn discovery_retains_missing_paths_without_setting_pyo3() {
    let fixture = Fixture::new();
    let output = fixture.command().output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["python_sdk"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["path"].is_null())
    );
    assert_eq!(report["child_environment"], serde_json::json!([]));
    assert_eq!(report["msvc_setup"], serde_json::Value::Null);
}

#[test]
fn doctor_rejects_arbitrary_probe_arguments() {
    let fixture = Fixture::new();
    let output = fixture.command().arg("--run-anything").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
