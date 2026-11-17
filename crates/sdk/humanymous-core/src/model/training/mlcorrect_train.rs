//! Process shell of the offline retrain step (legacy `cmd/mlcorrect-train/main.go`
//! at baseline 981d25c; SoT model-training-correction, row "both training
//! commands"). The `mlcorrect-train` binary is a thin wrapper over [`run`].
//!
//! It reads the frozen GOLD ANCHOR plus optional ORACLE-CONFIRMED labels,
//! optionally warm-starts from the current bundle, runs the promotion gate
//! ([`run_gate`]) and writes a CANDIDATE bundle ONLY IF every gate passed. It
//! does not deploy: a passing candidate is written and its lowercase SHA-256
//! digest printed on stdout for the separate signing/staging step. A failed
//! gate exits [`EXIT_GATE_FAILED`] and writes NOTHING (an existing output file
//! is left untouched). Diagnostics go to stderr.
//!
//! ```text
//! mlcorrect-train -anchor gold.jsonl -oracle confirmed.jsonl -base current.json -out candidate.json
//! ```
//!
//! Deliberate differences from Go:
//! - The base bundle is read ONCE into a bounded buffer
//!   ([`MAX_BUNDLE_BYTES`]); validation and warm start use that same buffer
//!   (Go validated the file, then re-read it).
//! - Oracle records whose provenance is weak or unknown
//!   ([`Admission::Quarantined`], e.g. `source:"challenge"`) are excluded from
//!   training and counted on stderr; Go admitted them as bot labels. The
//!   anchor is the operator-reviewed gold set and is admitted as given.
//! - JSONL input uses the shared bounded reader: any invalid record, a
//!   missing timestamp (at split time) or an exceeded budget rejects the run.
//! - Positional arguments are a usage error (Go silently ignored them);
//!   numeric flags accept decimal notation only.
//! - The candidate is serialized by `serde_json` with one-space indentation;
//!   float text may differ from Go's `encoding/json`, so the digest identifies
//!   the Rust-written bytes, not a Go-written artifact.
use super::gate::{GateConfig, run_gate};
use super::record::{Admission, Limits, Sample, read_jsonl};
use crate::behavior::features::schema_hash;
use crate::model::correction::{MAX_BUNDLE_BYTES, digest};
use crate::model::inference::{Mlp, MlpBundle, Predictor};
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Read, Write};

/// Candidate written (or nothing to do): success.
pub const EXIT_OK: i32 = 0;
/// Input, precondition or output failure.
pub const EXIT_ERROR: i32 = 1;
/// Command-line usage error.
pub const EXIT_USAGE: i32 = 2;
/// Promotion gates failed; nothing was written.
pub const EXIT_GATE_FAILED: i32 = 3;

const USAGE: &str = "\
Usage of mlcorrect-train:
  -anchor string
    \tfrozen GOLD ANCHOR JSONL (catalog bots + confirmed humans) — required
  -base string
    \tcurrent serving bundle to WARM-START from (optional; cold-start if absent)
  -epochs int
    \ttraining epochs (default 60)
  -hidden int
    \thidden units (must match base to warm-start) (default 16)
  -lr float
    \tlearning rate (default 0.05)
  -max-human-fpr float
    \tBLOCKING: max per-cohort human false-positive rate (default 0.005)
  -min-bot-tpr float
    \tBLOCKING: minimum bot TPR (catalog no-regression floor) (default 0.8)
  -min-cohort-humans int
    \tmin humans a cohort needs before its FPR gate is trusted (default 20)
  -oracle string
    \toracle-confirmed labels JSONL from the self-correcting loop (optional)
  -out string
    \tcandidate bundle output path (written only if gates pass) (default \"candidate.json\")
  -require-cohort string
    \tcohort that must appear in the test window (repeatable; Rust addition)
  -seed int
    \trng seed (default 1)
  -test-frac float
    \tTESSERACT time-ordered holdout fraction (default 0.25)
";

/// Parsed command line with the legacy defaults.
#[derive(Debug, Clone, PartialEq)]
struct Options {
    anchor: String,
    oracle: String,
    base: String,
    out: String,
    hidden: i64,
    epochs: i64,
    lr: f64,
    seed: i64,
    test_frac: f64,
    max_human_fpr: f64,
    min_bot_tpr: f64,
    min_cohort_humans: i64,
    required_cohorts: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            anchor: String::new(),
            oracle: String::new(),
            base: String::new(),
            out: "candidate.json".to_owned(),
            hidden: 16,
            epochs: 60,
            lr: 0.05,
            seed: 1,
            test_frac: 0.25,
            max_human_fpr: 0.005,
            min_bot_tpr: 0.80,
            min_cohort_humans: 20,
            required_cohorts: Vec::new(),
        }
    }
}

enum Parsed {
    Run(Box<Options>),
    Help,
}

fn number<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value {value:?} for flag -{name}: parse error"))
}

/// Go `flag` syntax: `-name value`, `-name=value`, one or two dashes.
fn parse(args: &[String]) -> Result<Parsed, String> {
    let mut options = Options::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--" || !arg.starts_with('-') || arg == "-" {
            return Err(format!("unexpected positional argument {arg:?}"));
        }
        let flag = arg
            .strip_prefix("--")
            .unwrap_or_else(|| &arg[1..])
            .to_owned();
        let (name, inline) = match flag.split_once('=') {
            Some((name, value)) => (name.to_owned(), Some(value.to_owned())),
            None => (flag, None),
        };
        if name == "h" || name == "help" {
            return Ok(Parsed::Help);
        }
        const KNOWN: [&str; 13] = [
            "anchor",
            "oracle",
            "base",
            "out",
            "hidden",
            "epochs",
            "lr",
            "seed",
            "test-frac",
            "max-human-fpr",
            "min-bot-tpr",
            "min-cohort-humans",
            "require-cohort",
        ];
        if !KNOWN.contains(&name.as_str()) {
            return Err(format!("flag provided but not defined: -{name}"));
        }
        let value = match inline {
            Some(value) => value,
            None => rest
                .next()
                .cloned()
                .ok_or_else(|| format!("flag needs an argument: -{name}"))?,
        };
        match name.as_str() {
            "anchor" => options.anchor = value,
            "oracle" => options.oracle = value,
            "base" => options.base = value,
            "out" => options.out = value,
            "hidden" => options.hidden = number(&name, &value)?,
            "epochs" => options.epochs = number(&name, &value)?,
            "lr" => options.lr = number(&name, &value)?,
            "seed" => options.seed = number(&name, &value)?,
            "test-frac" => options.test_frac = number(&name, &value)?,
            "max-human-fpr" => options.max_human_fpr = number(&name, &value)?,
            "min-bot-tpr" => options.min_bot_tpr = number(&name, &value)?,
            "min-cohort-humans" => options.min_cohort_humans = number(&name, &value)?,
            _ => options.required_cohorts.push(value),
        }
    }
    Ok(Parsed::Run(Box::new(options)))
}

/// Legacy candidate version: `mlp-corrected-YYYYMMDD-HHMMSS` in UTC.
pub fn version_stamp(now: time::OffsetDateTime) -> String {
    let now = now.to_offset(time::UtcOffset::UTC);
    format!(
        "mlp-corrected-{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn read_samples(path: &str, kind: &str) -> Result<Vec<Sample>, String> {
    let file = File::open(path).map_err(|error| format!("open {path}: {error}"))?;
    read_jsonl(BufReader::new(file), &schema_hash(), Limits::default())
        .map_err(|error| format!("{kind}: bad JSONL input: {error}"))
}

/// Read the base bundle once into a bounded buffer and validate that buffer.
fn read_base(path: &str) -> Result<(MlpBundle, String), String> {
    let file = File::open(path).map_err(|error| format!("load base bundle: {path}: {error}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BUNDLE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("load base bundle: {path}: {error}"))?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(format!(
            "load base bundle: {path}: larger than {MAX_BUNDLE_BYTES} bytes"
        ));
    }
    let model = Mlp::from_json(&bytes).map_err(|error| format!("load base bundle: {error}"))?;
    let bundle = serde_json::from_slice::<MlpBundle>(&bytes)
        .map_err(|error| format!("load base bundle: {error}"))?;
    Ok((bundle, model.bundle_version().to_owned()))
}

fn rate(value: Option<f32>, precision: usize) -> String {
    value.map_or_else(|| "n/a".to_owned(), |v| format!("{v:.precision$}"))
}

/// Run the command. `version` names the candidate (see [`version_stamp`]).
/// Returns the process exit code; stdout receives only the candidate digest.
pub fn run(
    args: &[String],
    version: &str,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    match run_inner(args, version, stdout, stderr) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(stderr, "mlcorrect-train: {message}");
            EXIT_ERROR
        }
    }
}

fn run_inner(
    args: &[String],
    version: &str,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<i32, String> {
    let options = match parse(args) {
        Ok(Parsed::Run(options)) => options,
        Ok(Parsed::Help) => {
            let _ = write!(stderr, "{USAGE}");
            return Ok(EXIT_OK);
        }
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            let _ = write!(stderr, "{USAGE}");
            return Ok(EXIT_USAGE);
        }
    };
    if options.anchor.is_empty() {
        return Err(
            "-anchor is required (the frozen gold anchor prevents model collapse)".to_owned(),
        );
    }
    let mut samples = read_samples(&options.anchor, "anchor")?;
    if !options.oracle.is_empty() {
        let oracle = read_samples(&options.oracle, "oracle")?;
        let total = oracle.len();
        let admitted: Vec<_> = oracle
            .into_iter()
            .filter(|sample| sample.admission == Admission::ReviewRequired)
            .collect();
        if admitted.len() != total {
            let _ = writeln!(
                stderr,
                "mlcorrect-train: oracle: excluded {} of {total} records with weak or unknown provenance (quarantined)",
                total - admitted.len()
            );
        }
        samples.extend(admitted);
    }

    // Warm-start from the current bundle when provided (incremental continuity).
    let base = if options.base.is_empty() {
        None
    } else {
        let (bundle, base_version) = read_base(&options.base)?;
        let _ = writeln!(
            stderr,
            "mlcorrect-train: warm-starting from {} ({base_version})",
            options.base
        );
        Some(bundle)
    };

    let report = run_gate(
        &samples,
        base.as_ref(),
        &GateConfig {
            version,
            hidden: options.hidden,
            epochs: options.epochs,
            learning_rate: options.lr,
            seed: options.seed,
            test_fraction: options.test_frac,
            max_human_fpr: options.max_human_fpr,
            min_bot_tpr: options.min_bot_tpr,
            min_cohort_humans: usize::try_from(options.min_cohort_humans.max(0))
                .unwrap_or(usize::MAX),
            required_cohorts: &options.required_cohorts,
        },
    )
    .map_err(|error| error.to_string())?;
    let _ = writeln!(
        stderr,
        "mlcorrect-train: {} train / {} test (TESSERACT time-split)",
        report.n_train, report.n_test
    );
    let _ = writeln!(
        stderr,
        "mlcorrect-train: test acc={}  botTPR={}  aggHumanFPR={}  AUT={}",
        rate(report.accuracy, 3),
        rate(report.bot_tpr, 3),
        rate(report.aggregate_human_fpr, 3),
        rate(report.aut, 3)
    );
    for (cohort, stat) in &report.cohorts {
        let _ = writeln!(
            stderr,
            "  cohort {cohort:<12} humans={:<4} FP={:<3} FPR={}",
            stat.humans,
            stat.human_fp,
            rate(stat.fpr(), 4)
        );
    }

    let Some(candidate) = report.candidate else {
        let _ = writeln!(
            stderr,
            "mlcorrect-train: PROMOTION GATES FAILED — candidate discarded, nothing written:"
        );
        for failure in &report.failures {
            let _ = writeln!(stderr, "  ✗ {failure}");
        }
        return Ok(EXIT_GATE_FAILED);
    };

    let mut raw = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut raw, formatter);
    candidate
        .serialize(&mut serializer)
        .map_err(|error| format!("encode candidate: {error}"))?;
    std::fs::write(&options.out, &raw)
        .map_err(|error| format!("write {}: {error}", options.out))?;
    let candidate_digest = digest(&raw);
    let _ = writeln!(
        stderr,
        "mlcorrect-train: GATES PASSED — wrote {} ({} bytes)",
        options.out,
        raw.len()
    );
    let _ = writeln!(
        stderr,
        "mlcorrect-train: bundle={} digest={candidate_digest}",
        candidate.version
    );
    // stdout carries the digest for the signing/staging step to consume.
    writeln!(stdout, "{candidate_digest}").map_err(|error| format!("stdout: {error}"))?;
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests;
