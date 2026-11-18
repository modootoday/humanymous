use std::io::Cursor;

use png::{BitDepth, ColorType, Encoder};

use super::*;
use crate::watermark::codec::png::fixtures;
use crate::watermark::frame::{MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};
use crate::watermark::payload::{Payload, WatermarkKey};

fn limits() -> Limits {
    Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
}

fn key() -> WatermarkKey {
    WatermarkKey::new(b"master-secret-key".to_vec()).unwrap()
}

fn uniform_rgb(width: u32, height: u32, value: u8) -> Vec<u8> {
    let data = vec![value; width as usize * height as usize * 3];
    fixtures::encode(width, height, ColorType::Rgb, BitDepth::Eight, &data)
}

/// Read row-major R,G,B low bits, MSB first, from independently decoded RGB/RGBA.
fn low_bits(decoded: &fixtures::Decoded, bytes: usize) -> Vec<u8> {
    let stride = match decoded.color {
        ColorType::Rgb => 3,
        ColorType::Rgba => 4,
        other => panic!("unexpected output colour {other:?}"),
    };
    let mut out = vec![0u8; bytes];
    let channels = decoded
        .data
        .chunks_exact(stride)
        .flat_map(|pixel| &pixel[..3]);
    for (index, channel) in channels.take(bytes * 8).enumerate() {
        out[index / 8] |= (channel & 1) << (7 - index % 8);
    }
    out
}

/// Legacy internal/watermark TestLSB_DirectRoundTrip.
#[test]
fn direct_round_trip_recovers_issued_carrier() {
    let payload = Payload {
        tag_id: 5,
        session_id: "s".into(),
        asset_id: "/a".into(),
        bucket: 1,
    };
    let carrier = key().carrier(&payload).unwrap();
    let out = encode(&fixtures::legacy_real_png(), &carrier, limits()).unwrap();
    assert_eq!(recover(&out, limits()).unwrap(), carrier);
}

#[test]
fn layout_is_length_prefixed_row_major_msb_first_rgb_low_bits() {
    let source = uniform_rgb(8, 8, 0x80);
    let out = encode(&source, b"issued", limits()).unwrap();
    let decoded = fixtures::decode(&out);
    assert_eq!(decoded.color, ColorType::Rgb);
    assert_eq!(decoded.depth, BitDepth::Eight);
    let frame = frame::encode(b"issued", limits()).unwrap();
    assert_eq!(frame.len(), 16);
    let mut expected = 16u32.to_be_bytes().to_vec();
    expected.extend_from_slice(&frame);
    assert_eq!(low_bits(&decoded, expected.len()), expected);
    // u32be(16): the only set bit among the first 32 is bit index 27, which is
    // the red channel of pixel 9 (row 1, column 1).
    assert!(decoded.data[..27].iter().all(|channel| *channel == 0x80));
    assert_eq!(decoded.data[27], 0x81);
    // Unused channels keep their value; every channel moved by at most one.
    let used = expected.len() * 8;
    assert!(decoded.data[used..].iter().all(|channel| *channel == 0x80));
    assert!(
        decoded
            .data
            .iter()
            .all(|channel| channel.abs_diff(0x80) <= 1)
    );
}

#[test]
fn alpha_is_unchanged_and_premultiplied_model_matches_legacy() {
    // 8x8 RGBA: opaque, partially transparent and fully transparent pixels.
    let mut data = Vec::new();
    for index in 0..64u32 {
        data.extend_from_slice(match index % 4 {
            0 => &[200, 100, 50, 128],
            1 => &[10, 20, 30, 255],
            2 => &[255, 128, 0, 1],
            _ => &[90, 90, 90, 0],
        });
    }
    let source = fixtures::encode(8, 8, ColorType::Rgba, BitDepth::Eight, &data);
    let raster = Raster::decode(&source, Purpose::Recover).unwrap();
    // Golden legacy premultiplication: (c*0x101 * a*0x101 / 0xffff) >> 8.
    assert_eq!(
        &raster.pix[..16],
        &[100, 50, 25, 128, 10, 20, 30, 255, 1, 0, 0, 1, 0, 0, 0, 0]
    );
    let out = encode(&source, b"alpha", limits()).unwrap();
    let decoded = fixtures::decode(&out);
    assert_eq!(decoded.color, ColorType::Rgba);
    for (before, after) in data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(decoded.data.as_chunks::<4>().0)
    {
        assert_eq!(before[3], after[3], "alpha must never carry data");
    }
    // Legacy limitation reproduced, not fixed: the premultiply/un-premultiply
    // round trip destroys low bits in transparent and partially transparent
    // pixels, so this image does not recover. Only opaque pixels are reliable.
    assert_eq!(recover(&out, limits()), Err(Error::NoFrame));
}

#[test]
fn legacy_unpremultiply_golden_vectors_including_overflow_truncation() {
    assert_eq!(unpremultiply([100, 50, 25, 128]), [199, 99, 49, 128]);
    assert_eq!(unpremultiply([7, 8, 9, 0]), [0, 0, 0, 0]);
    assert_eq!(unpremultiply([7, 8, 9, 255]), [7, 8, 9, 255]);
    // A written low bit can exceed alpha: 101*0xffff/100 = 66190; >>8 = 258.
    assert_eq!(unpremultiply([101, 100, 0, 100]), [2, 255, 0, 100]);
    assert_eq!(premultiply(0x8000, 0x8000), 64);
}

#[test]
fn sixteen_bit_gray_and_palette_inputs_follow_legacy_conversion() {
    let rgb16 = fixtures::encode(
        1,
        1,
        ColorType::Rgb,
        BitDepth::Sixteen,
        &[0x12, 0x34, 0xab, 0xcd, 0xff, 0x00],
    );
    let raster = Raster::decode(&rgb16, Purpose::Recover).unwrap();
    assert_eq!(raster.pix, [0x12, 0xab, 0xff, 0xff]);

    let gray = fixtures::encode(2, 1, ColorType::Grayscale, BitDepth::Eight, &[7, 200]);
    let raster = Raster::decode(&gray, Purpose::Recover).unwrap();
    assert_eq!(raster.pix, [7, 7, 7, 255, 200, 200, 200, 255]);

    let palette = fixtures::indexed(2, 1, &[255, 0, 0, 0, 0, 255], Some(&[64, 255]), &[0, 1]);
    let raster = Raster::decode(&palette, Purpose::Recover).unwrap();
    assert_eq!(raster.pix, [64, 0, 0, 64, 0, 0, 255, 255]);

    // A large enough palette image round trips and its alpha survives.
    let indices = vec![1u8; 16 * 16];
    let source = fixtures::indexed(16, 16, &[255, 0, 0, 0, 0, 255], Some(&[64]), &indices);
    let out = encode(&source, b"palette", limits()).unwrap();
    assert_eq!(recover(&out, limits()).unwrap(), b"palette");
}

#[test]
fn capacity_is_checked_and_too_small_images_fail_with_capacity() {
    // 14-byte carrier -> 24-byte frame -> 224 payload bits; 8x9 gives 216.
    let carrier = [0x5a; 14];
    assert_eq!(
        encode(&uniform_rgb(8, 9, 0), &carrier, limits()),
        Err(Error::CarrierCapacity)
    );
    // 75 pixels give 225 bits, the first size that admits the frame.
    let out = encode(&uniform_rgb(15, 5, 0), &carrier, limits()).unwrap();
    assert_eq!(recover(&out, limits()).unwrap(), carrier);
}

#[test]
fn malformed_unmarked_oversized_and_animated_inputs_fail_closed() {
    assert_eq!(
        encode(b"not-png", b"x", limits()),
        Err(Error::InvalidFormat)
    );
    assert_eq!(recover(b"not-png", limits()), Err(Error::InvalidFormat));
    assert_eq!(
        encode(&fixtures::legacy_minimal_png(), b"x", limits()),
        Err(Error::InvalidFormat)
    );
    // All-even channels: length prefix zero.
    assert_eq!(
        recover(&uniform_rgb(8, 8, 0x80), limits()),
        Err(Error::NoFrame)
    );
    // All-odd channels: length prefix 0xffffffff exceeds the legacy bound.
    assert_eq!(
        recover(&uniform_rgb(8, 8, 0x81), limits()),
        Err(Error::NoFrame)
    );
    let wide = fixtures::encode(
        MAX_AXIS + 1,
        1,
        ColorType::Grayscale,
        BitDepth::Eight,
        &vec![0; MAX_AXIS as usize + 1],
    );
    assert_eq!(encode(&wide, b"x", limits()), Err(Error::ResourceLimit));
    assert_eq!(recover(&wide, limits()), Err(Error::ResourceLimit));

    let mut animated = Vec::new();
    {
        let mut encoder = Encoder::new(Cursor::new(&mut animated), 16, 16);
        encoder.set_color(ColorType::Rgb);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_animated(1, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0; 16 * 16 * 3]).unwrap();
        writer.finish().unwrap();
    }
    assert_eq!(encode(&animated, b"x", limits()), Err(Error::InvalidFormat));
}
