//! Bounded aggregate store of confirmed-abuse behavior clusters.
//!
//! Admission accepts only independently confirmed abuse ([`Evidence`]).
//! Automatic enforcement results, shadow outcomes and model probabilities are
//! not admissible, which keeps a false positive from reinforcing itself.
use super::SimilarityError;
use crate::behavior::features::{FEATURE_DIM, FeatureVector};
use std::collections::{BTreeMap, VecDeque};

/// Most clusters held; a full store refuses instead of evicting.
pub const MAX_CLUSTERS: usize = 256;
/// Match times kept per cluster; recurrence saturates at this count.
pub const MAX_MATCH_EVENTS: usize = 1024;
/// Confirmed members a cluster needs before it can match anything.
pub const MIN_SUPPORT: u64 = 5;
/// Per-dimension variance floor so a tight cluster cannot divide by zero.
pub const VARIANCE_FLOOR: f64 = 0.01;
/// A confirmed vector joins the nearest cluster within this standardized distance.
pub const JOIN_DISTANCE: f64 = 2.0;
/// A cluster not reinforced by confirmed evidence for this long expires (30 days).
pub const RETENTION_SECS: u64 = 30 * 24 * 3600;

/// Independently confirmed abuse; the only admissible evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// A measured red-team catalog session against this deployment.
    CatalogBot,
    /// An operator confirmed the session as abuse on review.
    OperatorConfirmed,
}

/// Stable cluster handle; never reused within a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClusterId(pub u64);

/// The nearest matchable cluster for a probe vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    /// Nearest cluster with at least [`MIN_SUPPORT`] members.
    pub cluster: ClusterId,
    /// `exp(-d^2 / 2)` of the standardized distance, in `[0, 1]`.
    pub similarity: f64,
}

/// Read-only aggregate view for the admin panel; no member data.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterView {
    /// Cluster handle.
    pub id: ClusterId,
    /// Confirmed members aggregated.
    pub support: u64,
    /// Members admitted from the red-team catalog.
    pub catalog_support: u64,
    /// Last confirmed admission time (caller clock, seconds).
    pub last_confirmed: u64,
    /// Matches recorded within the store's match history.
    pub matches_held: usize,
}

#[derive(Debug, Clone)]
struct Cluster {
    count: u64,
    catalog: u64,
    mean: [f64; FEATURE_DIM],
    m2: [f64; FEATURE_DIM],
    last_confirmed: u64,
    matches: VecDeque<u64>,
}

impl Cluster {
    fn variance(&self, dim: usize) -> f64 {
        let sample = if self.count > 1 {
            self.m2[dim] / (self.count - 1) as f64
        } else {
            0.0
        };
        sample.max(0.0) + VARIANCE_FLOOR
    }

    /// Root-mean-square standardized distance over all dimensions.
    fn distance(&self, vector: &[f64; FEATURE_DIM]) -> f64 {
        let sum: f64 = (0..FEATURE_DIM)
            .map(|dim| {
                let delta = vector[dim] - self.mean[dim];
                delta * delta / self.variance(dim)
            })
            .sum();
        (sum / FEATURE_DIM as f64).sqrt()
    }

    fn admit(&mut self, vector: &[f64; FEATURE_DIM]) {
        self.count = self.count.saturating_add(1);
        let n = self.count as f64;
        for (dim, value) in vector.iter().enumerate() {
            let delta = value - self.mean[dim];
            self.mean[dim] += delta / n;
            self.m2[dim] += delta * (value - self.mean[dim]);
        }
    }
}

/// Map a standardized distance to a similarity in `[0, 1]`.
pub fn similarity(distance: f64) -> f64 {
    (-0.5 * distance * distance).exp().clamp(0.0, 1.0)
}

/// Bounded confirmed-abuse cluster store. Single owner; the runtime owns the
/// clock and supplies monotonic seconds.
#[derive(Debug, Default)]
pub struct PatternStore {
    clusters: BTreeMap<ClusterId, Cluster>,
    next_id: u64,
    clock: u64,
}

fn widen(vector: &FeatureVector) -> Result<[f64; FEATURE_DIM], SimilarityError> {
    let mut wide = [0.0; FEATURE_DIM];
    for (slot, value) in wide.iter_mut().zip(vector) {
        if !value.is_finite() {
            return Err(SimilarityError::Nonfinite);
        }
        *slot = f64::from(*value);
    }
    Ok(wide)
}

impl PatternStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn tick(&mut self, now: u64) -> Result<(), SimilarityError> {
        if now < self.clock {
            return Err(SimilarityError::ClockRegressed);
        }
        self.clock = now;
        Ok(())
    }

    /// Admit one confirmed-abuse vector. It joins the nearest cluster within
    /// [`JOIN_DISTANCE`], otherwise starts a new cluster. Validation happens
    /// before any state changes.
    pub fn admit(
        &mut self,
        vector: &FeatureVector,
        evidence: Evidence,
        now: u64,
    ) -> Result<ClusterId, SimilarityError> {
        let wide = widen(vector)?;
        if now < self.clock {
            return Err(SimilarityError::ClockRegressed);
        }
        let nearest = self
            .clusters
            .iter()
            .map(|(id, cluster)| (*id, cluster.distance(&wide)))
            .filter(|(_, distance)| *distance <= JOIN_DISTANCE)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let id = match nearest {
            Some((id, _)) => id,
            None => {
                if self.clusters.len() >= MAX_CLUSTERS {
                    return Err(SimilarityError::Capacity);
                }
                let id = ClusterId(self.next_id);
                self.next_id += 1;
                self.clusters.insert(
                    id,
                    Cluster {
                        count: 0,
                        catalog: 0,
                        mean: [0.0; FEATURE_DIM],
                        m2: [0.0; FEATURE_DIM],
                        last_confirmed: now,
                        matches: VecDeque::new(),
                    },
                );
                id
            }
        };
        self.tick(now)?;
        let cluster = self
            .clusters
            .get_mut(&id)
            .ok_or(SimilarityError::UnknownCluster)?;
        cluster.admit(&wide);
        if evidence == Evidence::CatalogBot {
            cluster.catalog = cluster.catalog.saturating_add(1);
        }
        cluster.last_confirmed = now;
        Ok(id)
    }

    /// Nearest matchable cluster, or `None` when no cluster has enough support.
    /// Read-only: probing never changes the store.
    pub fn nearest(&self, vector: &FeatureVector) -> Result<Option<Match>, SimilarityError> {
        let wide = widen(vector)?;
        Ok(self
            .clusters
            .iter()
            .filter(|(_, cluster)| cluster.count >= MIN_SUPPORT)
            .map(|(id, cluster)| (*id, cluster.distance(&wide)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(cluster, distance)| Match {
                cluster,
                similarity: similarity(distance),
            }))
    }

    /// Record that a session matched `cluster` at `now`. Match history feeds
    /// recurrence only; it never changes the cluster's mean or variance.
    pub fn record_match(&mut self, cluster: ClusterId, now: u64) -> Result<(), SimilarityError> {
        if !self.clusters.contains_key(&cluster) {
            return Err(SimilarityError::UnknownCluster);
        }
        self.tick(now)?;
        let entry = self
            .clusters
            .get_mut(&cluster)
            .ok_or(SimilarityError::UnknownCluster)?;
        if entry.matches.len() >= MAX_MATCH_EVENTS {
            entry.matches.pop_front();
        }
        entry.matches.push_back(now);
        Ok(())
    }

    /// Matches recorded for `cluster` within `window` seconds before `now`,
    /// saturating at [`MAX_MATCH_EVENTS`].
    pub fn recurrence(
        &self,
        cluster: ClusterId,
        window: u64,
        now: u64,
    ) -> Result<u64, SimilarityError> {
        let entry = self
            .clusters
            .get(&cluster)
            .ok_or(SimilarityError::UnknownCluster)?;
        if now < self.clock {
            return Err(SimilarityError::ClockRegressed);
        }
        let since = now.saturating_sub(window);
        Ok(entry.matches.iter().filter(|at| **at >= since).count() as u64)
    }

    /// Drop clusters not reinforced within [`RETENTION_SECS`] and match times
    /// older than `max_window`. Returns the number of clusters removed.
    pub fn expire(&mut self, now: u64, max_window: u64) -> Result<usize, SimilarityError> {
        self.tick(now)?;
        let before = self.clusters.len();
        self.clusters
            .retain(|_, cluster| now.saturating_sub(cluster.last_confirmed) <= RETENTION_SECS);
        let since = now.saturating_sub(max_window);
        for cluster in self.clusters.values_mut() {
            while cluster.matches.front().is_some_and(|at| *at < since) {
                cluster.matches.pop_front();
            }
        }
        Ok(before - self.clusters.len())
    }

    /// Remove one cluster entirely (operator retirement or erasure). The
    /// runtime decides whether this needs dual control.
    pub fn retire(&mut self, cluster: ClusterId) -> Result<(), SimilarityError> {
        self.clusters
            .remove(&cluster)
            .map(|_| ())
            .ok_or(SimilarityError::UnknownCluster)
    }

    /// Aggregate views in cluster-id order.
    pub fn views(&self) -> Vec<ClusterView> {
        self.clusters
            .iter()
            .map(|(id, cluster)| ClusterView {
                id: *id,
                support: cluster.count,
                catalog_support: cluster.catalog,
                last_confirmed: cluster.last_confirmed,
                matches_held: cluster.matches.len(),
            })
            .collect()
    }

    /// Clusters held.
    pub fn len(&self) -> usize {
        self.clusters.len()
    }

    /// Whether the store holds no cluster.
    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }
}

#[cfg(test)]
mod tests;
