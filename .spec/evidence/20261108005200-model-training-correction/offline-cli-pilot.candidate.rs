//! REVIEW CANDIDATE: CLI tests/offline_training.rs; real binary, no NativeReady.
//! NOT_RUN. Polling thresholds are cooperative, not hard time or byte limits.
//! Spawn/kill/try_wait/filesystem calls may block. An outer hosted supervisor
//! with process-tree custody and output/storage controls is required before runs.
//! Simple owned test paths do not establish hostile-race file custody.
use std::{cell::{Cell, RefCell}, fs::{File, OpenOptions}, io::Write, path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio}, time::{Duration, Instant}};
use sha2::{Digest, Sha256};

struct Owned {
    directory: PathBuf,
    files: RefCell<Vec<PathBuf>>,
    child: RefCell<Option<Child>>,
    incomplete: Cell<bool>,
}
impl Owned {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!("humanymous-offline-command-{}-{}",
                std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self { directory: path, files: RefCell::new(Vec::new()),
                    child: RefCell::new(None), incomplete: Cell::new(false) },
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
        if self.child.borrow().is_some() || self.incomplete.get() {
            return Err(std::io::Error::other("pilot custody incomplete; files retained"));
        }
        let mut files = self.files.borrow_mut();
        while let Some(path) = files.last() { std::fs::remove_file(path)?; files.pop(); }
        std::fs::remove_dir(&self.directory)
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if self.child.get_mut().is_some() || self.incomplete.get() {
            // A panic must not silently drop an unsettled child. Preserve its
            // actual handle and paths in process-lifetime custody for supervisor
            // escalation. This is NOT settlement or an eventual cleanup claim.
            type Retained = (Option<Child>, PathBuf, Vec<PathBuf>);
            static RETAINED: std::sync::Mutex<Vec<Retained>> = std::sync::Mutex::new(Vec::new());
            let child = self.child.get_mut().take();
            eprintln!("PILOT_CUSTODY_INCOMPLETE pid={:?} retained={}",
                child.as_ref().map(Child::id), self.directory.display());
            RETAINED.lock().unwrap_or_else(|poison| poison.into_inner()).push((
                child, self.directory.clone(), std::mem::take(self.files.get_mut())));
            return;
        }
        // Only unwind fallback; every successful test must assert explicit finish.
        if self.directory.exists() {
            if let Err(error) = self.finish() {
                eprintln!("PILOT_CLEANUP_INCOMPLETE retained={} kind={:?}", self.directory.display(), error.kind());
            }
        }
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
#[derive(Debug)]
struct PilotFailure {
    stage: &'static str,
    kind: Option<std::io::ErrorKind>,
    pid: Option<u32>,
    custody_incomplete: bool,
}

fn failure(stage: &'static str, kind: Option<std::io::ErrorKind>, pid: Option<u32>, incomplete: bool) -> PilotFailure {
    PilotFailure { stage, kind, pid, custody_incomplete: incomplete }
}

fn log_threshold(dir: &Owned, log: &str) -> Result<bool, PilotFailure> {
    let mut oversized = false;
    for ext in ["stdout", "stderr"] {
        let size = std::fs::metadata(dir.path(&format!("{log}.{ext}")))
            .map_err(|e| failure("log observation", Some(e.kind()), None, false))?.len();
        oversized |= size > 1 << 20;
    }
    // A child can write arbitrarily more bytes between polls; this is an
    // observation threshold only, never a hard output/storage cap.
    Ok(oversized)
}

fn stop_owned(dir: &Owned, clock: &Instant) -> Result<(), PilotFailure> {
    let mut owner = dir.child.borrow_mut();
    let Some(child) = owner.as_mut() else { return Ok(()); };
    let pid = child.id();
    // kill itself has no hard completion bound in std::process. No blocking
    // wait() follows it; cooperative polling uses the ORIGINAL pre-spawn clock.
    let mut last_error = child.kill().err().map(|e| e.kind());
    loop {
        if clock.elapsed() >= Duration::from_secs(35) {
            dir.incomplete.set(true);
            return Err(failure("termination/reap incomplete", last_error, Some(pid), true));
        }
        match child.try_wait() {
            Ok(Some(_)) => { owner.take(); return Ok(()); }
            Ok(None) => {}
            Err(error) => { last_error = Some(error.kind()); }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run(dir: &Owned, args: &[String], output: &str, log: &str) -> Result<ExitStatus, PilotFailure> {
    // Includes path/log preparation and spawn. It never resets after spawn or
    // after an observation failure. OS calls still have no hard time bound.
    let clock = Instant::now();
    if dir.child.borrow().is_some() || dir.incomplete.get() {
        return Err(failure("prior custody incomplete", None, None, true));
    }
    let existed = dir.path(output).try_exists()
        .map_err(|e| failure("output precondition", Some(e.kind()), None, false))?;
    let stdout = dir.create(&format!("{log}.stdout"));
    let stderr = dir.create(&format!("{log}.stderr"));
    let child = Command::new(env!("CARGO_BIN_EXE_humanymous")).args(args)
        .stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).spawn()
        .map_err(|e| {
            // No Child handle was returned. Do not assert OS-level absence of
            // a partially created process; retain paths for outer supervision.
            dir.incomplete.set(true);
            failure("spawn returned no handle", Some(e.kind()), None, true)
        })?;
    let pid = child.id();
    *dir.child.borrow_mut() = Some(child);
    let observed = loop {
        if clock.elapsed() >= Duration::from_secs(30) {
            break Err(failure("elapsed threshold", None, Some(pid), false));
        }
        match log_threshold(dir, log) {
            Ok(true) => break Err(failure("output threshold", None, Some(pid), false)),
            Ok(false) => {}
            Err(error) => break Err(error),
        }
        let observation = match dir.child.borrow_mut().as_mut() {
            Some(child) => child.try_wait(),
            None => { dir.incomplete.set(true); return Err(failure("lost owner", None, Some(pid), true)); }
        };
        match observation {
            Ok(Some(status)) => { dir.child.borrow_mut().take(); break Ok(status); }
            Ok(None) => {}
            Err(error) => break Err(failure("try_wait", Some(error.kind()), Some(pid), false)),
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Every observation-error path still owns the Child while requesting stop.
    // On incomplete settlement it remains in Owned and then the retained registry.
    if observed.is_err() { stop_owned(dir, &clock)?; }
    if !existed && dir.path(output).try_exists().map_err(|e| {
        dir.incomplete.set(true);
        failure("output ownership observation", Some(e.kind()), Some(pid), true)
    })? {
        // The controlled child exclusively received this fresh output path.
        // Retain even unsuccessful child's partial file as a cleanup obligation.
        dir.files.borrow_mut().push(dir.path(output));
    }
    if log_threshold(dir, log)? {
        return Err(failure("final output threshold", None, Some(pid), false));
    }
    if let Err(error) = &observed {
        eprintln!("PILOT_FAILURE stage={} kind={:?} pid={:?} custody_incomplete={}",
            error.stage, error.kind, error.pid, error.custody_incomplete);
    }
    observed
}

#[test]
fn real_offline_command_emits_unsigned_interoperable_data_and_preserves_sentinel() {
    let dir = Owned::new(); dir.write("train.jsonl", &data(0)); dir.write("holdout.jsonl", &data(100));
    assert!(run(&dir, &args(&dir, "model.json"), "model.json", "first").unwrap().success());
    let model = std::fs::read(dir.path("model.json")).unwrap();
    let bundle: humanymous_core::model::inference::MlpBundle = serde_json::from_slice(&model).unwrap();
    assert_eq!(bundle.version, "offline-process"); assert_eq!(bundle.mean[1], 0.5);
    assert_eq!(bundle.cal_a, 1.0); assert_eq!(bundle.cal_b.to_bits(), 0);
    let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.path("first.stdout")).unwrap()).unwrap();
    assert_eq!(receipt["activation"], false); assert_eq!(receipt["signed"], false);
    assert_eq!(receipt["sha256"], hex::encode(Sha256::digest(&model)));
    assert!(run(&dir, &args(&dir, "repeat.json"), "repeat.json", "repeat").unwrap().success());
    assert_eq!(std::fs::read(dir.path("repeat.json")).unwrap(), model);
    assert!(!run(&dir, &args(&dir, "model.json"), "model.json", "collision").unwrap().success());
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
        assert!(!run(&dir, &command, "model.json", "refusal").unwrap().success());
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
