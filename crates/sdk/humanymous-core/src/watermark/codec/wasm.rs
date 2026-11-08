//! WASM custom-section carrier with standards validation via wasmparser.

use wasmparser::{Validator, WasmFeatures};

use crate::watermark::{
    Error,
    frame::{self, Limits},
};

/// WASM binary magic (`\0asm`).
pub const MAGIC: &[u8; 4] = &[0x00, 0x61, 0x73, 0x6d];
const SECTION_NAME: &[u8] = b"hmwm";
/// Source-bound minimal module fixture (magic + version only). Defense-only.
pub const MINIMAL_MODULE: &[u8] = include_bytes!("fixtures/minimal_module.wasm");

/// Validate a core module; component model proposals stay disabled for this profile.
pub fn validate_module(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() < 8 || !bytes.starts_with(MAGIC) {
        return Err(Error::InvalidFormat);
    }
    let mut features = WasmFeatures::default();
    features.remove(WasmFeatures::COMPONENT_MODEL);
    Validator::new_with_features(features)
        .validate_all(bytes)
        .map_err(|_| Error::InvalidFormat)?;
    Ok(())
}

/// Append a named `hmwm` custom section after a validated module.
pub fn encode(source: &[u8], carrier: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    validate_module(source)?;
    let framed = frame::encode(carrier, limits)?;
    let mut body = Vec::with_capacity(1 + SECTION_NAME.len() + framed.len());
    append_uleb128(&mut body, u32::try_from(SECTION_NAME.len()).map_err(|_| Error::ResourceLimit)?)?;
    body.extend_from_slice(SECTION_NAME);
    body.extend_from_slice(&framed);
    let mut section = Vec::with_capacity(1 + 5 + body.len());
    section.push(0);
    append_uleb128(
        &mut section,
        u32::try_from(body.len()).map_err(|_| Error::ResourceLimit)?,
    )?;
    section.extend_from_slice(&body);
    let size = source
        .len()
        .checked_add(section.len())
        .ok_or(Error::ResourceLimit)?;
    limits.output(size)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(source);
    out.extend_from_slice(&section);
    validate_module(&out)?;
    Ok(out)
}

/// Recover a framed carrier from the first CRC-valid `hmwm` custom section.
pub fn recover(source: &[u8], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.input(source)?;
    if source.len() < 8 || !source.starts_with(MAGIC) {
        return Err(Error::InvalidFormat);
    }
    let mut pos = 8usize;
    let mut found = None;
    while pos < source.len() {
        let id = source[pos];
        pos += 1;
        let (size, n) = read_uleb128(&source[pos..])?;
        pos = pos.checked_add(n).ok_or(Error::InvalidFormat)?;
        let end = pos.checked_add(size).ok_or(Error::InvalidFormat)?;
        if end > source.len() {
            return Err(Error::InvalidFormat);
        }
        let body = &source[pos..end];
        pos = end;
        if id != 0 {
            continue;
        }
        let (name_len, m) = read_uleb128(body)?;
        let name_start = m;
        let name_end = name_start.checked_add(name_len).ok_or(Error::InvalidFormat)?;
        if name_end > body.len() {
            return Err(Error::InvalidFormat);
        }
        if &body[name_start..name_end] != SECTION_NAME {
            continue;
        }
        match frame::recover(&body[name_end..], limits) {
            Ok(carrier) => match &found {
                Some(existing) if existing != &carrier => return Err(Error::Ambiguous),
                Some(_) => {}
                None => found = Some(carrier),
            },
            Err(Error::NoFrame) => {}
            Err(Error::Ambiguous) => return Err(Error::Ambiguous),
            Err(other) => return Err(other),
        }
    }
    found.ok_or(Error::NoFrame)
}

fn append_uleb128(out: &mut Vec<u8>, mut value: u32) -> Result<(), Error> {
    for _ in 0..5 {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return Ok(());
        }
    }
    Err(Error::ResourceLimit)
}

fn read_uleb128(input: &[u8]) -> Result<(usize, usize), Error> {
    let mut result = 0u32;
    let mut shift = 0u32;
    for (index, byte) in input.iter().copied().enumerate() {
        if index >= 5 {
            return Err(Error::InvalidFormat);
        }
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((result as usize, index + 1));
        }
        shift += 7;
    }
    Err(Error::InvalidFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> Limits {
        Limits::new(64 * 1024, 64 * 1024, 8).unwrap()
    }

    #[test]
    fn minimal_fixture_validates_and_round_trips_custom_section() {
        assert_eq!(
            MINIMAL_MODULE,
            &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
        );
        validate_module(MINIMAL_MODULE).unwrap();
        let out = encode(MINIMAL_MODULE, b"issued-wasm", limits()).unwrap();
        assert!(out.starts_with(MAGIC));
        assert_eq!(recover(&out, limits()).unwrap(), b"issued-wasm");
        validate_module(&out).unwrap();
    }

    #[test]
    fn malformed_magic_and_missing_section_fail_closed() {
        assert_eq!(
            encode(b"not-wasm", b"x", limits()),
            Err(Error::InvalidFormat)
        );
        assert_eq!(recover(MINIMAL_MODULE, limits()), Err(Error::NoFrame));
    }
}
