//! Low-level legacy text carriers; source grammar validation belongs to issuance.

use super::base32;
use crate::{
    Error,
    frame::{self, Budget, Candidates, Limits},
};

const MARKER: &[u8] = b"!hmwm:";

/// Trusted syntax selected after the composition root validates source semantics.
/// Merely choosing a variant does not prove input grammar validity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Syntax {
    /// Insignificant trailing JSON whitespace, including scalar JSON.
    Json,
    /// HTML/XML/SVG comment.
    Markup,
    /// JavaScript/CSS block comment.
    BlockComment,
}

/// Append the historical carrier representation. This validates UTF-8/NUL/size,
/// not JSON/HTML/XML/JS/CSS grammar. Do not expose it as successful issuance
/// without W03 source validation and W07 ledger commit/response ordering.
pub fn encode(
    source: &[u8],
    carrier: &[u8],
    syntax: Syntax,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    if source.contains(&0) || std::str::from_utf8(source).is_err() {
        return Err(Error::InvalidText);
    }
    let frame = frame::encode(carrier, limits)?;
    let (prefix, suffix): (&[u8], &[u8]) = match syntax {
        Syntax::Json => (b"\n", b""),
        Syntax::Markup => (b"\n<!--!hmwm:", b"!-->\n"),
        Syntax::BlockComment => (b"\n/*!hmwm:", b"!*/\n"),
    };
    let payload_len = match syntax {
        Syntax::Json => frame.len().checked_mul(8),
        _ => frame.len().div_ceil(5).checked_mul(8),
    }
    .ok_or(Error::ResourceLimit)?;
    let size = source
        .len()
        .checked_add(prefix.len())
        .and_then(|size| size.checked_add(payload_len))
        .and_then(|size| size.checked_add(suffix.len()))
        .ok_or(Error::ResourceLimit)?;
    limits.output(size)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(source);
    out.extend_from_slice(prefix);
    if syntax == Syntax::Json {
        for byte in frame {
            for bit in (0..8).rev() {
                out.push(if byte & (1 << bit) == 0 { b' ' } else { b'\t' });
            }
        }
    } else {
        out.extend_from_slice(&base32::encode(&frame)?);
    }
    out.extend_from_slice(suffix);
    Ok(out)
}

/// Scan all comment/whitespace candidates with a shared work budget. Historical
/// byte recovery does not require valid UTF-8 or a truthful MIME declaration.
pub fn recover(source: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    let mut candidates = Candidates::new();
    let mut budget = Budget::new(limits);
    for (offset, window) in source.windows(MARKER.len()).enumerate() {
        if window != MARKER {
            continue;
        }
        budget.candidate()?;
        let rest = &source[offset + MARKER.len()..];
        // The historical marker alphabet excludes whitespace and lower case.
        let end = rest
            .iter()
            .take_while(|byte| matches!(**byte,b'A'..=b'Z'|b'2'..=b'7'|b'='))
            .count();
        if rest.get(end) != Some(&b'!') {
            continue;
        }
        if let Some(raw) = base32::decode(&rest[..end])? {
            frame::scan(&raw, &mut candidates, &mut budget)?;
        }
    }
    let trailing = source
        .iter()
        .rev()
        .take_while(|byte| matches!(**byte, b' ' | b'\t'))
        .count();
    if trailing >= 8 {
        let raw: Vec<u8> = source[source.len() - trailing..]
            .chunks_exact(8)
            .map(|bits| {
                bits.iter()
                    .fold(0, |byte, bit| (byte << 1) | u8::from(*bit == b'\t'))
            })
            .collect();
        frame::scan(&raw, &mut candidates, &mut budget)?;
    }
    candidates.finish()
}

#[cfg(test)]
mod tests;
