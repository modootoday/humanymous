//! Process-shell behavior of legacy `cmd/ml-train/main.go` (baseline 981d25c)
//! over owned temporary files. Legacy `cmd/ml-train` has no Go test file; the
//! goldens below are Go-OBSERVED: the oracle binary built from 981d25c with
//! go1.25.3 linux/amd64 was run on 2026-11-18 with the same flags, and the
//! SHA-256 of each written bundle is recorded. Equal digests mean the Rust
//! generator, shuffle, split, standardization, SGD and JSON layout reproduce
//! the Go bundle byte for byte.
use super::*;
use crate::model::correction::digest;
use std::path::PathBuf;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "humanymous-ml-train-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().to_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The date the Go goldens were written (it is part of the bundle version).
fn golden_day() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1795033620).unwrap()
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

fn run_with(list: &[&str]) -> (i32, String) {
    let mut stderr = Vec::new();
    let code = run(&args(list), golden_day(), &mut stderr);
    (code, String::from_utf8(stderr).unwrap())
}

fn assert_go_bundle(flags: &[&str], go_sha256: &str, go_bytes: usize, report: &str) {
    let dir = TempDir::new();
    let out = dir.path("model.json");
    let mut list = flags.to_vec();
    list.extend(["-out", &out]);
    let (code, stderr) = run_with(&list);
    assert_eq!(code, EXIT_OK, "{stderr}");
    let raw = std::fs::read(&out).unwrap();
    assert_eq!(raw.len(), go_bytes, "{stderr}");
    assert_eq!(digest(&raw), go_sha256, "{stderr}");
    assert!(stderr.contains(report), "{stderr}");
    assert!(
        stderr.ends_with(&format!("ml-train: wrote {out} ({go_bytes} bytes)\n")),
        "{stderr}"
    );
}

#[test]
fn synthetic_bootstrap_matches_go_bundle_bytes() {
    assert_go_bundle(
        &["-gen", "1000", "-epochs", "3", "-seed", "7", "-hidden", "8"],
        "5f886d9d0c81faed9f0afb78b181c23ffba8a35ab7571ad151f1e067ec5c453f",
        4571,
        "ml-train: generated 1000 SYNTHETIC bootstrap samples (not production data)\n\
         ml-train: val acc=1.000  bot TPR=1.000  human FPR=0.000  \
         (bundle mlp-bootstrap-20261118-h8, schema e2a7c0fa0cc130ee8c595787)\n",
    );
}

#[test]
fn negative_seed_and_learning_rate_match_go_bundle_bytes() {
    assert_go_bundle(
        &["-gen=300", "--epochs", "2", "-seed", "-5", "-lr", "0.1"],
        "0c7ba704a301a38bf8fd04920bb7cb3f11fa80a76eb49182057661af2a197fbc",
        8202,
        "(bundle mlp-bootstrap-20261118-h16, schema e2a7c0fa0cc130ee8c595787)",
    );
}

/// The Go golden `data.jsonl`: 60 clear humans and 60 headless bots.
fn labeled_data() -> String {
    let mut lines = String::new();
    for i in 0..120_u32 {
        if i % 2 == 0 {
            lines.push_str(&format!(
                r#"{{"label": 0, "behavior": {{"mouse": {{"samples": {}, "meanCurvature": 0.3, "velocityStdDev": 0.6}}, "key": {{"keystrokes": 10}}, "events": {{"totalEvents": 50}}, "durationS": {}}}}}"#,
                40 + i % 7,
                8.0 + f64::from(i % 5) * 0.5
            ));
        } else {
            lines.push_str(&format!(
                r#"{{"label": 1, "behavior": {{"events": {{"totalEvents": {}}}, "durationS": {}}}}}"#,
                2 + i % 3,
                5.0 + f64::from(i % 4) * 0.25
            ));
        }
        lines.push('\n');
    }
    lines
}

#[test]
fn labeled_jsonl_training_matches_go_bundle_bytes() {
    let dir = TempDir::new();
    let data = dir.path("data.jsonl");
    std::fs::write(&data, labeled_data()).unwrap();
    assert_go_bundle(
        &["-data", &data, "-epochs", "5", "-seed", "3"],
        "b9e256639f30785ca0275dd8dca26d5ea3e4977fec1c75d6be4807d074955141",
        8023,
        &format!("ml-train: loaded 120 labeled samples from {data}\n"),
    );
}

#[test]
fn too_few_samples_fails_without_writing() {
    let dir = TempDir::new();
    let out = dir.path("model.json");
    let (code, stderr) = run_with(&["-gen", "50", "-out", &out]);
    assert_eq!(code, EXIT_ERROR);
    assert_eq!(
        stderr,
        "ml-train: generated 50 SYNTHETIC bootstrap samples (not production data)\n\
         ml-train: need >=100 samples, got 50\n"
    );
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn invalid_or_missing_data_fails_without_writing() {
    let dir = TempDir::new();
    let out = dir.path("model.json");
    let data = dir.path("bad.jsonl");
    std::fs::write(&data, format!("{}not json\n", labeled_data())).unwrap();
    let (code, stderr) = run_with(&["-data", &data, "-out", &out]);
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.starts_with("ml-train: bad JSONL input:"), "{stderr}");
    let (code, stderr) = run_with(&["-data", &dir.path("absent.jsonl"), "-out", &out]);
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.starts_with("ml-train: open "), "{stderr}");
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn generation_is_bounded_instead_of_unbounded() {
    let dir = TempDir::new();
    let out = dir.path("model.json");
    let (code, stderr) = run_with(&["-gen", "100001", "-out", &out]);
    assert_eq!(code, EXIT_ERROR);
    assert_eq!(stderr, "ml-train: more than 100000 samples\n");
    let mut rng = GoRng::new(1, 10).unwrap();
    assert_eq!(
        generate_synthetic(&mut rng, 10),
        Err(RngError::DrawLimit),
        "draw budget exhaustion is an error, not a truncated dataset"
    );
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn usage_help_and_positional_arguments() {
    let (code, stderr) = run_with(&["-h"]);
    assert_eq!((code, stderr.as_str()), (EXIT_OK, USAGE));
    let (code, stderr) = run_with(&["-bogus"]);
    assert_eq!(code, EXIT_USAGE);
    assert_eq!(
        stderr,
        format!("flag provided but not defined: -bogus\n{USAGE}")
    );
    let (code, stderr) = run_with(&["-epochs", "x"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(stderr.starts_with("invalid value \"x\" for flag -epochs"));
    let (code, stderr) = run_with(&["-out"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(stderr.starts_with("flag needs an argument: -out"));
    // Go stopped at the first positional and ignored the rest; refuse instead.
    let (code, stderr) = run_with(&["extra", "-gen", "100"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(stderr.starts_with("unexpected positional argument \"extra\""));
}

#[test]
fn defaults_and_version_follow_legacy_flags() {
    assert_eq!(
        Options::default(),
        Options {
            gen_count: 0,
            data: String::new(),
            out: "model.json".to_owned(),
            hidden: 16,
            epochs: 60,
            lr: 0.05,
            seed: 1,
        }
    );
    assert_eq!(
        version_stamp(golden_day(), 16),
        "mlp-bootstrap-20261118-h16"
    );
    let offset = golden_day().to_offset(time::UtcOffset::from_hms(9, 0, 0).unwrap());
    assert_eq!(version_stamp(offset, 0), "mlp-bootstrap-20261118-h0");
}

#[test]
fn legacy_default_run_fits_the_bootstrap_work_profile() {
    // 20000 generated samples -> 16000 training rows x 60 epochs x 16 hidden x 27.
    let units = 16_000_u64 * 60 * 16 * crate::behavior::features::FEATURE_DIM as u64;
    assert_eq!(units, 414_720_000);
    let profile = TrainLimits::legacy_bootstrap();
    assert!(units <= profile.work_units && units > TrainLimits::default().work_units);
    assert!(16_000 * 60 <= profile.updates);
}
