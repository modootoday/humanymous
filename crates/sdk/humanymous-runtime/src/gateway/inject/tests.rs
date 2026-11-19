//! Ports of legacy internal/gate/inject_test.go and the inject rounds
//! R108-R110 of wargame_escalation_r81_test.go at baseline 981d25c.
use super::*;

const SNIPPET: &str = "<!--hmn-injected--><script src=\"/__hmn/loader.js\"></script>";

/// Emits the source in fixed-size chunks to exercise streaming.
struct Chunked<'a> {
    source: &'a [u8],
    size: usize,
}

impl Read for Chunked<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = self.size.min(self.source.len()).min(out.len());
        out[..count].copy_from_slice(&self.source[..count]);
        self.source = &self.source[count..];
        Ok(count)
    }
}

fn run(html: &str, size: usize) -> (String, Outcome) {
    let mut injector = Injector::new(
        Chunked {
            source: html.as_bytes(),
            size,
        },
        SNIPPET,
    );
    let mut out = String::new();
    injector.read_to_string(&mut out).unwrap();
    (out, injector.outcome().unwrap())
}

#[test]
fn injects_right_after_head_across_every_split() {
    let html = "<html><head><title>x</title></head><body>hi</body></html>";
    for size in [1, 3, 7, 4096] {
        let (out, outcome) = run(html, size);
        assert_eq!(outcome, Outcome::Injected, "size {size}");
        assert_eq!(out, html.replacen("<head>", &format!("<head>{SNIPPET}"), 1));
    }
}

#[test]
fn falls_back_to_before_body() {
    let (out, outcome) = run("<html><body>no head here</body></html>", 5);
    assert_eq!(outcome, Outcome::Injected);
    assert!(out.contains(&format!("{SNIPPET}<body>")), "{out}");
}

#[test]
fn marker_prevents_a_second_injection() {
    let html = format!("<html><head>{MARKER}</head><body>x</body></html>");
    let (out, outcome) = run(&html, 6);
    assert_eq!(outcome, Outcome::Skipped(Skipped::AlreadyPresent));
    assert_eq!(out, html);
    assert_eq!(Skipped::AlreadyPresent.reason(), "already_present");
}

#[test]
fn non_html_passes_through_unchanged() {
    let body = "just some plain text, definitely not html, no tags at all";
    let (out, outcome) = run(body, 8);
    assert_eq!(outcome, Outcome::Skipped(Skipped::PointNotFound));
    assert_eq!(Skipped::PointNotFound.reason(), "point_not_found");
    assert_eq!(out, body);
}

#[test]
fn oversize_headless_body_is_bounded_and_loses_no_bytes() {
    // An unclosed `<head` never yields a point; the look-ahead budget ends the
    // search and every byte passes through unchanged (HR-27a).
    let big = format!("<html><head{}", "A".repeat(MAX_LOOKAHEAD + 1024));
    let (out, outcome) = run(&big, 4096);
    assert_eq!(outcome, Outcome::Skipped(Skipped::PointNotFound));
    assert_eq!(out, big);
    // An open head with a huge body injects once and keeps every byte.
    let opened = format!("<html><head>{}", "A".repeat(MAX_LOOKAHEAD + 1024));
    let (out, outcome) = run(&opened, 4096);
    assert_eq!(outcome, Outcome::Injected);
    assert_eq!(out.len(), opened.len() + SNIPPET.len());
}

#[test]
fn insertion_point_order_and_case() {
    assert_eq!(insertion_point(b"<HTML><HEAD lang=x><b>"), Some(19));
    assert_eq!(insertion_point(b"<head"), None);
    assert_eq!(insertion_point(b"<html></HEAD><body>"), Some(6));
    assert_eq!(insertion_point(b"<html><BODY class=a>"), Some(6));
    assert_eq!(insertion_point(b"plain"), None);
}

#[test]
fn insertion_point_precedence_keeps_byte_offsets() {
    // Without an opening <head, </head> wins over a later <body.
    assert_eq!(insertion_point(b"<title></Head><BODY>"), Some(7));
    // Non-ASCII text before the tag keeps byte offsets exact: the search is
    // ASCII case-insensitive, so no Unicode lowering can shift them.
    let text = "\u{130}\u{130}<head>".as_bytes();
    assert_eq!(insertion_point(text), Some(text.len()));
}

/// A marker split across reads right after the point is still detected: the
/// injector waits for the bytes after the point before deciding.
#[test]
fn marker_split_across_reads_after_the_point_is_detected() {
    let html = format!("<html><head>{MARKER}<title>x</title></head>");
    for size in 1..=MARKER.len() + 8 {
        let (out, outcome) = run(&html, size);
        assert_eq!(
            outcome,
            Outcome::Skipped(Skipped::AlreadyPresent),
            "size {size}"
        );
        assert_eq!(out, html);
    }
}

/// Go TestWargameR108_InjectIdempotentDouble: a second pass over injected
/// HTML adds no second loader.
#[test]
fn wargame_r108_inject_idempotent_double() {
    let (once, _) = run("<html><head></head><body>x</body></html>", 64);
    let (twice, outcome) = run(&once, 64);
    assert_eq!(outcome, Outcome::Skipped(Skipped::AlreadyPresent));
    assert_eq!(twice.matches("loader.js").count(), 1);
}

/// Go TestWargameR109_NoHTMLNoInject: a JSON body receives no loader.
#[test]
fn wargame_r109_no_html_no_inject() {
    let (out, outcome) = run(r#"{"json":true}"#, 64);
    assert!(!out.contains("loader.js"));
    assert_eq!(outcome, Outcome::Skipped(Skipped::PointNotFound));
}

/// Go TestWargameR110_OversizeHeadGivesUp only required no panic on a 1 MiB
/// tagless body; this also pins that it passes through unchanged.
#[test]
fn wargame_r110_oversize_head_gives_up() {
    let big = "a".repeat(1 << 20);
    let (out, outcome) = run(&big, 4096);
    assert_eq!(outcome, Outcome::Skipped(Skipped::PointNotFound));
    assert_eq!(out, big);
}

#[test]
fn upstream_error_surfaces_after_the_buffered_bytes() {
    struct Failing(bool);
    impl Read for Failing {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                return Err(io::Error::other("upstream reset"));
            }
            self.0 = true;
            out[..5].copy_from_slice(b"<html");
            Ok(5)
        }
    }
    let mut injector = Injector::new(Failing(false), SNIPPET);
    let mut out = [0; 64];
    assert_eq!(injector.read(&mut out).unwrap(), 5);
    assert_eq!(&out[..5], b"<html");
    assert_eq!(
        injector.outcome(),
        Some(Outcome::Skipped(Skipped::PointNotFound))
    );
    assert_eq!(
        injector.read(&mut out).unwrap_err().to_string(),
        "upstream reset"
    );
}
