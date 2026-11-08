//! Ordered source translations. No production threshold installation.
use super::{Error, increment, probability};
use crate::model::inference::Prediction;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Calibration {
    pub(crate) theta: f32, pub(crate) budget: f32, pub(crate) gamma: f32,
    pub(crate) fp_ema: f32, pub(crate) humans: u64, pub(crate) bots: u64,
}
impl Calibration {
    pub(crate) fn new(budget: f32, theta: f32, gamma: f32) -> Result<Self, Error> {
        if [budget, theta, gamma].iter().any(|v| !v.is_finite()) { return Err(Error::Nonfinite); }
        let budget = if budget <= 0.0 || budget >= 1.0 { 0.005 } else { budget };
        let theta = if theta <= 0.0 || theta >= 1.0 { 0.5 } else { theta };
        let gamma = if gamma <= 0.0 || gamma > 0.5 { 0.01 } else { gamma };
        Ok(Self { theta, budget, gamma, fp_ema: budget, humans: 0, bots: 0 })
    }
    pub(crate) fn observe(&mut self, human: bool, p: f32) -> Result<(), Error> {
        probability(p)?;
        if !human { self.bots = increment(self.bots)?; return Ok(()); }
        let humans = increment(self.humans)?;
        let flagged = if p >= self.theta { 1.0 } else { 0.0 };
        let mut theta = self.theta + self.gamma * (flagged - self.budget);
        theta = theta.clamp(0.5, 1.0);
        let fp_ema = self.fp_ema + 0.02_f32 * (flagged - self.fp_ema);
        if !theta.is_finite() || !fp_ema.is_finite() { return Err(Error::Nonfinite); }
        self.theta = theta; self.fp_ema = fp_ema; self.humans = humans;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageHinkley {
    pub(crate) delta: f64, pub(crate) lambda: f64, pub(crate) n: u64,
    pub(crate) mean: f64, pub(crate) cumulative: f64, pub(crate) minimum: f64,
    pub(crate) alarm: bool,
}
impl PageHinkley {
    pub(crate) fn new(delta: f64, lambda: f64) -> Result<Self, Error> {
        if !delta.is_finite() || !lambda.is_finite() { return Err(Error::Nonfinite); }
        Ok(Self { delta: if delta <= 0.0 { 0.005 } else { delta },
            lambda: if lambda <= 0.0 { 5.0 } else { lambda }, n: 0,
            mean: 0.0, cumulative: 0.0, minimum: 0.0, alarm: false })
    }
    pub(crate) fn update(&mut self, x: f64) -> Result<bool, Error> {
        if !x.is_finite() || !(0.0..=1.0).contains(&x) { return Err(Error::Nonfinite); }
        let n = increment(self.n)?;
        let mean = self.mean + (x - self.mean) / n as f64;
        let cumulative = self.cumulative + (x - mean - self.delta);
        let minimum = if cumulative < self.minimum { cumulative } else { self.minimum };
        if [mean, cumulative, minimum].iter().any(|v| !v.is_finite()) { return Err(Error::Nonfinite); }
        let alarm = (cumulative - minimum) > self.lambda;
        *self = Self { n, mean, cumulative, minimum, alarm, ..self.clone() };
        Ok(alarm)
    }
    pub(crate) fn reset(&mut self) { self.n = 0; self.mean = 0.0; self.cumulative = 0.0; self.minimum = 0.0; self.alarm = false; }
}

pub(crate) fn psi(reference: &[f64], current: &[f64]) -> Result<f64, Error> {
    if reference.len() != current.len() || reference.is_empty() || reference.len() > 64 { return Err(Error::Shape); }
    let mut result = 0.0;
    for (&r, &c) in reference.iter().zip(current) {
        if !r.is_finite() || !c.is_finite() || !(0.0..=1.0).contains(&r) || !(0.0..=1.0).contains(&c) { return Err(Error::Nonfinite); }
        let r = if r < 1e-6 { 1e-6 } else { r };
        let c = if c < 1e-6 { 1e-6 } else { c };
        result += (c - r) * (c / r).ln();
        if !result.is_finite() { return Err(Error::Nonfinite); }
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Drift {
    pub(crate) mimic: PageHinkley, pub(crate) oracle: PageHinkley,
    pub(crate) last_psi: f64, pub(crate) events: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DriftReading { pub(crate) mimic: bool, pub(crate) oracle: bool, pub(crate) covariate: bool, pub(crate) psi: f64, pub(crate) fired: bool }
impl Drift {
    pub(crate) fn new() -> Result<Self, Error> { Ok(Self { mimic: PageHinkley::new(0.01, 6.0)?, oracle: PageHinkley::new(0.01, 6.0)?, last_psi: 0.0, events: 0 }) }
    pub(crate) fn update_psi(&mut self, value: f64) -> Result<(), Error> {
        if !value.is_finite() || value < 0.0 { return Err(Error::Nonfinite); }
        self.last_psi = value; Ok(())
    }
    /// Snapshot is read-only. Legacy Gate incremented events on every dashboard read.
    pub(crate) fn snapshot(&self) -> DriftReading {
        let (m, o, c) = (self.mimic.alarm, self.oracle.alarm, self.last_psi > 0.25);
        DriftReading { mimic: m, oracle: o, covariate: c, psi: self.last_psi,
            fired: u8::from(m) + u8::from(o) + u8::from(c) >= 2 }
    }
    pub(crate) fn record_event(&mut self) -> Result<DriftReading, Error> {
        let reading = self.snapshot();
        if reading.fired { self.events = increment(self.events)?; }
        Ok(reading)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PassGuard { pub(crate) fraction: f32, pub(crate) baseline: f32, pub(crate) ema: f32, pub(crate) attempts: u64 }
impl PassGuard {
    pub(crate) fn new(fraction: f32, baseline: f32) -> Result<Self, Error> {
        if !fraction.is_finite() || !baseline.is_finite() { return Err(Error::Nonfinite); }
        let fraction = if fraction <= 0.0 || fraction > 1.0 { 0.4 } else { fraction };
        let baseline = if baseline <= 0.0 || baseline >= 1.0 { 0.6 } else { baseline };
        Ok(Self { fraction, baseline, ema: baseline, attempts: 0 })
    }
    pub(crate) fn observe(&mut self, solved: bool) -> Result<(), Error> {
        let attempts = increment(self.attempts)?;
        let sample = if solved { 1.0 } else { 0.0 };
        let ema = self.ema + 0.01_f32 * (sample - self.ema);
        if !ema.is_finite() { return Err(Error::Nonfinite); }
        self.ema = ema; self.attempts = attempts; Ok(())
    }
    pub(crate) fn anomalous(&self) -> bool { self.ema > self.baseline + 0.25 }
    pub(crate) fn admit_count(&self, others: usize) -> Result<usize, Error> {
        if others > 100_000 { return Err(Error::Capacity); }
        if self.anomalous() { return Ok(0); }
        if self.fraction >= 1.0 { return Ok(others); }
        let result = (others as f32 * self.fraction) / (1.0 - self.fraction);
        if !result.is_finite() || result > 100_000.0 { return Err(Error::Capacity); }
        Ok(result as usize)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Shadow {
    pub(crate) n: u64, pub(crate) both: u64, pub(crate) sum_abs: f64,
    pub(crate) agree: u64, pub(crate) shadow_more: u64, pub(crate) active_more: u64,
    pub(crate) active_abstained: u64, pub(crate) shadow_abstained: u64,
}
impl Shadow {
    pub(crate) fn observe(&mut self, active: Prediction, shadow: Prediction, theta: f32) -> Result<(), Error> {
        probability(theta)?;
        if !active.abstain { probability(active.p_bot)?; }
        if !shadow.abstain { probability(shadow.p_bot)?; }
        let mut next = self.clone(); next.n = increment(next.n)?;
        if active.abstain && !shadow.abstain { next.active_abstained = increment(next.active_abstained)?; }
        if shadow.abstain && !active.abstain { next.shadow_abstained = increment(next.shadow_abstained)?; }
        if !active.abstain && !shadow.abstain {
            next.both = increment(next.both)?;
            // Subtract/abs in f32 BEFORE converting to f64, as the original.
            next.sum_abs += f64::from((active.p_bot - shadow.p_bot).abs());
            match (active.p_bot >= theta, shadow.p_bot >= theta) {
                (a, s) if a == s => next.agree = increment(next.agree)?,
                (false, true) => next.shadow_more = increment(next.shadow_more)?,
                _ => next.active_more = increment(next.active_more)?,
            }
        }
        *self = next; Ok(())
    }
    pub(crate) fn mean_delta(&self) -> Option<f64> { (self.both > 0).then(|| self.sum_abs / self.both as f64) }
    pub(crate) fn agreement(&self) -> Option<f64> { (self.both > 0).then(|| self.agree as f64 / self.both as f64) }
}
