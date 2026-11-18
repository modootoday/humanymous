//! Pure versioned snapshot format written by the operator updater.
//!
//! One JSON document, `humanymous-ip-intel/1`, with per-source provenance and
//! canonical prefixes. [`parse`] and [`encode`] share one validation, so a file
//! the updater can write is exactly a file the runtime accepts. Every bound is
//! checked before the content is used; a refusal changes no caller state.
use super::{IntelCategory, prefix_set::Prefix};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Exact format identifier of this version.
pub const FORMAT: &str = "humanymous-ip-intel/1";
/// Largest accepted snapshot file.
pub const MAX_SNAPSHOT_BYTES: usize = 64 << 20;
/// Largest number of source records.
pub const MAX_SOURCES: usize = 64;
/// Largest total number of prefixes across all sources.
pub const MAX_PREFIXES: usize = 2_000_000;
/// Largest length of a free-text field (licence, detail, URL, path).
pub const MAX_TEXT: usize = 512;
/// Tolerated clock skew for `generated_at` and `fetched_at` in the future.
pub const MAX_FUTURE_SKEW_S: u64 = 300;

/// Static refusal codes; no file content is interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    /// File exceeds [`MAX_SNAPSHOT_BYTES`].
    TooLarge,
    /// Not valid JSON for this schema, unknown field, or bad prefix text.
    Malformed,
    /// `format` is not [`FORMAT`].
    Format,
    /// Source or prefix count exceeds the bounds, or no source is present.
    Bounds,
    /// Source name is not `[a-z0-9-]{1,64}` or is duplicated.
    SourceName,
    /// A free-text field is empty where required, too long or has controls.
    Text,
    /// Status and prefixes disagree (ok needs prefixes; others need none).
    Status,
    /// Git commit is not 40 or 64 lowercase hex characters.
    Commit,
    /// A timestamp is in the future beyond the tolerated skew, or a fetch is
    /// later than generation.
    Timestamp,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "snapshot_too_large",
            Self::Malformed => "snapshot_malformed",
            Self::Format => "snapshot_format",
            Self::Bounds => "snapshot_bounds",
            Self::SourceName => "snapshot_source_name",
            Self::Text => "snapshot_text",
            Self::Status => "snapshot_status",
            Self::Commit => "snapshot_commit",
            Self::Timestamp => "snapshot_timestamp",
        })
    }
}
impl std::error::Error for SnapshotError {}

/// Whole snapshot document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Always [`FORMAT`].
    pub format: String,
    /// Unix seconds when the updater produced this file; drives staleness.
    pub generated_at: u64,
    /// Per-source records, sorted by name.
    pub sources: Vec<SourceRecord>,
}

/// One source's provenance, outcome and prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    /// Registry name, `[a-z0-9-]{1,64}`.
    pub name: String,
    /// Category the prefixes belong to.
    pub category: IntelCategory,
    /// Licence/terms note copied from the registry.
    pub licence: String,
    /// Where the bytes came from.
    pub origin: Origin,
    /// Acquisition outcome. A missing or invalid source stays visible here.
    pub status: SourceStatus,
    /// Static reason for a non-ok status; empty when ok.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Canonical, sorted, de-duplicated prefixes; empty unless ok.
    pub prefixes: Vec<Prefix>,
}

/// Provenance of one source's bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    /// A pinned git submodule checkout. `commit` is the checked-out commit,
    /// absent when the submodule is not initialized.
    Git {
        /// Repository-relative submodule path.
        path: String,
        /// Checked-out commit SHA, when initialized.
        commit: Option<String>,
    },
    /// A public HTTP endpoint fetched by the operator updater.
    Http {
        /// Fetched URL (first URL when a source spans several).
        url: String,
        /// Unix seconds of the fetch; zero when the fetch did not happen.
        fetched_at: u64,
    },
    /// Bytes supplied by the operator from a local file instead of fetching.
    File {
        /// Unix seconds when the updater read the file.
        read_at: u64,
    },
}

/// Acquisition outcome of one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    /// Fetched, normalized and validated.
    Ok,
    /// Not available (submodule not initialized, offline, fetch failed).
    Missing,
    /// Available but refused by its format normalizer or bounds.
    Invalid,
}

impl Snapshot {
    /// Build a normalized snapshot: sources sorted by name, prefixes sorted and
    /// de-duplicated. Validation happens in [`encode`].
    pub fn new(generated_at: u64, mut sources: Vec<SourceRecord>) -> Self {
        sources.sort_by(|a, b| a.name.cmp(&b.name));
        for source in &mut sources {
            source.prefixes.sort_unstable();
            source.prefixes.dedup();
        }
        Self {
            format: FORMAT.into(),
            generated_at,
            sources,
        }
    }

    /// Total prefix count over all sources.
    pub fn prefix_count(&self) -> usize {
        self.sources.iter().map(|s| s.prefixes.len()).sum()
    }
}

/// Parse and validate snapshot bytes. `now_s` is the injected clock used only
/// to refuse timestamps in the future.
pub fn parse(bytes: &[u8], now_s: u64) -> Result<Snapshot, SnapshotError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotError::TooLarge);
    }
    let snapshot: Snapshot = serde_json::from_slice(bytes).map_err(|_| SnapshotError::Malformed)?;
    validate(&snapshot)?;
    if snapshot.generated_at > now_s.saturating_add(MAX_FUTURE_SKEW_S) {
        return Err(SnapshotError::Timestamp);
    }
    Ok(snapshot)
}

/// Validate and encode as compact JSON with a trailing newline.
pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, SnapshotError> {
    validate(snapshot)?;
    let mut bytes = serde_json::to_vec(snapshot).map_err(|_| SnapshotError::Malformed)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotError::TooLarge);
    }
    Ok(bytes)
}

/// Whether `name` is a valid source name.
pub fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn text(value: &str, required: bool) -> Result<(), SnapshotError> {
    if (required && value.is_empty())
        || value.len() > MAX_TEXT
        || value.chars().any(char::is_control)
    {
        return Err(SnapshotError::Text);
    }
    Ok(())
}

fn validate(snapshot: &Snapshot) -> Result<(), SnapshotError> {
    if snapshot.format != FORMAT {
        return Err(SnapshotError::Format);
    }
    if snapshot.sources.is_empty()
        || snapshot.sources.len() > MAX_SOURCES
        || snapshot.prefix_count() > MAX_PREFIXES
    {
        return Err(SnapshotError::Bounds);
    }
    let mut names = BTreeSet::new();
    for source in &snapshot.sources {
        if !valid_name(&source.name) || !names.insert(source.name.as_str()) {
            return Err(SnapshotError::SourceName);
        }
        text(&source.licence, true)?;
        text(&source.detail, false)?;
        match (
            &source.status,
            source.prefixes.is_empty(),
            source.detail.is_empty(),
        ) {
            (SourceStatus::Ok, false, true) => {}
            (SourceStatus::Missing | SourceStatus::Invalid, true, false) => {}
            _ => return Err(SnapshotError::Status),
        }
        match &source.origin {
            Origin::Git { path, commit } => {
                text(path, true)?;
                match commit {
                    Some(commit) if valid_commit(commit) => {}
                    None if source.status == SourceStatus::Missing => {}
                    _ => return Err(SnapshotError::Commit),
                }
            }
            Origin::Http { url, fetched_at } => {
                text(url, true)?;
                if *fetched_at > snapshot.generated_at {
                    return Err(SnapshotError::Timestamp);
                }
            }
            Origin::File { read_at } => {
                if *read_at > snapshot.generated_at {
                    return Err(SnapshotError::Timestamp);
                }
            }
        }
    }
    Ok(())
}

fn valid_commit(commit: &str) -> bool {
    matches!(commit.len(), 40 | 64)
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests;
