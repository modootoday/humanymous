// Copyright 2009 The Go Authors. All rights reserved.
// Adapted from Go1.26.3 math/rand/normal.go; see LICENSE.
use super::{DrawSource, RngError, source, tables::{FN, KN, WN}};

pub(super) const RN: f64 = 3.442619855899;

pub(super) fn sample(source: &mut impl DrawSource) -> Result<f64, RngError> {
    loop {
        let j = source::uint32(source)? as i32;
        let index = (j & 0x7f) as usize;
        let mut x = f64::from(j) * f64::from(WN[index]);
        if j.unsigned_abs() < KN[index] {
            return Ok(x);
        }
        if index == 0 {
            loop {
                x = -source::uniform(source)?.ln() * (1.0 / RN);
                let y = -source::uniform(source)?.ln();
                if y + y >= x * x {
                    break;
                }
            }
            return Ok(if j > 0 { RN + x } else { -RN - x });
        }
        let uniform = source::uniform(source)? as f32;
        let left = FN[index] + uniform * (FN[index - 1] - FN[index]);
        let right = (-0.5 * x * x).exp() as f32;
        if left < right {
            return Ok(x);
        }
    }
}
