use super::model::{Limits, SelectionError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    time::{Duration, Instant},
};

/// Cooperative observation only: a blocking read cannot be forcibly interrupted.
pub trait Clock {
    fn elapsed(&mut self) -> Duration;
}

pub struct SystemClock(Instant);
impl Default for SystemClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for SystemClock {
    fn elapsed(&mut self) -> Duration {
        self.0.elapsed()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadStatus {
    Complete,
    FileLimit,
    AggregateLimit,
    Deadline,
    ClockRegression,
    IoError,
    InvalidReadCount,
    ArithmeticOverflow,
    ProviderRefused,
    ProviderUnknown,
    Changed,
}

#[derive(Debug, Serialize)]
pub struct ContentRead {
    pub status: ReadStatus,
    pub actual_read_bytes: u64,
    pub read_count_known: bool,
    pub probe_bytes: u64,
    pub sha256: Option<String>,
    pub error_kind: Option<String>,
}
impl ContentRead {
    pub fn complete(&self) -> bool {
        self.status == ReadStatus::Complete && self.sha256.is_some()
    }
    pub(crate) fn refused(status: ReadStatus, kind: &str) -> Self {
        Self {
            status,
            actual_read_bytes: 0,
            read_count_known: true,
            probe_bytes: 0,
            sha256: None,
            error_kind: Some(kind.into()),
        }
    }
}

pub struct ReadBudget {
    pub(crate) limits: Limits,
    actual: u64,
    probes: u64,
    last_elapsed: Duration,
    count_known: bool,
}

impl ReadBudget {
    pub fn new(limits: Limits) -> Result<Self, SelectionError> {
        limits.validate()?;
        Ok(Self {
            limits,
            actual: 0,
            probes: 0,
            last_elapsed: Duration::ZERO,
            count_known: true,
        })
    }
    pub fn actual_read_bytes(&self) -> u64 {
        self.actual
    }
    pub fn probe_bytes(&self) -> u64 {
        self.probes
    }
    pub fn read_count_known(&self) -> bool {
        self.count_known
    }
    pub fn elapsed(&self) -> Duration {
        self.last_elapsed
    }

    pub(crate) fn check<C: Clock>(&mut self, clock: &mut C) -> Result<(), ReadStatus> {
        let now = clock.elapsed();
        if now < self.last_elapsed {
            return Err(ReadStatus::ClockRegression);
        }
        self.last_elapsed = now;
        if now >= self.limits.max_elapsed {
            return Err(ReadStatus::Deadline);
        }
        Ok(())
    }

    fn account(&mut self, bytes: u64, probe: bool) -> Result<(), ReadStatus> {
        let actual = self
            .actual
            .checked_add(bytes)
            .ok_or(ReadStatus::ArithmeticOverflow)?;
        let probes = self
            .probes
            .checked_add(if probe { bytes } else { 0 })
            .ok_or(ReadStatus::ArithmeticOverflow)?;
        if actual > self.limits.max_total_read_bytes {
            return Err(ReadStatus::AggregateLimit);
        }
        self.actual = actual;
        self.probes = probes;
        Ok(())
    }

    pub fn read<R: Read, C: Clock>(&mut self, reader: &mut R, clock: &mut C) -> ContentRead {
        let start = self.actual;
        let probe_start = self.probes;
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut error_kind = None;
        let status = loop {
            if !self.count_known {
                break ReadStatus::ProviderUnknown;
            }
            if let Err(status) = self.check(clock) {
                break status;
            }
            let aggregate_remaining = self.limits.max_total_read_bytes - self.actual;
            if aggregate_remaining == 0 {
                break ReadStatus::AggregateLimit;
            }
            let file_bytes = self.actual - start;
            let file_remaining = self.limits.max_file_bytes.saturating_sub(file_bytes);
            let probing = file_remaining == 0;
            let requested = if probing { 1 } else { file_remaining };
            let count = requested.min(aggregate_remaining).min(buffer.len() as u64) as usize;
            let read = reader.read(&mut buffer[..count]);
            let bytes = match read {
                Ok(n) if n <= count => n,
                Ok(_) => {
                    self.count_known = false;
                    break ReadStatus::InvalidReadCount;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    if let Err(status) = self.check(clock) {
                        break status;
                    }
                    continue;
                }
                Err(e) => {
                    error_kind = Some(format!("{:?}", e.kind()));
                    break self.check(clock).err().unwrap_or(ReadStatus::IoError);
                }
            };
            if let Err(status) = self.account(bytes as u64, probing) {
                break status;
            }
            if let Err(status) = self.check(clock) {
                break status;
            }
            if bytes == 0 {
                break ReadStatus::Complete;
            }
            if probing {
                break ReadStatus::FileLimit;
            }
            hash.update(&buffer[..bytes]);
        };
        ContentRead {
            status,
            actual_read_bytes: self.actual - start,
            read_count_known: self.count_known,
            probe_bytes: self.probes - probe_start,
            sha256: (status == ReadStatus::Complete).then(|| format!("{:x}", hash.finalize())),
            error_kind,
        }
    }
}

#[cfg(test)]
mod accounting_tests {
    use super::*;
    #[test]
    fn counters_reject_overflow_without_wrapping() {
        let mut budget = ReadBudget::new(Limits::default()).unwrap();
        budget.limits.max_total_read_bytes = u64::MAX;
        budget.actual = u64::MAX;
        assert_eq!(
            budget.account(1, false),
            Err(ReadStatus::ArithmeticOverflow)
        );
        assert_eq!(budget.actual, u64::MAX);
    }
}
