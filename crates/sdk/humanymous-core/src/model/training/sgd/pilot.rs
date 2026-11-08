//! Actual owned-file pilot source; NOT a CLI/process or product qualification.
use super::*;
use crate::model::inference::{Mlp, Predictor};
use crate::model::training::{
    evaluate::{Counts, LabeledPrediction, evaluate},
    record::{Admission, Limits, Record, Sample, read_jsonl},
    standardize::standardize,
};
use crate::signals::report::BehaviorSummary;
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
enum PilotError {
    Input,
    Admission,
    Support,
    Training(TrainError),
    Evaluation,
    // An owned file may exist after an I/O failure; it is never published success.
    Output { owned_unpublished: bool },
}

fn read(path: &Path, limits: Limits) -> Result<Vec<Sample>, PilotError> {
    let file = File::open(path).map_err(|_| PilotError::Input)?;
    read_jsonl(BufReader::new(file), &schema_hash(), limits).map_err(|_| PilotError::Input)
}

/// Complete prevalidation, train and evaluate BEFORE output creation. The
/// synthetic caller explicitly reviews records; source strings alone grant none.
fn offline_files(
    train_path: &Path,
    holdout_path: &Path,
    output_path: &Path,
    owner: &OwnedDirectory,
    parser_limits: Limits,
    config: TrainConfig<'_>,
    limits: TrainLimits,
) -> Result<Counts, PilotError> {
    let train_samples = read(train_path, parser_limits)?;
    let holdout = read(holdout_path, parser_limits)?;
    let mut ids = BTreeSet::new();
    for sample in train_samples.iter().chain(&holdout) {
        if sample.admission != Admission::ReviewRequired
            || sample.record.cohort.is_none()
            || !sample.record.ts.is_some_and(|id| ids.insert(id))
        {
            return Err(PilotError::Admission);
        }
    }
    if !holdout.iter().any(|s| s.record.label == 0) || !holdout.iter().any(|s| s.record.label == 1)
    {
        return Err(PilotError::Support);
    }
    if holdout.len() > limits.samples {
        return Err(PilotError::Input);
    }
    let vectors: Vec<_> = train_samples.iter().map(|s| s.features).collect();
    let statistics =
        standardize(&vectors, &schema_hash(), limits.samples).map_err(|_| PilotError::Input)?;
    let rows: Vec<_> = train_samples
        .iter()
        .map(|s| TrainingRow {
            features: &s.features,
            label: s.record.label,
        })
        .collect();
    let result = train(config, &rows, &statistics, &schema_hash(), None, limits)
        .map_err(PilotError::Training)?;
    let model = Mlp::from_bundle(result.candidate.clone()).map_err(|_| PilotError::Evaluation)?;
    let pairs: Vec<_> = holdout
        .iter()
        .map(|s| LabeledPrediction {
            label: s.record.label,
            prediction: model.predict(&s.features),
        })
        .collect();
    let expected = evaluate(&pairs, limits.samples).map_err(|_| PilotError::Evaluation)?;
    if expected.accuracy().is_none()
        || expected.human_fpr().is_none()
        || expected.bot_tpr().is_none()
    {
        return Err(PilotError::Support);
    }
    let bytes = serde_json::to_vec(&result.candidate).map_err(|_| PilotError::Evaluation)?;
    let mut file = owner
        .create_file(output_path)
        .map_err(|_| PilotError::Output {
            owned_unpublished: false,
        })?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| PilotError::Output {
            owned_unpublished: true,
        })?;
    drop(file);
    let reread = std::fs::read(output_path).map_err(|_| PilotError::Output {
        owned_unpublished: true,
    })?;
    let reloaded = Mlp::from_json(&reread).map_err(|_| PilotError::Output {
        owned_unpublished: true,
    })?;
    if reread != bytes
        || reloaded.schema_hash() != schema_hash()
        || reloaded.bundle_version() != config.version
        || holdout.iter().zip(&pairs).any(|(s, before)| {
            let after = reloaded.predict(&s.features);
            after.abstain != before.prediction.abstain
                || after.p_bot.to_bits() != before.prediction.p_bot.to_bits()
        })
    {
        return Err(PilotError::Output {
            owned_unpublished: true,
        });
    }
    // Success is only an offline output receipt, never activation or promotion.
    Ok(expected)
}

struct OwnedDirectory {
    path: PathBuf,
    created: std::cell::RefCell<Vec<PathBuf>>,
    finished: std::cell::Cell<bool>,
}

impl OwnedDirectory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "humanymous-sgd-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    return Self {
                        path,
                        created: std::cell::RefCell::new(Vec::new()),
                        finished: std::cell::Cell::new(false),
                    };
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("owned pilot directory: {e}"),
            }
        }
        panic!("owned pilot directory capacity");
    }
    fn path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
    fn create_file(&self, path: &Path) -> std::io::Result<File> {
        if self.finished.get() || path.parent() != Some(self.path.as_path()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "outside owned pilot directory",
            ));
        }
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        // Track immediately after successful creation, before fallible writes.
        // This simple pilot ledger is not hostile-race/native file custody.
        self.created.borrow_mut().push(path.to_path_buf());
        Ok(file)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let mut file = self.create_file(&self.path(name))?;
        file.write_all(bytes)
    }
    fn finish(&self) -> std::io::Result<()> {
        if self.finished.get() {
            return Ok(());
        }
        let mut created = self.created.borrow_mut();
        while let Some(path) = created.last() {
            std::fs::remove_file(path)?;
            created.pop();
        }
        // An untracked collision prevents directory removal and is reported.
        std::fs::remove_dir(&self.path)?;
        self.finished.set(true);
        Ok(())
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        // Unwind fallback only. Tests must assert explicit finish(), so ignored
        // fallback errors cannot turn cleanup failure into a passing receipt.
        let _ = self.finish();
    }
}

fn records(first: i64, count: usize, only_human: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..count {
        let label = u8::from(!only_human && i % 2 == 0);
        let mut behavior = BehaviorSummary {
            duration_s: 1.0 + f64::from(label) * 4.0,
            ..Default::default()
        };
        behavior.mouse.samples = 10 + i64::from(label) * 100;
        let record = Record {
            label,
            behavior,
            ts: Some(first + i as i64),
            cohort: Some(if i % 2 == 0 { "cohort-a" } else { "cohort-b" }.into()),
            source: Some(if label == 1 { "catalog" } else { "pass" }.into()),
        };
        serde_json::to_writer(&mut bytes, &record).unwrap();
        bytes.push(b'\n');
    }
    bytes
}

fn pilot_config() -> TrainConfig<'static> {
    TrainConfig {
        hidden: 8,
        epochs: 40,
        learning_rate: 0.1,
        seed: 2,
        version: "offline-file-pilot",
    }
}

#[test]
fn actual_file_training_roundtrip_is_reproducible_and_never_overwrites_sentinel() {
    let dir = OwnedDirectory::new();
    dir.write("train.jsonl", &records(0, 64, false)).unwrap();
    dir.write("holdout.jsonl", &records(1000, 16, false))
        .unwrap();
    let run = |name| {
        offline_files(
            &dir.path("train.jsonl"),
            &dir.path("holdout.jsonl"),
            &dir.path(name),
            &dir,
            Limits::default(),
            pilot_config(),
            TrainLimits::default(),
        )
    };
    let first = run("candidate.json").unwrap();
    let repeated = run("repeat.json").unwrap();
    assert_eq!(first, repeated);
    assert_eq!(first.total, 16);
    assert_eq!(
        std::fs::read(dir.path("candidate.json")).unwrap(),
        std::fs::read(dir.path("repeat.json")).unwrap()
    );
    let sentinel = std::fs::read(dir.path("candidate.json")).unwrap();
    assert_eq!(
        run("candidate.json"),
        Err(PilotError::Output {
            owned_unpublished: false
        })
    );
    assert_eq!(std::fs::read(dir.path("candidate.json")).unwrap(), sentinel);
    dir.finish().unwrap();
}

#[test]
fn actual_file_late_invalid_quarantine_support_and_capacity_fail_before_output() {
    for case in 0..7 {
        let dir = OwnedDirectory::new();
        let mut training = records(0, 8, false);
        let mut validation = records(1000, 4, false);
        let mut parser = Limits::default();
        let mut limits = TrainLimits::default();
        let mut config = pilot_config();
        match case {
            0 => training.extend_from_slice(b"{\"label\":2}\n"),
            1 => {
                let record = Record {
                    label: 0,
                    behavior: BehaviorSummary::default(),
                    ts: Some(900),
                    cohort: Some("default".into()),
                    source: None,
                };
                serde_json::to_writer(&mut training, &record).unwrap();
                training.push(b'\n');
            }
            2 => validation = records(1000, 4, true),
            3 => parser.records = 3,
            4 => parser.total_bytes = 10,
            5 => limits.work_units = 1,
            _ => config.learning_rate = f32::MAX,
        }
        dir.write("train.jsonl", &training).unwrap();
        dir.write("holdout.jsonl", &validation).unwrap();
        let expected = match case {
            0 | 3 | 4 => PilotError::Input,
            1 => PilotError::Admission,
            2 => PilotError::Support,
            5 => PilotError::Training(TrainError::Limit),
            _ => PilotError::Training(TrainError::Nonfinite),
        };
        assert_eq!(
            offline_files(
                &dir.path("train.jsonl"),
                &dir.path("holdout.jsonl"),
                &dir.path("candidate.json"),
                &dir,
                parser,
                config,
                limits
            ),
            Err(expected)
        );
        assert!(!dir.path("candidate.json").exists());
        dir.finish().unwrap();
    }
}

#[test]
fn actual_collision_is_unowned_and_cleanup_failure_is_reported() {
    let dir = OwnedDirectory::new();
    let collision = dir.path("candidate.json");
    let mut foreign = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&collision)
        .unwrap();
    foreign.write_all(b"unowned sentinel").unwrap();
    drop(foreign);
    assert_eq!(
        dir.write("candidate.json", b"replacement")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert!(dir.created.borrow().is_empty());
    let failure = dir
        .finish()
        .expect_err("unowned entry must prevent cleanup success");
    assert!(dir.path.exists());
    assert!(!dir.finished.get());
    assert_eq!(std::fs::read(&collision).unwrap(), b"unowned sentinel");
    // The fixture that created the collision settles its own file explicitly.
    assert_ne!(failure.kind(), std::io::ErrorKind::NotFound);
    std::fs::remove_file(collision).unwrap();
    dir.finish().unwrap();
}
