//! Checked fixed-capacity proof descriptors, preserving canonical proof order.
const CAPACITY: usize = 65;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Range {
    pub start: u64,
    pub size: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Plan {
    total: u64,
    ranges: [Range; CAPACITY],
    len: usize,
}
fn split(size: u64) -> u64 {
    debug_assert!(size >= 2);
    1 << (u64::BITS - 1 - (size - 1).leading_zeros())
}
impl Plan {
    fn new(total: u64) -> Self {
        Self {
            total,
            ranges: [Range { start: 0, size: 0 }; CAPACITY],
            len: 0,
        }
    }
    fn push(&mut self, start: u64, size: u64) -> Option<()> {
        if size == 0 || start.checked_add(size)? > self.total || self.len == CAPACITY {
            return None;
        }
        self.ranges[self.len] = Range { start, size };
        self.len += 1;
        Some(())
    }
    pub fn inclusion(index: u64, size: u64) -> Option<Self> {
        if index >= size {
            return None;
        }
        let mut plan = Self::new(size);
        plan.inclusion_at(index, 0, size)?;
        Some(plan)
    }
    fn inclusion_at(&mut self, index: u64, offset: u64, size: u64) -> Option<()> {
        if size <= 1 {
            return Some(());
        }
        let k = split(size);
        let right = offset.checked_add(k)?;
        if index < k {
            self.inclusion_at(index, offset, k)?;
            self.push(right, size - k)
        } else {
            self.inclusion_at(index - k, right, size - k)?;
            self.push(offset, k)
        }
    }
    pub fn consistency(old: u64, new: u64) -> Option<Self> {
        if old == 0 || old >= new {
            return None;
        }
        let mut plan = Self::new(new);
        plan.consistency_at(old, 0, new, true)?;
        Some(plan)
    }
    fn consistency_at(&mut self, old: u64, offset: u64, size: u64, complete: bool) -> Option<()> {
        if old == size {
            return if complete {
                Some(())
            } else {
                self.push(offset, size)
            };
        }
        let k = split(size);
        let right = offset.checked_add(k)?;
        if old <= k {
            self.consistency_at(old, offset, k, complete)?;
            self.push(right, size - k)
        } else {
            self.consistency_at(old - k, right, size - k, false)?;
            self.push(offset, k)
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn visits(&self) -> impl Iterator<Item = (usize, Range)> + '_ {
        let mut order = [0; CAPACITY];
        for (index, slot) in order[..self.len].iter_mut().enumerate() {
            *slot = index;
        }
        order[..self.len].sort_unstable_by_key(|&index| self.ranges[index].start);
        order
            .into_iter()
            .take(self.len)
            .map(|index| (index, self.ranges[index]))
    }
}

#[cfg(test)]
mod tests;
