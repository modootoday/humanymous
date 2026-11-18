//! Behavior of legacy cmd/server/resource_handlers.go (baseline 981d25c). The
//! legacy file has no Go test; these cases pin each branch of the handler, the
//! helpers and the documented fail-closed W07/W09 differences.
use super::*;
use humanymous_core::signals::report::{KeyFeatures, MouseFeatures};
use humanymous_core::watermark::ledger::Configuration;

fn headers(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn request<'a>(headers: &'a [(String, String)], rit_enforced: bool) -> ResourceRequest<'a> {
    ResourceRequest {
        session_id: "sid-1",
        headers,
        host: "example.test",
        rit_enforced,
    }
}

fn behavior(samples: i64, keystrokes: i64) -> BehaviorSummary {
    BehaviorSummary {
        mouse: MouseFeatures {
            samples,
            ..MouseFeatures::default()
        },
        key: KeyFeatures {
            keystrokes,
            ..KeyFeatures::default()
        },
        ..BehaviorSummary::default()
    }
}

fn ledger() -> Ledger {
    Ledger::new(
        Configuration {
            record_ttl: Duration::from_secs(3600),
            reservation_ttl: Duration::from_secs(10),
            capacity: 16,
        },
        Duration::ZERO,
    )
    .unwrap()
}

fn key() -> WatermarkKey {
    WatermarkKey::new(b"resource-handler-test-key".to_vec()).unwrap()
}

fn limits() -> Limits {
    Limits::new(1 << 20, 2 << 20, 8).unwrap()
}

fn issuer<'a>(key: &'a WatermarkKey, ledger: &'a Ledger, fallback: Fallback) -> Issuer<'a> {
    Issuer {
        key,
        ledger,
        limits: limits(),
        fallback,
        now: Duration::from_secs(5),
        bucket: 20_410,
        ip_pseudonym: Some("pseudonym".to_owned()),
    }
}

fn tracer<'a>(key: &'a WatermarkKey, ledger: &'a Ledger) -> Tracer<'a> {
    Tracer {
        ops_token: "secret",
        key,
        ledger,
        limits: limits(),
        now: Duration::from_secs(6),
        epoch_unix: 1_795_033_620,
    }
}

const SERVE_ESSENTIAL: Gated = Gated {
    tier: Tier::Essential,
    decision: Decision::Serve,
};

#[test]
fn asset_path_cleans_and_rejects_parent_segments() {
    assert_eq!(
        asset_path("/res/a/./b//c.css").as_deref(),
        Some("/a/b/c.css")
    );
    assert_eq!(asset_path("/res/a/../b.png").as_deref(), Some("/b.png"));
    // Rooted clean absorbs leading `..`, exactly like Go path.Clean.
    assert_eq!(
        asset_path("/res/../../etc/passwd").as_deref(),
        Some("/etc/passwd")
    );
    assert_eq!(asset_path("/res/").as_deref(), Some("/"));
    // Any `..` left in a segment name is Go's 400 bad path.
    assert_eq!(asset_path("/res/a/..b/c"), None);
    assert_eq!(asset_path("/res/x..y.png"), None);
    assert_eq!(clean("a/../../b"), "../b");
    assert_eq!(clean(""), ".");
}

#[test]
fn mime_type_uses_the_go_builtin_table() {
    assert_eq!(mime_type("/app.JS"), "text/javascript; charset=utf-8");
    assert_eq!(mime_type("/x.css"), "text/css; charset=utf-8");
    assert_eq!(mime_type("/x.png"), "image/png");
    assert_eq!(mime_type("/m.wasm"), "application/wasm");
    assert_eq!(mime_type("/d.json"), "application/json");
    assert_eq!(mime_type("/v.mp4"), "application/octet-stream");
    assert_eq!(mime_type("/dir.png/noext"), "application/octet-stream");
    assert_eq!(mime_type("/x.xml"), "text/xml; charset=utf-8");
}

#[test]
fn session_verdict_and_liveness_follow_legacy_lookup() {
    let lively = behavior(4, 0);
    let still = behavior(3, 0);
    let typed = behavior(0, 1);
    assert_eq!(session_verdict(None), "CHALLENGE");
    let unscored = SessionView {
        verdict: "",
        behavior: &lively,
    };
    assert_eq!(session_verdict(Some(unscored)), "CHALLENGE");
    let allowed = SessionView {
        verdict: "ALLOW",
        behavior: &still,
    };
    assert_eq!(session_verdict(Some(allowed)), "ALLOW");
    assert!(!session_liveness(None));
    assert!(session_liveness(Some(unscored)));
    assert!(!session_liveness(Some(allowed)));
    assert!(session_liveness(Some(SessionView {
        verdict: "ALLOW",
        behavior: &typed,
    })));
}

#[test]
fn external_referer_is_a_host_substring_check() {
    assert!(!external_referer("", "example.test"));
    assert!(!external_referer(
        "https://example.test/page",
        "example.test"
    ));
    assert!(external_referer("https://other.test/", "example.test"));
    // Substring semantics are legacy: a host embedded anywhere counts as local.
    assert!(!external_referer(
        "https://evil.test/?example.test",
        "example.test"
    ));
}

#[test]
fn gate_header_and_withheld_responses_match_legacy() {
    let lively = behavior(10, 0);
    let video = headers(&[]);
    let allowed = Some(SessionView {
        verdict: "ALLOW",
        behavior: &lively,
    });
    // ALLOW + heavy + RIT enforced without a token -> challenge (451).
    let gated = gate_resource(
        &request(&video, true),
        "/v.mp4",
        "application/octet-stream",
        10,
        allowed,
    );
    assert_eq!(gated.header(), "challenge:T3-heavy");
    let response = withheld_response(gated).unwrap();
    assert_eq!(response.status, 451);
    assert_eq!(response.header("X-HM-Gate"), Some("challenge:T3-heavy"));
    assert_eq!(response.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(response.body, b"verification required for heavy resource\n");
    // With the token it is served.
    let token = headers(&[("x-hm-token", "t")]);
    let gated = gate_resource(
        &request(&token, true),
        "/v.mp4",
        "application/octet-stream",
        10,
        allowed,
    );
    assert_eq!(gated.decision, Decision::Serve);
    assert!(withheld_response(gated).is_none());

    // Unknown session -> CHALLENGE verdict -> media downgrade preview.
    let gated = gate_resource(
        &request(&video, false),
        "/big.png",
        "image/png",
        300 * 1024,
        None,
    );
    assert_eq!(gated.header(), "downgrade:T2-media");
    let response = withheld_response(gated).unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(response.header("X-HM-Downgrade"), Some("preview"));
    assert_eq!(
        response.body,
        b"[preview] full resource withheld until this session reaches the required trust level"
    );

    // DENY + media -> 403.
    let denied = Some(SessionView {
        verdict: "DENY",
        behavior: &lively,
    });
    let gated = gate_resource(
        &request(&video, false),
        "/big.png",
        "image/png",
        300 * 1024,
        denied,
    );
    let response = withheld_response(gated).unwrap();
    assert_eq!(response.status, 403);
    assert_eq!(response.header("X-HM-Gate"), Some("deny:T2-media"));
    assert_eq!(response.body, b"resource blocked for this session\n");

    // Essential is always served, whatever the verdict.
    let gated = gate_resource(
        &request(&video, true),
        "/a.css",
        "text/css; charset=utf-8",
        10,
        denied,
    );
    assert_eq!(gated, SERVE_ESSENTIAL);
}

#[test]
fn resource_signals_cover_embed_and_media_tiers_only() {
    let media = MediaTracker::new();
    let at = time::OffsetDateTime::from_unix_timestamp(1_795_033_620).unwrap();
    let embed = headers(&[("Sec-Fetch-Dest", "iframe")]);
    let ids = |signals: Vec<Signal>| {
        signals
            .into_iter()
            .map(|signal| signal.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(resource_signals(
            &request(&embed, false),
            Tier::Embed,
            &media,
            at
        )),
        ["l5.embed.token_stripped"]
    );
    // Light tier: no media observation and no embed context -> nothing.
    let plain = headers(&[("Referer", "https://other.test/")]);
    assert!(resource_signals(&request(&plain, false), Tier::Light, &media, at).is_empty());
    // Media tier from a foreign referer without a token.
    assert_eq!(
        ids(resource_signals(
            &request(&plain, false),
            Tier::Media,
            &media,
            at
        )),
        ["l5.media.hotlink", "l5.resource.rit_absent"]
    );
}

#[test]
fn issue_commits_the_ledger_before_returning_marked_bytes() {
    let (key, ledger) = (key(), ledger());
    let source = b"body{color:red}";
    let response = issue(
        &issuer(&key, &ledger, Fallback::Forbid),
        SERVE_ESSENTIAL,
        "sid-1",
        "/a.css",
        "text/css; charset=utf-8",
        source,
    );
    assert_eq!(response.status, 200);
    assert_eq!(response.header("X-HM-Gate"), Some("serve:T0-essential"));
    assert_eq!(
        response.header("Content-Type"),
        Some("text/css; charset=utf-8")
    );
    assert_eq!(response.header("Cache-Control"), Some("no-store"));
    assert_eq!(response.header("X-HM-Watermark"), Some("text-comment"));
    assert_ne!(response.body, source);

    // The issued copy traces back to the session through the operator view.
    let traced = trace(
        &tracer(&key, &ledger),
        Some("Bearer secret"),
        "POST",
        "text/css; charset=utf-8",
        |_| Ok(response.body.clone()),
    );
    assert_eq!(traced.status, 200);
    assert_eq!(
        std::str::from_utf8(&traced.body).unwrap(),
        concat!(
            r#"{"result":{"sessionId":"sid-1","assetId":"/a.css","mime":"text/css; charset=utf-8","#,
            r#""channel":"text-comment","issuedAt":"2026-11-18T20:27:05Z","forged":false},"traced":true}"#,
            "\n"
        )
    );
}

#[test]
fn watermark_failure_never_serves_unmarked_bytes() {
    let (key, ledger) = (key(), ledger());
    // Not a PNG: the codec refuses and no opaque fallback is permitted (W07).
    let response = issue(
        &issuer(&key, &ledger, Fallback::Forbid),
        SERVE_ESSENTIAL,
        "sid-1",
        "/x.png",
        "image/png",
        b"not a png",
    );
    assert_eq!(response.status, 503);
    assert!(response.body.is_empty());
    assert_eq!(response.header("X-HM-Watermark"), None);
    // The cancelled reservation does not leave a live record behind.
    let traced = trace(
        &tracer(&key, &ledger),
        Some("Bearer secret"),
        "POST",
        "image/png",
        |_| Ok(b"not a png".to_vec()),
    );
    assert!(
        std::str::from_utf8(&traced.body)
            .unwrap()
            .contains(r#""traced":false"#)
    );
}

#[test]
fn trace_gates_before_reading_and_bounds_the_body() {
    let (key, ledger) = (key(), ledger());
    let tracer = tracer(&key, &ledger);
    let never = |_: usize| -> std::io::Result<Vec<u8>> { panic!("body read before the gate") };
    let response = trace(&tracer, None, "POST", "", never);
    assert_eq!(
        (response.status, response.body.as_slice()),
        (404, NOT_FOUND_BODY)
    );
    let response = trace(&tracer, Some("Bearer wrong"), "POST", "", never);
    assert_eq!(response.status, 404);
    let disabled = Tracer {
        ops_token: "",
        ..tracer
    };
    assert_eq!(
        trace(&disabled, Some("Bearer "), "POST", "", never).status,
        404
    );
    let response = trace(&tracer, Some("Bearer secret"), "GET", "", never);
    assert_eq!(
        (response.status, response.body.as_slice()),
        (405, &b"POST only\n"[..])
    );

    let response = trace(&tracer, Some("Bearer secret"), "POST", "", |limit| {
        assert_eq!(limit, MAX_TRACE_BODY);
        Ok(vec![0; limit + 1])
    });
    assert_eq!(
        (response.status, response.body.as_slice()),
        (400, &b"read error\n"[..])
    );
    let response = trace(&tracer, Some("Bearer secret"), "POST", "", |_| {
        Err(std::io::Error::other("reset"))
    });
    assert_eq!(response.status, 400);

    let response = trace(&tracer, Some("Bearer secret"), "POST", "text/plain", |_| {
        Ok(b"no carrier here".to_vec())
    });
    assert_eq!(response.header("Content-Type"), Some("application/json"));
    assert_eq!(
        response.body,
        b"{\"reason\":\"no_frame\",\"traced\":false}\n"
    );
}

#[test]
fn forged_carrier_is_not_an_attribution() {
    let (key, ledger) = (key(), ledger());
    let issued = issue(
        &issuer(&key, &ledger, Fallback::Forbid),
        SERVE_ESSENTIAL,
        "sid-1",
        "/a.css",
        "text/css; charset=utf-8",
        b"body{}",
    );
    let other = WatermarkKey::new(b"a different server key".to_vec()).unwrap();
    let response = trace(
        &tracer(&other, &ledger),
        Some("Bearer secret"),
        "POST",
        "text/css",
        |_| Ok(issued.body.clone()),
    );
    assert_eq!(response.body, b"{\"reason\":\"forged\",\"traced\":false}\n");
    // An unknown tag on a fresh ledger is not bot evidence either.
    let fresh = self::ledger();
    let response = trace(
        &tracer(&key, &fresh),
        Some("Bearer secret"),
        "POST",
        "text/css",
        |_| Ok(issued.body.clone()),
    );
    assert_eq!(
        response.body,
        b"{\"reason\":\"unknown\",\"traced\":false}\n"
    );
}

#[test]
fn channel_label_follows_legacy_names() {
    assert_eq!(channel_label(&["bin-trailer".to_owned()]), "bin-trailer");
    assert_eq!(
        channel_label(&["png-text".to_owned(), "png-lsb".to_owned()]),
        "png-multi"
    );
    assert_eq!(
        channel_label(&["png-lsb".to_owned(), "png-text".to_owned()]),
        "png-multi"
    );
}

#[test]
fn traffic_view_is_owner_or_operator_only() {
    assert_eq!(
        traffic_session("/api/traffic/sid-1", Some("sid-1"), "", None),
        Some("sid-1")
    );
    assert_eq!(
        traffic_session("/api/traffic/sid-1", Some("sid-2"), "", None),
        None
    );
    assert_eq!(traffic_session("/api/traffic/sid-1", None, "", None), None);
    // An empty sid never matches an empty cookie.
    assert_eq!(traffic_session("/api/traffic/", Some(""), "", None), None);
    assert_eq!(
        traffic_session("/api/traffic/sid-9", None, "secret", Some("Bearer secret")),
        Some("sid-9")
    );
    assert_eq!(not_found().status, 404);
}
