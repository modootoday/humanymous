//! Preserve an unknown denominator when initialization fails before any report.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
};

fn initial_report(mode: &str, os_error: Option<i32>) -> Option<Value> {
    let workspace = matches!(mode, "workspace" | "standard-user-workspace");
    if !workspace
        && !matches!(
            mode,
            "native" | "standard-user" | "service" | "standard-user-service"
        )
    {
        return None;
    }
    Some(
        json!({"schema":if workspace{json!("humanymous.workspace-probe/1")}else{json!(1)},"kind":if workspace{"workspace"}else{"native"},"scope":if workspace{"workspace-default-features"}else{"four-package-native-process-component"},"qualification":if workspace{"implemented-workspace-component"}else{"component-only"},"full_runtime_qualified":false,"status":"failed","source_sha":null,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"errors":1,"skips":null,"initialization_failure":{"reported":true,"os_error_code":os_error},"coverage":{"required_harnesses":null,"executed_harnesses":null,"harnesses":[],"complete":false}}),
    )
}

pub(crate) fn record(path: &Path, mode: &str, error: &io::Error) -> io::Result<()> {
    let Some(report) = initial_report(mode, error.raw_os_error()) else {
        return Ok(());
    };
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    file.write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialization_is_failed_with_unknown_source_and_denominators() {
        for mode in [
            "workspace",
            "standard-user-workspace",
            "native",
            "standard-user-service",
        ] {
            let report = initial_report(mode, Some(5)).unwrap();
            assert_eq!(report["status"], "failed");
            assert_eq!(report["full_runtime_qualified"], false);
            assert!(report["source_sha"].is_null());
            assert!(report["coverage"]["required_harnesses"].is_null());
            assert!(report["coverage"]["executed_harnesses"].is_null());
            assert_eq!(report["coverage"]["complete"], false);
        }
        assert!(initial_report("summarize", None).is_none());
    }
}
