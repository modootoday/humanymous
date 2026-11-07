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
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
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

struct OwnedDirectory(PathBuf);

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
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("owned pilot directory: {e}"),
            }
        }
        panic!("owned pilot directory capacity");
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.path(name))
            .unwrap();
        file.write_all(bytes).unwrap();
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        // Fixed names under a directory this process successfully created, no
        // recursive deletion, foreign paths, or deleting an unowned collision.
        for name in [
            "train.jsonl",
            "holdout.jsonl",
            "candidate.json",
            "repeat.json",
        ] {
            let _ = std::fs::remove_file(self.path(name));
        }
        let _ = std::fs::remove_dir(&self.0);
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
    dir.write("train.jsonl", &records(0, 64, false));
    dir.write("holdout.jsonl", &records(1000, 16, false));
    let run = |name| {
        offline_files(
            &dir.path("train.jsonl"),
            &dir.path("holdout.jsonl"),
            &dir.path(name),
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
        dir.write("train.jsonl", &training);
        dir.write("holdout.jsonl", &validation);
        assert!(
            offline_files(
                &dir.path("train.jsonl"),
                &dir.path("holdout.jsonl"),
                &dir.path("candidate.json"),
                parser,
                config,
                limits
            )
            .is_err()
        );
        assert!(!dir.path("candidate.json").exists());
    }
}
