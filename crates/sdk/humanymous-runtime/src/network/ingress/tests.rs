use super::*;
fn claim() -> H2Claim<'static> {
    H2Claim {
        stream: 1,
        method: "POST",
        authority: "capture.invalid",
        target: "/ingress?q=1",
        body_bytes: 3,
    }
}
fn fields() -> Vec<(Vec<u8>, Vec<u8>)> {
    [
        (":method", "POST"),
        (":authority", "capture.invalid"),
        (":scheme", "https"),
        (":path", "/ingress?q=1"),
        ("content-length", "3"),
    ]
    .into_iter()
    .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
    .collect()
}
fn field(name: &str, value: &str) -> (Vec<u8>, Vec<u8>) {
    (name.as_bytes().to_vec(), value.as_bytes().to_vec())
}
#[test]
fn decoded_data_never_claims_raw_or_native_provenance() {
    let projected = project_h2(&fields(), &claim()).unwrap();
    assert!(!projected.order_reliable && !projected.casing_reliable);
    assert_eq!(projected.method, "POST");
    assert_eq!(projected.names, ["content-length"]);
}
#[test]
fn each_missing_pseudo_field_refuses_instead_of_defaulting() {
    for index in 0..4 {
        let mut f = fields();
        f.remove(index);
        assert_eq!(project_h2(&f, &claim()), Err(Refusal::MissingField));
    }
}
#[test]
fn every_required_singleton_duplicate_is_ambiguous_even_if_equal() {
    for index in 0..5 {
        let mut f = fields();
        f.push(f[index].clone());
        assert_eq!(project_h2(&f, &claim()), Err(Refusal::AmbiguousField));
    }
    for name in ["user-agent", "cookie", "forwarded", "via"] {
        let mut f = fields();
        f.push(field(name, "a=b"));
        f.push(field(name, "a=b"));
        assert_eq!(project_h2(&f, &claim()), Err(Refusal::AmbiguousField));
    }
}
#[test]
fn parsed_request_mismatch_has_no_normalized_match_fallback() {
    let mut c = claim();
    c.method = "GET";
    assert_eq!(project_h2(&fields(), &c), Err(Refusal::RequestMismatch));
    c = claim();
    c.authority = "other.invalid";
    assert_eq!(project_h2(&fields(), &c), Err(Refusal::RequestMismatch));
    c = claim();
    c.target = "/ingress";
    assert_eq!(project_h2(&fields(), &c), Err(Refusal::RequestMismatch));
}
#[test]
fn contradictory_host_and_non_tls_scheme_refuse() {
    let mut f = fields();
    f.push(field("host", "other.invalid"));
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::AmbiguousField));
    f = fields();
    f[2] = field(":scheme", "http");
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::Protocol));
}
#[test]
fn content_length_must_be_unambiguous_decimal_and_match_observed_count() {
    for value in ["+3", "3,3", "-1", "18446744073709551616"] {
        let mut f = fields();
        f[4] = field("content-length", value);
        assert_eq!(project_h2(&f, &claim()), Err(Refusal::InvalidField));
    }
    let mut c = claim();
    c.body_bytes = 4;
    assert_eq!(project_h2(&fields(), &c), Err(Refusal::BodyLength));
}
#[test]
fn invalid_utf8_control_and_field_budgets_refuse() {
    let mut f = fields();
    f.push((b"user-agent".to_vec(), vec![0xff]));
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::InvalidField));
    f = fields();
    f.push(field("user-agent", "x\r\ny"));
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::InvalidField));
    f = fields();
    f.extend(std::iter::repeat_n(field("x-tag", "a"), 124));
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::Budget));
    f = fields();
    f.push((b"x-tag".to_vec(), vec![b'a'; 65536]));
    assert_eq!(project_h2(&f, &claim()), Err(Refusal::Budget));
}
#[test]
fn unknown_repeated_fields_keep_order_without_claiming_reliability() {
    let mut f = fields();
    f.push(field("x-tag", "one"));
    f.push(field("x-tag", "two"));
    let p = project_h2(&f, &claim()).unwrap();
    assert_eq!(p.names, ["content-length", "x-tag", "x-tag"]);
    assert!(!p.order_reliable);
}
