use super::*;

fn binding() -> Binding<'static> {
    Binding {
        installation: "i",
        tenant: "t",
        origin_identity: "o",
        revision: "r",
        authority: "example.test",
        session: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        method: "POST",
        target: "/api?q=1",
        counter: 1,
        bucket: 9,
        deadline: 2000,
        content_type: "\t text/plain \t",
        application_authorization: " Bearer example ",
        body: b"abc",
    }
}

#[test]
fn fixed_byte_vector_has_exact_field_order_lengths_and_terminal_newline() {
    let value = canonical(&binding()).unwrap();
    let expected = concat!(
        "humanymous.browser/0.1\n1:i\n1:t\n1:o\n1:r\n12:example.test\n",
        "43:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n4:POST\n8:/api?q=1\n",
        "1:1\n1:9\n4:2000\n10:text/plain\n14:Bearer example\n",
        "64:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n"
    );
    assert_eq!(value, expected);
    // Independently computed with .NET SHA256/HMACSHA256, including big-endian
    // counter1 and bucket9 appended by the existing token primitive.
    assert_eq!(
        hex::encode(Sha256::digest(value.as_bytes())),
        "5adf0a6ad00983604cbbe60101f6c0320b7f78850bc759518cf7106fc81396d4"
    );
    assert_eq!(
        super::super::compute_token(b"seed", &value, 1, 9),
        "4QwQccK0NkhFMsxdltz1xNqEwaV2EeWwk5hoOhPxHOg"
    );
}

#[test]
fn bounded_request_and_header_rejections_are_static() {
    for target in [
        "//evil/a", "/a/../b", "/x%2fy", "/bad%", "/a#f", "/é", "/a b",
    ] {
        let mut b = binding();
        b.target = target;
        assert_eq!(canonical(&b), Err(BindingError::Request));
    }
    for method in ["post", "CONNECT", "TRACE", "POST\n"] {
        let mut b = binding();
        b.method = method;
        assert_eq!(canonical(&b), Err(BindingError::Request));
    }
    for header in [
        "Bearer secret\r\nInjected: yes",
        "a\tb",
        "é",
        "a\0b",
        "a\u{7f}",
    ] {
        let mut b = binding();
        b.application_authorization = header;
        assert_eq!(canonical(&b), Err(BindingError::Header));
    }
    let large = "a".repeat(8193);
    let mut b = binding();
    b.application_authorization = &large;
    assert_eq!(canonical(&b), Err(BindingError::Header));
    let body = vec![0; 1_048_577];
    let mut b = binding();
    b.body = &body;
    assert_eq!(canonical(&b), Err(BindingError::Body));
    let mut b = binding();
    b.method = "GET";
    assert_eq!(canonical(&b), Err(BindingError::Request));
}

#[test]
fn session_scope_and_integer_bounds_do_not_silently_normalize() {
    for session in [
        "",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB",
    ] {
        let mut b = binding();
        b.session = session;
        assert_eq!(canonical(&b), Err(BindingError::Session));
    }
    let mut b = binding();
    b.tenant = "t\n";
    assert_eq!(canonical(&b), Err(BindingError::Scope));
    let mut b = binding();
    b.authority = "user@example.test";
    assert_eq!(canonical(&b), Err(BindingError::Scope));
    let mut b = binding();
    b.counter = 0;
    assert_eq!(canonical(&b), Err(BindingError::Integer));
    for field in [0, 1, 2] {
        let mut b = binding();
        match field {
            0 => b.counter = MAX_SAFE_INTEGER + 1,
            1 => b.bucket = MAX_SAFE_INTEGER + 1,
            _ => b.deadline = MAX_SAFE_INTEGER + 1,
        }
        assert_eq!(canonical(&b), Err(BindingError::Integer));
    }
    let mut b = binding();
    b.counter = MAX_SAFE_INTEGER;
    b.bucket = MAX_SAFE_INTEGER;
    b.deadline = MAX_SAFE_INTEGER;
    assert!(canonical(&b).is_ok());
}

#[test]
fn every_scope_request_header_and_body_field_changes_the_signature() {
    let baseline = canonical(&binding()).unwrap();
    for field in 0..14 {
        let mut b = binding();
        match field {
            0 => b.installation = "other",
            1 => b.tenant = "other",
            2 => b.origin_identity = "other",
            3 => b.revision = "other",
            4 => b.authority = "other.test",
            5 => b.session = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE",
            6 => b.method = "PUT",
            7 => b.target = "/other",
            8 => b.counter = 2,
            9 => b.bucket = 10,
            10 => b.deadline = 2001,
            11 => b.content_type = "text/plain; charset=utf-8",
            12 => b.application_authorization = "Bearer  example",
            _ => b.body = b"abcd",
        }
        let changed = canonical(&b).unwrap();
        assert_ne!(changed, baseline, "field{field}");
        assert_ne!(
            super::super::compute_token(b"seed", &changed, b.counter, b.bucket),
            super::super::compute_token(b"seed", &baseline, 1, 9)
        );
    }
}
