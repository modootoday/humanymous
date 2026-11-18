//! Resource gating compatibility with the legacy oracle: signed URLs (including
//! Unicode and signed expiry boundaries), tier classification, the serving
//! matrix, embed inspection and media tracking.
use humanymous_core::integrity::resource::{
    Decision, EmbedContext, MediaEvent, MediaTracker, Tier, classify, gate, inspect, sign,
    signed_url, verify,
};
use humanymous_core::signals::observation::Signal;
use serde::Deserialize;
use serde_json::Value;
use time::{Duration, OffsetDateTime};

#[derive(Deserialize)]
struct Case {
    key: String,
    session: String,
    asset: String,
    expiry: i64,
    signature: String,
    url: String,
    at_max: bool,
}

#[test]
fn signed_urls_and_expiry_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/resource.json")).unwrap();
    assert_eq!(cases.len(), 81);
    for case in cases {
        assert_eq!(
            sign(case.key.as_bytes(), &case.session, &case.asset, case.expiry),
            case.signature
        );
        assert_eq!(
            signed_url(case.key.as_bytes(), &case.session, &case.asset, case.expiry),
            case.url
        );
        assert_eq!(
            verify(
                case.key.as_bytes(),
                &case.session,
                &case.asset,
                case.expiry,
                i64::MAX,
                &case.signature
            ),
            case.at_max
        );
    }
}

fn ids(signals: &[Signal]) -> Vec<&str> {
    signals.iter().map(|signal| signal.id.as_str()).collect()
}

#[test]
fn classify_matches_go_table() {
    for (path, mime, size, dest, want) in [
        (
            "/app.js",
            "application/javascript",
            1000,
            "script",
            Tier::Essential,
        ),
        ("/style.css", "text/css", 500, "style", Tier::Essential),
        ("/movie.mp4", "video/mp4", 5 << 20, "video", Tier::Heavy),
        (
            "/big.bin",
            "application/octet-stream",
            3 << 20,
            "",
            Tier::Heavy,
        ),
        ("/photo.jpg", "image/jpeg", 500 << 10, "image", Tier::Media),
        ("/icon.png", "image/png", 4 << 10, "image", Tier::Light),
        ("/frame.html", "text/html", 1000, "iframe", Tier::Embed),
    ] {
        assert_eq!(classify(path, mime, size, dest), want, "{path} {mime}");
    }
}

#[test]
fn gate_matrix_matches_go_table() {
    for (tier, verdict, rit, live, want) in [
        (Tier::Essential, "DENY", false, false, Decision::Serve),
        (Tier::Heavy, "ALLOW", true, true, Decision::Serve),
        (Tier::Heavy, "ALLOW", false, false, Decision::Challenge),
        (Tier::Heavy, "CHALLENGE", true, true, Decision::Downgrade),
        (Tier::Heavy, "DENY", true, true, Decision::DenyServe),
        (Tier::Embed, "DENY", true, true, Decision::DenyServe),
        (Tier::Light, "DENY", false, false, Decision::Serve),
    ] {
        assert_eq!(gate(tier, verdict, rit, live), want, "{tier} {verdict}");
    }
}

fn range_event(session_id: &str, at: OffsetDateTime) -> MediaEvent<'_> {
    MediaEvent {
        session_id,
        is_range: true,
        external_ref: false,
        has_rit: true,
        at,
    }
}

#[test]
fn media_range_storm_after_rapid_ranges() {
    let tracker = MediaTracker::new();
    let base = OffsetDateTime::from_unix_timestamp(1000).unwrap();
    let stormed = (0..10).any(|index| {
        ids(&tracker.observe(&range_event(
            "s1",
            base + Duration::milliseconds(index * 100),
        )))
        .contains(&"l5.media.range_storm")
    });
    assert!(stormed, "expected range_storm after rapid range requests");
}

#[test]
fn media_no_storm_when_spread() {
    let tracker = MediaTracker::new();
    let base = OffsetDateTime::from_unix_timestamp(1000).unwrap();
    for index in 0..10 {
        let signals = tracker.observe(&range_event("s2", base + Duration::seconds(index)));
        assert!(
            !ids(&signals).contains(&"l5.media.range_storm"),
            "spread-out ranges should not storm"
        );
    }
}

#[test]
fn embed_token_stripped_and_origin_disallowed() {
    let signals = inspect(&EmbedContext {
        sec_fetch_dest: "iframe",
        origin: "https://evil.com",
        allowed_hosts: &["myapp.local"],
        rit_present: false,
    });
    let ids = ids(&signals);
    assert!(
        ids.contains(&"l5.embed.token_stripped"),
        "expected token_stripped"
    );
    assert!(
        ids.contains(&"l5.embed.origin_disallowed"),
        "expected origin_disallowed (non-empty allow-list needed)"
    );
}

#[derive(Deserialize)]
struct GateFixture {
    classify: Vec<(String, String, i64, String, String)>,
    gate: Vec<(String, String, bool, bool, String)>,
    embed: Vec<EmbedCase>,
    media: Vec<MediaCase>,
}

#[derive(Deserialize)]
struct EmbedCase {
    dest: String,
    origin: String,
    allowed: Vec<String>,
    rit: bool,
    signals: Option<Vec<Value>>,
}

#[derive(Deserialize)]
struct MediaCase {
    session: String,
    range: bool,
    external: bool,
    rit: bool,
    at_nanos: i64,
    signals: Option<Vec<Value>>,
}

/// Compare in the decoded wire domain: Go encodes whole floats without a
/// fraction and the frozen decoder reads untyped numbers as floats, so both
/// sides pass through the same JSON encode/decode.
fn wire(signals: &[Signal]) -> Vec<Signal> {
    signals
        .iter()
        .map(|signal| serde_json::from_value(serde_json::to_value(signal).unwrap()).unwrap())
        .collect()
}

fn oracle(signals: &Option<Vec<Value>>) -> Vec<Signal> {
    signals
        .iter()
        .flatten()
        .map(|signal| serde_json::from_value(signal.clone()).unwrap())
        .collect()
}

fn tier_from_label(label: &str) -> Tier {
    [
        Tier::Essential,
        Tier::Light,
        Tier::Media,
        Tier::Heavy,
        Tier::Embed,
    ]
    .into_iter()
    .find(|tier| tier.as_str() == label)
    .unwrap()
}

#[test]
fn classify_gate_embed_and_media_match_go_oracle() {
    let fixture: GateFixture =
        serde_json::from_str(include_str!("fixtures/resource_gate.json")).unwrap();
    assert_eq!(fixture.classify.len(), 2379);
    for (path, mime, size, dest, want) in &fixture.classify {
        assert_eq!(
            classify(path, mime, *size, dest).as_str(),
            want,
            "{path:?} {mime:?} {size} {dest:?}"
        );
    }
    assert_eq!(fixture.gate.len(), 120);
    for (tier, verdict, rit, live, want) in &fixture.gate {
        assert_eq!(
            gate(tier_from_label(tier), verdict, *rit, *live).to_string(),
            *want,
            "{tier} {verdict:?} {rit} {live}"
        );
    }
    assert_eq!(fixture.embed.len(), 504);
    for case in &fixture.embed {
        let allowed: Vec<&str> = case.allowed.iter().map(String::as_str).collect();
        let actual = inspect(&EmbedContext {
            sec_fetch_dest: &case.dest,
            origin: &case.origin,
            allowed_hosts: &allowed,
            rit_present: case.rit,
        });
        assert_eq!(
            wire(&actual),
            oracle(&case.signals),
            "{:?} {:?} {:?} {}",
            case.dest,
            case.origin,
            case.allowed,
            case.rit
        );
    }
    assert_eq!(fixture.media.len(), 600);
    let tracker = MediaTracker::new();
    for (index, case) in fixture.media.iter().enumerate() {
        let actual = tracker.observe(&MediaEvent {
            session_id: &case.session,
            is_range: case.range,
            external_ref: case.external,
            has_rit: case.rit,
            at: OffsetDateTime::from_unix_timestamp_nanos(i128::from(case.at_nanos)).unwrap(),
        });
        assert_eq!(wire(&actual), oracle(&case.signals), "media event {index}");
    }
}
