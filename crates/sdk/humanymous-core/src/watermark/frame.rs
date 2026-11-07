//! Historical CRC envelope with checked bounds and deterministic candidate recovery.
//!
//! IEEE CRC32 detects corruption, not authenticity or error correction. HMAC and
//! authenticated ledger lookup remain separate requirements.

use crate::watermark::Error;

const MAGIC: &[u8; 4] = b"HMWM";
/// Largest input admitted by any carrier parser in this component profile.
pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
/// Largest encoded resource admitted by this component profile.
pub const MAX_OUTPUT_BYTES: usize = 33 * 1024 * 1024;
/// Maximum marker/frame candidates consumed across one recovery operation.
pub const MAX_CANDIDATES: usize = 64;
/// Maximum carrier representable by the legacy unsigned sixteen-bit length.
pub const MAX_CARRIER_BYTES: usize = u16::MAX as usize;

/// Explicit component admission caps. Private fields prevent unchecked changes.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    input: usize,
    output: usize,
    candidates: usize,
}

impl Limits {
    /// Select nonzero caps within the accepted component maxima, not an SLA.
    pub fn new(input: usize, output: usize, candidates: usize) -> Result<Self, Error> {
        if input == 0
            || input > MAX_INPUT_BYTES
            || output == 0
            || output > MAX_OUTPUT_BYTES
            || candidates == 0
            || candidates > MAX_CANDIDATES
        {
            return Err(Error::InvalidConfiguration);
        }
        Ok(Self {
            input,
            output,
            candidates,
        })
    }

    pub(in crate::watermark) fn input(self, data: &[u8]) -> Result<(), Error> {
        if data.len() > self.input {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }

    pub(in crate::watermark) fn output(self, size: usize) -> Result<(), Error> {
        if size > self.output {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
}

/// IEEE CRC32, reflected polynomial0xedb88320, complemented initial/final state.
/// Standard definition: https://www.w3.org/TR/png-3/#D-CRCAppendix
pub(in crate::watermark) fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// Encode the exact legacy magic/u16be length/carrier/u32be CRC envelope.
pub fn encode(carrier: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    let length = u16::try_from(carrier.len()).map_err(|_| Error::ResourceLimit)?;
    let size = carrier.len().checked_add(10).ok_or(Error::ResourceLimit)?;
    limits.output(size)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(carrier);
    out.extend_from_slice(&crc32(carrier).to_be_bytes());
    Ok(out)
}

/// Recover a unique CRC-valid carrier through bounded noise/corruption.
/// This accepts a generic historical frame; use strict split_carrier afterwards
/// when the caller requires an exact fourteen-byte watermark carrier.
pub fn recover(data: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(data)?;
    let mut candidates = Candidates::new();
    let mut budget = Budget::new(limits);
    scan(data, &mut candidates, &mut budget)?;
    candidates.finish()
}

pub(in crate::watermark) struct Budget {
    remaining: usize,
}

impl Budget {
    pub(in crate::watermark) fn new(limits: Limits) -> Self {
        Self {
            remaining: limits.candidates,
        }
    }
    pub(in crate::watermark) fn candidate(&mut self) -> Result<(), Error> {
        self.remaining = self.remaining.checked_sub(1).ok_or(Error::ResourceLimit)?;
        Ok(())
    }
}

pub(in crate::watermark) struct Candidates {
    carrier: Option<Vec<u8>>,
    ambiguous: bool,
}

impl Candidates {
    pub(in crate::watermark) fn new() -> Self {
        Self {
            carrier: None,
            ambiguous: false,
        }
    }
    fn add(&mut self, carrier: &[u8]) {
        match &self.carrier {
            Some(previous) => self.ambiguous |= previous != carrier,
            None => self.carrier = Some(carrier.to_vec()),
        }
    }
    pub(in crate::watermark) fn finish(self) -> Result<Vec<u8>, Error> {
        if self.ambiguous {
            return Err(Error::Ambiguous);
        }
        self.carrier.ok_or(Error::NoFrame)
    }
}

pub(in crate::watermark) fn scan(
    data: &[u8],
    candidates: &mut Candidates,
    budget: &mut Budget,
) -> Result<(), Error> {
    for (offset, window) in data.windows(MAGIC.len()).enumerate() {
        if window != MAGIC {
            continue;
        }
        budget.candidate()?;
        let Some(length_bytes) = data.get(offset + 4..offset + 6) else {
            continue;
        };
        let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        let start = offset + 6;
        let end = start + length;
        let Some(carrier) = data.get(start..end) else {
            continue;
        };
        let Some(crc) = data.get(end..end + 4) else {
            continue;
        };
        if crc32(carrier) == u32::from_be_bytes([crc[0], crc[1], crc[2], crc[3]]) {
            candidates.add(carrier);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
