//! Pure versioned snapshot format written by the operator updater.
//!
//! One JSON document, `humanymous-ip-intel/1`, with per-source provenance and
//! canonical prefixes. [`parse`] and [`encode`] share one validation, so a file
//! the updater can write is exactly a file the runtime accepts. Every bound is
//! checked before the content is used; a refusal changes no caller state.
//!
//! Scoring categories carry a breadth guard ([`check_breadth`]): a prefix
//! shorter than [`SCORING_FLOOR`] or a category covering more than
//! [`SCORING_MAX_V4`]/[`SCORING_MAX_V6`] addresses refuses the whole snapshot,
//! so one over-broad upstream line can never mark every client datacenter.
use super::{
    IntelCategory,
    prefix_set::{Floor, Prefix, PrefixSet},
    source,
};
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
/// Shortest prefix a scoring category may carry: IPv4 /8, IPv6 /16. Real
/// cloud data reaches /8 (AWS `3.0.0.0/8`) and IPv6 /20.
pub const SCORING_FLOOR: Floor = Floor { v4: 8, v6: 16 };
/// Most IPv4 addresses one scoring category may cover after merging: a
/// quarter of the family. All datacenter sources together cover about 10%.
pub const SCORING_MAX_V4: u64 = 1 << 30;
/// Most IPv6 addresses one scoring category may cover after merging: the size
/// of one /12 (1/512 of global unicast). Real data covers about 2^110.
pub const SCORING_MAX_V6: u128 = 1 << 116;

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
    /// A scoring-category prefix is shorter than [`SCORING_FLOOR`].
    TooBroad,
    /// A scoring category covers more than [`SCORING_MAX_V4`] or
    /// [`SCORING_MAX_V6`] addresses.
    Coverage,
    /// Sources are not sorted by name, or a source's prefixes are not
    /// strictly ascending (sorted and unique).
    Order,
    /// A registry source carries a category other than its registered one.
    Category,
    /// A git origin's checkout record disagrees with its commit or status.
    Checkout,
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
            Self::TooBroad => "snapshot_prefix_too_broad",
            Self::Coverage => "snapshot_coverage_cap",
            Self::Order => "snapshot_order",
            Self::Category => "snapshot_category",
            Self::Checkout => "snapshot_checkout",
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
    /// Category the prefixes belong to. For a registry source it must equal
    /// the registered category, so editing a file cannot promote a source.
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
        /// Pin and cleanliness check of the checkout; present with `commit`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checkout: Option<Checkout>,
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

/// How a submodule checkout compared with the parent repository's pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkout {
    /// Commit the parent repository's gitlink pins; `None` when unreadable.
    pub gitlink: Option<String>,
    /// The checkout had no tracked or untracked changes.
    pub clean: bool,
    /// The operator explicitly accepted a checkout that is not pinned and
    /// clean. False whenever the checkout matched its gitlink and was clean.
    pub unpinned_override: bool,
}

impl Checkout {
    /// Whether `commit` is exactly the pinned gitlink and the tree is clean.
    pub fn pinned(&self, commit: &str) -> bool {
        self.clean && self.gitlink.as_deref() == Some(commit)
    }
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

/// Breadth guard for one category's prefixes, shared by the updater (per
/// source, before writing) and [`validate`] (per category, on encode and on
/// every runtime load). Shadow-only categories are not bounded here: they
/// never reach scoring, and blocklists legitimately carry bogon ranges.
pub fn check_breadth(category: IntelCategory, prefixes: &[Prefix]) -> Result<(), SnapshotError> {
    if !category.feeds_scoring() {
        return Ok(());
    }
    for prefix in prefixes {
        SCORING_FLOOR
            .check(prefix)
            .map_err(|_| SnapshotError::TooBroad)?;
    }
    let (v4, v6) = PrefixSet::new(prefixes.iter().copied()).coverage();
    if v4 > SCORING_MAX_V4 || v6 > SCORING_MAX_V6 {
        return Err(SnapshotError::Coverage);
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
        if source::find(&source.name).is_some_and(|spec| spec.category != source.category) {
            return Err(SnapshotError::Category);
        }
        if !source.prefixes.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(SnapshotError::Order);
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
            Origin::Git {
                path,
                commit,
                checkout,
            } => {
                text(path, true)?;
                match commit {
                    Some(commit) if valid_commit(commit) => {}
                    None if source.status == SourceStatus::Missing => {}
                    _ => return Err(SnapshotError::Commit),
                }
                match (commit, checkout) {
                    (None, None) => {}
                    (Some(commit), Some(checkout))
                        if checkout.gitlink.as_deref().is_none_or(valid_commit)
                            && (source.status != SourceStatus::Ok
                                || checkout.pinned(commit) != checkout.unpinned_override) => {}
                    _ => return Err(SnapshotError::Checkout),
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
    if !snapshot
        .sources
        .windows(2)
        .all(|pair| pair[0].name < pair[1].name)
    {
        return Err(SnapshotError::Order);
    }
    for category in IntelCategory::ALL {
        let prefixes: Vec<Prefix> = snapshot
            .sources
            .iter()
            .filter(|s| s.category == category)
            .flat_map(|s| s.prefixes.iter().copied())
            .collect();
        check_breadth(category, &prefixes)?;
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
