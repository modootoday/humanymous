//! Bounded legacy JSONL records and explicit provenance quarantine.
use crate::behavior::features::{FeatureVector, extract, schema_hash};
use crate::signals::report::BehaviorSummary;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::BufRead;

/// Legacy aggregate interchange format. Missing labels and behavior are errors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// Independent label: zero for human, one for bot.
    pub label: u8,
    /// Aggregate summary, never raw input events.
    pub behavior: BehaviorSummary,
    /// Missing timestamp remains distinct from an explicit zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    /// Accessibility cohort; empty denotes the default cohort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<String>,
    /// Untrusted provenance claim retained for external admission review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Stable failure categories contain no input body, path or parser diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    /// Configuration exceeds a ceiling or contains a zero limit.
    InvalidLimits,
    /// Dataset schema does not match the shared extractor.
    SchemaMismatch,
    /// Label is outside the binary training label domain.
    InvalidLabel,
    /// Cohort or source exceeds the UTF-8 byte budget.
    MetadataTooLong,
    /// Raw summary contains a nonfinite floating-point field.
    NonfiniteAggregate,
    /// Shared extraction produced a nonfinite float32 feature.
    NonfiniteFeatures,
    /// Input failed to read; underlying details are intentionally excluded.
    ReadFailed,
    /// A physical line or total byte budget was exceeded.
    ByteLimit,
    /// Record count budget was exceeded.
    RecordLimit,
    /// Record JSON, required fields or field types are invalid.
    InvalidJson,
    /// Distinct normalized cohort budget was exceeded.
    CohortLimit,
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "training record: {self:?}")
    }
}

impl std::error::Error for RecordError {}

/// Validated features with original aggregate provenance retained.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Shared serving feature transform output.
    pub features: FeatureVector,
    /// Original validated aggregate record.
    pub record: Record,
    /// Provenance eligibility only; not authenticated truth or gate approval.
    pub admission: Admission,
}

/// Weak or unknown labels cannot silently join an offline training batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Consistent catalog-bot or solved-Pass-human claim, still requiring review.
    ReviewRequired,
    /// Weak, missing, unknown or inconsistent provenance.
    Quarantined,
}

/// Configurable limits may only tighten the accepted safety ceilings.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Bytes per physical line, including its newline when present.
    pub line_bytes: usize,
    /// Total input bytes, including blank lines and delimiters.
    pub total_bytes: usize,
    /// Maximum parsed records, including quarantined records.
    pub records: usize,
    /// Maximum distinct normalized cohorts, including quarantine.
    pub cohorts: usize,
    /// UTF-8 bytes per source or cohort string.
    pub string_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            line_bytes: 1 << 20,
            total_bytes: 128 << 20,
            records: 100_000,
            cohorts: 64,
            string_bytes: 128,
        }
    }
}

impl Limits {
    fn validate(self) -> Result<(), RecordError> {
        let max = Self::default();
        if [
            (self.line_bytes, max.line_bytes),
            (self.total_bytes, max.total_bytes),
            (self.records, max.records),
            (self.cohorts, max.cohorts),
            (self.string_bytes, max.string_bytes),
        ]
        .iter()
        .any(|(value, ceiling)| *value == 0 || value > ceiling)
        {
            return Err(RecordError::InvalidLimits);
        }
        Ok(())
    }
}

impl Record {
    /// Validate before extracting; inference's permissive wire defaults are not labels.
    pub fn sample(self, expected_schema: &str, limits: Limits) -> Result<Sample, RecordError> {
        limits.validate()?;
        if expected_schema != schema_hash() {
            return Err(RecordError::SchemaMismatch);
        }
        if self.label > 1 {
            return Err(RecordError::InvalidLabel);
        }
        if self.cohort.as_deref().unwrap_or_default().len() > limits.string_bytes
            || self.source.as_deref().unwrap_or_default().len() > limits.string_bytes
        {
            return Err(RecordError::MetadataTooLong);
        }
        let b = &self.behavior;
        let (m, k, e) = (&b.mouse, &b.key, &b.events);
        if [
            b.duration_s,
            m.mean_curvature,
            m.velocity_std_dev,
            m.accel_entropy,
            m.straight_line_frac,
            m.max_teleport_px,
            m.mean_jerk,
            m.coalesced_ratio,
            k.mean_dwell_ms,
            k.dwell_std_dev_ms,
            k.mean_flight_ms,
            k.flight_std_dev,
            k.zero_dwell_frac,
            e.untrusted_frac,
            e.burst_var_ms,
            e.min_reaction_ms,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(RecordError::NonfiniteAggregate);
        }
        let features = extract(b);
        if features.iter().any(|value| !value.is_finite()) {
            return Err(RecordError::NonfiniteFeatures);
        }
        let admission = match (self.source.as_deref(), self.label) {
            (Some("catalog"), 1) | (Some("pass"), 0) => Admission::ReviewRequired,
            _ => Admission::Quarantined,
        };
        Ok(Sample {
            features,
            record: self,
            admission,
        })
    }
}

/// Read a bounded stream atomically: any invalid record rejects the whole result.
/// Schema is supplied by the dataset admission context because legacy rows lack it.
/// This reader neither authenticates provenance nor starts training.
pub fn read_jsonl(
    mut reader: impl BufRead,
    expected_schema: &str,
    limits: Limits,
) -> Result<Vec<Sample>, RecordError> {
    limits.validate()?;
    if expected_schema != schema_hash() {
        return Err(RecordError::SchemaMismatch);
    }
    let mut samples = Vec::new();
    let mut cohorts = BTreeSet::new();
    let mut line = Vec::new();
    let mut total = 0usize;
    loop {
        let available = reader.fill_buf().map_err(|_| RecordError::ReadFailed)?;
        if available.is_empty() {
            if !line.is_empty() {
                push_line(&line, expected_schema, limits, &mut samples, &mut cohorts)?;
            }
            return Ok(samples);
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if count > limits.total_bytes - total || count > limits.line_bytes - line.len() {
            return Err(RecordError::ByteLimit);
        }
        line.extend_from_slice(&available[..count]);
        total += count;
        reader.consume(count);
        if newline.is_some() {
            push_line(&line, expected_schema, limits, &mut samples, &mut cohorts)?;
            line.clear();
        }
    }
}

fn push_line(
    line: &[u8],
    schema: &str,
    limits: Limits,
    samples: &mut Vec<Sample>,
    cohorts: &mut BTreeSet<String>,
) -> Result<(), RecordError> {
    if line.iter().all(u8::is_ascii_whitespace) {
        return Ok(());
    }
    if samples.len() >= limits.records {
        return Err(RecordError::RecordLimit);
    }
    let record: Record = serde_json::from_slice(line).map_err(|_| RecordError::InvalidJson)?;
    let sample = record.sample(schema, limits)?;
    let cohort = sample
        .record
        .cohort
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("default");
    if !cohorts.contains(cohort) && cohorts.len() >= limits.cohorts {
        return Err(RecordError::CohortLimit);
    }
    cohorts.insert(cohort.to_owned());
    samples.push(sample);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{BufReader, Cursor};
    use std::path::PathBuf;

    fn record() -> Record {
        Record {
            label: 0,
            behavior: BehaviorSummary::default(),
            ts: Some(42),
            cohort: Some("keyboard".into()),
            source: Some("pass".into()),
        }
    }

    fn bytes(records: &[Record]) -> Vec<u8> {
        let mut data = Vec::new();
        for record in records {
            serde_json::to_writer(&mut data, record).unwrap();
            data.push(b'\n');
        }
        data
    }

    struct OwnedInput(PathBuf);

    impl OwnedInput {
        fn create(data: &[u8]) -> Self {
            use std::io::Write;
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "humanymous-training-record-{}-{sequence}.jsonl",
                std::process::id()
            ));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            file.write_all(data).unwrap();
            file.flush().unwrap();
            Self(path)
        }

        fn read(&self, limits: Limits) -> Result<Vec<Sample>, RecordError> {
            let file = File::open(&self.0).unwrap();
            read_jsonl(BufReader::with_capacity(7, file), &schema_hash(), limits)
        }
    }

    impl Drop for OwnedInput {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).unwrap();
        }
    }

    #[test]
    fn actual_file_reader_accepts_rejects_and_preserves_caller_batch_on_error() {
        let first = record();
        let mut second = first.clone();
        second.cohort = Some("other".into());
        let data = bytes(&[first.clone(), second]);
        let input = OwnedInput::create(&data);
        let accepted = input.read(Limits::default()).unwrap();
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].features, extract(&first.behavior));
        for (limits, expected) in [
            (
                Limits {
                    total_bytes: data.len() - 1,
                    ..Limits::default()
                },
                RecordError::ByteLimit,
            ),
            (
                Limits {
                    line_bytes: 8,
                    ..Limits::default()
                },
                RecordError::ByteLimit,
            ),
            (
                Limits {
                    records: 1,
                    ..Limits::default()
                },
                RecordError::RecordLimit,
            ),
            (
                Limits {
                    cohorts: 1,
                    ..Limits::default()
                },
                RecordError::CohortLimit,
            ),
        ] {
            assert_eq!(input.read(limits).unwrap_err(), expected);
        }
        let mut partial = bytes(&[first]);
        partial.extend_from_slice(b"invalid-sensitive-row\n");
        let invalid = OwnedInput::create(&partial);
        let mut caller_batch = accepted;
        let result = invalid.read(Limits::default());
        assert_eq!(result.as_ref().unwrap_err(), &RecordError::InvalidJson);
        if let Ok(replacement) = result {
            caller_batch = replacement;
        }
        assert_eq!(caller_batch.len(), 2);
        assert_eq!(caller_batch[0].record.ts, Some(42));
    }

    #[test]
    fn round_trip_reuses_features_and_retains_quarantined_provenance() {
        let human = record();
        let mut weak = human.clone();
        weak.label = 1;
        weak.source = Some("challenge".into());
        let data = bytes(&[human.clone(), weak.clone()]);
        let got = read_jsonl(Cursor::new(data), &schema_hash(), Limits::default()).unwrap();
        assert_eq!(got[0].record, human);
        assert_eq!(got[0].features, extract(&human.behavior));
        assert_eq!(got[0].admission, Admission::ReviewRequired);
        assert_eq!(got[1].record, weak);
        assert_eq!(got[1].admission, Admission::Quarantined);
        for (source, label, expected) in [
            ("catalog", 1, Admission::ReviewRequired),
            ("catalog", 0, Admission::Quarantined),
            ("pass", 1, Admission::Quarantined),
            ("", 0, Admission::Quarantined),
            ("model", 1, Admission::Quarantined),
        ] {
            let mut row = record();
            row.source = Some(source.into());
            row.label = label;
            assert_eq!(
                row.sample(&schema_hash(), Limits::default())
                    .unwrap()
                    .admission,
                expected
            );
        }
    }

    #[test]
    fn rejects_invalid_labels_schema_nonfinite_and_overflow_features() {
        let limits = Limits::default();
        let mut row = record();
        row.label = 2;
        assert!(row.sample(&schema_hash(), limits).is_err());
        assert!(record().sample("stale", limits).is_err());
        let mut row = record();
        row.behavior.duration_s = f64::NAN;
        assert!(row.sample(&schema_hash(), limits).is_err());
        let mut row = record();
        row.behavior.mouse.mean_curvature = f64::MAX;
        assert!(row.sample(&schema_hash(), limits).is_err());
        for data in [
            b"{}".as_slice(),
            b"{\"label\":0}",
            b"{\"label\":-1,\"behavior\":{}}",
            b"not json",
        ] {
            assert!(read_jsonl(Cursor::new(data), &schema_hash(), limits).is_err());
        }
    }

    #[test]
    fn stream_limits_cover_small_buffers_exact_edges_and_blank_lines() {
        let data = bytes(&[record()]);
        let limits = Limits {
            line_bytes: data.len(),
            total_bytes: data.len(),
            ..Limits::default()
        };
        let tiny = BufReader::with_capacity(3, Cursor::new(&data));
        assert_eq!(read_jsonl(tiny, &schema_hash(), limits).unwrap().len(), 1);
        for tighter in [
            Limits {
                line_bytes: data.len() - 1,
                ..limits
            },
            Limits {
                total_bytes: data.len() - 1,
                ..limits
            },
            Limits {
                string_bytes: 1,
                ..limits
            },
        ] {
            assert!(read_jsonl(Cursor::new(&data), &schema_hash(), tighter).is_err());
        }
        let mut padded = data.clone();
        padded.push(b'\n');
        assert!(read_jsonl(Cursor::new(padded), &schema_hash(), limits).is_err());
        let mut no_newline = data;
        no_newline.pop();
        assert_eq!(
            read_jsonl(Cursor::new(no_newline), &schema_hash(), limits)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn record_cohort_and_configuration_caps_fail_closed() {
        let a = record();
        let mut b = a.clone();
        b.cohort = Some("other".into());
        let data = bytes(&[a, b]);
        for limits in [
            Limits {
                records: 1,
                ..Limits::default()
            },
            Limits {
                cohorts: 1,
                ..Limits::default()
            },
            Limits {
                records: 100_001,
                ..Limits::default()
            },
            Limits {
                cohorts: 0,
                ..Limits::default()
            },
        ] {
            assert!(read_jsonl(Cursor::new(&data), &schema_hash(), limits).is_err());
        }
        let mut row = record();
        row.source = Some("é".repeat(65));
        assert!(row.sample(&schema_hash(), Limits::default()).is_err());
    }

    #[test]
    fn required_fields_never_default_an_unlabeled_record_to_human() {
        for data in [
            b"{\"behavior\":{}}".as_slice(),
            b"{\"label\":0}",
            b"{\"label\":null,\"behavior\":{}}",
            b"{\"label\":0,\"behavior\":null}",
        ] {
            assert_eq!(
                read_jsonl(Cursor::new(data), &schema_hash(), Limits::default()).unwrap_err(),
                RecordError::InvalidJson,
            );
        }
        let explicit = read_jsonl(
            Cursor::new(b"{\"label\":0,\"behavior\":{}}"),
            &schema_hash(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(explicit[0].record.label, 0);
        assert_eq!(explicit[0].admission, Admission::Quarantined);
    }

    #[test]
    fn optional_metadata_preserves_missing_versus_explicit_zero_and_empty() {
        let missing: Record = serde_json::from_slice(b"{\"label\":0,\"behavior\":{}}").unwrap();
        let explicit: Record = serde_json::from_slice(
            b"{\"label\":0,\"behavior\":{},\"ts\":0,\"cohort\":\"\",\"source\":\"\"}",
        )
        .unwrap();
        assert_eq!(missing.ts, None);
        assert_eq!(explicit.ts, Some(0));
        assert_eq!(missing.cohort, None);
        assert_eq!(explicit.cohort.as_deref(), Some(""));
        assert_eq!(missing.source, None);
        let round_trip: Record =
            serde_json::from_slice(&serde_json::to_vec(&missing).unwrap()).unwrap();
        assert_eq!(round_trip, missing);
        assert_eq!(
            missing
                .sample(&schema_hash(), Limits::default())
                .unwrap()
                .admission,
            Admission::Quarantined
        );
    }

    #[test]
    fn negative_values_keep_shared_clamping_and_errors_disclose_no_input() {
        let mut row = record();
        row.behavior.mouse.samples = -5;
        row.behavior.key.mean_dwell_ms = -10.0;
        row.behavior.mouse.mean_curvature = -0.25;
        let expected = extract(&row.behavior);
        let sample = row.sample(&schema_hash(), Limits::default()).unwrap();
        assert_eq!(sample.features, expected);
        assert_eq!(sample.features[0], 0.0);
        assert_eq!(sample.features[1], -0.25);
        let error = read_jsonl(
            Cursor::new(b"sensitive-input-body"),
            &schema_hash(),
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error, RecordError::InvalidJson);
        assert!(!error.to_string().contains("sensitive-input-body"));
    }
}
