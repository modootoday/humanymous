// Copyright 2009 The Go Authors. All rights reserved.
// Adapted from Go1.26.3 math/rand/rng.go and rand.go; see LICENSE.
use super::{DrawSource, MAX_DRAWS, RngError, tables::COOKED};

const MODULUS: i64 = (1_i64 << 31) - 1;
const MASK: u64 = (1_u64 << 63) - 1;

/// An offline diagnostic snapshot. State is predictable and is not secret material.
/// Snapshots produced by Rust are Rust-derived, not observations of Go internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Circular tap index in the Go state representation.
    pub tap: usize,
    /// Circular feedback index in the Go state representation.
    pub feed: usize,
    /// Two's-complement state words, represented as unsigned bits.
    pub words: [u64; 607],
    /// Primitive Uint64 calls consumed, including rejection draws.
    pub draws: u64,
    /// Fixed ceiling assigned when this stream was constructed.
    pub draw_limit: u64,
}

/// Explicitly seeded, single-stream Go1.26.3 generator; no global/default RNG.
#[derive(Clone, PartialEq, Eq)]
pub struct GoRng {
    state: Snapshot,
}

impl GoRng {
    /// Normalize a signed seed exactly as Go NewSource, with a finite draw budget.
    /// Seeding itself consumes no output draws. No model or runtime is initialized.
    pub fn new(seed: i64, draw_limit: u64) -> Result<Self, RngError> {
        if draw_limit == 0 || draw_limit > MAX_DRAWS {
            return Err(RngError::InvalidBudget);
        }
        let mut normalized = seed % MODULUS;
        if normalized < 0 {
            normalized += MODULUS;
        }
        if normalized == 0 {
            normalized = 89_482_311;
        }
        let mut x = normalized;
        let mut words = [0; 607];
        for index in -20_i32..607 {
            x = seed_step(x);
            if index >= 0 {
                let mut word = (x as u64) << 40;
                x = seed_step(x);
                word ^= (x as u64) << 20;
                x = seed_step(x);
                word ^= x as u64;
                word ^= COOKED[index as usize] as u64;
                words[index as usize] = word;
            }
        }
        Ok(Self {
            state: Snapshot { tap: 0, feed: 334, words, draws: 0, draw_limit },
        })
    }

    /// Copy the current diagnostic state, with no mutation or extra RNG draws.
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }

    /// Next raw Uint64 state output. Overflow is intentional two's-complement wrap.
    pub fn uint64(&mut self) -> Result<u64, RngError> {
        if self.state.draws == self.state.draw_limit {
            return Err(RngError::DrawLimit);
        }
        self.state.tap = (self.state.tap + 606) % 607;
        self.state.feed = (self.state.feed + 606) % 607;
        let value = self.state.words[self.state.feed]
            .wrapping_add(self.state.words[self.state.tap]);
        self.state.words[self.state.feed] = value;
        self.state.draws += 1;
        Ok(value)
    }

    /// Go Int63 output, preserving the underlying primitive draw count.
    pub fn int63(&mut self) -> Result<i64, RngError> {
        Ok((self.uint64()? & MASK) as i64)
    }

    /// Go Float64 conversion, including retry when integer rounding produces1.
    /// A budget failure restores the state from before this compound call.
    pub fn float64(&mut self) -> Result<f64, RngError> {
        let mut candidate = self.clone();
        let value = uniform(&mut candidate)?;
        *self = candidate;
        Ok(value)
    }
}

impl DrawSource for GoRng {
    fn draw_u64(&mut self) -> Result<u64, RngError> {
        self.uint64()
    }
}

fn seed_step(x: i64) -> i64 {
    let mut next = 48_271 * (x % 44_488) - 3_399 * (x / 44_488);
    if next < 0 {
        next += MODULUS;
    }
    next
}

pub(super) fn uint32(source: &mut impl DrawSource) -> Result<u32, RngError> {
    Ok(((source.draw_u64()? & MASK) >> 31) as u32)
}

pub(super) fn uniform(source: &mut impl DrawSource) -> Result<f64, RngError> {
    loop {
        let value = (source.draw_u64()? & MASK) as f64 / 9_223_372_036_854_775_808.0;
        if value != 1.0 {
            return Ok(value);
        }
    }
}
