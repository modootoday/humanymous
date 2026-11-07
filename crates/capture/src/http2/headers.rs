use super::Error;
use std::collections::BTreeSet;

type Fields = Vec<(Vec<u8>, Vec<u8>)>;

pub(super) fn decode(
    decoder: &mut httlib_hpack::Decoder<'static>,
    bytes: Vec<u8>,
    trailers: bool,
) -> Result<Fields, Error> {
    let mut fields = Vec::new();
    let mut total = 0usize;
    let mut pseudo = BTreeSet::new();
    let mut regular = false;
    let mut remaining = bytes.as_slice();
    while !remaining.is_empty() {
        // The dependency enforces the maximum table size. Updates are legal
        // only before the first header representation, not midway through it.
        if remaining[0] & 0xe0 == 0x20 && !fields.is_empty() {
            return Err(Error::Malformed);
        }
        // Validate every encoded integer and string span before entering the
        // dependency, whose truncated continuation paths may index unchecked.
        // Give it exactly one representation, independent of Vec capacity.
        let length = representation(remaining)?;
        let mut encoded = remaining[..length].to_vec();
        remaining = &remaining[length..];
        let mut one = Vec::with_capacity(1);
        decoder
            .decode_exact(&mut encoded, &mut one)
            .map_err(|_| Error::Malformed)?;
        if !encoded.is_empty() {
            return Err(Error::Malformed);
        }
        for (name, value, _) in one {
            total = total
                .checked_add(name.len() + value.len() + 32)
                .ok_or(Error::Budget)?;
            if fields.len() >= 128 || total > 65_536 {
                return Err(Error::Budget);
            }
            let is_pseudo = name.starts_with(b":");
            let token = if is_pseudo { &name[1..] } else { &name[..] };
            if token.is_empty() || token.iter().any(|b| !matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'))
                || value.iter().any(|b| matches!(b, 0..=8 | 10..=31 | 127))
                || value.first().is_some_and(|b| matches!(b, b' ' | b'\t'))
                || value.last().is_some_and(|b| matches!(b, b' ' | b'\t')) {
                return Err(Error::Malformed);
            }
            if is_pseudo {
                if trailers
                    || regular
                    || !matches!(
                        name.as_slice(),
                        b":method" | b":scheme" | b":authority" | b":path"
                    )
                    || !pseudo.insert(name.clone())
                    || value.is_empty()
                {
                    return Err(Error::Malformed);
                }
            } else {
                regular = true;
                if matches!(
                    name.as_slice(),
                    b"connection"
                        | b"proxy-connection"
                        | b"keep-alive"
                        | b"transfer-encoding"
                        | b"upgrade"
                ) || name == b"te" && value != b"trailers"
                {
                    return Err(Error::Malformed);
                }
            }
            fields.push((name, value));
        }
    }
    if !trailers {
        // CONNECT (including extended CONNECT) is deliberately outside this
        // request-capture profile; never invent absent scheme/path evidence.
        if pseudo.len() != 4
            || fields
                .iter()
                .any(|(n, v)| n == b":method" && v == b"CONNECT")
        {
            return Err(Error::Malformed);
        }
    }
    Ok(fields)
}

fn integer(bytes: &[u8], prefix: u8) -> Result<(usize, usize), Error> {
    let mask = (1u8 << prefix) - 1;
    let mut value = usize::from(*bytes.first().ok_or(Error::Malformed)? & mask);
    if value < usize::from(mask) {
        return Ok((value, 1));
    }
    for index in 1..5 {
        let byte = *bytes.get(index).ok_or(Error::Malformed)?;
        value = value
            .checked_add(usize::from(byte & 0x7f) << (7 * (index - 1)))
            .ok_or(Error::Malformed)?;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
    }
    Err(Error::Malformed)
}
fn string(bytes: &[u8]) -> Result<usize, Error> {
    let (length, prefix) = integer(bytes, 7)?;
    let total = length.checked_add(prefix).ok_or(Error::Malformed)?;
    bytes.get(..total).ok_or(Error::Malformed)?;
    Ok(total)
}
fn representation(bytes: &[u8]) -> Result<usize, Error> {
    let first = *bytes.first().ok_or(Error::Malformed)?;
    if first & 0x80 != 0 {
        return integer(bytes, 7).map(|(_, used)| used);
    }
    if first & 0xe0 == 0x20 {
        return integer(bytes, 5).map(|(_, used)| used);
    }
    let prefix = if first & 0x40 != 0 { 6 } else { 4 };
    let (name, mut used) = integer(bytes, prefix)?;
    if name == 0 {
        used += string(bytes.get(used..).ok_or(Error::Malformed)?)?;
    }
    used += string(bytes.get(used..).ok_or(Error::Malformed)?)?;
    Ok(used)
}
