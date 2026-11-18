//! Process shell of the bootstrap trainer (legacy `cmd/ml-train/main.go` at
//! baseline 981d25c; SoT model-training-correction, row "both training
//! commands"). The `ml-train` binary is a thin wrapper over [`run`].
//!
//! It trains the small behavioral MLP into a JSON bundle, either from real
//! labeled traces (`-data`, JSONL `{label, behavior}`) or from the SYNTHETIC
//! bootstrap generator, whose human and bot behavior summaries are drawn from
//! distributions grounded in the red-team catalog's bot values and the scoring
//! tests' human values. Synthetic data exercises the pipeline; it is not a
//! substitute for real trace collection.
//!
//! ```text
//! ml-train -gen 20000 -out model.json          # bootstrap on synthetic grounded data
//! ml-train -data labeled.jsonl -out model.json # real labeled traces
//! ```
//!
//! Writing a bundle is not activation: nothing here stages, signs, promotes or
//! serves a model, and no production threshold or verdict band is touched.
//!
//! One seeded Go-compatible stream ([`GoRng`]) drives generation and then the
//! pre-split shuffle, exactly as Go's single `rand.New(rand.NewSource(seed))`;
//! the trainer seeds its own stream from the same seed. With the same flags the
//! trained weights match the Go oracle bit for bit (see the tests).
//!
//! Deliberate differences from Go:
//! - Generation and the pre-split shuffle are bounded: at most
//!   [`MAX_SAMPLES`] samples and [`MAX_DRAWS`] primitive draws; exceeding
//!   either is an error, never a silently truncated dataset. Training uses the
//!   explicit legacy bootstrap work profile, which admits the legacy defaults.
//! - `-data` uses the shared bounded JSONL reader: any invalid record or an
//!   exceeded budget rejects the run (Go rejected only malformed JSON).
//! - Positional arguments are a usage error (Go stopped parsing at the first
//!   one and silently ignored the rest); numeric flags accept decimal only.
//! - Absent class support prints `n/a` instead of Go's `0.000`.
//!
//! The bundle is written in Go's `json.MarshalIndent(b, "", " ")` byte layout
//! ([`crate::model::go_json`]), so for the same flags and date the file and its
//! SHA-256 digest are identical to the Go oracle's.
use super::evaluate::{LabeledPrediction, evaluate};
use super::record::{Limits, read_jsonl};
use super::rng::{GoRng, MAX_DRAWS, MAX_SAMPLES, RngError};
use super::sgd::{TrainConfig, TrainLimits, TrainingRow, train};
use super::standardize::standardize;
use crate::behavior::features::{FeatureVector, extract, schema_hash};
use crate::model::go_json;
use crate::model::inference::{Mlp, MlpBundle, Predictor};
use crate::signals::report::{BehaviorSummary, EventFeatures, KeyFeatures, MouseFeatures};
use std::fs::File;
use std::io::{BufReader, Write};

/// Bundle written: success.
pub const EXIT_OK: i32 = 0;
/// Input, precondition, training or output failure.
pub const EXIT_ERROR: i32 = 1;
/// Command-line usage error.
pub const EXIT_USAGE: i32 = 2;

/// Legacy default synthetic sample count when `-gen` is not positive.
pub const DEFAULT_GENERATED: usize = 20_000;

/// Legacy minimum number of samples before training.
pub const MIN_SAMPLES: usize = 100;

const USAGE: &str = "\
Usage of ml-train:
  -data string
    \tpath to labeled JSONL {label:0|1, behavior:{...}} (real traces)
  -epochs int
    \ttraining epochs (default 60)
  -gen int
    \tgenerate N synthetic bootstrap samples instead of reading -data
  -hidden int
    \thidden units (default 16)
  -lr float
    \tlearning rate (default 0.05)
  -out string
    \toutput bundle path (default \"model.json\")
  -seed int
    \trng seed (default 1)
";

/// Parsed command line with the legacy defaults.
#[derive(Debug, Clone, PartialEq)]
struct Options {
    gen_count: i64,
    data: String,
    out: String,
    hidden: i64,
    epochs: i64,
    lr: f64,
    seed: i64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            gen_count: 0,
            data: String::new(),
            out: "model.json".to_owned(),
            hidden: 16,
            epochs: 60,
            lr: 0.05,
            seed: 1,
        }
    }
}

enum Parsed {
    Run(Options),
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
        let flag = arg.strip_prefix("--").unwrap_or_else(|| &arg[1..]);
        let (name, inline) = match flag.split_once('=') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (flag, None),
        };
        if name == "h" || name == "help" {
            return Ok(Parsed::Help);
        }
        const KNOWN: [&str; 7] = ["gen", "data", "out", "hidden", "epochs", "lr", "seed"];
        if !KNOWN.contains(&name) {
            return Err(format!("flag provided but not defined: -{name}"));
        }
        let value = match inline {
            Some(value) => value,
            None => rest
                .next()
                .cloned()
                .ok_or_else(|| format!("flag needs an argument: -{name}"))?,
        };
        match name {
            "gen" => options.gen_count = number(name, &value)?,
            "data" => options.data = value,
            "out" => options.out = value,
            "hidden" => options.hidden = number(name, &value)?,
            "epochs" => options.epochs = number(name, &value)?,
            "lr" => options.lr = number(name, &value)?,
            _ => options.seed = number(name, &value)?,
        }
    }
    Ok(Parsed::Run(options))
}

/// Legacy bundle version: `mlp-bootstrap-YYYYMMDD-h<hidden>` in UTC, with the
/// `-hidden` flag value as given (Go formatted the raw flag).
pub fn version_stamp(now: time::OffsetDateTime, hidden: i64) -> String {
    let now = now.to_offset(time::UtcOffset::UTC);
    format!(
        "mlp-bootstrap-{:04}{:02}{:02}-h{hidden}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

/// One labeled training example: extracted features and 0 (human) / 1 (bot).
pub type LabeledFeatures = (FeatureVector, u8);

/// Non-negative Gaussian: `mean + NormFloat64() * sd`, clamped at zero.
fn nz(rng: &mut GoRng, mean: f64, sd: f64) -> Result<f64, RngError> {
    let value = mean + rng.norm_float64()? * sd;
    Ok(if value < 0.0 { 0.0 } else { value })
}

/// Go `int(float64)` of a non-negative finite draw.
fn count(value: f64) -> i64 {
    value as i64
}

/// Legacy `humanBehavior`. Draws happen in Go's composite-literal order.
fn human_behavior(rng: &mut GoRng) -> Result<BehaviorSummary, RngError> {
    // Struct-literal fields evaluate in source order, matching Go.
    Ok(BehaviorSummary {
        mouse: MouseFeatures {
            samples: count(nz(rng, 55.0, 18.0)?) + 8,
            mean_curvature: nz(rng, 0.35, 0.12)?,
            velocity_std_dev: nz(rng, 0.6, 0.18)?,
            accel_entropy: nz(rng, 2.1, 0.3)?,
            straight_line_frac: nz(rng, 0.18, 0.08)?,
            pause_count: count(nz(rng, 3.0, 2.0)?),
            max_teleport_px: nz(rng, 45.0, 20.0)?,
            mean_jerk: nz(rng, 0.4, 0.15)?,
            coalesced_ratio: nz(rng, 3.5, 1.0)? + 1.5,
        },
        key: KeyFeatures {
            keystrokes: count(nz(rng, 16.0, 8.0)?),
            mean_dwell_ms: nz(rng, 95.0, 20.0)?,
            dwell_std_dev_ms: nz(rng, 28.0, 10.0)? + 6.0,
            mean_flight_ms: nz(rng, 140.0, 35.0)?,
            flight_std_dev: nz(rng, 35.0, 12.0)? + 8.0,
            ..KeyFeatures::default()
        },
        events: EventFeatures {
            total_events: count(nz(rng, 60.0, 20.0)?) + 5,
            click_count: count(nz(rng, 2.0, 1.0)?),
            burst_var_ms: nz(rng, 400.0, 150.0)?,
            min_reaction_ms: nz(rng, 280.0, 90.0)? + 120.0,
            ..EventFeatures::default()
        },
        duration_s: nz(rng, 8.0, 3.0)? + 1.0,
    })
}

/// Legacy `botBehavior`: one of four automation archetypes chosen by `Intn(4)`.
fn bot_behavior(rng: &mut GoRng) -> Result<BehaviorSummary, RngError> {
    let mut b = BehaviorSummary::default();
    match rng.intn(4)? {
        // Scripted-smooth (humanizer): straight, low variance, synthetic coalesced.
        0 => {
            b.mouse = MouseFeatures {
                samples: count(nz(rng, 40.0, 12.0)?) + 5,
                straight_line_frac: nz(rng, 0.9, 0.06)?,
                velocity_std_dev: nz(rng, 0.05, 0.04)?,
                mean_curvature: nz(rng, 0.03, 0.02)?,
                mean_jerk: nz(rng, 0.05, 0.03)?,
                accel_entropy: nz(rng, 0.6, 0.3)?,
                coalesced_ratio: nz(rng, 1.0, 0.1)?,
                max_teleport_px: nz(rng, 60.0, 30.0)?,
                ..MouseFeatures::default()
            };
            b.key = KeyFeatures {
                keystrokes: count(nz(rng, 12.0, 6.0)?),
                mean_dwell_ms: nz(rng, 90.0, 15.0)?,
                dwell_std_dev_ms: nz(rng, 2.0, 2.0)?,
                mean_flight_ms: nz(rng, 40.0, 10.0)?,
                flight_std_dev: nz(rng, 2.0, 2.0)?,
                zero_dwell_frac: nz(rng, 0.6, 0.2)?,
                ..KeyFeatures::default()
            };
            b.events = EventFeatures {
                total_events: count(nz(rng, 50.0, 15.0)?) + 5,
                click_count: count(nz(rng, 2.0, 1.0)?),
                ..EventFeatures::default()
            };
            b.duration_s = nz(rng, 6.0, 2.0)? + 1.0;
        }
        // No interaction (headless).
        1 => {
            b.events.total_events = count(nz(rng, 2.0, 1.0)?);
            b.duration_s = nz(rng, 5.0, 2.0)? + 2.0;
        }
        // Untrusted / CDP-injected events.
        2 => {
            b.mouse = MouseFeatures {
                samples: count(nz(rng, 20.0, 10.0)?),
                coalesced_ratio: nz(rng, 1.0, 0.05)?,
                straight_line_frac: nz(rng, 0.7, 0.15)?,
                ..MouseFeatures::default()
            };
            b.events = EventFeatures {
                total_events: count(nz(rng, 40.0, 15.0)?) + 5,
                untrusted_frac: nz(rng, 0.8, 0.2)?,
                no_move_clicks: count(nz(rng, 3.0, 2.0)?),
                synthetic_flags: count(nz(rng, 4.0, 2.0)?),
                click_count: count(nz(rng, 3.0, 2.0)?),
                ..EventFeatures::default()
            };
            b.duration_s = nz(rng, 6.0, 2.0)? + 1.0;
        }
        // AI-agent cadence: think-gaps plus fast low-variance bursts.
        _ => {
            b.mouse = MouseFeatures {
                samples: count(nz(rng, 25.0, 10.0)?),
                coalesced_ratio: nz(rng, 1.0, 0.1)?,
                max_teleport_px: nz(rng, 300.0, 120.0)?,
                straight_line_frac: nz(rng, 0.85, 0.1)?,
                ..MouseFeatures::default()
            };
            b.events = EventFeatures {
                total_events: count(nz(rng, 30.0, 12.0)?) + 5,
                long_gap_count: count(nz(rng, 5.0, 2.0)?) + 1,
                burst_var_ms: nz(rng, 8.0, 5.0)?,
                min_reaction_ms: nz(rng, 30.0, 15.0)?,
                click_count: count(nz(rng, 4.0, 2.0)?),
                ..EventFeatures::default()
            };
            b.duration_s = nz(rng, 12.0, 4.0)? + 2.0;
        }
    }
    Ok(b)
}

/// Legacy `genSynthetic`: even indices are humans (label 0), odd are bots.
pub fn generate_synthetic(rng: &mut GoRng, n: usize) -> Result<Vec<LabeledFeatures>, RngError> {
    if n > MAX_SAMPLES {
        return Err(RngError::SampleLimit);
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i % 2 == 0 {
            out.push((extract(&human_behavior(rng)?), 0));
        } else {
            out.push((extract(&bot_behavior(rng)?), 1));
        }
    }
    Ok(out)
}

fn read_data(path: &str) -> Result<Vec<LabeledFeatures>, String> {
    let file = File::open(path).map_err(|error| format!("open {path}: {error}"))?;
    let samples = read_jsonl(BufReader::new(file), &schema_hash(), Limits::default())
        .map_err(|error| format!("bad JSONL input: {error}"))?;
    Ok(samples
        .into_iter()
        .map(|sample| (sample.features, sample.record.label))
        .collect())
}

fn rate(value: Option<f32>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |v| format!("{v:.3}"))
}

fn rng_message(error: RngError) -> String {
    match error {
        RngError::SampleLimit => format!("more than {MAX_SAMPLES} samples"),
        RngError::DrawLimit => {
            format!("random stream exceeded {MAX_DRAWS} draws; use fewer samples")
        }
        other => other.to_string(),
    }
}

/// Train a bundle from labeled examples: seeded pre-split shuffle on `rng`,
/// first fifth held out for validation, standardize and train on the rest.
/// Returns the bundle and the validation report line (without the prefix).
pub fn train_bundle(
    rng: &mut GoRng,
    samples: Vec<LabeledFeatures>,
    hidden: i64,
    epochs: i64,
    learning_rate: f64,
    seed: i64,
    version: &str,
) -> Result<(MlpBundle, String), String> {
    if samples.len() < MIN_SAMPLES {
        return Err(format!(
            "need >={MIN_SAMPLES} samples, got {}",
            samples.len()
        ));
    }
    if !learning_rate.is_finite() {
        return Err("learning rate must be finite".to_owned());
    }
    let mut order: Vec<usize> = (0..samples.len()).collect();
    rng.shuffle(&mut order).map_err(rng_message)?;
    let shuffled: Vec<LabeledFeatures> = order.iter().map(|&index| samples[index]).collect();
    let n_val = shuffled.len() / 5;
    let (val, train_set) = shuffled.split_at(n_val);

    let schema = schema_hash();
    let vectors: Vec<_> = train_set.iter().map(|(features, _)| *features).collect();
    let statistics =
        standardize(&vectors, &schema, MAX_SAMPLES).map_err(|error| error.to_string())?;
    let rows: Vec<_> = train_set
        .iter()
        .map(|(features, label)| TrainingRow {
            features,
            label: *label,
        })
        .collect();
    let bundle = train(
        TrainConfig {
            hidden,
            epochs,
            learning_rate: learning_rate as f32,
            seed,
            version,
        },
        &rows,
        &statistics,
        &schema,
        None,
        TrainLimits::legacy_bootstrap(),
    )
    .map_err(|error| error.to_string())?
    .candidate;

    // Evaluate through the existing inference boundary; no forked forward pass.
    let model = Mlp::from_bundle(bundle.clone())
        .map_err(|_| "trained bundle failed validation".to_owned())?;
    let pairs: Vec<_> = val
        .iter()
        .map(|(features, label)| LabeledPrediction {
            label: *label,
            prediction: model.predict(features),
        })
        .collect();
    let counts = evaluate(&pairs, MAX_SAMPLES).map_err(|error| error.to_string())?;
    let report = format!(
        "val acc={}  bot TPR={}  human FPR={}  (bundle {}, schema {})",
        rate(counts.accuracy()),
        rate(counts.bot_tpr()),
        rate(counts.human_fpr()),
        bundle.version,
        bundle.schema_hash
    );
    Ok((bundle, report))
}

/// Serialize a bundle exactly like Go's `json.MarshalIndent(b, "", " ")`.
pub fn encode_bundle(bundle: &MlpBundle) -> Result<Vec<u8>, String> {
    go_json::to_vec_indent(bundle, b" ").map_err(|error| format!("encode bundle: {error}"))
}

/// Run the command. `now` stamps the bundle version (see [`version_stamp`]).
/// Returns the process exit code; diagnostics go to stderr, stdout is unused.
pub fn run(args: &[String], now: time::OffsetDateTime, stderr: &mut impl Write) -> i32 {
    match run_inner(args, now, stderr) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(stderr, "ml-train: {message}");
            EXIT_ERROR
        }
    }
}

fn run_inner(
    args: &[String],
    now: time::OffsetDateTime,
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

    let mut rng = GoRng::new(options.seed, MAX_DRAWS).map_err(rng_message)?;
    let samples = if options.data.is_empty() {
        let n = if options.gen_count <= 0 {
            DEFAULT_GENERATED
        } else {
            usize::try_from(options.gen_count).unwrap_or(usize::MAX)
        };
        let samples = generate_synthetic(&mut rng, n).map_err(rng_message)?;
        let _ = writeln!(
            stderr,
            "ml-train: generated {} SYNTHETIC bootstrap samples (not production data)",
            samples.len()
        );
        samples
    } else {
        let samples = read_data(&options.data)?;
        let _ = writeln!(
            stderr,
            "ml-train: loaded {} labeled samples from {}",
            samples.len(),
            options.data
        );
        samples
    };

    let version = version_stamp(now, options.hidden);
    let (bundle, report) = train_bundle(
        &mut rng,
        samples,
        options.hidden,
        options.epochs,
        options.lr,
        options.seed,
        &version,
    )?;
    let _ = writeln!(stderr, "ml-train: {report}");

    let raw = encode_bundle(&bundle)?;
    std::fs::write(&options.out, &raw)
        .map_err(|error| format!("write {}: {error}", options.out))?;
    let _ = writeln!(
        stderr,
        "ml-train: wrote {} ({} bytes)",
        options.out,
        raw.len()
    );
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests;
