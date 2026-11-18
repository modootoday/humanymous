//! Cross-language wire compatibility shared by every contract consumer.
pub mod decode;
use serde::Deserialize;
use std::fmt::Write;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct UnicodeData {
    version: String,
    lower: Vec<[u32; 2]>,
    printable: Vec<[u32; 2]>,
}

fn unicode() -> &'static UnicodeData {
    static DATA: OnceLock<UnicodeData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("unicode.json")).expect("validated frozen Unicode tables")
    })
}

/// Unicode edition frozen by the source wire contract.
pub fn unicode_version() -> &'static str {
    &unicode().version
}

/// Quote a UTF-8 string with Go encoding/json's default HTML and line-separator
/// escaping. This does not claim general JSON number/struct canonicalization.
pub fn quote_json_go(value: &str) -> String {
    let mut output = Vec::new();
    quote_json_go_to(value, &mut output).expect("vector write cannot fail");
    String::from_utf8(output).expect("quoted UTF-8 remains UTF-8")
}

/// Write the same Go-compatible JSON string without allocating an escaped copy.
/// A sink error is returned immediately; partial output must not be committed.
pub fn quote_json_go_to(value: &str, output: &mut impl std::io::Write) -> std::io::Result<()> {
    output.write_all(b"\"")?;
    for character in value.chars() {
        let escaped = match character {
            '"' => Some("\\\""),
            '\\' => Some("\\\\"),
            '\u{8}' => Some("\\b"),
            '\u{c}' => Some("\\f"),
            '\n' => Some("\\n"),
            '\r' => Some("\\r"),
            '\t' => Some("\\t"),
            '<' => Some("\\u003c"),
            '>' => Some("\\u003e"),
            '&' => Some("\\u0026"),
            '\u{2028}' => Some("\\u2028"),
            '\u{2029}' => Some("\\u2029"),
            _ => None,
        };
        if let Some(escaped) = escaped {
            output.write_all(escaped.as_bytes())?;
        } else if character <= '\u{1f}' {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let code = character as usize;
            output.write_all(&[b'\\', b'u', b'0', b'0', HEX[code >> 4], HEX[code & 15]])?;
        } else {
            output.write_all(character.encode_utf8(&mut [0; 4]).as_bytes())?;
        }
    }
    output.write_all(b"\"")
}

/// Represent a float32 payload using its shortest round-trip decimal, as Go's
/// JSON encoder does for float32 interface values. Nonfinite values become null,
/// matching serde_json's float serialization; valid model outputs are finite.
pub fn float32_json_value(value: f32) -> serde_json::Value {
    if !value.is_finite() {
        return serde_json::Value::Null;
    }
    serde_json::from_str(&value.to_string()).expect("finite float32 JSON number")
}

/// Shortest round-trip decimal digits of a finite value: sign, significant
/// digits without leading zeros (empty for zero) and the decimal point
/// position, so that the magnitude is `0.digits * 10^point`.
fn shortest_digits(value: f64) -> (bool, Vec<u8>, i32) {
    let negative = value.is_sign_negative();
    if value == 0.0 {
        return (negative, Vec::new(), 0);
    }
    // Rust's LowerExp without precision is the shortest round-trip form,
    // the same digit string Go's strconv selects with precision -1.
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').expect("LowerExp has an exponent");
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    (negative, digits, exponent + 1)
}

/// Go strconv `%e` layout of shortest digits with `precision` fraction digits.
fn go_exponent(output: &mut String, digits: &[u8], point: i32, precision: usize) {
    output.push(char::from(*digits.first().unwrap_or(&b'0')));
    if precision > 0 {
        output.push('.');
        for index in 1..=precision {
            output.push(char::from(*digits.get(index).unwrap_or(&b'0')));
        }
    }
    let exponent = if digits.is_empty() { 0 } else { point - 1 };
    output.push('e');
    output.push(if exponent < 0 { '-' } else { '+' });
    let magnitude = exponent.unsigned_abs();
    if magnitude < 10 {
        output.push('0');
    }
    write!(output, "{magnitude}").expect("string write");
}

/// Go strconv `%f` layout of shortest digits with `precision` fraction digits.
fn go_fixed(output: &mut String, digits: &[u8], point: i32, precision: usize) {
    if point > 0 {
        for index in 0..point as usize {
            output.push(char::from(*digits.get(index).unwrap_or(&b'0')));
        }
    } else {
        output.push('0');
    }
    if precision > 0 {
        output.push('.');
        for index in 1..=precision as i32 {
            let position = point + index - 1;
            let digit = usize::try_from(position)
                .ok()
                .and_then(|position| digits.get(position))
                .copied()
                .unwrap_or(b'0');
            output.push(char::from(digit));
        }
    }
}

fn go_nonfinite(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value == f64::INFINITY {
        Some("+Inf")
    } else if value == f64::NEG_INFINITY {
        Some("-Inf")
    } else {
        None
    }
}

/// Format a float64 like Go `fmt` `%v` (`strconv.FormatFloat(v, 'g', -1, 64)`):
/// shortest digits, exponent form below 1e-4 or from 1e21 digit position six.
pub fn format_float_go(value: f64) -> String {
    if let Some(text) = go_nonfinite(value) {
        return text.into();
    }
    let (negative, digits, point) = shortest_digits(value);
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    let exponent = point - 1;
    if !digits.is_empty() && !(-4..6).contains(&exponent) {
        go_exponent(&mut output, &digits, point, digits.len() - 1);
    } else {
        let precision = usize::try_from(digits.len() as i32 - point).unwrap_or(0);
        go_fixed(&mut output, &digits, point, precision);
    }
    output
}

/// Format a float64 like Go `fmt` `%.Nf`, including Go's non-finite spellings.
pub fn format_fixed_go(value: f64, precision: usize) -> String {
    match go_nonfinite(value) {
        Some(text) => text.into(),
        None => format!("{value:.precision$}"),
    }
}

/// Encode a float64 as Go `encoding/json` does. Go refuses NaN and infinities
/// with an unsupported-value error, so they yield `None` here.
pub fn json_float_go(value: f64) -> Option<String> {
    if go_nonfinite(value).is_some() {
        return None;
    }
    let (negative, digits, point) = shortest_digits(value);
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-6..1e21).contains(&magnitude) {
        go_exponent(&mut output, &digits, point, digits.len() - 1);
        // Go trims one leading zero from a negative two-digit exponent.
        if let Some(index) = output.rfind("e-0") {
            output.remove(index + 2);
        }
    } else {
        let precision = usize::try_from(digits.len() as i32 - point).unwrap_or(0);
        go_fixed(&mut output, &digits, point, precision);
    }
    Some(output)
}

/// Unicode simple lowercase used by the frozen Go wire contracts.
/// Unlike full string lowercase, this neither expands dotted I nor applies
/// contextual final sigma mappings.
pub fn simple_lowercase(value: &str) -> String {
    if value.is_ascii() {
        return value.to_ascii_lowercase();
    }
    let mappings = &unicode().lower;
    value
        .chars()
        .map(|character| {
            mappings
                .binary_search_by_key(&(character as u32), |pair| pair[0])
                .ok()
                .and_then(|index| char::from_u32(mappings[index][1]))
                .unwrap_or(character)
        })
        .collect()
}

/// Quote a valid UTF-8 string with the frozen Go `%q` diagnostic escaping.
pub fn quote_go(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{7}' => result.push_str("\\a"),
            '\u{8}' => result.push_str("\\b"),
            '\u{c}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{b}' => result.push_str("\\v"),
            _ => {
                let code = character as u32;
                let ranges = &unicode().printable;
                let index = ranges.partition_point(|range| range[0] <= code);
                if index > 0 && code <= ranges[index - 1][1] {
                    result.push(character);
                } else if code < 32 || code == 127 {
                    write!(result, "\\x{code:02x}").expect("string write");
                } else if code < 65536 {
                    write!(result, "\\u{code:04x}").expect("string write");
                } else {
                    write!(result, "\\U{code:08x}").expect("string write");
                }
            }
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_json_matches_prior_encoder_for_control_and_unicode_ranges() {
        let text: String = (0..=0x10ffff).filter_map(char::from_u32).collect();
        let expected = serde_json::to_string(&text)
            .unwrap()
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        assert_eq!(quote_json_go(&text), expected);
        let mut bytes = Vec::new();
        quote_json_go_to(&text, &mut bytes).unwrap();
        assert_eq!(bytes, expected.as_bytes());
    }

    #[test]
    fn streaming_json_propagates_partial_sink_failure() {
        struct Limited(Vec<u8>);
        impl std::io::Write for Limited {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let remaining = 4_usize.saturating_sub(self.0.len());
                if remaining == 0 {
                    return Err(std::io::Error::other("injected sink failure"));
                }
                let count = remaining.min(bytes.len());
                self.0.extend_from_slice(&bytes[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut sink = Limited(Vec::new());
        assert!(quote_json_go_to("<secret", &mut sink).is_err());
        assert_eq!(sink.0, b"\"\\u0");
    }
    #[test]
    fn unicode_simple_mapping_does_not_expand_or_depend_on_context() {
        assert_eq!(simple_lowercase("İ ΟΣ User-Agent"), "i οσ user-agent");
    }
    #[test]
    fn json_string_escaping_preserves_go_signed_byte_contracts() {
        assert_eq!(
            quote_json_go("<&>\u{2028}\u{2029}"),
            "\"\\u003c\\u0026\\u003e\\u2028\\u2029\""
        );
        assert_eq!(quote_json_go("é\n\"\\\0"), "\"é\\n\\\"\\\\\\u0000\"");
        assert_eq!(quote_json_go("\\u003c"), "\"\\\\u003c\"");
    }
    #[test]
    fn tables_and_quoting_match_frozen_unicode_contract() {
        let data = unicode();
        assert_eq!(data.version, "15.0.0");
        assert!(data.lower.windows(2).all(|pair| pair[0][0] < pair[1][0]));
        assert!(
            data.printable
                .windows(2)
                .all(|pair| pair[0][1] < pair[1][0])
        );
        #[derive(Deserialize)]
        struct Case {
            input: String,
            lower: String,
            quoted: String,
        }
        #[derive(Deserialize)]
        struct Corpus {
            cases: Vec<Case>,
        }
        let corpus: Corpus = serde_json::from_str(include_str!("unicode.json")).unwrap();
        assert_eq!(corpus.cases.len(), 8);
        for case in corpus.cases {
            assert_eq!(simple_lowercase(&case.input), case.lower);
            assert_eq!(quote_go(&case.input), case.quoted);
        }
    }
    /// Values captured from Go 1.26.3 `%v`, `%.1f` and `json.Marshal` at the
    /// frozen oracle commit 981d25c (settings overlay golden capture).
    #[test]
    fn float_formatting_matches_go_fmt_and_encoding_json() {
        for (value, v, fixed, json) in [
            (0.0, "0", "0.0", Some("0")),
            (-0.0, "-0", "-0.0", Some("-0")),
            (-1.0, "-1", "-1.0", Some("-1")),
            (2.5, "2.5", "2.5", Some("2.5")),
            (29.95, "29.95", "29.9", Some("29.95")),
            (60.05, "60.05", "60.0", Some("60.05")),
            (60.25, "60.25", "60.2", Some("60.25")),
            (0.05, "0.05", "0.1", Some("0.05")),
            (100_000.0, "100000", "100000.0", Some("100000")),
            (1e6, "1e+06", "1000000.0", Some("1000000")),
            (
                123_456_789.0,
                "1.23456789e+08",
                "123456789.0",
                Some("123456789"),
            ),
            (0.0001, "0.0001", "0.0", Some("0.0001")),
            (0.00001, "1e-05", "0.0", Some("0.00001")),
            (1e-7, "1e-07", "0.0", Some("1e-7")),
            (1.5e-7, "1.5e-07", "0.0", Some("1.5e-7")),
            (
                1e20,
                "1e+20",
                "100000000000000000000.0",
                Some("100000000000000000000"),
            ),
            (1e21, "1e+21", "1000000000000000000000.0", Some("1e+21")),
            (5e-324, "5e-324", "0.0", Some("5e-324")),
            (2.675, "2.675", "2.7", Some("2.675")),
            (
                33.333333333333336,
                "33.333333333333336",
                "33.3",
                Some("33.333333333333336"),
            ),
            (f64::NAN, "NaN", "NaN", None),
            (f64::INFINITY, "+Inf", "+Inf", None),
            (f64::NEG_INFINITY, "-Inf", "-Inf", None),
        ] {
            assert_eq!(format_float_go(value), v, "{value:e}");
            assert_eq!(format_fixed_go(value, 1), fixed, "{value:e}");
            assert_eq!(json_float_go(value).as_deref(), json, "{value:e}");
        }
    }
    #[test]
    fn float32_payload_does_not_expand_to_float64_noise() {
        assert_eq!(float32_json_value(0.2), serde_json::json!(0.2));
        assert_eq!(float32_json_value(0.8), serde_json::json!(0.8));
        assert_eq!(float32_json_value(f32::NAN), serde_json::Value::Null);
    }
}
