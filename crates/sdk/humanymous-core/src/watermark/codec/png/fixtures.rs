//! Test-only PNG builders and an independent pixel-only strip, using the `png`
//! crate directly rather than the carrier code under test.

use std::io::Cursor;

use png::{BitDepth, ColorType, Decoder, Encoder, Transformations};

/// Encode raw samples with an explicit colour type and depth.
pub(in crate::watermark) fn encode(
    width: u32,
    height: u32,
    color: ColorType,
    depth: BitDepth,
    data: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

/// Indexed PNG with an optional tRNS table.
pub(in crate::watermark) fn indexed(
    width: u32,
    height: u32,
    palette: &[u8],
    trns: Option<&[u8]>,
    indices: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(ColorType::Indexed);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_palette(palette.to_vec());
        if let Some(trns) = trns {
            encoder.set_trns(trns.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(indices).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

/// Legacy `realPNG`: 64x64 opaque gradient `(x*3, y*3, (x+y)*2)`, written as
/// 8-bit RGB exactly as the legacy encoder writes an opaque RGBA image.
pub(in crate::watermark) fn legacy_real_png() -> Vec<u8> {
    let mut data = Vec::with_capacity(64 * 64 * 3);
    for y in 0..64u8 {
        for x in 0..64u8 {
            data.extend_from_slice(&[x * 3, y * 3, (x + y) * 2]);
        }
    }
    encode(64, 64, ColorType::Rgb, BitDepth::Eight, &data)
}

/// Decoded samples plus their layout, after palette/tRNS expansion.
pub(in crate::watermark) struct Decoded {
    pub width: u32,
    pub height: u32,
    pub color: ColorType,
    pub depth: BitDepth,
    pub data: Vec<u8>,
}

/// Decode with the `png` crate only (palette and low-bit expansion).
pub(in crate::watermark) fn decode(bytes: &[u8]) -> Decoded {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    data.truncate(info.buffer_size());
    Decoded {
        width: info.width,
        height: info.height,
        color: info.color_type,
        depth: info.bit_depth,
        data,
    }
}

/// Legacy `reencodePixelsOnly`: decode, then write only the pixels again,
/// discarding every ancillary chunk (the metadata-strip attack).
pub(in crate::watermark) fn strip_metadata_by_reencode(bytes: &[u8]) -> Vec<u8> {
    let decoded = decode(bytes);
    encode(
        decoded.width,
        decoded.height,
        decoded.color,
        decoded.depth,
        &decoded.data,
    )
}

/// Legacy `minimalPNG`: signature, zero-filled IHDR and IEND, no IDAT. It is
/// structurally chunked but has invalid zero dimensions.
pub(in crate::watermark) fn legacy_minimal_png() -> Vec<u8> {
    let mut out = super::SIGNATURE.to_vec();
    for (ctype, data) in [(b"IHDR", [0u8; 13].as_slice()), (b"IEND", &[])] {
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        out.extend_from_slice(ctype);
        out.extend_from_slice(data);
        let mut crc_input = ctype.to_vec();
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crate::watermark::frame::crc32(&crc_input).to_be_bytes());
    }
    out
}
