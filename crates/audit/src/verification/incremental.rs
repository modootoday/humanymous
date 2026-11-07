//! Compatibility priority and prefix coverage, separate from interleaved work.
//! This accumulator is diagnostic state, never authenticated EOF or authority.
use super::{Mismatch, Report, Scope, checks::Failure};
use std::io;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Work {
    pub record_attempts: usize,
    pub record_successes: usize,
    pub base_attempts: usize,
    pub base_successes: usize,
    pub witness_attempts: usize,
    pub witness_successes: usize,
}

pub(super) struct Accumulator {
    hmac_checked: bool,
    records: usize,
    checkpoints: usize,
    witnesses: usize,
    first_record: Option<Failure>,
    first_base: Option<Failure>,
    first_witness: Option<Failure>,
    work: Work,
    poisoned: bool,
}

fn increment(value: &mut usize) -> io::Result<()> {
    *value = value.checked_add(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::FileTooLarge,
            "verification counter resource limit exceeded",
        )
    })?;
    Ok(())
}

impl Accumulator {
    pub fn new(hmac_checked: bool) -> Self {
        Self {
            hmac_checked,
            records: 0,
            checkpoints: 0,
            witnesses: 0,
            first_record: None,
            first_base: None,
            first_witness: None,
            work: Work::default(),
            poisoned: false,
        }
    }

    fn begin(&mut self) -> io::Result<()> {
        if self.poisoned {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "verification accumulator failed",
            ));
        }
        self.poisoned = true;
        Ok(())
    }

    pub fn record(&mut self, failure: Option<Failure>) -> io::Result<()> {
        self.begin()?;
        increment(&mut self.work.record_attempts)?;
        if failure.is_none() {
            increment(&mut self.work.record_successes)?;
        }
        if self.first_record.is_none() {
            match failure {
                Some(failure) => self.first_record = Some(failure),
                None => increment(&mut self.records)?,
            }
        }
        self.poisoned = false;
        Ok(())
    }

    pub fn checkpoint(&mut self, failure: Option<Failure>) -> io::Result<()> {
        self.begin()?;
        increment(&mut self.work.base_attempts)?;
        if failure.is_none() {
            increment(&mut self.work.base_successes)?;
        }
        if self.first_base.is_none() {
            match failure {
                Some(failure) => self.first_base = Some(failure),
                None => increment(&mut self.checkpoints)?,
            }
        }
        self.poisoned = false;
        Ok(())
    }

    pub fn witness(&mut self, at: u64, valid: bool) -> io::Result<()> {
        self.begin()?;
        increment(&mut self.work.witness_attempts)?;
        if valid {
            increment(&mut self.work.witness_successes)?;
        }
        if self.first_witness.is_none() {
            if valid {
                increment(&mut self.witnesses)?;
            } else {
                self.first_witness = Some(Failure::witness(at));
            }
        }
        self.poisoned = false;
        Ok(())
    }

    pub fn actual_work(&self) -> &Work {
        &self.work
    }

    /// Caller must finish the owning scan's EOF/source checks separately.
    /// No output from this diagnostic-only method permits business effects.
    pub fn finish(mut self) -> io::Result<(Report, Work)> {
        self.begin()?;
        let mut scope = Scope {
            records: self.records,
            hmac_checked: self.hmac_checked,
            checkpoints: 0,
            witnessed_checkpoints: 0,
        };
        let report = if self.work.record_attempts == 0 {
            Report::failure(Mismatch::EmptyChain, 0, "empty chain is not a pass", scope)
        } else if let Some(failure) = self.first_record {
            failure.report(scope)
        } else {
            scope.checkpoints = self.checkpoints;
            if let Some(failure) = self.first_base {
                failure.report(scope)
            } else {
                scope.witnessed_checkpoints = self.witnesses;
                match self.first_witness {
                    Some(failure) => failure.report(scope),
                    None => Report::success(scope),
                }
            }
        };
        Ok((report, self.work))
    }
}

#[cfg(test)]
mod tests;
