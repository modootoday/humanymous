use crate::history::{
    Unavailable,
    anomaly::{Config, Detector, Measurement},
};
use std::time::Duration;

pub(super) struct Population {
    detector: Detector,
}
impl Population {
    pub(super) fn new(config: Config) -> Self {
        Self {
            detector: Detector::new(config),
        }
    }
    pub(super) fn observe(
        &mut self,
        count: usize,
        now: Duration,
    ) -> Result<Measurement, Unavailable> {
        self.detector.observe("pop", count as f64, now)
    }
    pub(super) fn gc(&mut self, now: Duration, ttl: Duration) -> Result<(), Unavailable> {
        self.detector.gc(now, ttl).map(|_| ())
    }
    pub(super) fn reset(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.detector.reset(now)
    }
}
