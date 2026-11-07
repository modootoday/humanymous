use super::{Case, Corpus};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn positive() -> Value {
    json!({"protocol":"humanymous.adapter/0.1","id":"01234567-89ab-4cde-8fab-0123456789ab",
        "adapter":{"id":"humanymous.node","version":"0.1.0-experimental.1","installation":"example-installation"},
        "tenant":"example-tenant","origin":"example-origin","revision":"example-revision",
        "deadline":2000,"method":"POST","target":"/example?q=1","authority":"example.test",
        "peer":"127.0.0.1","connectionId":"01234567-89ab-4cde-8fab-0123456789ab","capture":null,"stream":null,
        "headers":[["Host","example.test"],["Content-Length","3"],["X-Tag","first"],["x-tag","second"]],
        "headerMode":"node-raw","body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":3,
            "sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","base64":"YWJj"},
        "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":true,"completeBody":true}})
}
pub(super) fn case(id: &str, profile: &str, bytes: Vec<u8>, accept: bool) -> Case {
    Case {
        id: id.into(),
        profile: profile.into(),
        sha256: hex::encode(Sha256::digest(&bytes)),
        input_hex: hex::encode(bytes),
        accept,
    }
}

pub(super) fn corpus() -> Corpus {
    let original = positive();
    let raw = serde_json::to_vec(&original).expect("synthetic JSON");
    let mut cases = vec![case("node-raw-valid", "node", raw.clone(), true)];
    let mut fetch = original.clone();
    fetch["headerMode"] = json!("fetch-normalized");
    fetch["peer"] = Value::Null;
    fetch["connectionId"] = Value::Null;
    fetch["capabilities"]["rawHeaderOrder"] = json!(false);
    cases.push(case(
        "fetch-normalized-valid",
        "node",
        serde_json::to_vec(&fetch).unwrap(),
        true,
    ));
    for (id, mode, raw_order) in [
        ("nginx-raw-parser-valid", "nginx-raw", true),
        ("nginx-normalized-parser-valid", "nginx-normalized", false),
    ] {
        let mut nginx = fetch.clone();
        nginx["adapter"]["id"] = json!("humanymous.nginx");
        nginx["headerMode"] = json!(mode);
        nginx["capabilities"]["rawHeaderOrder"] = json!(raw_order);
        cases.push(case(id, "nginx", serde_json::to_vec(&nginx).unwrap(), true));
    }
    for (id, pointer, value) in [
        ("deadline-equal-now", "/deadline", json!(1000)),
        ("deadline-too-far", "/deadline", json!(31001)),
        ("unsafe-integer", "/deadline", json!(9007199254740992_u64)),
        (
            "unknown-protocol",
            "/protocol",
            json!("humanymous.adapter/0.2"),
        ),
        (
            "wrong-trusted-profile",
            "/adapter/id",
            json!("humanymous.nginx"),
        ),
        ("forged-capture", "/capture", json!({})),
        ("unsupported-stream", "/stream", json!({})),
        ("forged-tls", "/capabilities/rawTls", json!(true)),
        ("partial-body", "/body/complete", json!(false)),
        ("wrong-body-length", "/body/length", json!(2)),
        ("bad-base64-padding", "/body/base64", json!("YWJj=")),
        (
            "body-digest-mismatch",
            "/body/sha256",
            json!("0".repeat(64)),
        ),
        ("ambiguous-target", "/target", json!("/example/%2fhidden")),
        (
            "header-crlf",
            "/headers/2/1",
            json!("first\r\nInjected: yes"),
        ),
    ] {
        let mut v = original.clone();
        *v.pointer_mut(pointer).expect("authored pointer") = value;
        cases.push(case(id, "node", serde_json::to_vec(&v).unwrap(), false));
    }
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("peer");
    cases.push(case(
        "missing-nullable-is-invalid",
        "node",
        serde_json::to_vec(&missing).unwrap(),
        false,
    ));
    let mut unknown = original.clone();
    unknown["body"]["unknown"] = json!(true);
    cases.push(case(
        "unknown-nested-field",
        "node",
        serde_json::to_vec(&unknown).unwrap(),
        false,
    ));
    for (id, prefix) in [
        (
            "duplicate-key",
            b"{\"protocol\":\"humanymous.adapter/0.1\",".as_slice(),
        ),
        (
            "escaped-duplicate-key",
            b"{\"protoco\\u006c\":\"humanymous.adapter/0.1\",".as_slice(),
        ),
    ] {
        let mut b = prefix.to_vec();
        b.extend_from_slice(&raw[1..]);
        cases.push(case(id, "node", b, false));
    }
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&raw);
    cases.push(case("utf8-bom", "node", bom, false));
    let mut invalid = raw.clone();
    invalid.push(0xff);
    cases.push(case("invalid-utf8", "node", invalid, false));
    Corpus { schema: "humanymous.experimental-parser-vectors/1".into(),
        scope: "Synthetic adapter parser experiment; not Draft v0 wire conformance, authenticated transport, origin effects or independent interoperability".into(),
        now_ms: 1000, cases }
}
