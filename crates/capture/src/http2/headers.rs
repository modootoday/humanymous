use super::Error;
use std::collections::BTreeSet;

type Fields = Vec<(Vec<u8>, Vec<u8>)>;

pub(super) fn decode(
    decoder: &mut httlib_hpack::Decoder<'static>,
    mut bytes: Vec<u8>,
    trailers: bool,
) -> Result<Fields, Error> {
    let mut fields = Vec::new();
    let mut total = 0usize;
    let mut pseudo = BTreeSet::new();
    let mut regular = false;
    while !bytes.is_empty() {
        // The dependency enforces the maximum table size. Updates are legal
        // only before the first header representation, not midway through it.
        if bytes[0] & 0xe0 == 0x20 && !fields.is_empty() {
            return Err(Error::Malformed);
        }
        let before = bytes.len();
        let mut one = Vec::with_capacity(1);
        decoder
            .decode_exact(&mut bytes, &mut one)
            .map_err(|_| Error::Malformed)?;
        if bytes.len() >= before {
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
