//! Bounded low-level carrier transforms, distinct from trusted issuance.
//!
//! The composition root must validate source grammar/format and semantics before
//! encoding, then commit the ledger before exposing bytes. CRC alone is not a
//! verified trace. Issuance composition lives in `watermark::issuance` and
//! multi-codec recovery in `watermark::recovery`. PNG pixel LSB reproduces the
//! legacy premultiplied model; real-resource (V3) qualification stays separate.

mod base32;
pub mod binary;
pub mod mime;
pub mod png;
pub mod text;
pub mod wasm;

/// A concrete carrier channel, named with its legacy ledger/report label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    /// Markup/block comment marker or JSON trailing whitespace (`text-comment`).
    TextComment,
    /// Opaque trailing frame (`bin-trailer`).
    BinaryTrailer,
    /// PNG `hmwm` tEXt chunk (`png-text`).
    PngText,
    /// PNG RGB low bits (`png-lsb`).
    PngLsb,
    /// WASM `hmwm` custom section (`wasm-section`).
    WasmSection,
}

impl Channel {
    /// Legacy channel label, suitable for ledger issuance metadata.
    pub fn name(self) -> &'static str {
        match self {
            Self::TextComment => "text-comment",
            Self::BinaryTrailer => "bin-trailer",
            Self::PngText => "png-text",
            Self::PngLsb => "png-lsb",
            Self::WasmSection => "wasm-section",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{binary, mime, text};
    use crate::watermark::frame::{Limits, MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};
    use crate::watermark::payload::{Payload, WatermarkKey, split_carrier};

    /// Codec-level part of legacy internal/watermark TestRoundTrip_AllFamilies:
    /// for each text and opaque family, the carrier built from a payload is
    /// recovered intact and its tag verifies. Issuance ordering, the ledger
    /// and PNG/WASM stay with their own owners and are not covered here.
    #[test]
    fn every_text_and_opaque_family_round_trips_the_tag() {
        let limits = Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap();
        let key = WatermarkKey::new(b"watermark-test-key".to_vec()).unwrap();
        let payload = Payload {
            tag_id: 7,
            session_id: "session".into(),
            asset_id: "asset".into(),
            bucket: 42,
        };
        let carrier = key.carrier(&payload).unwrap();
        for (declaration, source) in [
            ("application/javascript", b"console.log(1);".as_slice()),
            ("text/css", b"body{color:red}"),
            ("text/html", b"<html><head></head><body>x</body></html>"),
            (
                "image/svg+xml",
                b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            ),
            ("application/json", b"{\"k\":\"v\"}"),
            ("application/octet-stream", b"\x00\x01\x02binary"),
        ] {
            let marked = match mime::select(declaration).unwrap() {
                mime::Family::Json => text::encode(source, &carrier, text::Syntax::Json, limits),
                mime::Family::Markup => {
                    text::encode(source, &carrier, text::Syntax::Markup, limits)
                }
                mime::Family::BlockComment => {
                    text::encode(source, &carrier, text::Syntax::BlockComment, limits)
                }
                mime::Family::Opaque => binary::encode_opaque(source, &carrier, limits),
                other => panic!("{declaration} selected {other:?}"),
            }
            .unwrap();
            let recovered = match mime::select(declaration).unwrap() {
                mime::Family::Opaque => binary::recover(&marked, limits),
                _ => text::recover(&marked, limits),
            }
            .unwrap();
            let parsed = split_carrier(&recovered).unwrap();
            assert_eq!(parsed.tag_id, 7, "{declaration}");
            assert!(
                key.verify_legacy(&payload, parsed.tag).unwrap(),
                "{declaration}"
            );
        }
    }
}
