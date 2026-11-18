//! PNG carrier transforms: `hmwm` tEXt metadata, RGB low-bit pixels and their
//! composite (legacy `png-multi`) with independent recovery.

#[cfg(test)]
pub(in crate::watermark) mod fixtures;
pub mod metadata;
pub mod pixels;

use std::io::Cursor;

use png::{Decoder, Limits as PngLimits};

use super::Channel;
use crate::watermark::{Error, frame::Limits};

/// PNG signature bytes from ISO/IEC 15948 / W3C PNG.
pub const SIGNATURE: &[u8; 8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Best-effort decoder byte budget; not a whole-task hard memory proof.
pub(in crate::watermark::codec) const DECODE_BYTE_BUDGET: usize = 128 * 1024 * 1024;

/// Fail closed when the buffer is not a PNG signature or the decoder rejects it.
pub fn admit(source: &[u8]) -> Result<(), Error> {
    if !source.starts_with(SIGNATURE) {
        return Err(Error::InvalidFormat);
    }
    let limits = PngLimits {
        bytes: DECODE_BYTE_BUDGET.min(source.len().saturating_mul(4).max(64 * 1024)),
    };
    let decoder = Decoder::new_with_limits(Cursor::new(source), limits);
    let reader = decoder.read_info().map_err(|_| Error::InvalidFormat)?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 {
        return Err(Error::InvalidFormat);
    }
    Ok(())
}

/// Which channels a composite PNG issuance must write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Policy {
    /// Pixels plus metadata; an image too small for pixels degrades, visibly,
    /// to metadata only (the legacy tEXt-only fallback).
    BestEffort,
    /// Both channels are required; a too-small image fails.
    RequireBoth,
}

/// Composite PNG issuance output with the channels actually written.
#[derive(Debug)]
pub struct Encoded {
    /// Final PNG bytes.
    pub bytes: Vec<u8>,
    /// Channels present in `bytes`, in the order they were applied.
    pub channels: Vec<Channel>,
    /// Why a channel was skipped under [`Policy::BestEffort`], if one was.
    pub degraded: Option<Error>,
}

/// Write the pixel channel first, then insert metadata after the final encode.
///
/// Deliberate correction of the legacy order: metadata-first followed by the
/// pixel re-encode discarded the tEXt chunk while still reporting `png-multi`.
/// Unlike the legacy codec, a PNG that fails to decode is not marked through
/// metadata alone; only a capacity shortfall degrades under best effort.
pub fn encode(
    source: &[u8],
    carrier: &[u8],
    policy: Policy,
    limits: Limits,
) -> Result<Encoded, Error> {
    match pixels::encode(source, carrier, limits) {
        Ok(marked) => Ok(Encoded {
            bytes: metadata::encode(&marked, carrier, limits)?,
            channels: vec![Channel::PngLsb, Channel::PngText],
            degraded: None,
        }),
        Err(Error::CarrierCapacity) if policy == Policy::BestEffort => Ok(Encoded {
            bytes: metadata::encode(source, carrier, limits)?,
            channels: vec![Channel::PngText],
            degraded: Some(Error::CarrierCapacity),
        }),
        Err(error) => Err(error),
    }
}

/// A carrier recovered from one or both PNG channels.
#[derive(Debug, Eq, PartialEq)]
pub struct Recovered {
    /// The unique recovered carrier.
    pub carrier: Vec<u8>,
    /// Channels that independently yielded this carrier (metadata first).
    pub channels: Vec<Channel>,
}

/// Recover each channel independently; distinct carriers are ambiguous.
///
/// The legacy codec returned the metadata carrier without reading pixels.
/// Resource-limit and ambiguity outcomes from either channel fail closed
/// instead of yielding a partial attribution.
pub fn recover(source: &[u8], limits: Limits) -> Result<Recovered, Error> {
    let attempts = [
        (Channel::PngText, metadata::recover(source, limits)),
        (Channel::PngLsb, pixels::recover(source, limits)),
    ];
    let mut found: Option<Recovered> = None;
    let mut all_invalid = true;
    for (channel, attempt) in attempts {
        match attempt {
            Ok(carrier) => match &mut found {
                Some(existing) if existing.carrier != carrier => return Err(Error::Ambiguous),
                Some(existing) => existing.channels.push(channel),
                None => {
                    found = Some(Recovered {
                        carrier,
                        channels: vec![channel],
                    })
                }
            },
            Err(error @ (Error::Ambiguous | Error::ResourceLimit)) => return Err(error),
            Err(Error::InvalidFormat) => {}
            Err(_) => all_invalid = false,
        }
    }
    match found {
        Some(recovered) => Ok(recovered),
        None if all_invalid => Err(Error::InvalidFormat),
        None => Err(Error::NoFrame),
    }
}

#[cfg(test)]
mod tests;
