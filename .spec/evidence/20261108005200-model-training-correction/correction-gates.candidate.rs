//! Bounded offline quality evidence bound to exact model and reviewed records.
//! No model activation, file I/O, implicit source authentication or score changes.
use super::{Error, lifecycle::ModelDigest};
use crate::behavior::features::schema_hash;
use crate::model::{inference::{Mlp, Predictor}, training::{record::{Sample, Admission, Limits},
    evaluate::{Counts, LabeledPrediction, evaluate}}};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Policy {
    pub(crate) max_human_fpr: f64, pub(crate) min_bot_tpr: f64,
    pub(crate) min_cohort_humans: usize, pub(crate) buckets: usize,
}
impl Default for Policy {
    fn default() -> Self { Self { max_human_fpr: 0.005, min_bot_tpr: 0.80, min_cohort_humans: 20, buckets: 5 } }
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GateReport {
    model: ModelDigest, evidence: ModelDigest, allowed: bool,
    pub(crate) counts: Counts, pub(crate) cohorts: BTreeMap<String, (usize, usize)>,
    pub(crate) aut: f32, pub(crate) max_train: i64, pub(crate) min_test: i64,
    pub(crate) missing_cohorts: Vec<String>, pub(crate) exceeded_cohorts: Vec<String>,
    pub(crate) bot_regression: bool,
}
impl GateReport {
    pub(crate) fn model(&self) -> ModelDigest { self.model }
    pub(crate) fn evidence(&self) -> ModelDigest { self.evidence }
    pub(crate) fn allowed(&self) -> bool { self.allowed }
}

/// Stable temporal partition, with explicit refusal for a tied cross-boundary
/// window. The legacy implementation allowed the tie despite its future claim.
pub(crate) fn temporal_ids(samples: &[Sample], fraction: f64) -> Result<(Vec<usize>, Vec<usize>), Error> {
    if !fraction.is_finite() || fraction <= 0.0 || fraction >= 1.0 { return Err(Error::Config); }
    if samples.len() < 2 || samples.len() > 100_000 { return Err(Error::Capacity); }
    if samples.iter().any(|s| s.record.ts.is_none()) { return Err(Error::Time); }
    let mut ids: Vec<_> = (0..samples.len()).collect();
    ids.sort_by_key(|&i| samples[i].record.ts);
    let cut = (samples.len() - (samples.len() as f64 * fraction) as usize).max(1);
    if cut >= ids.len() { return Err(Error::Support); }
    if samples[ids[cut-1]].record.ts >= samples[ids[cut]].record.ts { return Err(Error::Time); }
    let test = ids.split_off(cut); Ok((ids, test))
}

fn checked<'a>(samples: &'a [Sample], hasher: &mut Sha256,
    cohort_names: &mut BTreeSet<&'a str>, required: &mut BTreeSet<&'a str>) -> Result<(i64, i64), Error>
{
    let mut minimum = i64::MAX; let mut maximum = i64::MIN;
    for sample in samples {
        if sample.record.source.as_ref().is_some_and(|v| v.len() > 128)
            || sample.record.cohort.as_ref().is_some_and(|v| v.len() > 128) { return Err(Error::Capacity); }
        let fresh = sample.record.clone().sample(&schema_hash(), Limits::default()).map_err(|_| Error::Shape)?;
        if fresh.admission != Admission::ReviewRequired || sample.admission != fresh.admission { return Err(Error::Authority); }
        if sample.features.iter().zip(fresh.features).any(|(a,b)| a.to_bits() != b.to_bits()) { return Err(Error::Shape); }
        let cohort = sample.record.cohort.as_deref().ok_or(Error::Missing)?;
        let cohort = if cohort.is_empty() { "default" } else { cohort };
        cohort_names.insert(cohort); if cohort_names.len() > 64 { return Err(Error::Capacity); }
        if sample.record.label == 0 { required.insert(cohort); }
        let ts = sample.record.ts.ok_or(Error::Time)?; minimum = minimum.min(ts); maximum = maximum.max(ts);
        let raw = serde_json::to_vec(&sample.record).map_err(|_| Error::Shape)?;
        hasher.update((raw.len() as u64).to_be_bytes()); hasher.update(raw);
    }
    Ok((minimum, maximum))
}

pub(crate) fn assess(bytes: &[u8], train: &[Sample], test: &[Sample], policy: Policy) -> Result<GateReport, Error> {
    if [policy.max_human_fpr, policy.min_bot_tpr].iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) { return Err(Error::Config); }
    if policy.min_cohort_humans == 0 || policy.min_cohort_humans > 100_000 || policy.buckets == 0 || policy.buckets > 64 { return Err(Error::Config); }
    let n = train.len().checked_add(test.len()).ok_or(Error::Capacity)?;
    if !(100..=100_000).contains(&n) || train.is_empty() || test.is_empty() || bytes.is_empty() || bytes.len() > 1<<20 { return Err(Error::Capacity); }
    let predictor = Mlp::from_json(bytes).map_err(|_| Error::Shape)?;
    if predictor.schema_hash() != schema_hash() { return Err(Error::Schema); }
    let mut hash = Sha256::new(); hash.update(b"humanymous.model.offline-quality.v1\0");
    let model: ModelDigest = Sha256::digest(bytes).into(); hash.update(model);
    hash.update(policy.max_human_fpr.to_bits().to_be_bytes()); hash.update(policy.min_bot_tpr.to_bits().to_be_bytes());
    hash.update((policy.min_cohort_humans as u64).to_be_bytes()); hash.update((policy.buckets as u64).to_be_bytes());
    let mut names = BTreeSet::new(); let mut required = BTreeSet::new();
    hash.update((train.len() as u64).to_be_bytes());
    let (_, max_train) = checked(train, &mut hash, &mut names, &mut required)?;
    hash.update((test.len() as u64).to_be_bytes());
    let (min_test, _) = checked(test, &mut hash, &mut names, &mut required)?;
    if max_train >= min_test { return Err(Error::Time); }
    let mut ids: Vec<_> = (0..test.len()).collect(); ids.sort_by_key(|&i| test[i].record.ts);
    let pairs: Vec<_> = ids.iter().map(|&i| LabeledPrediction {
        label: test[i].record.label, prediction: predictor.predict(&test[i].features),
    }).collect();
    let counts = evaluate(&pairs, 100_000).map_err(|_| Error::Nonfinite)?;
    if counts.humans == 0 || counts.bots == 0 { return Err(Error::Support); }
    let mut cohorts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (&id, pair) in ids.iter().zip(&pairs) {
        if pair.label == 0 {
            let name = test[id].record.cohort.as_deref().ok_or(Error::Missing)?;
            let entry = cohorts.entry(if name.is_empty() { "default" } else { name }.to_owned()).or_default();
            entry.0 += 1; entry.1 += usize::from(pair.prediction.p_bot >= 0.5);
        }
    }
    let missing_cohorts: Vec<_> = required.into_iter().filter(|name| cohorts.get(*name)
        .is_none_or(|(n,_)| *n < policy.min_cohort_humans)).map(str::to_owned).collect();
    let exceeded_cohorts: Vec<_> = cohorts.iter().filter(|(_, (n,fp))| f64::from(*fp as f32 / *n as f32) > policy.max_human_fpr)
        .map(|(name,_)| name.clone()).collect();
    let bot_regression = f64::from(counts.bot_tpr().ok_or(Error::Support)?) < policy.min_bot_tpr;
    let mut sum = 0.0_f32; let mut used = 0_u32;
    for bucket in 0..policy.buckets {
        let lo = bucket * pairs.len() / policy.buckets; let hi = (bucket+1) * pairs.len() / policy.buckets;
        if lo < hi { sum += evaluate(&pairs[lo..hi], 100_000).map_err(|_| Error::Nonfinite)?.accuracy().ok_or(Error::Support)?; used += 1; }
    }
    let aut = sum / used as f32;
    let allowed = missing_cohorts.is_empty() && exceeded_cohorts.is_empty() && !bot_regression;
    Ok(GateReport { model, evidence: hash.finalize().into(), allowed, counts, cohorts, aut,
        max_train, min_test, missing_cohorts, exceeded_cohorts, bot_regression })
}
