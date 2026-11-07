//! Allocation-free lexical budgets, not a JSON validator or semantic decoder.
//! Unknown numeric literals never pass through a machine-number conversion.
const MAX_DEPTH: usize = 64;

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub strings: usize,
    pub array_items: usize,
    pub values: usize,
    pub depth: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Exhausted {
    InvalidLimits,
    Strings,
    ArrayItems,
    Values,
    Depth,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub strings: usize,
    pub values: usize,
    pub depth: usize,
}

#[derive(Clone, Copy)]
struct Container {
    array: bool,
    key: bool,
    items: usize,
}

fn add(value: &mut usize, count: usize, limit: usize, error: Exhausted) -> Result<(), Exhausted> {
    match value.checked_add(count) {
        Some(next) if next <= limit => {
            *value = next;
            Ok(())
        }
        _ => Err(error),
    }
}

/// Count resources before owned decoding. Success says only that the lexical
/// budgets fit. The existing semantic decoder must still accept the same bytes.
pub(super) fn check(bytes: &[u8], limits: Limits) -> Result<Counts, Exhausted> {
    if limits.strings == 0
        || limits.array_items == 0
        || limits.values == 0
        || limits.depth == 0
        || limits.depth > MAX_DEPTH
    {
        return Err(Exhausted::InvalidLimits);
    }
    let mut stack = [Container {
        array: false,
        key: true,
        items: 0,
    }; MAX_DEPTH];
    let mut depth: usize = 0;
    let mut counts = Counts::default();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b' ' | b'\r' | b'\n' | b'\t' => {
                at += 1;
                continue;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                at += 1;
                continue;
            }
            b',' | b':' => {
                if depth > 0 {
                    stack[depth - 1].key = byte == b',';
                }
                at += 1;
                continue;
            }
            _ => {}
        }
        let key = byte == b'"' && depth > 0 && !stack[depth - 1].array && stack[depth - 1].key;
        if !key {
            add(&mut counts.values, 1, limits.values, Exhausted::Values)?;
            if depth > 0 && stack[depth - 1].array {
                add(
                    &mut stack[depth - 1].items,
                    1,
                    limits.array_items,
                    Exhausted::ArrayItems,
                )?;
            }
        }
        match byte {
            b'{' | b'[' => {
                if depth >= limits.depth {
                    return Err(Exhausted::Depth);
                }
                stack[depth] = Container {
                    array: byte == b'[',
                    key: true,
                    items: 0,
                };
                depth += 1;
                counts.depth = counts.depth.max(depth);
                at += 1;
            }
            b'"' => {
                at += 1;
                while let Some(&byte) = bytes.get(at) {
                    at += 1;
                    if byte == b'"' {
                        break;
                    }
                    let length = if byte == b'\\' {
                        escape(bytes, &mut at)
                    } else {
                        1
                    };
                    add(
                        &mut counts.strings,
                        length,
                        limits.strings,
                        Exhausted::Strings,
                    )?;
                }
            }
            _ => {
                // Delimit literals without interpreting numeric magnitude or
                // validating syntax. Existing serde remains the sole decoder.
                while let Some(byte) = bytes.get(at) {
                    if matches!(
                        byte,
                        b' ' | b'\r'
                            | b'\n'
                            | b'\t'
                            | b','
                            | b':'
                            | b'{'
                            | b'}'
                            | b'['
                            | b']'
                            | b'"'
                    ) {
                        break;
                    }
                    at += 1;
                }
            }
        }
    }
    Ok(counts)
}

fn hex(bytes: &[u8]) -> Option<u16> {
    if bytes.len() != 4 {
        return None;
    }
    bytes.iter().try_fold(0_u16, |value, byte| {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return None,
        };
        Some(value * 16 + u16::from(digit))
    })
}

fn escape(bytes: &[u8], at: &mut usize) -> usize {
    let Some(&byte) = bytes.get(*at) else {
        return 0;
    };
    *at += 1;
    if byte != b'u' {
        return 1;
    }
    let Some(code) = bytes
        .get(*at..)
        .and_then(|tail| tail.get(..4))
        .and_then(hex)
    else {
        // Malformed escapes are left for the semantic decoder to reject.
        return 1;
    };
    *at += 4;
    if (0xd800..=0xdbff).contains(&code)
        && let Some(tail) = bytes.get(*at..)
        && tail.starts_with(b"\\u")
        && let Some(low) = tail.get(2..6).and_then(hex)
        && (0xdc00..=0xdfff).contains(&low)
    {
        *at += 6;
        return 4;
    }
    match code {
        0..=0x7f => 1,
        0x80..=0x7ff => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests;
