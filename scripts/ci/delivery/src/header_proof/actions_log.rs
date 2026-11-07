//! Strip only a bounded official-log timestamp wrapper. Payload base64 and JSON
//! are not trimmed, unescaped or rewritten; original decoded file bytes stay exact.
use super::{Limits, PREFIX};
use crate::{Result, require};

fn timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if !(20..=30).contains(&bytes.len())
        || !text.is_ascii()
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    if ![0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
        .iter()
        .all(|r| bytes[r.clone()].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    if bytes.len() != 20
        && (bytes[19] != b'.'
            || bytes.len() < 22
            || !bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<u8>().ok();
    let Some(month) = number(5..7).and_then(|n| time::Month::try_from(n).ok()) else {
        return false;
    };
    let Some(year) = text[..4].parse::<i32>().ok() else {
        return false;
    };
    let Some(day) = number(8..10) else {
        return false;
    };
    let Some(hour) = number(11..13) else {
        return false;
    };
    let Some(minute) = number(14..16) else {
        return false;
    };
    let Some(second) = number(17..19) else {
        return false;
    };
    year > 0
        && time::Date::from_calendar_date(year, month, day).is_ok()
        && time::Time::from_hms(hour, minute, second).is_ok()
}
pub(super) fn extract(bytes: &[u8], limits: &Limits) -> Result<String> {
    require(
        bytes.len() <= limits.log,
        "header official log byte bound refused",
    )?;
    let text = std::str::from_utf8(bytes).map_err(|_| "header official log UTF-8 refused")?;
    let mut protocol = String::new();
    let mut records = 0usize;
    for terminated in text.split_inclusive('\n') {
        let line = terminated.strip_suffix('\n').unwrap_or(terminated);
        let line = line.strip_suffix('\r').unwrap_or(line);
        let payload = if line.starts_with(PREFIX) {
            Some(line)
        } else if let Some((stamp, body)) = line.split_once(' ') {
            if timestamp(stamp) && body.starts_with(PREFIX) {
                Some(body)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(payload) = payload {
            require(
                terminated.ends_with('\n'),
                "header official protocol line unterminated",
            )?;
            require(
                payload.len() < limits.line && records < 4096,
                "header official protocol framing bound refused",
            )?;
            protocol.push_str(payload);
            protocol.push('\n');
            records += 1;
        } else {
            require(
                !line.contains(PREFIX),
                "header ambiguous official protocol prefix refused",
            )?;
        }
    }
    Ok(protocol)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_timestamp_wrapper_is_bounded_without_payload_normalization() {
        let p: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../adapters/native/header-proof-profile.json"
        ))
        .unwrap();
        let limits = Limits::profile(&p).unwrap();
        let body = format!("{PREFIX}{{\"type\":\"chunk\",\"base64\":\"AA0K/w==\"}}\n");
        for stamp in [
            "2026-11-07T17:06:36Z",
            "2026-11-07T17:06:36.8771284Z",
            "2024-02-29T23:59:59.123456789Z",
        ] {
            let log = format!("unrelated\n{stamp} {body}finished\n");
            assert_eq!(extract(log.as_bytes(), &limits).unwrap(), body);
        }
        assert_eq!(extract(body.as_bytes(), &limits).unwrap(), body);
        assert_eq!(
            extract(body.replace('\n', "\r\n").as_bytes(), &limits).unwrap(),
            body
        );
        for stamp in [
            "2026-02-29T17:06:36Z",
            "2026-11-07T24:06:36Z",
            "2026-11-07T17:06:36.Z",
            "2026-11-07T17:06:36.1234567890Z",
            "PRIVATE_PATH",
        ] {
            assert!(extract(format!("{stamp} {body}").as_bytes(), &limits).is_err());
        }
        assert!(extract(body.trim_end_matches('\n').as_bytes(), &limits).is_err());
        assert!(extract(format!("prefix {body}").as_bytes(), &limits).is_err());
        assert!(extract(&[0xff], &limits).is_err());
    }
}
