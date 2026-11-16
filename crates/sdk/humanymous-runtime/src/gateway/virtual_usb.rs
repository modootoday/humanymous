//! Read-only view of bounded virtual-USB lab ladder results (legacy
//! internal/gate/admin_vusb.go at baseline 981d25c).
//!
//! The view is disabled without an explicit directory. Each run directory
//! holds one `ladder-result.json`; files must be bounded regular files (no
//! symlinks), decode with no unknown fields or trailing JSON, and match the
//! fixed ladder schema. Any violation fails the whole view closed. Verification
//! is schema-only: these are lab results, not product measurements.
//!
//! Deliberate difference: run directories are visited in name order so the
//! candidate bound is deterministic (Go used directory order).
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Most recent runs returned.
pub const MAX_RUNS: usize = 32;
/// Run directories considered.
pub const MAX_CANDIDATES: usize = 128;
/// Maximum result size in bytes.
pub const MAX_RESULT_BYTES: u64 = 64 << 10;

const ENGINES: [&str; 2] = ["chromium", "firefox"];
const ORDER: [&str; 4] = [
    "external_input_virtual",
    "external_input_dom_virtual",
    "external_input_vusb",
    "external_input_dom_vusb",
];
const IME_ORDER: [&str; 3] = ["ko-KR", "zh-CN", "ja-JP"];

/// IME composition sub-result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Ime {
    /// Must be `humanymous.ime-composition-vusb/v1`.
    pub schema_version: String,
    /// Locale order.
    pub order: Vec<String>,
    /// Always 6.
    pub expected: i64,
    /// Measured cases.
    pub measured: i64,
    /// Passing cases.
    pub pass: i64,
    /// Failure labels.
    pub failures: Vec<String>,
}

/// Ladder terminal summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Ladder {
    /// Must be `humanymous.virtual-usb-ladder/v1`.
    pub schema_version: String,
    /// Whether this is a canonical run.
    pub canonical: bool,
    /// Browser engines.
    pub engines: Vec<String>,
    /// Ladder rung order.
    pub order: Vec<String>,
    /// Always 8.
    pub measured: i64,
    /// Passing rungs.
    pub pass: i64,
    /// Residual rungs.
    pub residual: i64,
    /// Failure labels.
    pub failures: Vec<String>,
    /// IME sub-result.
    pub ime: Ime,
}

/// One run with its file modification time in Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Result file modification time.
    pub recorded_at_unix: u64,
    /// Validated summary.
    pub result: Ladder,
}

/// Why the view failed closed; never echoes file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// The root is missing, not a directory or a symlink.
    Root,
    /// A result is not a bounded regular file.
    File,
    /// A result changed between stat and open.
    Changed,
    /// A result failed to decode or violates the schema.
    Schema,
    /// A canonical result is internally inconsistent.
    Canonical,
}

/// Admin view (legacy JSON shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    /// A results directory was configured.
    pub enabled: bool,
    /// The directory read cleanly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy: Option<bool>,
    /// Always `schema-only` when healthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<&'static str>,
    /// Newest first, at most `MAX_RUNS`.
    pub runs: Vec<Run>,
    /// Operator hint when disabled or unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

/// Build the admin view; `None` root disables it.
pub fn view(root: Option<&Path>) -> View {
    let Some(root) = root else {
        return View {
            enabled: false,
            healthy: None,
            verification: None,
            runs: Vec::new(),
            note: Some("Configure a virtual USB results directory to display bounded lab results."),
        };
    };
    match read(root) {
        Ok(runs) => View {
            enabled: true,
            healthy: Some(true),
            verification: Some("schema-only"),
            runs,
            note: None,
        },
        Err(_) => View {
            enabled: true,
            healthy: Some(false),
            verification: None,
            runs: Vec::new(),
            note: Some(
                "The configured result directory is unavailable or contains an invalid terminal summary.",
            ),
        },
    }
}

fn same(list: &[String], expected: &[&str]) -> bool {
    list.len() == expected.len() && list.iter().zip(expected).all(|(a, b)| a == b)
}

fn valid(r: &Ladder) -> Result<(), Invalid> {
    let i = &r.ime;
    let ok = r.schema_version == "humanymous.virtual-usb-ladder/v1"
        && r.measured == 8
        && r.pass >= 0
        && r.residual >= 0
        && r.pass + r.residual <= r.measured
        && r.failures.len() <= 32
        && same(&r.engines, &ENGINES)
        && same(&r.order, &ORDER)
        && i.schema_version == "humanymous.ime-composition-vusb/v1"
        && i.expected == 6
        && (0..=i.expected).contains(&i.measured)
        && (0..=i.measured).contains(&i.pass)
        && i.failures.len() <= 32
        && same(&i.order, &IME_ORDER);
    if !ok {
        return Err(Invalid::Schema);
    }
    if r.canonical
        && (!r.failures.is_empty()
            || r.pass + r.residual != 8
            || i.measured != 6
            || i.pass != 6
            || !i.failures.is_empty())
    {
        return Err(Invalid::Canonical);
    }
    Ok(())
}

fn unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}
#[cfg(not(unix))]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.len() == b.len() && a.modified().ok() == b.modified().ok()
}

/// Read and validate every run; any violation fails the whole read.
pub fn read(root: &Path) -> Result<Vec<Run>, Invalid> {
    let meta = fs::symlink_metadata(root).map_err(|_| Invalid::Root)?;
    if !meta.is_dir() {
        return Err(Invalid::Root);
    }
    let mut names: Vec<_> = fs::read_dir(root)
        .map_err(|_| Invalid::Root)?
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    names.sort();
    let mut candidates = Vec::new();
    for name in names {
        if candidates.len() >= MAX_CANDIDATES {
            break;
        }
        let dir = root.join(&name);
        if !fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
            continue;
        }
        let path = dir.join("ladder-result.json");
        let Ok(info) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !info.file_type().is_file() || info.len() > MAX_RESULT_BYTES {
            return Err(Invalid::File);
        }
        let modified = info.modified().map_or(0, unix);
        candidates.push((path, modified, info));
    }
    candidates.sort_by_key(|c| std::cmp::Reverse(c.1));
    candidates.truncate(MAX_RUNS);
    let mut runs = Vec::with_capacity(candidates.len());
    for (path, modified, info) in candidates {
        let file = File::open(&path).map_err(|_| Invalid::File)?;
        let opened = file.metadata().map_err(|_| Invalid::Changed)?;
        if !same_file(&info, &opened) {
            return Err(Invalid::Changed);
        }
        let mut bytes = Vec::new();
        file.take(MAX_RESULT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Invalid::File)?;
        if bytes.len() as u64 > MAX_RESULT_BYTES {
            return Err(Invalid::File);
        }
        let result: Ladder = serde_json::from_slice(&bytes).map_err(|_| Invalid::Schema)?;
        valid(&result)?;
        runs.push(Run {
            recorded_at_unix: modified,
            result,
        });
    }
    Ok(runs)
}

#[cfg(test)]
mod tests;
