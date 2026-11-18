//! PNG RGB least-significant-bit carrier (legacy `png-lsb`).
//!
//! Layout: `u32be(frame_len) || frame`, written MSB first into the low bit of
//! the R, G and B samples of each pixel in row-major order; alpha never carries
//! data and is never modified. Because PNG is lossless, the carrier survives an
//! ancillary-chunk strip and an exact pixel re-encode. It does not survive lossy
//! transformations, resampling or colour conversion.
//!
//! Pixel values follow the legacy Go `image.RGBA` model exactly. The decoded
//! default image is converted to premultiplied 8-bit RGBA (16-bit samples are
//! premultiplied at 16 bits and keep their high byte). Output is re-encoded as
//! 8-bit RGB when every alpha is opaque, else as 8-bit RGBA un-premultiplied
//! with the legacy encoder formula. Opaque pixels therefore change by at most
//! one per RGB channel; partially transparent pixels can shift further through
//! the premultiply round trip, exactly as the legacy codec did. The re-encode
//! drops every ancillary chunk, so metadata must be inserted afterwards.

use std::io::Cursor;

use png::{BitDepth, ColorType, Decoder, Encoder, Limits as PngLimits, Transformations};

use super::{DECODE_BYTE_BUDGET, SIGNATURE};
use crate::watermark::{
    Error,
    frame::{self, Limits},
};

/// Largest advertised framed length accepted by legacy recovery (`1 << 16`).
pub const MAX_FRAME_BYTES: usize = 1 << 16;
/// Largest admitted width or height.
pub const MAX_AXIS: u32 = 8192;
/// Largest admitted pixel count (decoded premultiplied RGBA is at most 16 MiB).
pub const MAX_PIXELS: u64 = 4_194_304;
const LENGTH_PREFIX: usize = 4;
const OPAQUE: u8 = 0xff;

/// Embed a framed carrier in the RGB low bits and re-encode the default image.
///
/// Fails with [`Error::CarrierCapacity`] before any pixel is decoded when
/// `3 * width * height < 8 * (4 + frame_len)`. Animated PNG is rejected because
/// re-encoding would silently flatten the animation.
pub fn encode(source: &[u8], carrier: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    let framed = frame::encode(carrier, limits)?;
    let length = u32::try_from(framed.len()).map_err(|_| Error::ResourceLimit)?;
    let mut payload = Vec::with_capacity(LENGTH_PREFIX + framed.len());
    payload.extend_from_slice(&length.to_be_bytes());
    payload.extend_from_slice(&framed);
    let mut raster = Raster::decode(source, Purpose::Issue(payload.len()))?;
    raster.write_bits(&payload);
    let out = raster.encode()?;
    limits.output(out.len())?;
    Ok(out)
}

/// Recover the framed carrier from the RGB low bits of the default image.
pub fn recover(source: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    let raster = Raster::decode(source, Purpose::Recover)?;
    let header = raster.read_bits(LENGTH_PREFIX).ok_or(Error::NoFrame)?;
    let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(Error::NoFrame);
    }
    let payload = raster
        .read_bits(LENGTH_PREFIX + length)
        .ok_or(Error::NoFrame)?;
    frame::recover(&payload[LENGTH_PREFIX..], limits)
}

#[derive(Clone, Copy)]
enum Purpose {
    /// New issuance of this many payload bytes; animation is rejected.
    Issue(usize),
    /// Legacy recovery from the default image.
    Recover,
}

/// Premultiplied 8-bit RGBA, row-major, matching legacy `image.RGBA.Pix`.
struct Raster {
    width: u32,
    height: u32,
    pix: Vec<u8>,
}

impl Raster {
    fn decode(source: &[u8], purpose: Purpose) -> Result<Self, Error> {
        if !source.starts_with(SIGNATURE) {
            return Err(Error::InvalidFormat);
        }
        let mut decoder = Decoder::new_with_limits(
            Cursor::new(source),
            PngLimits {
                bytes: DECODE_BYTE_BUDGET,
            },
        );
        decoder.set_transformations(Transformations::EXPAND);
        let mut reader = decoder.read_info().map_err(|_| Error::InvalidFormat)?;
        let info = reader.info();
        let (width, height) = (info.width, info.height);
        if width == 0 || height == 0 {
            return Err(Error::InvalidFormat);
        }
        let pixels = u64::from(width) * u64::from(height);
        if width > MAX_AXIS || height > MAX_AXIS || pixels > MAX_PIXELS {
            return Err(Error::ResourceLimit);
        }
        if let Purpose::Issue(payload_bytes) = purpose {
            if info.animation_control.is_some() {
                return Err(Error::InvalidFormat);
            }
            let needed = u64::try_from(payload_bytes)
                .ok()
                .and_then(|bytes| bytes.checked_mul(8))
                .ok_or(Error::ResourceLimit)?;
            if pixels * 3 < needed {
                return Err(Error::CarrierCapacity);
            }
        }
        let size = reader.output_buffer_size().ok_or(Error::ResourceLimit)?;
        let mut buffer = vec![0; size];
        let output = reader
            .next_frame(&mut buffer)
            .map_err(|_| Error::InvalidFormat)?;
        if output.width != width || output.height != height {
            return Err(Error::InvalidFormat);
        }
        let samples = match output.color_type {
            ColorType::Grayscale => 1,
            ColorType::GrayscaleAlpha => 2,
            ColorType::Rgb => 3,
            ColorType::Rgba => 4,
            ColorType::Indexed => return Err(Error::InvalidFormat),
        };
        let sample_bytes = match output.bit_depth {
            BitDepth::Eight => 1,
            BitDepth::Sixteen => 2,
            _ => return Err(Error::InvalidFormat),
        };
        let pixel_bytes = samples * sample_bytes;
        let row_bytes = width as usize * pixel_bytes;
        if output.line_size < row_bytes {
            return Err(Error::InvalidFormat);
        }
        let mut pix = Vec::with_capacity(width as usize * height as usize * 4);
        for row in buffer.chunks_exact(output.line_size).take(height as usize) {
            for pixel in row[..row_bytes].chunks_exact(pixel_bytes) {
                let sample = |index: usize| -> u32 {
                    if sample_bytes == 1 {
                        u32::from(pixel[index]) * 0x101
                    } else {
                        u32::from(u16::from_be_bytes([pixel[2 * index], pixel[2 * index + 1]]))
                    }
                };
                let (red, green, blue, alpha) = match samples {
                    1 => (sample(0), sample(0), sample(0), 0xffff),
                    2 => (sample(0), sample(0), sample(0), sample(1)),
                    3 => (sample(0), sample(1), sample(2), 0xffff),
                    _ => (sample(0), sample(1), sample(2), sample(3)),
                };
                pix.extend_from_slice(&[
                    premultiply(red, alpha),
                    premultiply(green, alpha),
                    premultiply(blue, alpha),
                    (alpha >> 8) as u8,
                ]);
            }
        }
        if pix.len() != width as usize * height as usize * 4 {
            return Err(Error::InvalidFormat);
        }
        Ok(Self { width, height, pix })
    }

    fn channels(&self) -> impl Iterator<Item = &u8> {
        self.pix
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| &pixel[..3])
    }

    /// Callers have already checked capacity; excess payload is never written.
    fn write_bits(&mut self, payload: &[u8]) {
        let bits = payload
            .iter()
            .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
        let channels = self
            .pix
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .flat_map(|pixel| pixel[..3].iter_mut());
        for (channel, bit) in channels.zip(bits) {
            *channel = (*channel & !1) | bit;
        }
    }

    fn read_bits(&self, bytes: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; bytes];
        let mut read = 0;
        for (index, channel) in self.channels().take(bytes * 8).enumerate() {
            out[index / 8] |= (channel & 1) << (7 - index % 8);
            read += 1;
        }
        (read == bytes * 8).then_some(out)
    }

    fn encode(&self) -> Result<Vec<u8>, Error> {
        let opaque = self
            .pix
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == OPAQUE);
        let (color, data) = if opaque {
            (ColorType::Rgb, self.channels().copied().collect::<Vec<_>>())
        } else {
            let data = self
                .pix
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|pixel| unpremultiply(*pixel))
                .collect();
            (ColorType::Rgba, data)
        };
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(color);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| Error::InvalidFormat)?;
        writer
            .write_image_data(&data)
            .map_err(|_| Error::InvalidFormat)?;
        writer.finish().map_err(|_| Error::InvalidFormat)?;
        Ok(out)
    }
}

/// Legacy `NRGBA64.RGBA()` then `RGBAModel` high byte: `(c * a / 0xffff) >> 8`.
fn premultiply(channel: u32, alpha: u32) -> u8 {
    ((channel * alpha / 0xffff) >> 8) as u8
}

/// Legacy encoder conversion of a premultiplied pixel to straight RGBA.
///
/// A low-bit write can leave a channel one above its alpha; the legacy encoder
/// then truncates the overflowing quotient to eight bits, reproduced here.
fn unpremultiply(pixel: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(pixel[3]);
    match pixel[3] {
        0 => [0; 4],
        OPAQUE => pixel,
        _ => {
            let convert = |channel: u8| ((u32::from(channel) * 0xffff / alpha) >> 8) as u8;
            [
                convert(pixel[0]),
                convert(pixel[1]),
                convert(pixel[2]),
                pixel[3],
            ]
        }
    }
}

#[cfg(test)]
mod tests;
