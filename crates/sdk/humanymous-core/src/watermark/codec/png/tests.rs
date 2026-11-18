use png::{BitDepth, ColorType};

use super::*;
use crate::watermark::frame::{MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

fn limits() -> Limits {
    Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
}

#[test]
fn composite_writes_pixels_then_metadata_and_both_recover_independently() {
    let encoded = encode(
        &fixtures::legacy_real_png(),
        b"issued-png",
        Policy::RequireBoth,
        limits(),
    )
    .unwrap();
    assert_eq!(encoded.channels, [Channel::PngLsb, Channel::PngText]);
    assert_eq!(encoded.degraded, None);
    admit(&encoded.bytes).unwrap();
    // Each channel alone yields the carrier.
    assert_eq!(
        metadata::recover(&encoded.bytes, limits()).unwrap(),
        b"issued-png"
    );
    assert_eq!(
        pixels::recover(&encoded.bytes, limits()).unwrap(),
        b"issued-png"
    );
    assert_eq!(
        recover(&encoded.bytes, limits()).unwrap(),
        Recovered {
            carrier: b"issued-png".to_vec(),
            channels: vec![Channel::PngText, Channel::PngLsb],
        }
    );
    // After an independent pixel-only re-encode only the pixel channel remains.
    let stripped = fixtures::strip_metadata_by_reencode(&encoded.bytes);
    assert_eq!(metadata::recover(&stripped, limits()), Err(Error::NoFrame));
    assert_eq!(
        recover(&stripped, limits()).unwrap(),
        Recovered {
            carrier: b"issued-png".to_vec(),
            channels: vec![Channel::PngLsb],
        }
    );
}

#[test]
fn small_image_degrades_visibly_or_fails_in_strict_mode() {
    let tiny = fixtures::encode(2, 2, ColorType::Rgb, BitDepth::Eight, &[9; 12]);
    let encoded = encode(&tiny, b"small", Policy::BestEffort, limits()).unwrap();
    assert_eq!(encoded.channels, [Channel::PngText]);
    assert_eq!(encoded.degraded, Some(Error::CarrierCapacity));
    assert_eq!(
        recover(&encoded.bytes, limits()).unwrap().channels,
        [Channel::PngText]
    );
    assert_eq!(
        encode(&tiny, b"small", Policy::RequireBoth, limits()).unwrap_err(),
        Error::CarrierCapacity
    );
}

#[test]
fn distinct_channel_carriers_are_ambiguous_and_malformed_inputs_fail_closed() {
    let pixels_only = pixels::encode(&fixtures::legacy_real_png(), b"one", limits()).unwrap();
    let mixed = metadata::encode(&pixels_only, b"two", limits()).unwrap();
    assert_eq!(recover(&mixed, limits()), Err(Error::Ambiguous));

    for policy in [Policy::BestEffort, Policy::RequireBoth] {
        assert_eq!(
            encode(&fixtures::legacy_minimal_png(), b"x", policy, limits()).unwrap_err(),
            Error::InvalidFormat
        );
    }
    assert_eq!(recover(b"not-png", limits()), Err(Error::InvalidFormat));
    assert_eq!(
        recover(&fixtures::legacy_real_png(), limits()),
        Err(Error::NoFrame)
    );
}
