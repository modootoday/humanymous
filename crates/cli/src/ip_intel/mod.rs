//! Operator IP reputation updater (`sot:root/ip-reputation`).
//!
//! I/O wiring only: read pinned submodule checkouts, operator files and public
//! HTTP bodies, normalize them with the runtime's single source contract, encode
//! one snapshot and replace the output file atomically. Parsing, validation and
//! the snapshot format live in `humanymous_runtime::network::intel`.
use humanymous_runtime::network::intel::{
    IntelCategory,
    prefix_set::{Prefix, PrefixSet},
    snapshot::{
        self, Checkout, Origin, SCORING_MAX_V4, SCORING_MAX_V6, Snapshot, SnapshotError,
        SourceRecord, SourceStatus,
    },
    source::{self, MAX_SOURCE_BYTES, REGISTRY, SourceOrigin, SourceSpec},
};
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
};

/// Fetch one public URL with a byte bound. Unit tests inject a fake; network
/// access never happens in tests.
pub trait Fetch {
    /// Body bytes, at most `limit`; `Err` carries a static reason code.
    fn fetch(&self, url: &str, limit: usize) -> Result<Vec<u8>, &'static str>;
}

/// HTTPS fetcher over the workspace's pinned ureq client.
pub struct HttpFetch(ureq::Agent);

impl Default for HttpFetch {
    fn default() -> Self {
        Self(
            ureq::Agent::config_builder()
                .max_redirects(3)
                .https_only(true)
                .timeout_global(Some(std::time::Duration::from_secs(60)))
                .build()
                .into(),
        )
    }
}

impl Fetch for HttpFetch {
    fn fetch(&self, url: &str, limit: usize) -> Result<Vec<u8>, &'static str> {
        let mut response = self
            .0
            .get(url)
            .header("User-Agent", "humanymous-ip-intel-updater")
            .call()
            .map_err(|_| "fetch_failed")?;
        response
            .body_mut()
            .with_config()
            .limit(limit as u64)
            .read_to_vec()
            .map_err(|_| "fetch_body_refused")
    }
}

/// What to build and where to write it.
#[derive(Debug, Clone, Default)]
pub struct UpdatePlan {
    /// Snapshot destination; replaced atomically.
    pub output: PathBuf,
    /// Repository root holding `third_party/ip-intel/<name>` checkouts.
    pub repository: PathBuf,
    /// Selected registry names; empty selects every source.
    pub sources: Vec<String>,
    /// Operator-supplied files that replace acquisition for a source.
    pub inputs: BTreeMap<String, PathBuf>,
    /// Operator-supplied URLs (required for rotating endpoints).
    pub urls: BTreeMap<String, String>,
    /// Never fetch over the network; HTTP sources become `missing`.
    pub offline: bool,
    /// Accept a submodule checkout whose HEAD differs from the parent
    /// repository's gitlink, or whose tree has changes. The snapshot records
    /// the override; without it such a source is refused as `invalid`.
    pub allow_unpinned: bool,
}

/// Per-source line of the update report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceLine {
    /// Registry name.
    pub name: String,
    /// Outcome.
    pub status: SourceStatus,
    /// Static reason for a non-ok outcome.
    pub detail: String,
    /// Prefix count.
    pub prefixes: usize,
    /// Git commit for pinned sources, when initialized.
    pub commit: Option<String>,
    /// Pin and cleanliness check for git sources, when initialized.
    pub checkout: Option<Checkout>,
}

/// Result of one update run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateReport {
    /// Snapshot `generated_at`.
    pub generated_at: u64,
    /// Total prefixes written.
    pub prefixes: usize,
    /// Per-source outcomes, including missing and invalid sources.
    pub sources: Vec<SourceLine>,
    /// Whether any selected source is missing or invalid.
    pub partial: bool,
}

/// Static detail of a scoring source dropped because its category's union
/// with the other sources exceeds the coverage cap.
pub const UNION_CAP_DROPPED: &str = "category_union_coverage_cap";

fn selected(plan: &UpdatePlan) -> io::Result<Vec<&'static SourceSpec>> {
    if plan.sources.is_empty() {
        return Ok(REGISTRY.iter().collect());
    }
    plan.sources
        .iter()
        .map(|name| {
            source::find(name).ok_or_else(|| io::Error::other(format!("unknown source: {name}")))
        })
        .collect()
}

/// Refuse a plan that names a source twice, names an unknown source, or
/// passes `--url` for a source whose endpoint is not operator-supplied. Run by
/// the argument parser and again by [`update`], so nothing is ignored.
pub fn check_plan(plan: &UpdatePlan) -> io::Result<()> {
    let mut seen = BTreeSet::new();
    for name in &plan.sources {
        if !seen.insert(name.as_str()) {
            return Err(io::Error::other(format!("duplicate source: {name}")));
        }
    }
    for name in plan.sources.iter().chain(plan.inputs.keys()) {
        if source::find(name).is_none() {
            return Err(io::Error::other(format!("unknown source: {name}")));
        }
    }
    for name in plan.urls.keys() {
        match source::find(name).map(|spec| spec.origin) {
            None => return Err(io::Error::other(format!("unknown source: {name}"))),
            Some(SourceOrigin::OperatorUrl { .. }) => {}
            Some(_) => {
                return Err(io::Error::other(format!(
                    "source {name} does not take an operator URL"
                )));
            }
        }
    }
    Ok(())
}

/// Build and atomically write one snapshot. Refuses to write when no source
/// succeeded, so an empty feed is never published as clean data.
pub fn update(plan: &UpdatePlan, fetch: &dyn Fetch, now_s: u64) -> io::Result<UpdateReport> {
    check_plan(plan)?;
    let mut records: Vec<SourceRecord> = selected(plan)?
        .into_iter()
        .map(|spec| acquire(spec, plan, fetch, now_s))
        .collect();
    fit_union_caps(&mut records);
    if !records.iter().any(|r| r.status == SourceStatus::Ok) {
        return Err(io::Error::other(
            "no source succeeded; snapshot not written",
        ));
    }
    let snapshot = Snapshot::new(now_s, records);
    let bytes = snapshot::encode(&snapshot).map_err(io::Error::other)?;
    write_atomically(&plan.output, &bytes)?;
    let sources: Vec<SourceLine> = snapshot
        .sources
        .iter()
        .map(|record| SourceLine {
            name: record.name.clone(),
            status: record.status,
            detail: record.detail.clone(),
            prefixes: record.prefixes.len(),
            commit: match &record.origin {
                Origin::Git { commit, .. } => commit.clone(),
                _ => None,
            },
            checkout: match &record.origin {
                Origin::Git { checkout, .. } => checkout.clone(),
                _ => None,
            },
        })
        .collect();
    Ok(UpdateReport {
        generated_at: now_s,
        prefixes: snapshot.prefix_count(),
        partial: sources.iter().any(|s| s.status != SourceStatus::Ok),
        sources,
    })
}

/// Fail closed per source when a scoring category's union exceeds the coverage
/// cap although every source fits alone: drop the ok source with the largest
/// coverage in the overflowing family (ties: the lowest name), mark it invalid
/// with [`UNION_CAP_DROPPED`], and repeat until the union fits. A union above
/// the cap is never written; the remaining sources still are.
fn fit_union_caps(records: &mut [SourceRecord]) {
    let coverage = |prefixes: &[Prefix]| PrefixSet::new(prefixes.iter().copied()).coverage();
    for category in IntelCategory::ALL {
        loop {
            let ok = |r: &SourceRecord| r.category == category && r.status == SourceStatus::Ok;
            let union: Vec<Prefix> = records
                .iter()
                .filter(|r| ok(r))
                .flat_map(|r| r.prefixes.iter().copied())
                .collect();
            if snapshot::check_breadth(category, &union) != Err(SnapshotError::Coverage) {
                break;
            }
            let (v4, v6) = coverage(&union);
            let (over_v4, over_v6) = (v4 > SCORING_MAX_V4, v6 > SCORING_MAX_V6);
            let Some(largest) = records.iter_mut().filter(|r| ok(r)).max_by_key(|r| {
                let (a, b) = coverage(&r.prefixes);
                (
                    if over_v6 { b } else { 0 },
                    if over_v4 { a } else { 0 },
                    Reverse(r.name.clone()),
                )
            }) else {
                break;
            };
            largest.status = SourceStatus::Invalid;
            largest.detail = UNION_CAP_DROPPED.into();
            largest.prefixes.clear();
        }
    }
}

/// Bytes plus provenance, or the provenance, outcome and static reason of a
/// source that is missing or refused before normalization.
type Acquired = Result<(Vec<Vec<u8>>, Origin), (Origin, SourceStatus, &'static str)>;

fn acquire(spec: &SourceSpec, plan: &UpdatePlan, fetch: &dyn Fetch, now_s: u64) -> SourceRecord {
    let acquired = if let Some(path) = plan.inputs.get(spec.name) {
        read_bounded(path)
            .map(|bytes| (vec![bytes], Origin::File { read_at: now_s }))
            .map_err(|reason| {
                (
                    Origin::File { read_at: now_s },
                    SourceStatus::Missing,
                    reason,
                )
            })
    } else {
        match spec.origin {
            SourceOrigin::Git { files, .. } => {
                git(spec, &plan.repository, files, plan.allow_unpinned)
            }
            SourceOrigin::Http { urls } => http(urls, plan.offline, fetch, now_s),
            SourceOrigin::OperatorUrl { discovery } => match plan.urls.get(spec.name) {
                Some(url) => http(&[url.as_str()], plan.offline, fetch, now_s),
                None => Err((
                    Origin::Http {
                        url: discovery.into(),
                        fetched_at: 0,
                    },
                    SourceStatus::Missing,
                    "operator_url_required",
                )),
            },
        }
    };
    let (status, detail, origin, prefixes) = match acquired {
        Err((origin, status, reason)) => (status, reason.to_owned(), origin, Vec::new()),
        Ok((bodies, origin)) => {
            let mut prefixes = Vec::new();
            let mut refused = None;
            for body in &bodies {
                match source::normalize(spec.format, body) {
                    Ok(mut more) => prefixes.append(&mut more),
                    Err(error) => {
                        refused = Some(error.to_string());
                        break;
                    }
                }
            }
            // The same breadth guard the snapshot validator applies on every
            // runtime load: an over-broad source is refused here, by itself.
            prefixes.sort_unstable();
            prefixes.dedup();
            if refused.is_none()
                && let Err(error) = snapshot::check_breadth(spec.category, &prefixes)
            {
                refused = Some(error.to_string());
            }
            match refused {
                Some(reason) => (SourceStatus::Invalid, reason, origin, Vec::new()),
                None => (SourceStatus::Ok, String::new(), origin, prefixes),
            }
        }
    };
    SourceRecord {
        name: spec.name.into(),
        category: spec.category,
        licence: spec.licence.into(),
        origin,
        status,
        detail,
        prefixes,
    }
}

fn git(spec: &SourceSpec, repository: &Path, files: &[&str], allow_unpinned: bool) -> Acquired {
    let path = spec.submodule_path().unwrap_or_default();
    let directory = repository.join(&path);
    let missing = |reason| {
        Err((
            Origin::Git {
                path: path.clone(),
                commit: None,
                checkout: None,
            },
            SourceStatus::Missing,
            reason,
        ))
    };
    let Some(commit) = checkout_commit(&directory) else {
        return missing("submodule_not_initialized");
    };
    // The gitlink is the source of truth for the data version: the checkout
    // must be exactly that commit, with no local edits, unless overridden.
    let mut checkout = Checkout {
        gitlink: gitlink(repository, &path),
        clean: checkout_clean(&directory),
        unpinned_override: false,
    };
    let refusal = if checkout.gitlink.is_none() {
        Some("submodule_gitlink_unreadable")
    } else if checkout.gitlink.as_deref() != Some(commit.as_str()) {
        Some("submodule_head_mismatch")
    } else if !checkout.clean {
        Some("submodule_dirty")
    } else {
        None
    };
    if let Some(reason) = refusal {
        if !allow_unpinned {
            return Err((
                Origin::Git {
                    path,
                    commit: Some(commit),
                    checkout: Some(checkout),
                },
                SourceStatus::Invalid,
                reason,
            ));
        }
        checkout.unpinned_override = true;
    }
    let mut bodies = Vec::new();
    for file in files {
        match read_bounded(&directory.join(file)) {
            Ok(bytes) => bodies.push(bytes),
            Err(_) => return missing("submodule_file_missing"),
        }
    }
    Ok((
        bodies,
        Origin::Git {
            path,
            commit: Some(commit),
            checkout: Some(checkout),
        },
    ))
}

/// Run `git` with a bounded, inherited-state-free environment; stdout on
/// success only.
fn run_git(directory: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

/// Commit the parent repository's `HEAD` pins at `path` (a mode 160000
/// gitlink), or `None` when it cannot be read.
pub fn gitlink(repository: &Path, path: &str) -> Option<String> {
    let listing = run_git(repository, &["ls-tree", "-z", "HEAD", "--", path])?;
    let entry = listing.split(|b| *b == 0).next()?;
    let (meta, listed) = std::str::from_utf8(entry).ok()?.split_once('\t')?;
    let mut fields = meta.split(' ');
    let (mode, kind, sha) = (fields.next()?, fields.next()?, fields.next()?);
    (mode == "160000" && kind == "commit" && listed == path && valid_commit(sha))
        .then(|| sha.to_owned())
}

/// Whether the checkout has no tracked or untracked changes. A failed status
/// reads as not clean.
pub fn checkout_clean(checkout: &Path) -> bool {
    run_git(
        checkout,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )
    .is_some_and(|out| out.is_empty())
}

fn valid_commit(commit: &str) -> bool {
    matches!(commit.len(), 40 | 64)
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn http(urls: &[&str], offline: bool, fetch: &dyn Fetch, now_s: u64) -> Acquired {
    let origin = |fetched_at| Origin::Http {
        url: urls.first().copied().unwrap_or_default().into(),
        fetched_at,
    };
    if offline {
        return Err((origin(0), SourceStatus::Missing, "offline"));
    }
    if urls.iter().any(|url| !url.starts_with("https://")) {
        return Err((origin(0), SourceStatus::Missing, "https_required"));
    }
    let mut bodies = Vec::new();
    for url in urls {
        match fetch.fetch(url, MAX_SOURCE_BYTES) {
            Ok(body) => bodies.push(body),
            Err(reason) => return Err((origin(0), SourceStatus::Missing, reason)),
        }
    }
    Ok((bodies, origin(now_s)))
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, &'static str> {
    let file = fs::File::open(path).map_err(|_| "input_unreadable")?;
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "input_unreadable")?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("input_too_large");
    }
    Ok(bytes)
}

/// Checked-out commit of a submodule working tree, read from its git metadata
/// without spawning git. `None` when the submodule is not initialized.
pub fn checkout_commit(checkout: &Path) -> Option<String> {
    let dot_git = checkout.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let text = fs::read_to_string(&dot_git).ok()?;
        let target = text.strip_prefix("gitdir:")?.trim();
        checkout.join(target)
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    let commit = match head.strip_prefix("ref:") {
        None => head.to_owned(),
        Some(reference) => {
            let reference = reference.trim();
            match fs::read_to_string(git_dir.join(reference)) {
                Ok(text) => text.trim().to_owned(),
                Err(_) => packed_ref(&git_dir, reference)?,
            }
        }
    };
    valid_commit(&commit).then_some(commit)
}

fn packed_ref(git_dir: &Path, reference: &str) -> Option<String> {
    let packed = fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let (sha, name) = line.split_once(' ')?;
        (name == reference).then(|| sha.to_owned())
    })
}

/// Replace `output` with `bytes`: write a new private sibling, sync it, then
/// rename over the destination so readers see the old or the new file only.
fn write_atomically(output: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = output
        .file_name()
        .ok_or_else(|| io::Error::other("output must name a file"))?;
    let mut temporary = name.to_owned();
    temporary.push(format!(".next-{}", std::process::id()));
    let temporary = output.with_file_name(temporary);
    let _ = fs::remove_file(&temporary);
    crate::service::storage::write_new(&temporary, bytes)?;
    if let Err(cause) = fs::rename(&temporary, output) {
        let _ = fs::remove_file(&temporary);
        return Err(cause);
    }
    Ok(())
}

/// Registry listing for `humanymous ip-intel sources`.
pub fn registry() -> serde_json::Value {
    serde_json::Value::Array(
        REGISTRY
            .iter()
            .map(|spec| {
                let origin = match spec.origin {
                    SourceOrigin::Git { repository, files } => serde_json::json!({
                        "git": {"path": spec.submodule_path(), "repository": repository, "files": files}
                    }),
                    SourceOrigin::Http { urls } => serde_json::json!({"http": {"urls": urls}}),
                    SourceOrigin::OperatorUrl { discovery } => {
                        serde_json::json!({"operator_url": {"discovery": discovery}})
                    }
                };
                serde_json::json!({
                    "name": spec.name,
                    "category": spec.category,
                    "feeds_scoring": spec.category.feeds_scoring(),
                    "licence": spec.licence,
                    "origin": origin,
                })
            })
            .collect(),
    )
}

/// Summary of an existing snapshot for `humanymous ip-intel inspect`.
pub fn inspect(path: &Path, now_s: u64) -> io::Result<serde_json::Value> {
    let bytes = fs::read(path)?;
    let snapshot = snapshot::parse(&bytes, now_s).map_err(io::Error::other)?;
    Ok(serde_json::json!({
        "format": snapshot.format,
        "generated_at": snapshot.generated_at,
        "age_s": now_s.saturating_sub(snapshot.generated_at),
        "prefixes": snapshot.prefix_count(),
        "sources": snapshot.sources.iter().map(|s| serde_json::json!({
            "name": s.name, "category": s.category, "status": s.status,
            "detail": s.detail, "prefixes": s.prefixes.len(), "origin": s.origin,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests;
