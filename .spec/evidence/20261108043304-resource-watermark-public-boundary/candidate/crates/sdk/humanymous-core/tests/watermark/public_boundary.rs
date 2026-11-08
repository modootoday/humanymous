//! Public API regression cases with immutable, package-local inputs.
use std::time::Duration;

use humanymous_core::watermark::{
    Error,
    codec::{
        binary,
        mime::{self, Family},
        text::{self, Syntax},
    },
    frame::{self, Limits},
    ledger::{Configuration, Issuance, Ledger, Lookup},
    payload::{Payload, WatermarkKey, split_carrier, split_legacy_carrier},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const FIXTURE: &[u8] = include_bytes!("fixtures/public-boundary.json");
const FIXTURE_SHA256: &str = include_str!("fixtures/public-boundary.sha256");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema: String,
    provenance: String,
    crc_carrier_hex: String,
    crc_frame_hex: String,
    layout_carrier_hex: String,
    layout_tag_id: u32,
    session_id: String,
    other_session_id: String,
    asset_id: String,
    other_asset_id: String,
    bucket: u64,
    json_inputs: Vec<String>,
}

fn fixture() -> Fixture {
    assert_eq!(hex::encode(Sha256::digest(FIXTURE)), FIXTURE_SHA256.trim());
    let fixture: Fixture = serde_json::from_slice(FIXTURE).unwrap();
    assert_eq!(fixture.schema, "humanymous.watermark.public-boundary/1");
    assert!(!fixture.provenance.is_empty());
    fixture
}

fn limits() -> Limits {
    Limits::new(4096, 8192, 64).unwrap()
}
fn key() -> WatermarkKey {
    WatermarkKey::new(b"synthetic-fixture-key-not-production".to_vec()).unwrap()
}
fn payload(id: u32, fixture: &Fixture) -> Payload {
    Payload {
        tag_id: id,
        session_id: fixture.session_id.clone(),
        asset_id: fixture.asset_id.clone(),
        bucket: fixture.bucket,
    }
}
fn ledger() -> Ledger {
    Ledger::new(
        Configuration {
            record_ttl: Duration::from_secs(10),
            reservation_ttl: Duration::from_secs(2),
            capacity: 2,
        },
        Duration::ZERO,
    )
    .unwrap()
}
fn issuance(payload: Payload) -> Issuance {
    Issuance {
        payload,
        mime: "application/octet-stream".into(),
        channels: vec!["bin-trailer".into()],
        ip_pseudonym: None,
    }
}

#[test]
fn package_local_frame_fixture_is_independent_of_runtime_issuance() {
    let fixture = fixture();
    let carrier = hex::decode(fixture.crc_carrier_hex).unwrap();
    let encoded = hex::decode(fixture.crc_frame_hex).unwrap();
    assert_eq!(frame::encode(&carrier, limits()).unwrap(), encoded);
    assert_eq!(frame::recover(&encoded, limits()).unwrap(), carrier);
    // A correct generic CRC frame is not a fourteen-byte authenticated carrier.
    assert_eq!(split_carrier(&carrier), Err(Error::InvalidCarrier));
}

#[test]
fn immutable_frame_truncations_and_every_single_bit_corruption_are_rejected() {
    let encoded = hex::decode(fixture().crc_frame_hex).unwrap();
    for length in 0..encoded.len() {
        assert_eq!(
            frame::recover(&encoded[..length], limits()),
            Err(Error::NoFrame),
            "prefix length {length}"
        );
    }
    for index in 0..encoded.len() {
        for bit in 0..8 {
            let mut corrupted = encoded.clone();
            corrupted[index] ^= 1 << bit;
            assert_eq!(
                frame::recover(&corrupted, limits()),
                Err(Error::NoFrame),
                "byte {index}, bit {bit}"
            );
        }
    }
}

#[test]
fn duplicate_frames_are_unique_but_distinct_carriers_are_ambiguous() {
    let frame = hex::decode(fixture().crc_frame_hex).unwrap();
    let mut repeated = frame.repeat(2);
    assert_eq!(frame::recover(&repeated, limits()).unwrap(), b"123456789");
    repeated.extend_from_slice(&frame::encode(b"different", limits()).unwrap());
    assert_eq!(frame::recover(&repeated, limits()), Err(Error::Ambiguous));
}

#[test]
fn strict_carrier_layout_and_explicit_legacy_suffix_policy_remain_public() {
    let fixture = fixture();
    let mut bytes = hex::decode(fixture.layout_carrier_hex).unwrap();
    let carrier = split_carrier(&bytes).unwrap();
    assert_eq!(carrier.tag_id, fixture.layout_tag_id);
    assert_eq!(
        carrier.tag,
        &[0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9]
    );
    assert!(!carrier.ignored_suffix);
    for length in 0..14 {
        assert_eq!(split_carrier(&bytes[..length]), Err(Error::InvalidCarrier));
    }
    bytes.extend_from_slice(b"suffix");
    assert_eq!(split_carrier(&bytes), Err(Error::InvalidCarrier));
    assert!(split_legacy_carrier(&bytes).unwrap().ignored_suffix);
}

#[test]
fn public_json_and_opaque_channels_preserve_values_and_prefixes() {
    let fixture = fixture();
    let carrier = key().carrier(&payload(1, &fixture)).unwrap();
    for json in fixture.json_inputs {
        let before: serde_json::Value = serde_json::from_str(&json).unwrap();
        let encoded = text::encode(json.as_bytes(), &carrier, Syntax::Json, limits()).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(before, after);
        assert_eq!(text::recover(&encoded, limits()).unwrap(), carrier);
    }
    let opaque = b"\0opaque\xff";
    let encoded = binary::encode_opaque(opaque, &carrier, limits()).unwrap();
    assert!(encoded.starts_with(opaque));
    assert_eq!(binary::recover(&encoded, limits()).unwrap(), carrier);
}

#[test]
fn identity_and_key_changes_do_not_verify_and_v1_id_limitation_is_explicit() {
    let fixture = fixture();
    let key = key();
    let original = payload(1, &fixture);
    let carrier = key.carrier(&original).unwrap();
    let tag = split_carrier(&carrier).unwrap().tag;
    assert!(key.verify_legacy(&original, tag).unwrap());
    let mut changed = original.clone();
    changed.session_id = fixture.other_session_id;
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    changed = original.clone();
    changed.asset_id = fixture.other_asset_id;
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    changed = original.clone();
    changed.bucket += 1;
    assert!(!key.verify_legacy(&changed, tag).unwrap());
    let other_key = WatermarkKey::new(b"wrong-synthetic-fixture-key".to_vec()).unwrap();
    assert!(!other_key.verify_legacy(&original, tag).unwrap());
    changed = original;
    changed.tag_id = 2;
    assert!(
        key.verify_legacy(&changed, tag).unwrap(),
        "legacy tag omits lookup ID; ledger ownership is still required"
    );
}

#[test]
fn crc_valid_forged_tag_is_not_successful_authentication() {
    let fixture = fixture();
    let key = key();
    let issued = payload(1, &fixture);
    let mut forged = key.carrier(&issued).unwrap();
    forged[4] ^= 1;
    let resource = binary::encode_opaque(b"synthetic resource", &forged, limits()).unwrap();
    let recovered = binary::recover(&resource, limits()).unwrap();
    let split = split_carrier(&recovered).unwrap();
    assert_eq!(split.tag_id, 1);
    assert!(!key.verify_legacy(&issued, split.tag).unwrap());
}

#[test]
fn live_lookup_requires_commit_and_mac_against_the_retained_record() {
    let fixture = fixture();
    let key = key();
    let ledger = ledger();
    let reservation = ledger.reserve(Duration::ZERO).unwrap();
    let issued = payload(reservation.tag_id(), &fixture);
    let carrier = key.carrier(&issued).unwrap();
    assert!(matches!(
        ledger.lookup(issued.tag_id, Duration::ZERO).unwrap(),
        Lookup::Unknown
    ));
    reservation
        .commit(issuance(issued), Duration::ZERO)
        .unwrap();
    let split = split_carrier(&carrier).unwrap();
    let Lookup::Live(record) = ledger.lookup(split.tag_id, Duration::ZERO).unwrap() else {
        panic!("committed record missing")
    };
    assert!(key.verify_legacy(record.payload(), split.tag).unwrap());
    assert_eq!(record.payload().session_id, fixture.session_id);
    let dropped = ledger.reserve(Duration::ZERO).unwrap();
    let cancelled_id = dropped.tag_id();
    drop(dropped);
    assert!(matches!(
        ledger.lookup(cancelled_id, Duration::ZERO).unwrap(),
        Lookup::Unknown
    ));
    assert!(ledger.reserve(Duration::ZERO).unwrap().tag_id() > cancelled_id);
}

#[test]
fn retention_and_restarted_ledger_cannot_promote_old_session_identity() {
    let fixture = fixture();
    let key = key();
    let original = ledger();
    let reservation = original.reserve(Duration::ZERO).unwrap();
    let issued = payload(reservation.tag_id(), &fixture);
    let carrier = key.carrier(&issued).unwrap();
    reservation
        .commit(issuance(issued), Duration::ZERO)
        .unwrap();
    assert!(matches!(
        original.lookup(1, Duration::from_secs(10)).unwrap(),
        Lookup::Live(_)
    ));
    assert!(matches!(
        original.lookup(1, Duration::from_secs(11)).unwrap(),
        Lookup::Expired
    ));
    let restarted = ledger();
    let reservation = restarted.reserve(Duration::ZERO).unwrap();
    let mut other = payload(reservation.tag_id(), &fixture);
    other.session_id = fixture.other_session_id;
    reservation.commit(issuance(other), Duration::ZERO).unwrap();
    let Lookup::Live(record) = restarted.lookup(1, Duration::ZERO).unwrap() else {
        panic!("new record missing")
    };
    assert!(
        !key.verify_legacy(record.payload(), split_carrier(&carrier).unwrap().tag)
            .unwrap()
    );
}

#[test]
fn public_admission_and_mime_identification_are_not_capability_promotion() {
    let fixture = fixture();
    let mut invalid = payload(1, &fixture);
    invalid.session_id.push('\0');
    assert_eq!(key().carrier(&invalid), Err(Error::InvalidPayload));
    assert_eq!(
        text::encode(b"\0", b"a", Syntax::Json, limits()),
        Err(Error::InvalidText)
    );
    assert_eq!(mime::select("application/pdf"), Ok(Family::Unsupported));
    assert_eq!(mime::select("text/plain"), Ok(Family::Unsupported));
    assert_eq!(mime::select("image/png"), Ok(Family::Png));
    assert_eq!(mime::select("application/wasm"), Ok(Family::Wasm));
    assert_eq!(
        mime::select("application/json\r\nforged: value"),
        Err(Error::InvalidMime)
    );
    assert_eq!(
        frame::recover(&[0; 4097], limits()),
        Err(Error::ResourceLimit)
    );
}
