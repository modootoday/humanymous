//! Compatibility tests mirroring legacy `internal/watermark/watermark_test.go`
//! and `lsb_test.go` through the public issuance, recovery and trace APIs.

use std::time::Duration;

use crate::watermark::{
    Error,
    codec::{Channel, png, wasm},
    frame::{Limits, MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES},
    issuance::{Fallback, SourceCheck, apply},
    ledger::{Configuration, Issuance, Ledger, Trace},
    payload::{Payload, WatermarkKey, split_carrier},
    recovery::recover,
};

fn limits() -> Limits {
    Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
}

/// Legacy `mk`.
fn mk() -> WatermarkKey {
    WatermarkKey::new(b"master-secret-key".to_vec()).unwrap()
}

/// Legacy `payload(id)`.
fn payload(id: u32) -> Payload {
    Payload {
        tag_id: id,
        session_id: "sess-abc".into(),
        asset_id: "/res/logo.png".into(),
        bucket: 20_300,
    }
}

/// Legacy `roundTrip`: Apply, Recover with the same MIME, split and verify.
fn round_trip(declaration: &str, source: &[u8]) -> Vec<Channel> {
    let p = payload(7);
    let applied = apply(&mk(), declaration, source, &p, Fallback::Forbid, limits())
        .unwrap_or_else(|error| panic!("[{declaration}] apply: {error}"));
    let recovered = recover(declaration, &applied.bytes, limits())
        .unwrap_or_else(|error| panic!("[{declaration}] recover: {error}"));
    assert!(recovered.preferred, "{declaration}");
    let carrier = split_carrier(&recovered.carrier).unwrap();
    assert_eq!(carrier.tag_id, 7, "{declaration}");
    assert!(
        mk().verify_legacy(&p, carrier.tag).unwrap(),
        "{declaration}"
    );
    for channel in &applied.channels {
        assert!(
            recovered.channels.contains(channel),
            "{declaration}: written channel {channel:?} must recover independently"
        );
    }
    applied.channels
}

/// Legacy TestRoundTrip_AllFamilies. The PNG source is a valid image: the
/// legacy zero-dimension `minimalPNG` is malformed evidence and is rejected
/// (see `minimal_legacy_png_is_rejected_not_trailer_marked`).
#[test]
fn apply_recover_round_trips_all_eight_legacy_families() {
    for (declaration, source, channels) in [
        (
            "application/javascript",
            b"function f(){return 1;}\n".as_slice(),
            vec![Channel::TextComment],
        ),
        ("text/css", b"body{margin:0}\n", vec![Channel::TextComment]),
        (
            "text/html",
            b"<!doctype html><h1>hi</h1>",
            vec![Channel::TextComment],
        ),
        (
            "image/svg+xml",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
            vec![Channel::TextComment],
        ),
        (
            "application/json",
            b"{\"a\":1,\"b\":[2,3]}",
            vec![Channel::TextComment],
        ),
        (
            "application/octet-stream",
            &[0x00, 0x01, 0x02, 0x03, 0xff],
            vec![Channel::BinaryTrailer],
        ),
        (
            "image/png",
            &png::fixtures::legacy_real_png(),
            vec![Channel::PngLsb, Channel::PngText],
        ),
        (
            "application/wasm",
            wasm::MINIMAL_MODULE,
            vec![Channel::WasmSection],
        ),
    ] {
        assert_eq!(round_trip(declaration, source), channels, "{declaration}");
    }
}

/// Legacy TestSemantic_JSONStaysValid.
#[test]
fn watermarked_json_still_parses_to_the_same_value() {
    let source = br#"{"user":"kim","roles":["a","b"]}"#;
    let applied = apply(
        &mk(),
        "application/json",
        source,
        &payload(1),
        Fallback::Forbid,
        limits(),
    )
    .unwrap();
    assert_eq!(applied.check, SourceCheck::JsonParsed);
    let before: serde_json::Value = serde_json::from_slice(source).unwrap();
    let after: serde_json::Value = serde_json::from_slice(&applied.bytes).unwrap();
    assert_eq!(after["user"], "kim");
    assert_eq!(before, after);
}

/// Legacy TestSemantic_PNGChunksIntact, on a valid image.
#[test]
fn watermarked_png_keeps_signature_critical_chunks_and_decodes() {
    let applied = apply(
        &mk(),
        "image/png",
        &png::fixtures::legacy_real_png(),
        &payload(2),
        Fallback::Forbid,
        limits(),
    )
    .unwrap();
    assert!(applied.bytes.starts_with(png::SIGNATURE));
    for chunk in [b"IHDR".as_slice(), b"IDAT", b"tEXt", b"IEND"] {
        assert!(
            applied.bytes.windows(4).any(|window| window == chunk),
            "{}",
            String::from_utf8_lossy(chunk)
        );
    }
    png::admit(&applied.bytes).unwrap();
    let decoded = png::fixtures::decode(&applied.bytes);
    assert_eq!((decoded.width, decoded.height), (64, 64));
}

/// Deliberate correction of the legacy `minimalPNG` success: zero dimensions
/// and no IDAT are rejected, and never trailer-marked even when opted in.
#[test]
fn minimal_legacy_png_is_rejected_not_trailer_marked() {
    for fallback in [Fallback::Forbid, Fallback::OpaqueTrailer] {
        assert_eq!(
            apply(
                &mk(),
                "image/png",
                &png::fixtures::legacy_minimal_png(),
                &payload(2),
                fallback,
                limits(),
            )
            .unwrap_err(),
            Error::InvalidFormat
        );
    }
}

/// Legacy TestSemantic_WASMStillHasMagic, plus actual module validation.
#[test]
fn watermarked_wasm_keeps_magic_and_validates() {
    let applied = apply(
        &mk(),
        "application/wasm",
        wasm::MINIMAL_MODULE,
        &payload(3),
        Fallback::Forbid,
        limits(),
    )
    .unwrap();
    assert!(applied.bytes.starts_with(wasm::MAGIC));
    assert_eq!(applied.check, SourceCheck::WasmValidated);
    wasm::validate_module(&applied.bytes).unwrap();
}

/// Legacy TestTrace_IdentifiesSessionAndDetectsForgery.
#[test]
fn trace_identifies_session_and_detects_forgery() {
    let ledger = Ledger::new(
        Configuration {
            record_ttl: Duration::from_secs(3600),
            reservation_ttl: Duration::from_secs(60),
            capacity: 16,
        },
        Duration::ZERO,
    )
    .unwrap();
    let reservation = ledger.reserve(Duration::from_secs(1)).unwrap();
    let p = Payload {
        tag_id: reservation.tag_id(),
        session_id: "victim-42".into(),
        asset_id: "/res/report.pdf".into(),
        bucket: 20_300,
    };
    let mime = "application/octet-stream";
    let applied = apply(&mk(), mime, b"SECRET DOC", &p, Fallback::Forbid, limits()).unwrap();
    reservation
        .commit(
            Issuance {
                payload: p.clone(),
                mime: mime.into(),
                channels: applied.channel_names(),
                ip_pseudonym: None,
            },
            Duration::from_secs(1),
        )
        .unwrap();

    let now = Duration::from_secs(2);
    let Trace::Verified(found) = ledger
        .trace(&mk(), mime, &applied.bytes, limits(), now)
        .unwrap()
    else {
        panic!("legitimate watermark was not verified");
    };
    assert_eq!(found.record.payload().session_id, "victim-42");
    assert_eq!(found.record.mime(), mime);
    assert_eq!(found.record.channels(), ["bin-trailer"]);
    assert_eq!(found.record.issued_at(), Duration::from_secs(1));
    assert_eq!(found.channels, [Channel::BinaryTrailer]);
    assert!(!found.ignored_suffix);

    // A carrier minted with the wrong key for the same ID is detected.
    let attacker = WatermarkKey::new(b"attacker-key".to_vec()).unwrap();
    let forged = apply(
        &attacker,
        mime,
        b"SECRET DOC",
        &p,
        Fallback::Forbid,
        limits(),
    )
    .unwrap();
    let Trace::Forged(diagnostic) = ledger
        .trace(&mk(), mime, &forged.bytes, limits(), now)
        .unwrap()
    else {
        panic!("forged watermark not detected");
    };
    assert_eq!(diagnostic.tag_id, p.tag_id);

    // Unknown IDs, expiry and unmarked bytes stay distinct from attribution.
    let mut stranger = p.clone();
    stranger.tag_id = 999;
    let unknown = apply(&mk(), mime, b"x", &stranger, Fallback::Forbid, limits()).unwrap();
    assert!(matches!(
        ledger
            .trace(&mk(), mime, &unknown.bytes, limits(), now)
            .unwrap(),
        Trace::Unknown { tag_id: 999 }
    ));
    assert!(matches!(
        ledger.trace(&mk(), mime, b"SECRET DOC", limits(), now),
        Err(Error::NoFrame)
    ));
    assert!(matches!(
        ledger
            .trace(
                &mk(),
                mime,
                &applied.bytes,
                limits(),
                Duration::from_secs(3602)
            )
            .unwrap(),
        Trace::Expired { tag_id } if tag_id == p.tag_id
    ));
}

/// Legacy TestLSB_SurvivesMetadataStrip.
#[test]
fn pixel_carrier_survives_metadata_strip_by_independent_reencode() {
    let source = png::fixtures::legacy_real_png();
    let p = Payload {
        tag_id: 99,
        session_id: "sess-lsb".into(),
        asset_id: "/photo.png".into(),
        bucket: 20_300,
    };
    let applied = apply(&mk(), "image/png", &source, &p, Fallback::Forbid, limits()).unwrap();
    let stripped = png::fixtures::strip_metadata_by_reencode(&applied.bytes);
    assert!(!stripped.windows(4).any(|window| window == b"tEXt"));
    let recovered = recover("image/png", &stripped, limits()).unwrap();
    assert_eq!(recovered.channels, [Channel::PngLsb]);
    let carrier = split_carrier(&recovered.carrier).unwrap();
    assert_eq!(carrier.tag_id, 99);
    assert!(mk().verify_legacy(&p, carrier.tag).unwrap());
}
