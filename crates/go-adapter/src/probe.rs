//! Pinned upstream fixture response assertions, never HTTP authorization.
//! The lab owns bounded outer HTTP decoding and actual process/network evidence.

/// Verify the go-httpbin dump endpoint preserves the exact request target/entity.
/// This checks a known fixture representation, not arbitrary HTTP request framing.
pub fn binary_dump(
    status: u16,
    response_body: &[u8],
    method: &str,
    target: &str,
    expected_entity: &[u8],
) -> bool {
    if status != 200
        || response_body.len() > humanymous_runtime::gateway::http::MAX_BODY
        || !humanymous_runtime::gateway::configuration::method(method)
        || !humanymous_runtime::gateway::configuration::target(target)
    {
        return false;
    }
    let Some(end) = response_body.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let line = format!("{method} {target} HTTP/1.1\r\n");
    response_body[..end + 2].starts_with(line.as_bytes())
        && response_body[end + 4..] == *expected_entity
}

/// Verify the single HEAD/cookie fixture without coalescing repeated cookies.
/// Supply individual header pairs from the actual outer response parser.
pub fn head_cookies<'a>(
    status: u16,
    headers: impl IntoIterator<Item = (&'a str, &'a str)>,
    entity: &[u8],
) -> bool {
    if status != 200 || !entity.is_empty() {
        return false;
    }
    let mut count = 0;
    let mut values = 0;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            count += 1;
            values |= match value {
                "a=1" => 1,
                "b=2" => 2,
                _ => return false,
            };
            if count > 2 {
                return false;
            }
        }
    }
    count == 2 && values == 3
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dump(entity: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "POST /dump/request?q=1%2B2 HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            entity.len()
        )
        .into_bytes();
        response.extend_from_slice(entity);
        response
    }
    #[test]
    fn binary_entity_and_query_are_not_text_reconstructions() {
        let entity = b"\0\xff\r\n\r\nz";
        let response = dump(entity);
        assert!(binary_dump(
            200,
            &response,
            "POST",
            "/dump/request?q=1%2B2",
            entity
        ));
        assert!(!binary_dump(
            200,
            &response,
            "POST",
            "/dump/request?q=1+2",
            entity
        ));
        assert!(!binary_dump(
            200,
            &response,
            "POST",
            "/dump/request?q=1%2B2",
            b"changed"
        ));
        assert!(!binary_dump(
            503,
            &response,
            "POST",
            "/dump/request?q=1%2B2",
            entity
        ));
    }
    #[test]
    fn incomplete_and_oversized_fixture_responses_cannot_verify() {
        assert!(!binary_dump(200, b"partial", "POST", "/dump/request", b""));
        let response = dump(&vec![0; humanymous_runtime::gateway::http::MAX_BODY]);
        assert!(!binary_dump(
            200,
            &response,
            "POST",
            "/dump/request?q=1%2B2",
            b""
        ));
        assert!(!binary_dump(200, &dump(b""), "POST", "/bad\r\ntarget", b""));
    }
    #[test]
    fn head_requires_separate_exact_cookies_and_no_entity() {
        let headers = [("set-cookie", "b=2"), ("Set-Cookie", "a=1")];
        assert!(head_cookies(200, headers, b""));
        assert!(!head_cookies(200, headers, b"unexpected"));
        assert!(!head_cookies(503, headers, b""));
        assert!(!head_cookies(200, [("Set-Cookie", "a=1, b=2")], b""));
        assert!(!head_cookies(
            200,
            [("Set-Cookie", "a=1"), ("Set-Cookie", "a=1")],
            b""
        ));
        assert!(!head_cookies(
            200,
            [
                ("Set-Cookie", "a=1"),
                ("Set-Cookie", "b=2"),
                ("Set-Cookie", "b=2")
            ],
            b""
        ));
    }
}
