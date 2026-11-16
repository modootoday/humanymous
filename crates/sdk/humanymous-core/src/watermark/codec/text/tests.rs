use super::*;
use crate::watermark::frame::{MAX_CANDIDATES, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

fn limits() -> Limits {
    Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
}

#[test]
fn all_text_channels_preserve_source_prefix_and_recover() {
    for (source, syntax) in [
        (b"42".as_slice(), Syntax::Json),
        (b"{\"a\":1}", Syntax::Json),
        (b"<svg/>", Syntax::Markup),
        (b"void 0;", Syntax::BlockComment),
    ] {
        let out = encode(source, b"carrier", syntax, limits()).unwrap();
        assert!(out.starts_with(source));
        assert_eq!(recover(&out, limits()).unwrap(), b"carrier");
        if syntax == Syntax::Json {
            let mut partial_group = out.clone();
            partial_group.push(b' ');
            assert_eq!(recover(&partial_group, limits()).unwrap(), b"carrier");
            assert!(
                out[source.len()..]
                    .iter()
                    .all(|byte| matches!(*byte, b' ' | b'\t' | b'\n'))
            );
        }
    }
}

#[test]
fn later_valid_marker_and_mixed_channel_ambiguity() {
    let mut source = b"/*!hmwm:MY======!*/".to_vec();
    source.extend_from_slice(&encode(b"", b"one", Syntax::BlockComment, limits()).unwrap());
    assert_eq!(recover(&source, limits()).unwrap(), b"one");
    source.extend_from_slice(&encode(b"", b"two", Syntax::Json, limits()).unwrap());
    assert_eq!(recover(&source, limits()), Err(Error::Ambiguous));
}

#[test]
fn input_output_and_candidate_caps_and_new_text_nul_rejection() {
    assert_eq!(
        encode(b"\0", b"a", Syntax::Json, limits()),
        Err(Error::InvalidText)
    );
    assert_eq!(
        encode(&[0xff], b"a", Syntax::Json, limits()),
        Err(Error::InvalidText)
    );
    let tiny = Limits::new(256, 8, 2).unwrap();
    assert_eq!(
        encode(b"1", b"a", Syntax::Json, tiny),
        Err(Error::ResourceLimit)
    );
    assert_eq!(recover(&vec![b'x'; 257], tiny), Err(Error::ResourceLimit));
    assert_eq!(
        recover(b"!hmwm:A!!hmwm:A!!hmwm:A!", tiny),
        Err(Error::ResourceLimit)
    );
}

/// Legacy internal/watermark TestSemantic_JSONStaysValid: marking scalar,
/// array and object JSON, including a source that already ends in
/// whitespace, keeps the document parsing to the same value.
#[test]
fn json_marking_keeps_the_parsed_value() {
    for source in [
        b"{\"user\":\"kim\",\"roles\":[\"a\",\"b\"]}".as_slice(),
        b"[1,\"two\",{\"three\":3}]",
        b"\"scalar\"",
        b"{\"a\":1}\n",
        b"[true] \t",
    ] {
        let out = encode(source, b"carrier", Syntax::Json, limits()).unwrap();
        let before: serde_json::Value = serde_json::from_slice(source).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(before, after);
        assert!(out.starts_with(source));
        assert_eq!(recover(&out, limits()).unwrap(), b"carrier");
    }
}
