//! Exact legacy braced marker. A matching string is not native-code attestation.

/// Recognize `{\s*[native code]\s*}` with ECMAScript whitespace semantics.
/// The acquisition boundary must enforce its source-text budget before copying.
pub fn native_source_marker(source: &str) -> bool {
    source.match_indices('{').any(|(offset, _)| {
        source[offset + 1..]
            .trim_start_matches(whitespace)
            .strip_prefix("[native code]")
            .is_some_and(|tail| tail.trim_start_matches(whitespace).starts_with('}'))
    })
}
fn whitespace(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_requires_braces_and_preserves_unanchored_legacy_match() {
        for source in [
            "function fetch() { [native code] }",
            "prefix{[native code]}suffix",
            "{no}{ [native code] }",
        ] {
            assert!(native_source_marker(source));
        }
        for source in [
            "[native code]",
            "{native code}",
            "{ [Native code] }",
            "{ [native  code] }",
            "{ [native code] suffix }",
        ] {
            assert!(!native_source_marker(source));
        }
    }
    #[test]
    fn ecmascript_whitespace_includes_bom_but_excludes_nel_and_zero_width_space() {
        for c in [
            '\t', '\n', '\r', '\u{b}', '\u{c}', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{200a}',
            '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
        ] {
            assert!(native_source_marker(&format!("{{{c}[native code]{c}}}")));
        }
        for c in ['\u{85}', '\u{180e}', '\u{200b}'] {
            assert!(!native_source_marker(&format!("{{{c}[native code]}}")));
        }
    }
}
