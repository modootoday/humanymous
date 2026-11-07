//! Deterministic MIME selection without broad substring or trailer fallbacks.

use crate::watermark::Error;

/// Explicit syntax selection; PNG/WASM entries do not imply implemented codecs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Family {
    /// JSON whitespace carrier, including scalar JSON.
    Json,
    /// HTML/XML/SVG comment carrier.
    Markup,
    /// JavaScript/CSS block comment carrier.
    BlockComment,
    /// Explicit application/octet-stream, still requiring owner qualification.
    Opaque,
    /// PNG codec remains pending alpha/resource qualification.
    Png,
    /// WASM codec remains pending standards validator adoption.
    Wasm,
    /// No admitted new-issuance transformation; never silently use a trailer.
    Unsupported,
}

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// Parse a bounded ASCII MIME declaration and select a known syntax.
/// Parameters may be tokens or quoted strings with quoted-pair escapes. Only
/// ASCII space/tab are treated as optional whitespace; CR/LF/NUL are forbidden.
pub fn select(declaration: &str) -> Result<Family, Error> {
    if declaration.len() > 256
        || declaration
            .bytes()
            .any(|byte| !byte.is_ascii() || byte.is_ascii_control() && byte != b'\t')
    {
        return Err(Error::InvalidMime);
    }
    let declaration = declaration.trim_matches([' ', '\t']);
    let (essence, parameters) = declaration.split_once(';').unwrap_or((declaration, ""));
    let essence = essence.trim_matches([' ', '\t']);
    let Some((media_type, subtype)) = essence.split_once('/') else {
        return Err(Error::InvalidMime);
    };
    if media_type.is_empty()
        || subtype.is_empty()
        || !media_type.bytes().all(token)
        || !subtype.bytes().all(token)
    {
        return Err(Error::InvalidMime);
    }
    if declaration.contains(';') {
        validate_parameters(parameters.as_bytes())?;
    }
    let media_type = media_type.to_ascii_lowercase();
    let subtype = subtype.to_ascii_lowercase();
    Ok(match (media_type.as_str(), subtype.as_str()) {
        ("application", "octet-stream") => Family::Opaque,
        ("application", "wasm") => Family::Wasm,
        ("image", "png") => Family::Png,
        ("application" | "text", "json") => Family::Json,
        ("application", suffix) if suffix.ends_with("+json") => Family::Json,
        ("text", "html" | "xml") | ("application", "xml") | ("image", "svg+xml") => Family::Markup,
        ("application", suffix) if suffix.ends_with("+xml") => Family::Markup,
        ("application" | "text", "javascript" | "ecmascript") | ("text", "css") => {
            Family::BlockComment
        }
        _ => Family::Unsupported,
    })
}

fn validate_parameters(mut rest: &[u8]) -> Result<(), Error> {
    loop {
        while rest
            .first()
            .is_some_and(|byte| matches!(*byte, b' ' | b'\t'))
        {
            rest = &rest[1..];
        }
        let name = rest.iter().take_while(|byte| token(**byte)).count();
        if name == 0 || rest.get(name) != Some(&b'=') {
            return Err(Error::InvalidMime);
        }
        rest = &rest[name + 1..];
        if rest.first() == Some(&b'"') {
            rest = &rest[1..];
            let mut closed = false;
            while let Some(byte) = rest.first() {
                match *byte {
                    b'"' => {
                        rest = &rest[1..];
                        closed = true;
                        break;
                    }
                    b'\\' => {
                        if rest.len() < 2 {
                            return Err(Error::InvalidMime);
                        }
                        rest = &rest[2..];
                    }
                    _ => rest = &rest[1..],
                }
            }
            if !closed {
                return Err(Error::InvalidMime);
            }
        } else {
            let value = rest.iter().take_while(|byte| token(**byte)).count();
            if value == 0 {
                return Err(Error::InvalidMime);
            }
            rest = &rest[value..];
        }
        while rest
            .first()
            .is_some_and(|byte| matches!(*byte, b' ' | b'\t'))
        {
            rest = &rest[1..];
        }
        if rest.is_empty() {
            return Ok(());
        }
        if rest.first() != Some(&b';') {
            return Err(Error::InvalidMime);
        }
        rest = &rest[1..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_parameters_and_explicit_families() {
        for (mime, family) in [
            (" Image/PNG ; note=\"a;b\\\"c\" ", Family::Png),
            ("application/wasm", Family::Wasm),
            ("application/problem+json;charset=utf-8", Family::Json),
            ("image/svg+xml", Family::Markup),
            ("text/css", Family::BlockComment),
            ("application/octet-stream", Family::Opaque),
        ] {
            assert_eq!(select(mime), Ok(family));
        }
        assert_eq!(select("application/notjavascript"), Ok(Family::Unsupported));
        assert_eq!(select("text/plain"), Ok(Family::Unsupported));
        assert_eq!(select("application/pdf"), Ok(Family::Unsupported));
    }

    #[test]
    fn malformed_declarations_never_become_fallbacks() {
        for mime in [
            "",
            "image",
            "image/png/x",
            "image /png",
            "image/png;",
            "image/png;x",
            "image/png;x=",
            "image/png;x=\"unclosed",
            "image/png\r\nX: y",
            "image/png\0",
            "\u{a0}image/png",
        ] {
            assert_eq!(select(mime), Err(Error::InvalidMime));
        }
    }
}
