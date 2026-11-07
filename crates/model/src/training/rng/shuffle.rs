// Copyright 2009 The Go Authors. All rights reserved.
// Adapted from Go1.26.3 math/rand/rand.go Shuffle/internal int31n; see LICENSE.
use super::{DrawSource, MAX_SAMPLES, RngError, source};

/// Lemire reduction from Go's INTERNAL int31n, not its public Int31n method.
pub(super) fn bounded(source: &mut impl DrawSource, bound: u32) -> Result<usize, RngError> {
    if bound == 0 || bound as usize > MAX_SAMPLES {
        return Err(RngError::SampleLimit);
    }
    let mut product = u64::from(source::uint32(source)?) * u64::from(bound);
    let mut low = product as u32;
    if low < bound {
        let threshold = bound.wrapping_neg() % bound;
        while low < threshold {
            product = u64::from(source::uint32(source)?) * u64::from(bound);
            low = product as u32;
        }
    }
    Ok((product >> 32) as usize)
}

pub(super) fn apply(source: &mut impl DrawSource, order: &mut [usize]) -> Result<(), RngError> {
    if order.len() > MAX_SAMPLES {
        return Err(RngError::SampleLimit);
    }
    for index in (1..order.len()).rev() {
        let other = bounded(source, (index + 1) as u32)?;
        order.swap(index, other);
    }
    Ok(())
}
