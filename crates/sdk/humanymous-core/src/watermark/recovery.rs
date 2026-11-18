//! Carrier recovery from possibly leaked bytes (legacy `extract.go` `Recover`).
//!
//! The declared MIME selects a preferred codec; every other codec is then tried
//! in a fixed order (PNG, WASM, text, binary trailer) so a resource whose type
//! is unknown, wrong or re-encoded can still be traced. The declaration is a
//! hint, never authority: it may be malformed and only changes the order.
//!
//! Deliberate corrections of the legacy dispatcher: a fixed order replaces Go
//! map iteration, every codec is consulted so that distinct carriers from
//! different channels fail as [`Error::Ambiguous`] instead of attributing the
//! first one found, and resource-limit outcomes fail closed. A recovered
//! carrier is CRC-valid only; it is not a verified trace until the ledger
//! record and HMAC are checked.

use crate::watermark::{
    Error,
    codec::{Channel, binary, mime, png, text, wasm},
    frame::Limits,
};

/// A recovered carrier and the channels that independently agreed on it.
#[derive(Debug, Eq, PartialEq)]
pub struct Recovered {
    /// The unique CRC-valid carrier.
    pub carrier: Vec<u8>,
    /// Agreeing channels in attempt order; the first is the legacy channel.
    pub channels: Vec<Channel>,
    /// Whether the codec preferred by the declared MIME produced the carrier.
    pub preferred: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Codec {
    Png,
    Wasm,
    Text,
    Binary,
}

const ORDER: [Codec; 4] = [Codec::Png, Codec::Wasm, Codec::Text, Codec::Binary];

/// Recover a carrier: the declared MIME's codec first, then the alternates.
pub fn recover(declared_mime: &str, source: &[u8], limits: Limits) -> Result<Recovered, Error> {
    limits.input(source)?;
    let preferred = preferred_codec(declared_mime);
    let order =
        std::iter::once(preferred).chain(ORDER.into_iter().filter(|codec| *codec != preferred));
    let mut found: Option<Recovered> = None;
    for codec in order {
        let (carrier, channels) = match attempt(codec, source, limits) {
            Ok(success) => success,
            Err(error @ (Error::Ambiguous | Error::ResourceLimit)) => return Err(error),
            Err(_) => continue,
        };
        match &mut found {
            Some(existing) if existing.carrier != carrier => return Err(Error::Ambiguous),
            Some(existing) => {
                for channel in channels {
                    if !existing.channels.contains(&channel) {
                        existing.channels.push(channel);
                    }
                }
            }
            None => {
                found = Some(Recovered {
                    carrier,
                    channels,
                    preferred: codec == preferred,
                })
            }
        }
    }
    found.ok_or(Error::NoFrame)
}

fn attempt(codec: Codec, source: &[u8], limits: Limits) -> Result<(Vec<u8>, Vec<Channel>), Error> {
    match codec {
        Codec::Png => png::recover(source, limits).map(|found| (found.carrier, found.channels)),
        Codec::Wasm => wasm::recover(source, limits).map(|c| (c, vec![Channel::WasmSection])),
        Codec::Text => text::recover(source, limits).map(|c| (c, vec![Channel::TextComment])),
        Codec::Binary => binary::recover(source, limits).map(|c| (c, vec![Channel::BinaryTrailer])),
    }
}

/// Strict selection when the declaration parses to a known family; otherwise
/// the legacy `familyKey` heuristic (lowercase, trim, drop parameters, broad
/// text substrings), which is acceptable only because it orders recovery.
fn preferred_codec(declaration: &str) -> Codec {
    match mime::select(declaration) {
        Ok(mime::Family::Png) => Codec::Png,
        Ok(mime::Family::Wasm) => Codec::Wasm,
        Ok(mime::Family::Json | mime::Family::Markup | mime::Family::BlockComment) => Codec::Text,
        Ok(mime::Family::Opaque) => Codec::Binary,
        Ok(mime::Family::Unsupported) | Err(_) => legacy_hint(declaration),
    }
}

const MAX_HINT_BYTES: usize = 256;
const LEGACY_TEXT_HINTS: [&str; 7] = [
    "javascript",
    "json",
    "xml",
    "svg",
    "css",
    "html",
    "ecmascript",
];

fn legacy_hint(declaration: &str) -> Codec {
    if declaration.len() > MAX_HINT_BYTES {
        return Codec::Binary;
    }
    let lowered = declaration.trim().to_ascii_lowercase();
    let essence = lowered.split(';').next().unwrap_or_default().trim();
    match essence {
        "image/png" => Codec::Png,
        "application/wasm" => Codec::Wasm,
        _ if essence.starts_with("text/")
            || LEGACY_TEXT_HINTS.iter().any(|hint| essence.contains(hint)) =>
        {
            Codec::Text
        }
        _ => Codec::Binary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watermark::codec::png::fixtures;
    use crate::watermark::frame::{MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

    fn limits() -> Limits {
        Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
    }

    #[test]
    fn preferred_codec_uses_strict_family_then_legacy_hint() {
        for (declaration, codec) in [
            ("image/png", Codec::Png),
            ("application/wasm", Codec::Wasm),
            ("application/json", Codec::Text),
            ("text/css", Codec::Text),
            ("application/octet-stream", Codec::Binary),
            // Legacy familyKey hints for declarations strict selection rejects.
            ("text/plain", Codec::Text),
            ("application/x-javascript-module", Codec::Text),
            (" IMAGE/PNG ;", Codec::Png),
            ("application/pdf", Codec::Binary),
            ("", Codec::Binary),
        ] {
            assert_eq!(preferred_codec(declaration), codec, "{declaration}");
        }
    }

    #[test]
    fn wrong_or_unknown_declarations_fall_through_to_alternate_codecs() {
        let marked = png::encode(
            &fixtures::legacy_real_png(),
            b"leak",
            png::Policy::RequireBoth,
            limits(),
        )
        .unwrap()
        .bytes;
        for declaration in ["text/plain", "application/pdf", "garbage", "image/png"] {
            let recovered = recover(declaration, &marked, limits()).unwrap();
            assert_eq!(recovered.carrier, b"leak");
            assert_eq!(recovered.channels, [Channel::PngText, Channel::PngLsb]);
            assert_eq!(recovered.preferred, declaration == "image/png");
        }
        let commented = text::encode(b"a{}", b"css", text::Syntax::BlockComment, limits()).unwrap();
        let recovered = recover("application/octet-stream", &commented, limits()).unwrap();
        assert_eq!(recovered.channels, [Channel::TextComment]);
        assert!(!recovered.preferred);
    }

    #[test]
    fn agreeing_channels_merge_and_distinct_carriers_are_ambiguous() {
        // A WASM section stores a raw frame, so the trailer scan agrees with it.
        let marked = wasm::encode(wasm::MINIMAL_MODULE, b"same", limits()).unwrap();
        let recovered = recover("application/wasm", &marked, limits()).unwrap();
        assert_eq!(
            recovered.channels,
            [Channel::WasmSection, Channel::BinaryTrailer]
        );
        let mut mixed = text::encode(b"x", b"one", text::Syntax::BlockComment, limits()).unwrap();
        mixed = binary::encode_opaque(&mixed, b"two", limits()).unwrap();
        assert_eq!(recover("text/css", &mixed, limits()), Err(Error::Ambiguous));
        assert_eq!(
            recover("text/css", b"no mark here", limits()),
            Err(Error::NoFrame)
        );
        let small = Limits::new(4, 64, 1).unwrap();
        assert_eq!(
            recover("text/css", b"12345", small),
            Err(Error::ResourceLimit)
        );
    }
}
