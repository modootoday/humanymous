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
    serde_json::to_string(value)
        .expect("string serialization cannot fail")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
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
    #[test]
    fn float32_payload_does_not_expand_to_float64_noise() {
        assert_eq!(float32_json_value(0.2), serde_json::json!(0.2));
        assert_eq!(float32_json_value(0.8), serde_json::json!(0.8));
        assert_eq!(float32_json_value(f32::NAN), serde_json::Value::Null);
    }
}
