//! PNG carrier transforms. Metadata is L2; pixel LSB stays fail-closed.

pub mod metadata;
pub mod pixels;

use std::io::Cursor;

use png::{Decoder, Limits as PngLimits};

use crate::watermark::Error;

/// PNG signature bytes from ISO/IEC 15948 / W3C PNG.
pub const SIGNATURE: &[u8; 8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Best-effort decoder byte budget; not a whole-task hard memory proof.
const DECODE_BYTE_BUDGET: usize = 128 * 1024 * 1024;

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
