//! Typed issuance composition (legacy `inject.go` `Apply`).
//!
//! Builds the legacy-v1 carrier, selects a codec from the strictly parsed MIME
//! and returns the marked bytes with the channels actually written, the source
//! check performed and SHA-256 digests of input and output. Every failure is a
//! typed error: no outcome returns the original unmarked bytes as a success.
//!
//! Legacy `Apply` silently fell back to the binary trailer on any codec error.
//! Here the trailer fallback is opt-in ([`Fallback::OpaqueTrailer`]), reported
//! in [`Applied::fallback`], limited to text-like or unsupported declarations
//! that failed source checks or have no transform, and never applied to
//! malformed PNG/WASM, malformed MIME declarations or resource-limit failures.
//! The ledger reservation/commit ordering (W07) remains the caller's duty.

use sha2::{Digest, Sha256};

use crate::watermark::{
    Error,
    codec::{Channel, binary, mime, png, text, wasm},
    frame::Limits,
    payload::{Payload, WatermarkKey},
};

/// Whether an opaque trailer may replace a failed or unsupported transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fallback {
    /// Fail with the typed codec error.
    Forbid,
    /// The caller asserts that trailing bytes are semantically permitted.
    OpaqueTrailer,
}

/// What was checked about the source before it was marked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceCheck {
    /// Parsed as JSON before and after marking.
    JsonParsed,
    /// Only UTF-8 and NUL-freedom; markup/script/style grammar is unchecked.
    Utf8Only,
    /// PNG decoded (pixels) or header-admitted (metadata-only degradation).
    PngDecoded,
    /// WASM module validated before and after marking.
    WasmValidated,
    /// Declared opaque; nothing about the bytes is checked.
    Opaque,
}

/// Successful issuance transform.
#[derive(Debug)]
pub struct Applied {
    /// Marked resource bytes.
    pub bytes: Vec<u8>,
    /// Channels actually written.
    pub channels: Vec<Channel>,
    /// Source check performed.
    pub check: SourceCheck,
    /// Error of the preferred transform when the opt-in trailer replaced it.
    pub fallback: Option<Error>,
    /// A channel skipped by a best-effort composite (PNG too small for pixels).
    pub degraded: Option<Error>,
    /// SHA-256 of the unmarked input.
    pub source_sha256: [u8; 32],
    /// SHA-256 of `bytes`.
    pub output_sha256: [u8; 32],
}

impl Applied {
    /// Channel labels for ledger issuance metadata.
    pub fn channel_names(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|channel| channel.name().to_owned())
            .collect()
    }
}

/// Mark `source` for `payload`, selecting the codec from `declared_mime`.
pub fn apply(
    key: &WatermarkKey,
    declared_mime: &str,
    source: &[u8],
    payload: &Payload,
    fallback: Fallback,
    limits: Limits,
) -> Result<Applied, Error> {
    limits.input(source)?;
    let carrier = key.carrier(payload)?;
    let family = mime::select(declared_mime)?;
    let (bytes, channels, check, degraded, fallback_from) =
        match transform(family, source, &carrier, limits) {
            Ok(Transformed {
                bytes,
                channels,
                check,
                degraded,
            }) => (bytes, channels, check, degraded, None),
            Err(error) if permits_fallback(fallback, family, error) => (
                binary::encode_opaque(source, &carrier, limits)?,
                vec![Channel::BinaryTrailer],
                SourceCheck::Opaque,
                None,
                Some(error),
            ),
            Err(error) => return Err(error),
        };
    Ok(Applied {
        source_sha256: Sha256::digest(source).into(),
        output_sha256: Sha256::digest(&bytes).into(),
        bytes,
        channels,
        check,
        fallback: fallback_from,
        degraded,
    })
}

struct Transformed {
    bytes: Vec<u8>,
    channels: Vec<Channel>,
    check: SourceCheck,
    degraded: Option<Error>,
}

impl Transformed {
    fn single(bytes: Vec<u8>, channel: Channel, check: SourceCheck) -> Self {
        Self {
            bytes,
            channels: vec![channel],
            check,
            degraded: None,
        }
    }
}

fn transform(
    family: mime::Family,
    source: &[u8],
    carrier: &[u8],
    limits: Limits,
) -> Result<Transformed, Error> {
    let comment = |syntax| {
        text::encode(source, carrier, syntax, limits)
            .map(|bytes| Transformed::single(bytes, Channel::TextComment, SourceCheck::Utf8Only))
    };
    match family {
        mime::Family::Json => {
            parse_json(source)?;
            let bytes = text::encode(source, carrier, text::Syntax::Json, limits)?;
            parse_json(&bytes)?;
            Ok(Transformed::single(
                bytes,
                Channel::TextComment,
                SourceCheck::JsonParsed,
            ))
        }
        mime::Family::Markup => comment(text::Syntax::Markup),
        mime::Family::BlockComment => comment(text::Syntax::BlockComment),
        mime::Family::Opaque => binary::encode_opaque(source, carrier, limits)
            .map(|bytes| Transformed::single(bytes, Channel::BinaryTrailer, SourceCheck::Opaque)),
        mime::Family::Png => {
            let encoded = png::encode(source, carrier, png::Policy::BestEffort, limits)?;
            Ok(Transformed {
                bytes: encoded.bytes,
                channels: encoded.channels,
                check: SourceCheck::PngDecoded,
                degraded: encoded.degraded,
            })
        }
        mime::Family::Wasm => wasm::encode(source, carrier, limits).map(|bytes| {
            Transformed::single(bytes, Channel::WasmSection, SourceCheck::WasmValidated)
        }),
        mime::Family::Unsupported => Err(Error::UnsupportedFormat),
    }
}

fn parse_json(bytes: &[u8]) -> Result<(), Error> {
    serde_json::from_slice::<serde::de::IgnoredAny>(bytes)
        .map(|_| ())
        .map_err(|_| Error::InvalidText)
}

fn permits_fallback(fallback: Fallback, family: mime::Family, error: Error) -> bool {
    fallback == Fallback::OpaqueTrailer
        && !matches!(family, mime::Family::Png | mime::Family::Wasm)
        && matches!(error, Error::InvalidText | Error::UnsupportedFormat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watermark::codec::png::fixtures;
    use crate::watermark::frame::{MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};
    use crate::watermark::payload::split_carrier;

    fn limits() -> Limits {
        Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
    }

    fn key() -> WatermarkKey {
        WatermarkKey::new(b"master-secret-key".to_vec()).unwrap()
    }

    fn payload() -> Payload {
        Payload {
            tag_id: 9,
            session_id: "sess".into(),
            asset_id: "/asset".into(),
            bucket: 20_300,
        }
    }

    #[test]
    fn result_reports_channels_check_and_digests() {
        let source = b"body{margin:0}\n";
        let applied = apply(
            &key(),
            "text/css",
            source,
            &payload(),
            Fallback::Forbid,
            limits(),
        )
        .unwrap();
        assert_eq!(applied.channels, [Channel::TextComment]);
        assert_eq!(applied.channel_names(), ["text-comment"]);
        assert_eq!(applied.check, SourceCheck::Utf8Only);
        assert_eq!(applied.fallback, None);
        assert_eq!(
            applied.source_sha256,
            <[u8; 32]>::from(Sha256::digest(source))
        );
        assert_eq!(
            applied.output_sha256,
            <[u8; 32]>::from(Sha256::digest(&applied.bytes))
        );
        assert_ne!(applied.source_sha256, applied.output_sha256);
        let carrier = text::recover(&applied.bytes, limits()).unwrap();
        assert_eq!(split_carrier(&carrier).unwrap().tag_id, 9);
    }

    #[test]
    fn unsupported_and_invalid_sources_fail_unless_trailer_is_opted_in() {
        for (declaration, source, error) in [
            ("text/plain", b"plain".as_slice(), Error::UnsupportedFormat),
            ("application/pdf", b"%PDF-", Error::UnsupportedFormat),
            ("application/json", b"{not json", Error::InvalidText),
            ("text/html", b"<p>\xff</p>", Error::InvalidText),
        ] {
            assert_eq!(
                apply(
                    &key(),
                    declaration,
                    source,
                    &payload(),
                    Fallback::Forbid,
                    limits()
                )
                .unwrap_err(),
                error,
                "{declaration}"
            );
            let applied = apply(
                &key(),
                declaration,
                source,
                &payload(),
                Fallback::OpaqueTrailer,
                limits(),
            )
            .unwrap();
            assert_eq!(applied.channels, [Channel::BinaryTrailer]);
            assert_eq!(applied.check, SourceCheck::Opaque);
            assert_eq!(applied.fallback, Some(error));
            assert!(applied.bytes.starts_with(source));
        }
    }

    #[test]
    fn malformed_png_wasm_mime_payload_and_limits_never_fall_back() {
        let png = fixtures::legacy_minimal_png();
        let cases: [(&str, &[u8], Error); 4] = [
            ("image/png", &png, Error::InvalidFormat),
            ("image/png", b"not a png", Error::InvalidFormat),
            ("application/wasm", b"\0asm\x02\0\0\0", Error::InvalidFormat),
            ("image/png\r\n", b"x", Error::InvalidMime),
        ];
        for (declaration, source, error) in cases {
            assert_eq!(
                apply(
                    &key(),
                    declaration,
                    source,
                    &payload(),
                    Fallback::OpaqueTrailer,
                    limits()
                )
                .unwrap_err(),
                error
            );
        }
        let mut zero = payload();
        zero.tag_id = 0;
        assert_eq!(
            apply(
                &key(),
                "text/css",
                b"a{}",
                &zero,
                Fallback::OpaqueTrailer,
                limits()
            )
            .unwrap_err(),
            Error::InvalidPayload
        );
        let small = Limits::new(64, 16, 1).unwrap();
        assert_eq!(
            apply(
                &key(),
                "text/plain",
                b"x",
                &payload(),
                Fallback::OpaqueTrailer,
                small
            )
            .unwrap_err(),
            Error::ResourceLimit
        );
    }

    #[test]
    fn small_png_is_marked_through_metadata_with_visible_degradation() {
        let tiny = fixtures::encode(
            2,
            2,
            ::png::ColorType::Rgb,
            ::png::BitDepth::Eight,
            &[1; 12],
        );
        let applied = apply(
            &key(),
            "image/png",
            &tiny,
            &payload(),
            Fallback::Forbid,
            limits(),
        )
        .unwrap();
        assert_eq!(applied.channels, [Channel::PngText]);
        assert_eq!(applied.degraded, Some(Error::CarrierCapacity));
        assert_eq!(applied.check, SourceCheck::PngDecoded);
    }
}
