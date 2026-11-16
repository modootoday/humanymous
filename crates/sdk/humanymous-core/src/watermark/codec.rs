//! Bounded low-level carrier transforms, distinct from trusted issuance.
//!
//! The composition root must validate source grammar/format and semantics before
//! encoding, then commit the ledger before exposing bytes. CRC alone is not a
//! verified trace. PNG metadata and WASM custom-section modules are L2; PNG LSB
//! stays fail-closed pending alpha/resource qualification.

mod base32;
pub mod binary;
pub mod mime;
pub mod png;
pub mod text;
pub mod wasm;

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
