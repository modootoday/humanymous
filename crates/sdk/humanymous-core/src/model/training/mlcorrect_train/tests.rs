//! Process-shell behavior of legacy `cmd/mlcorrect-train/main.go` over owned
//! temporary files: anchor/oracle/base inputs, warm start, exit 3 with no
//! write on gate failure, and the stdout digest.
use super::*;
use crate::model::training::record::Record;
use crate::signals::report::BehaviorSummary;
use std::path::PathBuf;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "humanymous-mlcorrect-train-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().to_owned()
    }
    fn write(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.path(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn record(label: u8, cohort: &str, bot_like: bool, i: usize, source: &str) -> Record {
    let mut behavior = BehaviorSummary {
        duration_s: if bot_like { 5.0 } else { 1.0 } + (i % 3) as f64 * 0.01,
        ..Default::default()
    };
    behavior.mouse.samples = if bot_like { 110 } else { 10 } + (i % 5) as i64;
    Record {
        label,
        behavior,
        ts: Some(i as i64),
        cohort: (!cohort.is_empty()).then(|| cohort.to_owned()),
        source: (!source.is_empty()).then(|| source.to_owned()),
    }
}

/// Same shape as the gate fixture: 3 bots : 2 typical : 1 edge human per 6.
fn anchor(count: usize, poison_edge: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..count {
        let r = match i % 6 {
            0 | 2 | 4 => record(1, "", true, i, "catalog"),
            1 | 5 => record(0, "typical", false, i, "pass"),
            _ => record(0, "edge", poison_edge, i, "pass"),
        };
        serde_json::to_writer(&mut bytes, &r).unwrap();
        bytes.push(b'\n');
    }
    bytes
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

const FAST: [&str; 8] = ["-hidden", "8", "-epochs", "40", "-lr", "0.1", "-seed", "2"];

fn invoke(list: &[String]) -> (i32, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(list, "mlp-corrected-test", &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn passing_gates_write_candidate_and_print_only_its_digest() {
    let dir = TempDir::new();
    let anchor_path = dir.write("gold.jsonl", &anchor(600, false));
    // One weak (challenge) bot label and one confirmed human from the oracle.
    let mut oracle = Vec::new();
    for r in [
        record(1, "", true, 600, "challenge"),
        record(0, "typical", false, 601, "pass"),
    ] {
        serde_json::to_writer(&mut oracle, &r).unwrap();
        oracle.push(b'\n');
    }
    let oracle_path = dir.write("oracle.jsonl", &oracle);
    let out = dir.path("candidate.json");
    let mut list = args(&[
        "-anchor",
        &anchor_path,
        "--oracle",
        &oracle_path,
        &format!("-out={out}"),
    ]);
    list.extend(args(&FAST));
    let (code, stdout, stderr) = invoke(&list);
    assert_eq!(code, EXIT_OK, "{stderr}");
    let raw = std::fs::read(&out).unwrap();
    assert_eq!(stdout, format!("{}\n", digest(&raw)));
    assert!(stderr.contains("excluded 1 of 2 records"), "{stderr}");
    assert!(stderr.contains("TESSERACT time-split"), "{stderr}");
    assert!(stderr.contains("GATES PASSED"), "{stderr}");
    assert!(raw.starts_with(b"{\n \"version\": \"mlp-corrected-test\""));
    let model = Mlp::from_json(&raw).unwrap();
    assert_eq!(model.bundle_version(), "mlp-corrected-test");
    assert_eq!(model.schema_hash(), schema_hash());

    // Warm start from the written candidate: the same buffer validates and seeds.
    let mut warm = args(&[
        "-anchor",
        &anchor_path,
        "-base",
        &out,
        "-out",
        &dir.path("warm.json"),
    ]);
    warm.extend(args(&FAST));
    let (code, stdout, stderr) = invoke(&warm);
    assert_eq!(code, EXIT_OK, "{stderr}");
    assert!(stderr.contains("warm-starting from"), "{stderr}");
    assert!(stderr.contains("(mlp-corrected-test)"), "{stderr}");
    assert_eq!(
        stdout,
        format!(
            "{}\n",
            digest(&std::fs::read(dir.path("warm.json")).unwrap())
        )
    );

    // A base whose hidden width differs cannot warm-start.
    let mut mismatch = warm.clone();
    mismatch[5] = dir.path("mismatch.json");
    mismatch[7] = "16".to_owned();
    let (code, _, stderr) = invoke(&mismatch);
    assert_eq!(code, EXIT_ERROR, "{stderr}");
    assert!(stderr.contains("WarmBase"), "{stderr}");
    assert!(!std::path::Path::new(&dir.path("mismatch.json")).exists());
}

#[test]
fn failed_gate_exits_3_and_leaves_existing_output_untouched() {
    let dir = TempDir::new();
    let anchor_path = dir.write("gold.jsonl", &anchor(600, true));
    let out = dir.write("candidate.json", b"existing sentinel");
    let mut list = args(&["-anchor", &anchor_path, "-out", &out]);
    list.extend(args(&FAST));
    let (code, stdout, stderr) = invoke(&list);
    assert_eq!(code, EXIT_GATE_FAILED, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("PROMOTION GATES FAILED"), "{stderr}");
    assert!(stderr.contains(r#"✗ cohort "edge""#), "{stderr}");
    assert_eq!(std::fs::read(&out).unwrap(), b"existing sentinel");

    // A missing required cohort also fails the gate without writing.
    let fresh = dir.path("fresh.json");
    let clean = dir.write("clean.jsonl", &anchor(600, false));
    let mut list = args(&[
        "-anchor",
        &clean,
        "-out",
        &fresh,
        "-require-cohort",
        "screen-reader",
    ]);
    list.extend(args(&FAST));
    let (code, stdout, stderr) = invoke(&list);
    assert_eq!(code, EXIT_GATE_FAILED, "{stderr}");
    assert!(stdout.is_empty());
    assert!(!std::path::Path::new(&fresh).exists());
}

#[test]
fn input_and_precondition_errors_exit_1_without_output() {
    let dir = TempDir::new();
    let out = dir.path("candidate.json");
    let (code, stdout, stderr) = invoke(&args(&["-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(stdout.is_empty());
    assert!(stderr.contains("-anchor is required"), "{stderr}");

    let few = dir.write("few.jsonl", &anchor(50, false));
    let (code, _, stderr) = invoke(&args(&["-anchor", &few, "-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.contains("need >=100 samples, got 50"), "{stderr}");

    let mut bad = anchor(200, false);
    bad.extend_from_slice(b"{\"label\":2}\n");
    let bad = dir.write("bad.jsonl", &bad);
    let (code, _, stderr) = invoke(&args(&["-anchor", &bad, "-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.contains("anchor: bad JSONL input"), "{stderr}");

    let gold = dir.write("gold.jsonl", &anchor(200, false));
    let base = dir.write("base.json", b"{not json");
    let (code, _, stderr) = invoke(&args(&["-anchor", &gold, "-base", &base, "-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.contains("load base bundle"), "{stderr}");

    let huge = dir.write("huge.json", &vec![b' '; MAX_BUNDLE_BYTES + 1]);
    let (code, _, stderr) = invoke(&args(&["-anchor", &gold, "-base", &huge, "-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(stderr.contains("larger than"), "{stderr}");

    let missing = dir.path("absent.jsonl");
    let (code, _, _) = invoke(&args(&["-anchor", &missing, "-out", &out]));
    assert_eq!(code, EXIT_ERROR);
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn flag_syntax_and_usage_errors_follow_go_flag() {
    for (list, code) in [
        (args(&["-h"]), EXIT_OK),
        (args(&["--help"]), EXIT_OK),
        (args(&["-nope", "1"]), EXIT_USAGE),
        (args(&["-hidden", "many"]), EXIT_USAGE),
        (args(&["-anchor"]), EXIT_USAGE),
        (args(&["-anchor", "a.jsonl", "extra"]), EXIT_USAGE),
    ] {
        let (actual, stdout, stderr) = invoke(&list);
        assert_eq!(actual, code, "{list:?}: {stderr}");
        assert!(stdout.is_empty());
        assert!(stderr.contains("Usage of mlcorrect-train"), "{stderr}");
    }
    let Ok(Parsed::Run(options)) = parse(&args(&[])) else {
        panic!("empty command line parses");
    };
    assert_eq!(*options, Options::default());
    assert_eq!(
        (options.hidden, options.epochs, options.lr, options.seed),
        (16, 60, 0.05, 1)
    );
    assert_eq!(
        (
            options.test_frac,
            options.max_human_fpr,
            options.min_bot_tpr,
            options.min_cohort_humans
        ),
        (0.25, 0.005, 0.80, 20)
    );
    assert_eq!(options.out, "candidate.json");
    let Ok(Parsed::Run(options)) = parse(&args(&[
        "--test-frac=0.5",
        "-min-cohort-humans",
        "-3",
        "-require-cohort",
        "a",
        "-require-cohort=b",
    ])) else {
        panic!("flags parse");
    };
    assert_eq!(options.test_frac, 0.5);
    assert_eq!(options.min_cohort_humans, -3);
    assert_eq!(options.required_cohorts, ["a", "b"]);
}

#[test]
fn version_stamp_matches_the_legacy_layout() {
    let at = time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    assert_eq!(version_stamp(at), "mlp-corrected-20270115-080000");
}
