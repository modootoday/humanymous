use unicode_normalization::{
    IsNormalized, UnicodeNormalization,
    char::{canonical_combining_class, decompose_compatible},
    is_nfkc_quick,
};

struct StreamSafe<'a> {
    chars: std::str::Chars<'a>,
    count: usize,
    pending: Option<char>,
}
impl Iterator for StreamSafe<'_> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if let Some(character) = self.pending.take() {
            return Some(character);
        }
        let character = self.chars.next()?;
        let (mut leading, mut trailing, mut starter) = (0, 0, false);
        decompose_compatible(character, |part| {
            // Go's stream-safe policy also counts backwards-combining zero-CCC
            // characters (notably Jamo V/T), unlike plain UAX15 stream_safe().
            let nonstarter = canonical_combining_class(part) != 0
                || is_nfkc_quick(std::iter::once(part)) == IsNormalized::Maybe;
            if nonstarter {
                trailing += 1;
                if !starter {
                    leading += 1;
                }
            } else {
                starter = true;
                trailing = 0;
            }
        });
        let joiner = self.count + leading > 30;
        if joiner {
            self.count = 0;
        }
        self.count = if leading > 0 {
            self.count + leading
        } else {
            trailing
        };
        if joiner {
            self.pending = Some(character);
            Some('\u{34f}')
        } else {
            Some(character)
        }
    }
}

/// The same Go stream-safe Unicode15 transform without collecting an intermediate
/// safe string. Each independent traversal starts with fresh normalization state.
pub(super) fn nfc_chars(value: &str) -> impl Iterator<Item = char> + '_ {
    StreamSafe {
        chars: value.chars(),
        count: 0,
        pending: None,
    }
    .nfc()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn nfc(value: &str) -> String {
        nfc_chars(value).collect()
    }
    #[test]
    fn unicode_data_and_stream_safe_boundary_are_frozen() {
        assert_eq!(unicode_normalization::UNICODE_VERSION, (15, 0, 0));
        assert_eq!(nfc("e\u{301}"), "é");
        assert!(!nfc(&format!("a{}", "\u{300}".repeat(30))).contains('\u{34f}'));
        assert!(nfc(&format!("a{}", "\u{300}".repeat(31))).contains('\u{34f}'));
    }

    #[test]
    fn canonical_decompositions_and_long_streams_match_go_unicode15() {
        #[derive(serde::Deserialize)]
        struct Text {
            input: String,
            expected: String,
        }
        #[derive(serde::Deserialize)]
        struct Corpus {
            unicode: String,
            texts: Vec<Text>,
        }
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../tests/fixtures/records.json")).unwrap();
        assert_eq!(corpus.unicode, "15.0.0");
        assert!(corpus.texts.len() > 13000);
        for case in corpus.texts {
            assert_eq!(nfc(&case.input), case.expected, "input {:?}", case.input);
        }
    }
}
