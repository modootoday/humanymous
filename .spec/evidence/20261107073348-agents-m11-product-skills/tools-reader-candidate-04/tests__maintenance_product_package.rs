#![forbid(unsafe_code)]
//! Actual binary staging boundary. Run only in the approved hosted allocation.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    kind: String,
    files: Vec<Entry>,
    runtime_qualified: bool,
    release_qualified: bool,
    source: SourceReceipt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceReceipt {
    repo_root: PathBuf,
    declared_source_commit: String,
    request_sha256: String,
    inputs: Vec<Entry>,
    actual_read_bytes: u64,
    elapsed_micros_before_write: u128,
    input_selection_declared_complete: bool,
    selected_content_matches: bool,
    canonical_tree_complete: bool,
    source_commit_verified: bool,
    provider_identity_qualified: bool,
    qualified_complete: bool,
    identity_status: String,
}

#[test]
fn product_bundle_contains_paired_suite_and_refuses_existing_output() {
    verify_bundle("product", "humanymous-product", "humanymous-development", 3);
}
#[test]
fn developer_bundle_contains_separate_five_case_suite() {
    verify_bundle(
        "developer",
        "humanymous-development",
        "humanymous-product",
        5,
    );
}

fn source_request(kind: &str) -> (PathBuf, serde_json::Value, Vec<u8>) {
    let variable = match kind {
        "product" => "HUMANYMOUS_MAINTENANCE_PRODUCT_STAGE_REQUEST",
        "developer" => "HUMANYMOUS_MAINTENANCE_DEVELOPER_STAGE_REQUEST",
        _ => panic!("unsupported stage kind"),
    };
    let path = PathBuf::from(std::env::var_os(variable).expect("required explicit stage request"));
    assert!(path.is_absolute());
    let meta = fs::symlink_metadata(&path).unwrap();
    assert!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 2 * 1024 * 1024);
    let mut bytes = Vec::new();
    fs::File::open(&path).unwrap().take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    let request: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(request["schema"], "humanymous.stage-eval-source.v1");
    assert_eq!(request["kind"], kind);
    (path, request, bytes)
}

fn source_root(request: &serde_json::Value) -> PathBuf {
    let root = PathBuf::from(request["repo_root"].as_str().expect("explicit selected source root"));
    assert!(root.is_absolute(), "source root must be absolute");
    assert!(!root.components().any(|c| matches!(
        c,
        std::path::Component::CurDir | std::path::Component::ParentDir
    )));
    for selected in root.ancestors() {
        let meta = fs::symlink_metadata(selected).expect("source ancestor missing");
        assert!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "unsafe source ancestor"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(
                meta.file_attributes() & 0x400,
                0,
                "source reparse point refused"
            );
        }
    }
    root
}

fn stage(request: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"))
        .arg("stage-eval").arg(request).arg(output)
        .output().expect("execute the exact Cargo-selected maintenance role")
}

fn mismatched_source_never_creates_output(request: &serde_json::Value, output: &Path) {
    let path = output.with_extension("bad-request.json");
    let rejected_output = output.with_extension("bad-output");
    let mut bad = request.clone();
    bad["files"][0]["sha256"] = "0".repeat(64).into();
    let bytes = serde_json::to_vec_pretty(&bad).unwrap();
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
    std::io::Write::write_all(&mut file, &bytes).unwrap();
    drop(file);
    let refused = stage(&path, &rejected_output);
    assert!(!refused.status.success(), "mismatched source must be refused");
    assert!(!rejected_output.exists(), "input failure must precede output creation");
    fs::remove_file(path).unwrap();
}

fn verify_bundle(kind: &str, plugin: &str, excluded: &str, expected_cases: usize) {
    let (request_path, request, request_bytes) = source_request(kind);
    let root = source_root(&request);
    let output = std::env::temp_dir().join(format!(
        "humanymous-{kind}-bundle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    assert!(!output.exists());
    let run = stage(&request_path, &output);
    assert!(
        run.status.success(),
        "stage failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.stdout.len() <= 2 * 1024 * 1024,
        "oversized stage receipt"
    );
    let receipt: Receipt = serde_json::from_slice(&run.stdout).expect("actual stage JSON receipt");
    assert_eq!(receipt.schema, "humanymous.skill-package.v1");
    assert_eq!(receipt.kind, kind);
    assert!(!receipt.runtime_qualified);
    assert!(!receipt.release_qualified);
    assert_eq!(receipt.source.repo_root, root);
    assert_eq!(receipt.source.declared_source_commit, request["source_commit"].as_str().unwrap());
    assert_eq!(receipt.source.request_sha256, format!("{:x}", Sha256::digest(&request_bytes)));
    assert!(receipt.source.input_selection_declared_complete && receipt.source.selected_content_matches);
    assert!(!receipt.source.canonical_tree_complete && !receipt.source.source_commit_verified);
    assert!(!receipt.source.provider_identity_qualified && !receipt.source.qualified_complete);
    assert!(receipt.source.identity_status.starts_with("unsupported_source_identity"));
    assert!(receipt.source.elapsed_micros_before_write < 30_000_000);
    let pins = request["files"].as_array().unwrap();
    assert_eq!(receipt.source.inputs.len(), pins.len());
    let mut expected_read = request_bytes.len() as u64;
    for (input, pin) in receipt.source.inputs.iter().zip(pins) {
        assert_eq!(input.path, pin["path"].as_str().unwrap());
        assert_eq!(input.bytes as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(input.sha256, pin["sha256"].as_str().unwrap());
        expected_read += input.bytes as u64;
    }
    assert_eq!(receipt.source.actual_read_bytes, expected_read);
    assert_eq!(fs::read(&request_path).unwrap(), request_bytes);
    assert!(!receipt.files.is_empty() && receipt.files.len() <= 512);
    for entry in &receipt.files {
        let path = Path::new(&entry.path);
        assert!(!path.is_absolute());
        assert!(
            path.components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
        );
        assert!(!entry.path.contains(['\\', ':']));
        let bytes = fs::read(output.join(path)).unwrap();
        assert_eq!(bytes.len(), entry.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), entry.sha256);
        assert!(!entry.path.contains(excluded));
        assert!(!entry.path.starts_with(".agents/"));
    }
    let cases = receipt
        .files
        .iter()
        .filter(|entry| {
            entry.path.starts_with(&format!("plugins/{plugin}/evals/"))
                && entry.path.ends_with("/prompt.md")
        })
        .count();
    assert_eq!(cases, expected_cases);
    mismatched_source_never_creates_output(&request, &output);
    let before = fs::read(output.join("bundle-receipt.json")).unwrap();
    let refused = stage(&request_path, &output);
    assert!(!refused.status.success(), "existing output must be refused");
    assert_eq!(
        fs::read(output.join("bundle-receipt.json")).unwrap(),
        before
    );
    // Only the fresh, successfully verified output selected by this test is removed.
    // Failures retain their output and diagnostics for the allocated runner.
    fs::remove_dir_all(output).unwrap();
}
