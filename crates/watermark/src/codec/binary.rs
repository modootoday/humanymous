//! Explicit opaque trailer channel. Never automatically applied to unknown MIME.

use crate::{
    Error,
    frame::{self, Limits},
};

/// Append a framed carrier only after the owner qualifies opaque trailing-byte
/// semantics. This does not make signed/structured binaries safe to modify.
pub fn encode_opaque(source: &[u8], carrier: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    let frame = frame::encode(carrier, limits)?;
    let size = source
        .len()
        .checked_add(frame.len())
        .ok_or(Error::ResourceLimit)?;
    limits.output(size)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(source);
    out.extend_from_slice(&frame);
    Ok(out)
}

/// Recover every bounded frame, not only the last magic marker. Distinct carriers
/// are ambiguous even if one occurs in resource bytes before the trailer.
pub fn recover(source: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    frame::recover(source, limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailer_preserves_bytes_and_invalid_late_magic_does_not_mask_carrier() {
        let limits = Limits::new(1024, 1024, 64).unwrap();
        let source = b"\0opaque\xff";
        let mut out = encode_opaque(source, b"issued", limits).unwrap();
        assert!(out.starts_with(source));
        out.extend_from_slice(b"HMWM\xff\xff");
        assert_eq!(recover(&out, limits).unwrap(), b"issued");
        out.extend_from_slice(&frame::encode(b"different", limits).unwrap());
        assert_eq!(recover(&out, limits), Err(Error::Ambiguous));
    }

    #[test]
    fn opaque_input_and_expanded_output_limits_are_enforced() {
        let limits = Limits::new(4, 14, 1).unwrap();
        assert_eq!(
            encode_opaque(b"12345", b"a", limits),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            encode_opaque(b"1234", b"a", limits),
            Err(Error::ResourceLimit)
        );
        assert_eq!(encode_opaque(b"123", b"a", limits).unwrap().len(), 14);
    }
}
