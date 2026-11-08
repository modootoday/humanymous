//! REVIEW CANDIDATE: private CLI offline_train command; no NativeReady dependency.
use humanymous_core::model::{
    inference::MlpBundle,
    training::{offline::{fit_offline, OfflineConfig, OfflineError, OfflineInput, OfflineLimits},
        record::{read_jsonl, Limits, Sample}},
};
use humanymous_core::behavior::features::schema_hash;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::{File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

const ARTIFACT_BYTES: u64 = 1 << 20;

#[derive(Debug)]
pub(super) enum CommandError {
    Arguments, ReviewRequired, Input, Base, Training(OfflineError), Serialize, OutputExists, OutputCreate,
    // Retain the owned path as a cleanup obligation; never erase an old sentinel.
    UnpublishedOutput(PathBuf),
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnpublishedOutput(path) => write!(f, "unsigned output retained after I/O failure: {}", path.display()),
            _ => write!(f, "offline training refused: {self:?}"),
        }
    }
}
impl std::error::Error for CommandError {}

struct Arguments {
    train: PathBuf, holdout: PathBuf, output: PathBuf, base: Option<PathBuf>,
    version: String, hidden: i64, epochs: i64, learning_rate: f32, seed: i64,
    minimum_cohort_humans: usize,
}

fn parse(args: Vec<String>) -> Result<Arguments, CommandError> {
    if args.len() > 32 { return Err(CommandError::Arguments); }
    let mut map = BTreeMap::new();
    let mut reviewed = false;
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        if flag == "--reviewed-input" {
            if reviewed { return Err(CommandError::Arguments); }
            reviewed = true;
            continue;
        }
        if !["--train", "--holdout", "--out", "--base", "--version", "--hidden",
            "--epochs", "--lr", "--seed", "--min-cohort-humans"].contains(&flag.as_str())
        { return Err(CommandError::Arguments); }
        let value = args.next().ok_or(CommandError::Arguments)?;
        if value.len() > 4096 || map.insert(flag, value).is_some() { return Err(CommandError::Arguments); }
    }
    if !reviewed { return Err(CommandError::ReviewRequired); }
    let mut get = |key: &str| map.remove(key).ok_or(CommandError::Arguments);
    let train = get("--train")?.into();
    let holdout = get("--holdout")?.into();
    let output = get("--out")?.into();
    let version = get("--version")?;
    let hidden = get("--hidden")?.parse().map_err(|_| CommandError::Arguments)?;
    let epochs = get("--epochs")?.parse().map_err(|_| CommandError::Arguments)?;
    let learning_rate = get("--lr")?.parse().map_err(|_| CommandError::Arguments)?;
    let seed = get("--seed")?.parse().map_err(|_| CommandError::Arguments)?;
    let minimum_cohort_humans = get("--min-cohort-humans")?.parse().map_err(|_| CommandError::Arguments)?;
    Ok(Arguments { train, holdout, output, base: map.remove("--base").map(PathBuf::from),
        version, hidden, epochs, learning_rate, seed, minimum_cohort_humans })
}

fn records(path: &Path) -> Result<Vec<Sample>, CommandError> {
    let file = File::open(path).map_err(|_| CommandError::Input)?;
    read_jsonl(BufReader::new(file), &schema_hash(), Limits::default()).map_err(|_| CommandError::Input)
}

fn base(path: &Path) -> Result<MlpBundle, CommandError> {
    let file = File::open(path).map_err(|_| CommandError::Base)?;
    let mut bytes = Vec::new();
    file.take(ARTIFACT_BYTES + 1).read_to_end(&mut bytes).map_err(|_| CommandError::Base)?;
    if bytes.len() as u64 > ARTIFACT_BYTES { return Err(CommandError::Base); }
    // Core performs complete finite/shape/schema validation before warm copy.
    serde_json::from_slice(&bytes).map_err(|_| CommandError::Base)
}

/// Genuine command caller. --reviewed-input acknowledges caller review; it does
/// not authenticate source strings, humans or oracle events.
pub(super) fn run(args: Vec<String>) -> Result<(), CommandError> {
    let a = parse(args)?;
    let train = records(&a.train)?;
    let holdout = records(&a.holdout)?;
    let base = a.base.as_deref().map(base).transpose()?;
    let schema = schema_hash();
    let report = fit_offline(OfflineInput { train: &train, holdout: &holdout, base: base.as_ref() },
        OfflineConfig { schema: &schema, version: &a.version, hidden: a.hidden, epochs: a.epochs,
            learning_rate: a.learning_rate, seed: a.seed },
        OfflineLimits { min_holdout_humans_per_cohort: a.minimum_cohort_humans, ..OfflineLimits::default() })
        .map_err(CommandError::Training)?;
    // Plain original MlpBundle JSON preserves Go input/output interchange.
    // Metrics/provenance flags are a separate unsigned data receipt.
    let bytes = serde_json::to_vec(&report.model).map_err(|_| CommandError::Serialize)?;
    if bytes.len() as u64 > ARTIFACT_BYTES { return Err(CommandError::Serialize); }
    let receipt = serde_json::to_vec(&serde_json::json!({
        "activation": false, "signed": false, "sha256": hex::encode(Sha256::digest(&bytes)),
        "metrics": report.metrics, "cohorts": report.cohorts, "summary": report.summary,
        "calibration": "legacy_train_reset_a1_b0", "go_bit_parity": "unqualified"
    })).map_err(|_| CommandError::Serialize)?;
    if receipt.len() as u64 > ARTIFACT_BYTES { return Err(CommandError::Serialize); }
    let retained = || CommandError::UnpublishedOutput(a.output.clone());
    let mut file = OpenOptions::new().read(true).write(true).create_new(true).open(&a.output)
        .map_err(|e| if e.kind() == std::io::ErrorKind::AlreadyExists {
            CommandError::OutputExists
        } else { CommandError::OutputCreate })?;
    file.write_all(&bytes).and_then(|()| file.sync_all()).map_err(|_| retained())?;
    file.seek(SeekFrom::Start(0)).map_err(|_| retained())?;
    let mut reread = Vec::new();
    Read::by_ref(&mut file).take(ARTIFACT_BYTES + 1).read_to_end(&mut reread).map_err(|_| retained())?;
    if reread != bytes { return Err(retained()); }
    drop(file);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&receipt).and_then(|()| stdout.write_all(b"\n")).map_err(|_| retained())?;
    Ok(())
}

#[cfg(test)]
#[path = "offline_train/tests.rs"]
mod tests;
