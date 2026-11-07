//! Bounded migration of the frozen Go observation report, never product evidence.
mod aggregate;
mod html;
#[cfg(test)]
mod tests;

pub use aggregate::{Accounting, Notice, ProfileSummary, Summary, aggregate};
pub use html::render;
use humanymous_test_support::json::decode_strict_json;
use serde_json::{Map, Value};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Maximum imported JSON bytes before parsing.
pub const INPUT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum scheduled legacy rows, including incomplete observations.
pub const MAX_ROWS: usize = 50_000;
/// Maximum bytes in each decoded field or caption.
pub const FIELD_BYTES: usize = 4096;
/// Maximum top-signal occurrences in a record, including duplicates.
pub const MAX_TOP: usize = 128;
/// Maximum rendered artifact size.
pub const OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Input-independent failures; imported diagnostics are never echoed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Input exceeded the byte cap.
    InputTooLarge,
    /// JSON syntax or decoded-key uniqueness failed.
    Json,
    /// A known legacy field has the wrong type.
    Record,
    /// A row, field, caption, path or vector exceeded its bound.
    Limit,
    /// Finite inputs overflowed the legacy risk accumulator.
    RiskOverflow,
    /// Rendering exceeded its byte cap.
    OutputTooLarge,
    /// Reading, staging or publishing failed.
    Io,
    /// Bounded temporary-name attempts all collided.
    Collision,
    /// CLI arguments were incomplete, duplicate or unknown.
    Arguments,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputTooLarge => "legacy input byte limit",
            Self::Json => "invalid legacy JSON or duplicate decoded key",
            Self::Record => "invalid legacy record field type",
            Self::Limit => "legacy report field or count limit",
            Self::RiskOverflow => "legacy risk accumulation overflow",
            Self::OutputTooLarge => "legacy rendered output limit",
            Self::Io => "legacy report filesystem operation failed",
            Self::Collision => "legacy report temporary name limit",
            Self::Arguments => "invalid report arguments",
        })
    }
}
impl std::error::Error for Error {}

/// Frozen results.json fields. Deliberately no Debug/Serialize for raw diagnostics.
#[derive(Clone, Default)]
pub struct Record {
    /// Declared profile name.
    pub profile: String,
    /// Declared grouping label, not verified participant identity.
    pub label: String,
    /// Legacy run ordinal; it does not determine aggregate counts.
    pub run: i64,
    /// Observed legacy verdict spelling.
    pub verdict: String,
    /// Imported finite risk, never recomputed by this tool.
    pub risk_score: f64,
    /// Imported rule ID.
    pub hard_rule_fired: String,
    /// Legacy outcome field; metrics derive from label/verdict instead.
    pub outcome: String,
    /// Signal occurrences, preserving repeated IDs.
    pub top: Vec<String>,
    /// Legacy skip takes precedence over metric eligibility.
    pub skipped: bool,
    /// Free-form diagnostic retained only for classification, never rendering.
    pub reason: String,
    /// Free-form error retained only for classification, never rendering.
    pub error: String,
}
impl Record {
    fn validate(&self) -> Result<(), Error> {
        if [
            &self.profile,
            &self.label,
            &self.verdict,
            &self.hard_rule_fired,
            &self.outcome,
            &self.reason,
            &self.error,
        ]
        .into_iter()
        .chain(self.top.iter())
        .any(|s| s.len() > FIELD_BYTES)
            || self.top.len() > MAX_TOP
        {
            return Err(Error::Limit);
        }
        if !self.risk_score.is_finite() {
            return Err(Error::Record);
        }
        Ok(())
    }
}

fn string(fields: &mut Map<String, Value>, name: &str) -> Result<String, Error> {
    match fields.remove(name) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(value)) if value.len() <= FIELD_BYTES => Ok(value),
        Some(Value::String(_)) => Err(Error::Limit),
        _ => Err(Error::Record),
    }
}
fn record(value: Value) -> Result<Record, Error> {
    let mut fields = match value {
        Value::Null => return Ok(Record::default()),
        Value::Object(fields) => fields,
        _ => return Err(Error::Record),
    };
    let run = match fields.remove("run") {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) => n.as_i64().ok_or(Error::Record)?,
        _ => return Err(Error::Record),
    };
    let risk_score = match fields.remove("riskScore") {
        None | Some(Value::Null) => 0.0,
        Some(Value::Number(n)) => n.as_f64().ok_or(Error::Record)?,
        _ => return Err(Error::Record),
    };
    let skipped = match fields.remove("skipped") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => value,
        _ => return Err(Error::Record),
    };
    let top = match fields.remove("top") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) if items.len() <= MAX_TOP => items
            .into_iter()
            .map(|v| match v {
                Value::Null => Ok(String::new()),
                Value::String(s) if s.len() <= FIELD_BYTES => Ok(s),
                Value::String(_) => Err(Error::Limit),
                _ => Err(Error::Record),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::Array(_)) => return Err(Error::Limit),
        _ => return Err(Error::Record),
    };
    let result = Record {
        profile: string(&mut fields, "profile")?,
        label: string(&mut fields, "label")?,
        run,
        verdict: string(&mut fields, "verdict")?,
        risk_score,
        hard_rule_fired: string(&mut fields, "hardRuleFired")?,
        outcome: string(&mut fields, "outcome")?,
        top,
        skipped,
        reason: string(&mut fields, "reason")?,
        error: string(&mut fields, "error")?,
    };
    // Go ignores unknown properties; do not retain or render their contents.
    result.validate()?;
    Ok(result)
}

/// Decode the legacy array, preserving Go defaults but rejecting key ambiguity.
pub fn decode(bytes: &[u8]) -> Result<Vec<Record>, Error> {
    if bytes.len() > INPUT_BYTES {
        return Err(Error::InputTooLarge);
    }
    let value = decode_strict_json(bytes, INPUT_BYTES).map_err(|_| Error::Json)?;
    bounded_value(&value)?;
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(rows) if rows.len() <= MAX_ROWS => rows.into_iter().map(record).collect(),
        Value::Array(_) => Err(Error::Limit),
        _ => Err(Error::Record),
    }
}

fn bounded_value(value: &Value) -> Result<(), Error> {
    match value {
        Value::String(s) if s.len() > FIELD_BYTES => return Err(Error::Limit),
        Value::Array(items) => {
            if items.len() > MAX_ROWS {
                return Err(Error::Limit);
            }
            for item in items {
                bounded_value(item)?;
            }
        }
        Value::Object(fields) => {
            if fields.len() > MAX_ROWS || fields.keys().any(|key| key.len() > FIELD_BYTES) {
                return Err(Error::Limit);
            }
            for item in fields.values() {
                bounded_value(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn path_bound(path: &Path) -> Result<(), Error> {
    if path.as_os_str().is_empty() || path.as_os_str().len() > FIELD_BYTES {
        return Err(Error::Limit);
    }
    Ok(())
}

/// Read at most the byte cap plus one; a changing file cannot bypass the bound.
pub fn load(path: &Path) -> Result<Vec<Record>, Error> {
    path_bound(path)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| Error::Io)?
        .take(INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    decode(&bytes)
}

struct Pending {
    path: PathBuf,
    committed: bool,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Stage a bounded artifact before native rename. Never delete the destination.
/// Windows replacement/share-denial and filesystem durability need native tests.
pub fn write_atomic(path: &Path, html: &str) -> Result<(), Error> {
    path_bound(path)?;
    if html.len() > OUTPUT_BYTES {
        return Err(Error::OutputTooLarge);
    }
    if path.file_name().is_none() {
        return Err(Error::Io);
    }
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => return Err(Error::Io),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::Io),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for _ in 0..32 {
        let serial = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".humanymous-report-{}-{serial}.tmp",
            std::process::id()
        ));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(Error::Io),
        };
        // A collision never constructs this guard and is never cleaned up by us.
        let mut pending = Pending {
            path: temp,
            committed: false,
        };
        let staged = file
            .write_all(html.as_bytes())
            .and_then(|()| file.sync_all());
        drop(file);
        staged.map_err(|_| Error::Io)?;
        fs::rename(&pending.path, path).map_err(|_| Error::Io)?;
        pending.committed = true;
        return Ok(());
    }
    Err(Error::Collision)
}

/// Generate an artifact; success means publication only, never product pass.
pub fn generate(input: &Path, output: &Path, note: &str) -> Result<Summary, Error> {
    path_bound(output)?;
    let summary = aggregate(&load(input)?)?;
    let html = render(&summary, note)?;
    write_atomic(output, &html)?;
    Ok(summary)
}
