use unicode_normalization::{
    IsNormalized, UnicodeNormalization,
    char::{canonical_combining_class, decompose_compatible},
    is_nfkc_quick,
};

pub(super) fn nfc(value: &str) -> String {
    let mut safe = String::with_capacity(value.len());
    let mut count = 0;
    for character in value.chars() {
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
        if count + leading > 30 {
            safe.push('\u{34f}');
            count = 0;
        }
        count = if leading > 0 {
            count + leading
        } else {
            trailing
        };
        safe.push(character);
    }
    safe.nfc().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
