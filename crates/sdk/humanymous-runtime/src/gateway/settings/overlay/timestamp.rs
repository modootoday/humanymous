//! Overlay timestamps with the oracle's Go 1.26 `time.Time` JSON behavior.
//!
//! Parsing accepts exactly the union of Go's RFC 3339 fast path and its
//! lenient `time.Parse(RFC3339)` fallback (strict checking is disabled
//! upstream, go.dev/issue/54580): one- or two-digit hours, `.` or `,` before
//! fractional seconds and zone offsets up to 24:60. Formatting is RFC 3339 with
//! trimmed nanoseconds and refuses zone hours above 23 like `MarshalJSON`.
//! A zero offset formats as `Z`, which matches the oracle with a UTC local zone.
use std::fmt::Write as _;

/// Civil date-time with an optional fixed offset; the default is Go's zero time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
    /// East-of-UTC offset in seconds; zero formats as `Z`.
    offset: i32,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            year: 1,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
            offset: 0,
        }
    }
}

/// Refused timestamp text or an unrepresentable zone on encoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimestampError {
    /// The text is not a timestamp the oracle accepts.
    Parse(String),
    /// The zone hour cannot be encoded as RFC 3339.
    ZoneHourOutOfRange,
}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(text) => write!(formatter, "invalid timestamp {text:?}"),
            Self::ZoneHourOutOfRange => {
                formatter.write_str("Time.MarshalJSON: timezone hour outside of range [0,23]")
            }
        }
    }
}

impl std::error::Error for TimestampError {}

fn is_leap(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in(month: u8, year: u16) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn digit(&self, offset: usize) -> Option<u8> {
        self.bytes
            .get(self.position + offset)
            .filter(|byte| byte.is_ascii_digit())
            .map(|byte| byte - b'0')
    }
    /// Go `getnum`: exactly two digits when fixed, otherwise one or two.
    fn number(&mut self, fixed: bool) -> Option<u8> {
        let first = self.digit(0)?;
        match self.digit(1) {
            Some(second) => {
                self.position += 2;
                Some(first * 10 + second)
            }
            None if fixed => None,
            None => {
                self.position += 1;
                Some(first)
            }
        }
    }
    fn literal(&mut self, byte: u8) -> Option<()> {
        (self.bytes.get(self.position) == Some(&byte)).then(|| self.position += 1)
    }
    fn rest(&self) -> &[u8] {
        &self.bytes[self.position..]
    }
}

impl Timestamp {
    /// Parse the raw JSON string content exactly as the oracle's
    /// `Time.UnmarshalJSON` does; JSON escapes are not interpreted.
    pub fn parse(text: &[u8]) -> Result<Self, TimestampError> {
        Self::parse_fields(text)
            .ok_or_else(|| TimestampError::Parse(String::from_utf8_lossy(text).into_owned()))
    }

    fn parse_fields(text: &[u8]) -> Option<Self> {
        let mut cursor = Cursor {
            bytes: text,
            position: 0,
        };
        if text.len() < 4 || !text[..4].iter().all(u8::is_ascii_digit) {
            return None;
        }
        let year = text[..4]
            .iter()
            .fold(0_u16, |year, digit| year * 10 + u16::from(digit - b'0'));
        cursor.position = 4;
        cursor.literal(b'-')?;
        let month = cursor
            .number(true)
            .filter(|month| (1..=12).contains(month))?;
        cursor.literal(b'-')?;
        let day = cursor.number(true)?;
        cursor.literal(b'T')?;
        let hour = cursor.number(false).filter(|hour| *hour < 24)?;
        cursor.literal(b':')?;
        let minute = cursor.number(true).filter(|minute| *minute < 60)?;
        cursor.literal(b':')?;
        let second = cursor.number(true).filter(|second| *second < 60)?;
        let mut nanosecond = 0_u32;
        if matches!(cursor.rest(), [b'.' | b',', next, ..] if next.is_ascii_digit()) {
            cursor.position += 1;
            let mut scale = 100_000_000_u32;
            while let Some(digit) = cursor.digit(0) {
                nanosecond += u32::from(digit) * scale;
                scale /= 10;
                cursor.position += 1;
            }
        }
        let offset = match cursor.rest() {
            [b'Z', ..] => {
                cursor.position += 1;
                0
            }
            [sign @ (b'+' | b'-'), ..] => {
                let sign = if *sign == b'-' { -1 } else { 1 };
                if cursor.rest().len() < 6 || cursor.rest()[3] != b':' {
                    return None;
                }
                cursor.position += 1;
                let hours = cursor.number(true)?;
                cursor.position += 1;
                let minutes = cursor.number(true)?;
                if hours > 24 || minutes > 60 {
                    return None;
                }
                sign * (i32::from(hours) * 60 + i32::from(minutes)) * 60
            }
            _ => return None,
        };
        if !cursor.rest().is_empty() || day < 1 || day > days_in(month, year) {
            return None;
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
            offset,
        })
    }

    /// RFC 3339 with trimmed nanoseconds, as Go `Time.MarshalJSON` writes it
    /// (without the surrounding quotes).
    pub fn format(&self) -> Result<String, TimestampError> {
        let mut text = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        );
        if self.nanosecond > 0 {
            let fraction = format!("{:09}", self.nanosecond);
            write!(text, ".{}", fraction.trim_end_matches('0')).expect("string write");
        }
        if self.offset == 0 {
            text.push('Z');
            return Ok(text);
        }
        let minutes = self.offset / 60;
        let (sign, minutes) = if minutes < 0 {
            ('-', -minutes)
        } else {
            ('+', minutes)
        };
        if minutes / 60 >= 24 {
            return Err(TimestampError::ZoneHourOutOfRange);
        }
        write!(text, "{sign}{:02}:{:02}", minutes / 60, minutes % 60).expect("string write");
        Ok(text)
    }
}
