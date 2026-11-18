//! Ports of legacy internal/gate/inject_test.go and the inject rounds
//! R108-R110 of wargame_escalation_r81_test.go (oracle 981d25c).
use super::*;

const SNIPPET: &str = "<!--hmn-injected--><script src=\"/__hmn/loader.js\"></script>";

/// Emits the source in fixed-size chunks to exercise streaming (Go
/// `chunkedReader`).
struct Chunked<'a> {
    bytes: &'a [u8],
    step: usize,
}

impl Read for Chunked<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let take = self.step.min(out.len()).min(self.bytes.len());
        out[..take].copy_from_slice(&self.bytes[..take]);
        self.bytes = &self.bytes[take..];
        Ok(take)
    }
}

/// Go `runInject`: read everything and report the decision.
fn run(body: &str, step: usize) -> (String, Outcome) {
    let mut reader = InjectingReader::new(
        Chunked {
            bytes: body.as_bytes(),
            step,
        },
        SNIPPET,
    );
    let mut out = Vec::new();
    reader.read_to_end(&mut out).unwrap();
    (String::from_utf8(out).unwrap(), reader.outcome())
}

#[test]
fn inject_after_head() {
    let html = "<html><head><title>x</title></head><body>hi</body></html>";
    for step in [1, 3, 7, 4096] {
        let (out, outcome) = run(html, step);
        assert_eq!(outcome, Outcome::Injected, "step {step}");
        // Add-only: origin bytes preserved, snippet right after <head>.
        assert_eq!(out, html.replacen("<head>", &format!("<head>{SNIPPET}"), 1));
    }
}

#[test]
fn inject_fallback_before_body() {
    let (out, outcome) = run("<html><body>no head here</body></html>", 5);
    assert_eq!(outcome, Outcome::Injected);
    assert!(out.contains(&format!("{SNIPPET}<body>")), "{out}");
}

#[test]
fn idempotent_no_double_inject() {
    let html = format!("<html><head>{MARKER}</head><body>x</body></html>");
    let (out, outcome) = run(&html, 6);
    assert_eq!(outcome, Outcome::NotInjected(Reason::AlreadyPresent));
    assert_eq!(out.matches(MARKER).count(), 1);
    assert_eq!(out, html);
}

#[test]
fn no_html_pass_through() {
    let body = "just some plain text, definitely not html, no tags at all";
    let (out, outcome) = run(body, 8);
    assert_eq!(outcome, Outcome::NotInjected(Reason::PointNotFound));
    assert_eq!(out, body);
}

#[test]
fn oversize_headless_body_gives_up() {
    // Go's case opens <head>, so the snippet lands right after it and no byte
    // is lost.
    let big = format!("<html><head>{}", "A".repeat(MAX_LOOKAHEAD + 1024));
    let (out, outcome) = run(&big, 4096);
    assert_eq!(outcome, Outcome::Injected);
    assert_eq!(out.len(), big.len() + SNIPPET.len());
    // A body whose boundary lies beyond the look-ahead budget passes through
    // untouched instead of buffering without limit (HR-27a).
    let late = format!("<html>{}<body>x</body>", "A".repeat(MAX_LOOKAHEAD + 1024));
    let (out, outcome) = run(&late, 4096);
    assert_eq!(outcome, Outcome::NotInjected(Reason::PointNotFound));
    assert_eq!(out, late);
}

#[test]
fn wargame_r108_inject_idempotent_double() {
    let (once, _) = run("<html><head></head><body>x</body></html>", 64);
    let (twice, outcome) = run(&once, 64);
    assert_eq!(outcome, Outcome::NotInjected(Reason::AlreadyPresent));
    assert_eq!(twice.matches("loader.js").count(), 1);
}

#[test]
fn wargame_r109_no_html_no_inject() {
    let (out, outcome) = run(r#"{"json":true}"#, 64);
    assert!(!out.contains("loader.js"));
    assert_eq!(outcome, Outcome::NotInjected(Reason::PointNotFound));
}

#[test]
fn wargame_r110_oversize_head_gives_up() {
    let big = "a".repeat(1 << 20);
    let (out, outcome) = run(&big, 4096);
    assert_eq!(outcome, Outcome::NotInjected(Reason::PointNotFound));
    assert_eq!(out, big);
}

#[test]
fn insertion_point_precedence_and_waiting() {
    assert_eq!(insertion_point(b"<html><HEAD lang=x>t"), Some(19));
    assert_eq!(insertion_point(b"<title></Head><BODY>"), Some(7));
    assert_eq!(insertion_point(b"<html><Body class=a>"), Some(6));
    // An unclosed <head waits for more bytes even when </head> follows later.
    assert_eq!(insertion_point(b"<html><head"), None);
    assert_eq!(insertion_point(b"no tags"), None);
    // Non-ASCII text before the tag keeps byte offsets exact.
    let text = "\u{130}\u{130}<head>".as_bytes();
    assert_eq!(insertion_point(text), Some(text.len()));
}

#[test]
fn marker_split_across_reads_after_the_point_is_detected() {
    let html = format!("<html><head>{MARKER}<title>x</title></head>");
    for step in 1..=MARKER.len() + 8 {
        let (out, outcome) = run(&html, step);
        assert_eq!(outcome, Outcome::NotInjected(Reason::AlreadyPresent));
        assert_eq!(out, html);
    }
}

#[test]
fn upstream_error_surfaces_after_the_buffered_bytes() {
    struct Failing(bool);
    impl Read for Failing {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if std::mem::replace(&mut self.0, true) {
                return Err(io::Error::other("upstream reset"));
            }
            out[..6].copy_from_slice(b"<html>");
            Ok(6)
        }
    }
    let mut reader = InjectingReader::new(Failing(false), SNIPPET);
    let mut out = [0u8; 64];
    assert_eq!(reader.read(&mut out).unwrap(), 6);
    assert_eq!(&out[..6], b"<html>");
    assert_eq!(
        reader.outcome(),
        Outcome::NotInjected(Reason::PointNotFound)
    );
    assert_eq!(
        reader.read(&mut out).unwrap_err().to_string(),
        "upstream reset"
    );
    assert_eq!(Reason::PointNotFound.as_str(), "point_not_found");
    assert_eq!(Reason::AlreadyPresent.as_str(), "already_present");
}
