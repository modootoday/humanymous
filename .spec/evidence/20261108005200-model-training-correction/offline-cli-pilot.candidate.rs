//! REVIEW CANDIDATE: CLI tests/offline_training.rs; real binary, no NativeReady.
//! NOT_RUN. Simple owned test paths do not establish hostile-race file custody.
use std::{cell::RefCell, fs::{File, OpenOptions}, io::Write, path::PathBuf,
    process::{Command, ExitStatus, Stdio}, time::{Duration, Instant}};
use sha2::{Digest, Sha256};

struct Owned {
    directory: PathBuf,
    files: RefCell<Vec<PathBuf>>,
}
impl Owned {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!("humanymous-offline-command-{}-{}",
                std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self { directory: path, files: RefCell::new(Vec::new()) },
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("owned pilot create: {e}"),
            }
        }
        panic!("owned pilot collision budget");
    }
    fn path(&self, name: &str) -> PathBuf { self.directory.join(name) }
    fn create(&self, name: &str) -> File {
        let path = self.path(name);
        let file = OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
        self.files.borrow_mut().push(path);
        file
    }
    fn write(&self, name: &str, bytes: &[u8]) { self.create(name).write_all(bytes).unwrap(); }
    fn finish(&self) -> std::io::Result<()> {
        let mut files = self.files.borrow_mut();
        while let Some(path) = files.last() { std::fs::remove_file(path)?; files.pop(); }
        std::fs::remove_dir(&self.directory)
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // Only unwind fallback; every successful test must assert explicit finish.
        if self.directory.exists() { let _ = self.finish(); }
    }
}

fn data(first: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..8 {
        let label = u8::from(i % 2 == 0);
        let row = serde_json::json!({"label": label,
            "behavior": {"mouse": {"meanCurvature": if label == 1 {0.75} else {0.25}}},
            "ts": first+i, "cohort": if i%4==1 {"a"} else {"b"},
            "source": if label==1 {"catalog"} else {"pass"}});
        serde_json::to_writer(&mut bytes, &row).unwrap(); bytes.push(b'\n');
    }
    bytes
}
fn args(dir: &Owned, output: &str) -> Vec<String> {
    vec!["model-train-offline".into(), "--train".into(), dir.path("train.jsonl").display().to_string(),
        "--holdout".into(), dir.path("holdout.jsonl").display().to_string(), "--out".into(),
        dir.path(output).display().to_string(), "--version".into(), "offline-process".into(),
        "--hidden".into(), "1".into(), "--epochs".into(), "0".into(), "--lr".into(),
        "0.03125".into(), "--seed".into(), "2".into(), "--min-cohort-humans".into(),
        "1".into(), "--reviewed-input".into()]
}
fn run(dir: &Owned, args: &[String], output: &str, log: &str) -> ExitStatus {
    let existed = dir.path(output).exists();
    let stdout = dir.create(&format!("{log}.stdout"));
    let stderr = dir.create(&format!("{log}.stderr"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous")).args(args)
        .stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).spawn().unwrap();
    let clock = Instant::now();
    let result = loop {
        if let Some(status) = child.try_wait().unwrap() { break status; }
        let oversized = ["stdout", "stderr"].iter().any(|ext| {
            std::fs::metadata(dir.path(&format!("{log}.{ext}"))).unwrap().len() > 1<<20
        });
        if clock.elapsed() > Duration::from_secs(30) || oversized {
            child.kill().unwrap(); child.wait().unwrap();
            panic!("owned offline command exceeded time/output budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !existed && dir.path(output).exists() {
        // The controlled child exclusively received this fresh output path.
        // Retain even unsuccessful child's partial file as a cleanup obligation.
        dir.files.borrow_mut().push(dir.path(output));
    }
    result
}

#[test]
fn real_offline_command_emits_unsigned_interoperable_data_and_preserves_sentinel() {
    let dir = Owned::new(); dir.write("train.jsonl", &data(0)); dir.write("holdout.jsonl", &data(100));
    assert!(run(&dir, &args(&dir, "model.json"), "model.json", "first").success());
    let model = std::fs::read(dir.path("model.json")).unwrap();
    let bundle: humanymous_core::model::inference::MlpBundle = serde_json::from_slice(&model).unwrap();
    assert_eq!(bundle.version, "offline-process"); assert_eq!(bundle.mean[1], 0.5);
    assert_eq!(bundle.cal_a, 1.0); assert_eq!(bundle.cal_b.to_bits(), 0);
    let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.path("first.stdout")).unwrap()).unwrap();
    assert_eq!(receipt["activation"], false); assert_eq!(receipt["signed"], false);
    assert_eq!(receipt["sha256"], hex::encode(Sha256::digest(&model)));
    assert!(run(&dir, &args(&dir, "repeat.json"), "repeat.json", "repeat").success());
    assert_eq!(std::fs::read(dir.path("repeat.json")).unwrap(), model);
    assert!(!run(&dir, &args(&dir, "model.json"), "model.json", "collision").success());
    assert_eq!(std::fs::read(dir.path("model.json")).unwrap(), model);
    dir.finish().unwrap();
}

#[test]
fn real_offline_command_refuses_bad_time_admission_support_shape_and_numeric_config() {
    for case in 0..6 {
        let dir = Owned::new();
        dir.write("train.jsonl", &data(0));
        let mut holdout = data(if case == 0 { 7 } else { 100 });
        if case == 1 {
            holdout = String::from_utf8(holdout).unwrap().replace("\"pass\"", "\"challenge\"").into_bytes();
        }
        if case == 2 {
            holdout = String::from_utf8(holdout).unwrap().replace("\"a\"", "\"b\"").into_bytes();
        }
        dir.write("holdout.jsonl", &holdout);
        let mut command = args(&dir, "model.json");
        if case == 3 { command[10] = "257".into(); }
        if case == 4 { command[14] = "NaN".into(); }
        if case == 5 {
            dir.write("base.json", b"{}");
            command.extend(["--base".into(), dir.path("base.json").display().to_string()]);
        }
        assert!(!run(&dir, &command, "model.json", "refusal").success());
        let expected = match case {
            0 => "Training(Time)", 1 => "Training(Admission)", 2 => "Training(Support)",
            3 => "Training(Limits)", 4 => "Training(Config)", _ => "Training(WarmBase)",
        };
        let stderr = std::fs::read_to_string(dir.path("refusal.stderr")).unwrap();
        assert!(stderr.contains(expected), "unexpected refusal category: {stderr}");
        assert!(!dir.path("model.json").exists());
        dir.finish().unwrap();
    }
}
