//! Bounded eight-bucket frozen-reference PSI monitor; source-only candidate.
use super::{Error, increment, numeric::psi};
use crate::behavior::features::{FEATURE_DIM, FeatureVector};

const BUCKETS: usize = 8;
const MAX_WINDOW: usize = 4096;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeatureMonitor {
    window: usize, reference: Vec<FeatureVector>, frozen: bool,
    edges: [[f64; BUCKETS + 1]; FEATURE_DIM],
    fractions: [[f64; BUCKETS]; FEATURE_DIM],
    current: [[u32; BUCKETS]; FEATURE_DIM], n: usize,
    pub(crate) last_max: f64, observations: u64,
}

pub(crate) fn bucket(edges: &[f64; BUCKETS + 1], value: f64) -> usize {
    (0..BUCKETS).rev().find(|&b| value >= edges[b]).unwrap_or(0)
}

impl FeatureMonitor {
    pub(crate) fn new(window: i64) -> Result<Self, Error> {
        let window = if window <= 0 { 500 } else { usize::try_from(window).map_err(|_| Error::Capacity)? };
        if window > MAX_WINDOW { return Err(Error::Capacity); }
        let mut reference = Vec::new();
        reference.try_reserve_exact(window).map_err(|_| Error::Allocation)?;
        Ok(Self { window, reference, frozen: false, edges: [[0.0; 9]; FEATURE_DIM],
            fractions: [[0.0; 8]; FEATURE_DIM], current: [[0; 8]; FEATURE_DIM],
            n: 0, last_max: 0.0, observations: 0 })
    }
    pub(crate) fn ready(&self) -> bool { self.frozen }
    fn freeze(&mut self) {
        for dimension in 0..FEATURE_DIM {
            let mut sorted: Vec<f64> = self.reference.iter().map(|x| f64::from(x[dimension])).collect();
            // Match numeric ordering; do not invent a signed-zero ordering.
            sorted.sort_by(|a,b| if a < b { std::cmp::Ordering::Less }
                else if a > b { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal });
            for b in 0..=BUCKETS { self.edges[dimension][b] = sorted[b * (sorted.len() - 1) / BUCKETS]; }
            let mut counts = [0_u32; BUCKETS];
            for value in sorted { counts[bucket(&self.edges[dimension], value)] += 1; }
            for (b, count) in counts.iter().enumerate() { self.fractions[dimension][b] = f64::from(*count) / self.window as f64; }
        }
        self.frozen = true;
        self.reference = Vec::new();
    }
    pub(crate) fn observe(&mut self, features: &[f32]) -> Result<Option<f64>, Error> {
        if features.len() != FEATURE_DIM { return Err(Error::Shape); }
        if features.iter().any(|v| !v.is_finite()) { return Err(Error::Nonfinite); }
        let observations = increment(self.observations)?;
        if !self.frozen {
            let mut row = [0.0; FEATURE_DIM]; row.copy_from_slice(features);
            self.reference.push(row);
            if self.reference.len() == self.window { self.freeze(); }
            self.observations = observations;
            return Ok(None);
        }
        let mut counts = self.current;
        for (i, &value) in features.iter().enumerate() { counts[i][bucket(&self.edges[i], f64::from(value))] += 1; }
        let n = self.n + 1;
        if n < self.window {
            self.current = counts; self.n = n; self.observations = observations;
            return Ok(None);
        }
        let mut maximum = 0.0_f64;
        for (i, counts) in counts.iter().enumerate() {
            let fractions: [f64; BUCKETS] = std::array::from_fn(|b| f64::from(counts[b]) / n as f64);
            maximum = maximum.max(psi(&self.fractions[i], &fractions)?);
        }
        self.current = [[0; BUCKETS]; FEATURE_DIM]; self.n = 0;
        self.last_max = maximum; self.observations = observations;
        Ok(Some(maximum))
    }
}
