//! PNG tEXt metadata carrier (`hmwm` keyword + padded base32 frame).

use super::admit;
use crate::watermark::{
    Error,
    codec::base32,
    frame::{self, Limits},
};

const KEYWORD: &[u8] = b"hmwm";
const CHUNK_TEXT: &[u8; 4] = b"tEXt";
const CHUNK_IEND: &[u8; 4] = b"IEND";

/// Insert a `hmwm` tEXt chunk before IEND after PNG admission.
pub fn encode(source: &[u8], carrier: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    admit(source)?;
    let iend = find_chunk(source, CHUNK_IEND).ok_or(Error::InvalidFormat)?;
    let framed = frame::encode(carrier, limits)?;
    let encoded = base32::encode(&framed)?;
    let mut text = Vec::with_capacity(KEYWORD.len() + 1 + encoded.len());
    text.extend_from_slice(KEYWORD);
    text.push(0);
    text.extend_from_slice(&encoded);
    let chunk = build_chunk(CHUNK_TEXT, &text)?;
    let size = iend
        .checked_add(chunk.len())
        .and_then(|mid| mid.checked_add(source.len() - iend))
        .ok_or(Error::ResourceLimit)?;
    limits.output(size)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&source[..iend]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&source[iend..]);
    admit(&out)?;
    Ok(out)
}

/// Recover the first CRC-valid framed carrier from a `hmwm` tEXt chunk.
pub fn recover(source: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    if !source.starts_with(super::SIGNATURE) {
        return Err(Error::InvalidFormat);
    }
    let mut pos = super::SIGNATURE.len();
    let mut found = None;
    while pos + 12 <= source.len() {
        let len = u32::from_be_bytes(source[pos..pos + 4].try_into().unwrap()) as usize;
        let type_end = pos.checked_add(8).ok_or(Error::InvalidFormat)?;
        let data_end = type_end.checked_add(len).ok_or(Error::InvalidFormat)?;
        let crc_end = data_end.checked_add(4).ok_or(Error::InvalidFormat)?;
        if crc_end > source.len() {
            break;
        }
        let ctype = &source[pos + 4..type_end];
        let data = &source[type_end..data_end];
        let declared = u32::from_be_bytes(source[data_end..crc_end].try_into().unwrap());
        let mut crc_input = Vec::with_capacity(4 + data.len());
        crc_input.extend_from_slice(ctype);
        crc_input.extend_from_slice(data);
        if frame::crc32(&crc_input) != declared {
            pos = crc_end;
            continue;
        }
        if ctype == CHUNK_TEXT
            && let Some((keyword, value)) = data.split_first_occurrence(&0)
            && keyword == KEYWORD
            && let Ok(Some(raw)) = base32::decode(value)
            && let Ok(carrier) = frame::recover(&raw, limits)
        {
            match found {
                Some(existing) if existing != carrier => return Err(Error::Ambiguous),
                Some(_) => {}
                None => found = Some(carrier),
            }
        }
        pos = crc_end;
    }
    found.ok_or(Error::NoFrame)
}

fn find_chunk(png: &[u8], want: &[u8; 4]) -> Option<usize> {
    let mut pos = super::SIGNATURE.len();
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().ok()?) as usize;
        if &png[pos + 4..pos + 8] == want {
            return Some(pos);
        }
        pos = pos.checked_add(12 + len)?;
    }
    None
}

fn build_chunk(ctype: &[u8; 4], data: &[u8]) -> Result<Vec<u8>, Error> {
    let len = u32::try_from(data.len()).map_err(|_| Error::ResourceLimit)?;
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(ctype);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(ctype);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&frame::crc32(&crc_input).to_be_bytes());
    Ok(out)
}

trait SplitFirstOccurrence {
    fn split_first_occurrence(&self, sep: &u8) -> Option<(&[u8], &[u8])>;
}

impl SplitFirstOccurrence for [u8] {
    fn split_first_occurrence(&self, sep: &u8) -> Option<(&[u8], &[u8])> {
        let idx = self.iter().position(|byte| byte == sep)?;
        Some((&self[..idx], &self[idx + 1..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use png::{BitDepth, ColorType, Encoder};
    use std::io::Cursor;

    fn limits() -> Limits {
        Limits::new(1024 * 1024, 1024 * 1024, 16).unwrap()
    }

    fn rgb_png(width: u32, height: u32, pixel: [u8; 3]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = Encoder::new(Cursor::new(&mut bytes), width, height);
            encoder.set_color(ColorType::Rgb);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let row = pixel.repeat(width as usize);
            let mut image = Vec::with_capacity(row.len() * height as usize);
            for _ in 0..height {
                image.extend_from_slice(&row);
            }
            writer.write_image_data(&image).unwrap();
        }
        bytes
    }

    #[test]
    fn metadata_round_trip_preserves_admitted_png() {
        let source = rgb_png(2, 2, [10, 20, 30]);
        let out = encode(&source, b"issued-png", limits()).unwrap();
        assert!(out.starts_with(super::super::SIGNATURE));
        assert_eq!(recover(&out, limits()).unwrap(), b"issued-png");
        admit(&out).unwrap();
    }

    #[test]
    fn non_png_and_missing_mark_fail_closed() {
        assert_eq!(
            encode(b"not-png", b"x", limits()),
            Err(Error::InvalidFormat)
        );
        let source = rgb_png(1, 1, [1, 2, 3]);
        assert_eq!(recover(&source, limits()), Err(Error::NoFrame));
    }
}
