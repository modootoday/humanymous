//! Actual executable behavior; component tests, not SDK/framework product e2e.
#[path = "../src/test_support.rs"]
mod test_support;
use std::process::Command;
use test_support::Directory;

fn command(root: &Directory) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_humanymous-native"));
    command.current_dir(&root.0);
    command
}

#[test]
fn dry_run_is_deterministic_and_never_inspects_or_creates_workspace() {
    let root = Directory::new();
    let args = [
        "init",
        "--framework",
        "electron",
        "--workspace",
        "does-not-exist",
    ];
    let first = command(&root).args(args).output().unwrap();
    let second = command(&root).args(args).arg("--dry-run").output().unwrap();
    assert!(first.status.success(), "{:?}", first.stderr);
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(root.count(), 0);
    let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(value["operation"], "dry_run");
    assert_eq!(value["manifest"]["qualified"], false);
    assert!(value["publication"].is_null());
}

#[test]
fn conflicts_or_unknown_commands_refuse_without_writes() {
    let root = Directory::new();
    for args in [
        vec!["verify"],
        vec!["init", "--framework", "electron", "--write", "--dry-run"],
        vec![
            "init",
            "--framework",
            "electron",
            "--output",
            "../outside.json",
        ],
        vec!["init", "--framework", "electron", "--force"],
    ] {
        let result = command(&root).args(args).output().unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert_eq!(root.count(), 0);
    }
}

#[test]
fn help_and_version_use_the_canonical_executable_name() {
    let root = Directory::new();
    for flag in ["--help", "--version"] {
        let result = command(&root).arg(flag).output().unwrap();
        assert!(result.status.success());
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains("humanymous-native"));
        assert!(!text.contains("create-humanymous-native"));
    }
    assert_eq!(root.count(), 0);
}

#[cfg(windows)]
#[test]
fn explicit_write_matches_manifest_and_repeated_write_preserves_original_bytes() {
    let root = Directory::new();
    let result = command(&root)
        .args(["init", "--framework", "electron", "--write", "--workspace"])
        .arg(&root.0)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["publication"]["published"], true);
    assert_eq!(report["publication"]["cleanup_complete"], true);
    let output = root.0.join("humanymous-native.json");
    let bytes = std::fs::read(&output).unwrap();
    let stored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(stored, report["manifest"]);
    assert_eq!(root.count(), 1);
    let again = command(&root)
        .args(["init", "--framework", "electron", "--write", "--workspace"])
        .arg(&root.0)
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(2));
    assert_eq!(std::fs::read(output).unwrap(), bytes);
    assert_eq!(root.count(), 1);
}

#[cfg(windows)]
#[test]
fn relative_workspace_is_not_silently_resolved_for_write() {
    let root = Directory::new();
    let result = command(&root)
        .args([
            "init",
            "--framework",
            "electron",
            "--write",
            "--workspace",
            ".",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(root.count(), 0);
}

#[cfg(not(windows))]
#[test]
fn unqualified_platform_write_is_explicitly_unsupported() {
    let root = Directory::new();
    let result = command(&root)
        .args(["init", "--framework", "electron", "--write", "--workspace"])
        .arg(&root.0)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("require Windows"));
    assert_eq!(root.count(), 0);
}

#[test]
fn option_token_cannot_be_swallowed_as_a_version_to_bypass_dry_run_conflict() {
    let root = Directory::new();
    let result = command(&root)
        .args([
            "init",
            "--framework",
            "electron",
            "--runtime-version",
            "--dry-run",
            "--write",
            "--workspace",
        ])
        .arg(&root.0)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert_eq!(root.count(), 0);
}
