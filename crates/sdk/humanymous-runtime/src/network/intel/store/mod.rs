//! Snapshot I/O boundary: bounded file load, atomic swap, staleness from an
//! injected clock and the operator status gauge.
//!
//! Fail-open for traffic, never silent: a missing, refused, stale or expired
//! snapshot denies nobody, and every one of those states is visible in
//! [`IntelStore::status`]. A refused reload keeps the previous snapshot.
use super::{
    Classification, IntelCategory, IntelReader, is_classifiable,
    prefix_set::PrefixSet,
    shadow::{ShadowCounters, ShadowStats},
    snapshot::{self, MAX_SNAPSHOT_BYTES, Origin, Snapshot, SnapshotError, SourceStatus},
    source::REGISTRY,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Read;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::SystemTime;

/// Store parameters, derived from the runtime configuration surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePolicy {
    /// Snapshot file written by the operator updater.
    pub path: PathBuf,
    /// Age after which the snapshot is reported stale (still used).
    pub max_age_s: u64,
    /// Age after which the snapshot is no longer used for classification.
    pub expire_after_s: u64,
    /// Categories loaded from the snapshot; others are ignored.
    pub categories: Vec<IntelCategory>,
}

/// Snapshot freshness at a given time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// No snapshot has ever been accepted in this process.
    Missing,
    /// Age is within `max_age_s`.
    Fresh,
    /// Older than `max_age_s`; still classifies, and is reported.
    Stale,
    /// Older than `expire_after_s`; classifies nothing, and is reported.
    Expired,
}

/// Static load refusal codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// File could not be opened, stat'ed or read.
    Io,
    /// File content was refused.
    Snapshot(SnapshotError),
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => f.write_str("snapshot_io"),
            Self::Snapshot(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for LoadError {}

/// Per-source summary without prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceSummary {
    /// Registry name.
    pub name: String,
    /// Category.
    pub category: IntelCategory,
    /// Acquisition outcome recorded by the updater.
    pub status: SourceStatus,
    /// Static reason for a non-ok status.
    pub detail: String,
    /// Prefix count.
    pub prefixes: usize,
    /// Provenance (git commit or fetch time).
    pub origin: Origin,
}

/// Request-path counters for the operator status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LookupStats {
    /// All classify calls.
    pub lookups: u64,
    /// No usable trusted client address on the request.
    pub unattributed: u64,
    /// Private, loopback, link-local or unspecified address.
    pub not_classifiable: u64,
    /// No usable snapshot (missing or expired).
    pub unavailable: u64,
    /// Classified against a stale snapshot.
    pub stale: u64,
    /// Requests that set the frozen `is_datacenter_ip` input.
    pub datacenter_inputs: u64,
}

/// Operator status / readiness gauge. Contains no client addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreStatus {
    /// Freshness state.
    pub state: Freshness,
    /// Whether datacenter ranges currently reach the scoring input.
    pub datacenter_scoring_active: bool,
    /// Whether scoring coverage is incomplete. Old data is never carried
    /// forward for a failed source; its coverage is simply absent, and this
    /// says so. See `degraded_reasons`.
    pub degraded: bool,
    /// Static reasons, in stable order: `snapshot_missing|stale|expired`,
    /// `reload_refused`, and per scoring source `source_absent:<name>`,
    /// `source_missing:<name>`, `source_invalid:<name>` or
    /// `source_stale:<name>` (fetched or read longer than `max_age_s` ago).
    pub degraded_reasons: Vec<String>,
    /// `generated_at` of the active snapshot.
    pub generated_at: Option<u64>,
    /// Age of the active snapshot in seconds.
    pub age_s: Option<u64>,
    /// Stale threshold.
    pub max_age_s: u64,
    /// Expiry threshold.
    pub expire_after_s: u64,
    /// SHA-256 of the active snapshot file bytes.
    pub snapshot_sha256: Option<String>,
    /// Enabled categories.
    pub categories: Vec<IntelCategory>,
    /// Categories that feed scoring (only datacenter can).
    pub scoring_categories: Vec<IntelCategory>,
    /// Enabled shadow-only categories.
    pub shadow_categories: Vec<IntelCategory>,
    /// Sources of the active snapshot, including missing/invalid ones.
    pub sources: Vec<SourceSummary>,
    /// Static code of the most recent refused load, if any.
    pub last_reload_error: Option<String>,
    /// Accepted loads in this process.
    pub reloads_ok: u64,
    /// Refused loads in this process.
    pub reloads_failed: u64,
    /// Request-path counters.
    pub lookups: LookupStats,
    /// Shadow-only category hits.
    pub shadow: ShadowStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

struct Loaded {
    generated_at: u64,
    sha256: String,
    stamp: FileStamp,
    sets: Vec<(IntelCategory, PrefixSet)>,
    sources: Vec<SourceSummary>,
}

impl Loaded {
    fn build(
        snapshot: Snapshot,
        sha256: String,
        stamp: FileStamp,
        enabled: &[IntelCategory],
    ) -> Self {
        let sets = enabled
            .iter()
            .map(|&category| {
                let prefixes = snapshot
                    .sources
                    .iter()
                    .filter(|s| s.category == category && s.status == SourceStatus::Ok)
                    .flat_map(|s| s.prefixes.iter().copied());
                (category, PrefixSet::new(prefixes))
            })
            .collect();
        let sources = snapshot
            .sources
            .into_iter()
            .map(|s| SourceSummary {
                prefixes: s.prefixes.len(),
                name: s.name,
                category: s.category,
                status: s.status,
                detail: s.detail,
                origin: s.origin,
            })
            .collect();
        Self {
            generated_at: snapshot.generated_at,
            sha256,
            stamp,
            sets,
            sources,
        }
    }

    fn contains(&self, category: IntelCategory, ip: IpAddr) -> bool {
        self.sets
            .iter()
            .any(|(c, set)| *c == category && set.contains(ip))
    }
}

#[derive(Default)]
struct Counters {
    lookups: AtomicU64,
    unattributed: AtomicU64,
    not_classifiable: AtomicU64,
    unavailable: AtomicU64,
    stale: AtomicU64,
    datacenter_inputs: AtomicU64,
}

fn bump(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}

/// Process-lifetime owner of the active snapshot.
pub struct IntelStore {
    policy: StorePolicy,
    current: RwLock<Option<Arc<Loaded>>>,
    last_error: Mutex<Option<LoadError>>,
    reloads_ok: AtomicU64,
    reloads_failed: AtomicU64,
    counters: Counters,
    shadow: ShadowCounters,
}

impl IntelStore {
    /// Create the store and attempt the first load. A failed first load is not
    /// an error: the store starts `missing`, denies nobody and reports it.
    pub fn open(policy: StorePolicy, now_s: u64) -> Self {
        let store = Self {
            policy,
            current: RwLock::new(None),
            last_error: Mutex::new(None),
            reloads_ok: AtomicU64::new(0),
            reloads_failed: AtomicU64::new(0),
            counters: Counters::default(),
            shadow: ShadowCounters::default(),
        };
        let _ = store.reload(now_s);
        store
    }

    /// Read, validate and atomically swap in the snapshot file. On refusal the
    /// active snapshot is unchanged and the failure is counted and reported.
    pub fn reload(&self, now_s: u64) -> Result<(), LoadError> {
        let result = self.load(now_s);
        let mut last_error = self
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match result {
            Ok(loaded) => {
                *self.current.write().unwrap_or_else(PoisonError::into_inner) =
                    Some(Arc::new(loaded));
                *last_error = None;
                bump(&self.reloads_ok);
                Ok(())
            }
            Err(error) => {
                *last_error = Some(error);
                bump(&self.reloads_failed);
                Err(error)
            }
        }
    }

    /// Reload only when the file's length or modification time changed since
    /// the active snapshot was read. Returns whether a reload was attempted.
    pub fn reload_if_changed(&self, now_s: u64) -> Result<bool, LoadError> {
        let stamp = std::fs::metadata(&self.policy.path)
            .map(|m| FileStamp {
                len: m.len(),
                modified: m.modified().ok(),
            })
            .map_err(|_| LoadError::Io);
        let active = self.active().map(|loaded| loaded.stamp);
        if let (Ok(stamp), Some(active)) = (&stamp, active)
            && *stamp == active
        {
            return Ok(false);
        }
        self.reload(now_s).map(|()| true)
    }

    /// Freshness of the active snapshot at `now_s`.
    pub fn freshness(&self, now_s: u64) -> Freshness {
        self.active().map_or(Freshness::Missing, |loaded| {
            self.age_state(loaded.generated_at, now_s)
        })
    }

    /// Operator status at `now_s`.
    pub fn status(&self, now_s: u64) -> StoreStatus {
        let active = self.active();
        let state = self.freshness(now_s);
        let enabled = self.policy.categories.clone();
        let datacenter_scoring_active = matches!(state, Freshness::Fresh | Freshness::Stale)
            && active.as_ref().is_some_and(|loaded| {
                loaded
                    .sets
                    .iter()
                    .any(|(c, set)| c.feeds_scoring() && !set.is_empty())
            });
        let load = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let last_reload_error = self
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .map(|e| e.to_string());
        let degraded_reasons =
            self.degraded_reasons(state, active.as_deref(), last_reload_error.is_some(), now_s);
        StoreStatus {
            state,
            datacenter_scoring_active,
            degraded: !degraded_reasons.is_empty(),
            degraded_reasons,
            generated_at: active.as_ref().map(|l| l.generated_at),
            age_s: active
                .as_ref()
                .map(|l| now_s.saturating_sub(l.generated_at)),
            max_age_s: self.policy.max_age_s,
            expire_after_s: self.policy.expire_after_s,
            snapshot_sha256: active.as_ref().map(|l| l.sha256.clone()),
            scoring_categories: enabled
                .iter()
                .copied()
                .filter(|c| c.feeds_scoring())
                .collect(),
            shadow_categories: enabled
                .iter()
                .copied()
                .filter(|c| !c.feeds_scoring())
                .collect(),
            categories: enabled,
            sources: active
                .as_ref()
                .map(|l| l.sources.clone())
                .unwrap_or_default(),
            last_reload_error,
            reloads_ok: load(&self.reloads_ok),
            reloads_failed: load(&self.reloads_failed),
            lookups: LookupStats {
                lookups: load(&self.counters.lookups),
                unattributed: load(&self.counters.unattributed),
                not_classifiable: load(&self.counters.not_classifiable),
                unavailable: load(&self.counters.unavailable),
                stale: load(&self.counters.stale),
                datacenter_inputs: load(&self.counters.datacenter_inputs),
            },
            shadow: self.shadow.stats(),
        }
    }

    /// Why scoring coverage is incomplete. Only enabled scoring categories
    /// count: shadow sources never change a verdict, so they never degrade it.
    fn degraded_reasons(
        &self,
        state: Freshness,
        active: Option<&Loaded>,
        reload_refused: bool,
        now_s: u64,
    ) -> Vec<String> {
        let scoring: Vec<IntelCategory> = self
            .policy
            .categories
            .iter()
            .copied()
            .filter(|c| c.feeds_scoring())
            .collect();
        if scoring.is_empty() {
            return Vec::new();
        }
        let mut reasons = Vec::new();
        match state {
            Freshness::Fresh => {}
            Freshness::Missing => reasons.push("snapshot_missing".to_owned()),
            Freshness::Stale => reasons.push("snapshot_stale".to_owned()),
            Freshness::Expired => reasons.push("snapshot_expired".to_owned()),
        }
        if reload_refused {
            reasons.push("reload_refused".to_owned());
        }
        let Some(active) = active else {
            return reasons;
        };
        let mut names: Vec<&str> = REGISTRY
            .iter()
            .filter(|spec| scoring.contains(&spec.category))
            .map(|spec| spec.name)
            .chain(
                active
                    .sources
                    .iter()
                    .filter(|s| scoring.contains(&s.category))
                    .map(|s| s.name.as_str()),
            )
            .collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
            let Some(source) = active.sources.iter().find(|s| s.name == name) else {
                reasons.push(format!("source_absent:{name}"));
                continue;
            };
            let acquired_at = match source.origin {
                Origin::Http { fetched_at, .. } => Some(fetched_at),
                Origin::File { read_at } => Some(read_at),
                // A pinned commit's age is the snapshot's age, reported above.
                Origin::Git { .. } => None,
            };
            match source.status {
                SourceStatus::Missing => reasons.push(format!("source_missing:{name}")),
                SourceStatus::Invalid => reasons.push(format!("source_invalid:{name}")),
                SourceStatus::Ok
                    if acquired_at
                        .is_some_and(|t| now_s.saturating_sub(t) > self.policy.max_age_s) =>
                {
                    reasons.push(format!("source_stale:{name}"))
                }
                SourceStatus::Ok => {}
            }
        }
        reasons
    }

    fn active(&self) -> Option<Arc<Loaded>> {
        self.current
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn age_state(&self, generated_at: u64, now_s: u64) -> Freshness {
        let age = now_s.saturating_sub(generated_at);
        if age <= self.policy.max_age_s {
            Freshness::Fresh
        } else if age <= self.policy.expire_after_s {
            Freshness::Stale
        } else {
            Freshness::Expired
        }
    }

    fn load(&self, now_s: u64) -> Result<Loaded, LoadError> {
        let mut file = std::fs::File::open(&self.policy.path).map_err(|_| LoadError::Io)?;
        let metadata = file.metadata().map_err(|_| LoadError::Io)?;
        if !metadata.is_file() {
            return Err(LoadError::Io);
        }
        if metadata.len() > MAX_SNAPSHOT_BYTES as u64 {
            return Err(LoadError::Snapshot(SnapshotError::TooLarge));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.by_ref()
            .take(MAX_SNAPSHOT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| LoadError::Io)?;
        let parsed = snapshot::parse(&bytes, now_s).map_err(LoadError::Snapshot)?;
        Ok(Loaded::build(
            parsed,
            hex::encode(Sha256::digest(&bytes)),
            FileStamp {
                len: metadata.len(),
                modified: metadata.modified().ok(),
            },
            &self.policy.categories,
        ))
    }
}

impl IntelReader for IntelStore {
    fn classify(&self, client: Option<IpAddr>, now_s: u64) -> Classification {
        bump(&self.counters.lookups);
        let Some(ip) = client else {
            bump(&self.counters.unattributed);
            return Classification::default();
        };
        if !is_classifiable(ip) {
            bump(&self.counters.not_classifiable);
            return Classification::default();
        }
        let Some(loaded) = self.active() else {
            bump(&self.counters.unavailable);
            return Classification::default();
        };
        match self.age_state(loaded.generated_at, now_s) {
            Freshness::Expired | Freshness::Missing => {
                bump(&self.counters.unavailable);
                return Classification::default();
            }
            Freshness::Stale => bump(&self.counters.stale),
            Freshness::Fresh => {}
        }
        let classification = Classification {
            datacenter: loaded.contains(IntelCategory::Datacenter, ip),
            tor_exit: loaded.contains(IntelCategory::TorExit, ip),
            blocklist: loaded.contains(IntelCategory::Blocklist, ip),
        };
        if classification.datacenter {
            bump(&self.counters.datacenter_inputs);
        }
        self.shadow.record(&classification);
        classification
    }
}

#[cfg(test)]
mod tests;
