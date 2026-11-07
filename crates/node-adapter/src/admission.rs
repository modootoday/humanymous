//! Process-wide body/decision ownership budget. Exhaustion is a transport error,
//! never an abuse score or motor/speed requirement.
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) struct Budget {
    active: AtomicUsize,
    limit: usize,
}
impl Budget {
    const fn new(limit: usize) -> Self {
        Self {
            active: AtomicUsize::new(0),
            limit,
        }
    }
    pub fn acquire(&self) -> napi::Result<Permit<'_>> {
        self.active
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.limit).then_some(count + 1)
            })
            .map_err(|_| crate::envelope::error("adapter capacity exhausted"))?;
        Ok(Permit(self))
    }
}
pub(crate) static HOST: Budget = Budget::new(64);
pub(crate) struct Permit<'a>(&'a Budget);
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_is_never_evicted_and_drop_restores_it() {
        let budget = Budget::new(2);
        let a = budget.acquire().unwrap();
        let b = budget.acquire().unwrap();
        assert!(budget.acquire().is_err());
        drop(a);
        assert!(budget.acquire().is_ok());
        drop(b);
        assert_eq!(budget.active.load(Ordering::Acquire), 0);
    }
}
