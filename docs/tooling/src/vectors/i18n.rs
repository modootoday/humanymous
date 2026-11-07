//! Separate authored byte-boundary corpus; the original corpus stays immutable.
use super::{
    Corpus,
    author::{case, positive},
};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) fn corpus() -> Corpus {
    let mut cases = Vec::new();
    for (id, body, encoded) in [
        (
            "binary-body-no-text-normalization",
            b"\0\xff\x80".as_slice(),
            "AP+A",
        ),
        ("composed-body-exact-bytes", "é".as_bytes(), "w6k="),
        ("decomposed-body-exact-bytes", "e\u{301}".as_bytes(), "ZcyB"),
    ] {
        let mut value = positive();
        value["headers"][1][1] = json!(body.len().to_string());
        value["body"]["length"] = json!(body.len());
        value["body"]["sha256"] = json!(hex::encode(Sha256::digest(body)));
        value["body"]["base64"] = json!(encoded);
        cases.push(case(id, "node", serde_json::to_vec(&value).unwrap(), true));
    }
    for (id, target, accept) in [
        ("unicode-target-preserved", "/문서/é?이름=값", true),
        ("percent-encoded-unicode-target", "/caf%C3%A9", true),
        ("invalid-decoded-target-utf8", "/%FF", false),
        ("encoded-target-nul", "/%00", false),
        ("encoded-dot-segment", "/%2e/child", false),
    ] {
        let mut value = positive();
        value["target"] = json!(target);
        cases.push(case(
            id,
            "node",
            serde_json::to_vec(&value).unwrap(),
            accept,
        ));
    }
    let mut header = positive();
    header["headers"][2][1] = json!("한국어 / العربية");
    cases.push(case(
        "unicode-json-header-value",
        "node",
        serde_json::to_vec(&header).unwrap(),
        true,
    ));
    let mut normalized = positive();
    normalized["body"]["length"] = json!(3);
    normalized["body"]["base64"] = json!("ZcyB");
    normalized["body"]["sha256"] = json!(hex::encode(Sha256::digest("é".as_bytes())));
    cases.push(case(
        "visually-similar-body-digest-mismatch",
        "node",
        serde_json::to_vec(&normalized).unwrap(),
        false,
    ));
    Corpus {
        schema: "humanymous.experimental-parser-vectors/1".into(),
        scope: "Synthetic internationalization and binary-body adapter parser cases; no generic Draft v0 normalization, transport fidelity or independent interoperability claim".into(),
        now_ms: 1000,
        cases,
    }
}
