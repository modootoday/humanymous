//! Request plans ported from the Go oracle's attack functions (981d25c
//! `cmd/redteam/attacks.go`, `main.go`, `h2raw.go`): the constants, header
//! composition helpers and one plan function per `-attack` name. See the
//! parent module for what a plan is and what it does not reproduce.
use super::{Http, LAB_HOST, Rit, Step};
use crate::wire::clienthello::{Mutation, Profile};
use crate::wire::h2::H2Profile;

// --- Oracle constants (cmd/redteam/main.go, attacks.go, h2raw.go) ---

pub(super) const CHROME_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
pub(super) const CHROME_PQ_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub(super) const ALT_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";
pub(super) const MAC_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
pub(super) const TOR_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; rv:115.0) Gecko/20100101 Firefox/115.0";
/// The short Chrome UA the oracle's report bodies carry.
pub(super) const UA_126: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36";
pub(super) const PYTHON_UA: &str = "python-requests/2.31.0";
pub(super) const SEC_CH_UA: &str =
    r#""Chromium";v="126", "Google Chrome";v="126", "Not.A/Brand";v="24""#;
pub(super) const SEC_CH_UA_131: &str =
    r#""Chromium";v="131", "Google Chrome";v="131", "Not.A/Brand";v="24""#;

pub(super) const COLLECT_BODY: &str = r#"{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36","signals":[]}"#;
pub(super) const TAMPERED_BODY: &str =
    r#"{"userAgent":"Mozilla/5.0 Chrome/126","signals":[],"injected":"evil"}"#;
pub(super) const JS_EVIDENCE_BODY: &str = concat!(
    r#"{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36","#,
    r#""engineVersion":"wasm-1.0.0","advanced":{"probed":true},"environment":{"probed":true},"#,
    r#""behavior":{"durationS":3,"mouse":{"samples":30,"velocityStdDev":0.5,"straightLineFrac":0.2,"meanJerk":0.4},"#,
    r#""key":{"keystrokes":10,"meanFlightMs":120,"flightStdDevMs":35,"dwellStdDevMs":28},"#,
    r#""events":{"totalEvents":40,"untrustedFrac":0}},"signals":[]}"#
);
pub(super) const UA_BODY_126: &str =
    r#"{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36","#;
pub(super) const PROXY_ADVANCED: &str = concat!(
    r#""advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,"#,
    r#""widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US","#,
);
pub(super) const PROXY_TAIL: &str = concat!(
    r#""colorGamut":"srgb","maxTouchPoints":0},"#,
    r#""environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},"#,
    r#""events":{"totalEvents":44}},"signals":[]}"#
);

/// Go `coherentReportBody` (h2raw.go), optionally with another UA.
pub(super) fn coherent_report_body(ua: &str) -> String {
    format!(
        concat!(
            r#"{{"userAgent":"{ua}","engineVersion":"wasm-1.0.0","#,
            r#""advanced":{{"probed":true,"mediaDeviceCount":3,"hasAudioInput":true,"hasVideoInput":true,"voiceCount":200,"#,
            r#""widevineSupported":true,"webgpuPresent":true,"webgpuVendor":"nvidia","webglVendor":"NVIDIA Corporation / NVIDIA GeForce RTX 3080","#,
            r#""audioSampleRate":48000,"connectionPresent":true,"connectionRtt":50,"batteryPresent":true,"batteryLevel":0.8,"#,
            r#""timezoneIana":"America/New_York","language":"en-US","colorGamut":"srgb","maxTouchPoints":0}},"#,
            r#""environment":{{"probed":true}},"#,
            r#""behavior":{{"durationS":8,"mouse":{{"samples":45,"velocityStdDev":0.6,"straightLineFrac":0.15,"accelEntropy":2.1,"meanJerk":0.4,"meanCurvature":0.3,"coalescedRatio":3.0}},"#,
            r#""key":{{"keystrokes":14,"meanDwellMs":95,"dwellStdDevMs":28,"meanFlightMs":140,"flightStdDevMs":35}},"#,
            r#""events":{{"totalEvents":60,"untrustedFrac":0,"clickCount":1}}}},"signals":[]}}"#
        ),
        ua = ua
    )
}

/// Go `proxyBody` / the inline proxy-family bodies: a coherent-ish report with
/// a per-attack fingerprintId and optional WebRTC address lists.
pub(super) fn proxy_body(ua: &str, fp: &str, webrtc: &str) -> String {
    format!(
        r#"{{"userAgent":"{ua}","fingerprintId":"{fp}","engineVersion":"wasm-1.0.0",{PROXY_ADVANCED}{webrtc}{PROXY_TAIL}"#
    )
}

/// Go `distributed` / `privacyEvasion` body with the given environment block.
pub(super) fn distributed_body(fp: &str, environment: &str) -> String {
    format!(
        concat!(
            "{head}",
            r#""fingerprintId":"{fp}","engineVersion":"wasm-1.0.0","advanced":{{"probed":true}},"#,
            r#""environment":{env},"behavior":{{"mouse":{{"samples":30,"velocityStdDev":0.4}},"#,
            r#""events":{{"totalEvents":40}}}},"signals":[]}}"#
        ),
        head = UA_BODY_126,
        fp = fp,
        env = environment
    )
}

/// Go `flood` body: JS evidence without a fingerprintId.
pub(super) fn flood_body() -> String {
    format!(
        concat!(
            "{head}",
            r#""engineVersion":"wasm-1.0.0","advanced":{{"probed":true}},"environment":{{"probed":true}},"#,
            r#""behavior":{{"mouse":{{"samples":30,"velocityStdDev":0.4}},"events":{{"totalEvents":40}}}},"signals":[]}}"#
        ),
        head = UA_BODY_126
    )
}

/// Go `signalForgery` body: forged server-only trust-upgrade signals.
pub(super) fn forgery_body() -> String {
    format!(
        concat!(
            "{head}",
            r#""engineVersion":"wasm-1.0.0","advanced":{{"probed":true}},"environment":{{"probed":true}},"#,
            r#""behavior":{{"mouse":{{"samples":30,"velocityStdDev":0.4}},"events":{{"totalEvents":40}}}},"#,
            r#""signals":[{{"id":"l7.pass.solved","verdict":"OK","collected":"server","score":1}},"#,
            r#"{{"id":"l7.pow.solved","verdict":"OK","collected":"server","score":1}}]}}"#
        ),
        head = UA_BODY_126
    )
}

/// Go `powFastSolve` / `powLaunder` collect body (WebGL contradictions).
pub(super) const POW_BODY: &str = concat!(
    r#"{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36","#,
    r#""engineVersion":"wasm-1.0.0","advanced":{"probed":true,"webglVendor":"Google Inc. (Intel)","webglRenderer":"llvmpipe (LLVM 15)"},"#,
    r#""environment":{"probed":true},"#,
    r#""behavior":{"mouse":{"samples":8,"velocityStdDev":0.05},"events":{"totalEvents":4}},"signals":[]}"#
);
pub(super) const POW_SOLUTION_BODY: &str = r#"{"bucket":{pow-bucket},"nonce":"{pow-nonce}"}"#;

/// Go `torExit` body (Tor Browser shaped report).
pub(super) const TOR_BODY: &str = concat!(
    r#"{"userAgent":"Mozilla/5.0 (Windows NT 10.0; rv:115.0) Gecko/20100101 Firefox/115.0","#,
    r#""fingerprintId":"t0rbr0wser000001t0rbr0","engineVersion":"wasm-1.0.0","#,
    r#""advanced":{"probed":true,"audioSampleRate":44100,"voiceCount":0,"mediaDeviceCount":0,"#,
    r#""widevineSupported":false,"timezoneIana":"UTC","language":"en-US","#,
    r#""webrtcHostAddrs":[],"webrtcPublicAddrs":[],"#,
    r#""colorGamut":"srgb","maxTouchPoints":0},"#,
    r#""environment":{"probed":true,"adBlock":true,"gpc":true,"doNotTrack":"1","trackingProtection":true},"#,
    r#""behavior":{"mouse":{"samples":28,"velocityStdDev":0.45},"events":{"totalEvents":36}},"signals":[]}"#
);

/// Go `chromeH2HeaderOrder`: the raw framer's on-wire regular header order.
pub(super) const CHROME_H2_HEADER_ORDER: &[&str] = &[
    "sec-ch-ua",
    "sec-ch-ua-mobile",
    "sec-ch-ua-platform",
    "upgrade-insecure-requests",
    "user-agent",
    "content-type",
    "accept",
    "sec-fetch-site",
    "sec-fetch-mode",
    "sec-fetch-dest",
    "accept-encoding",
    "accept-language",
    "x-hm-token",
    "x-hm-n",
    "x-hm-tb",
];

// --- Header composition helpers (mirror the Go helpers) ---

pub(super) type Headers = Vec<(String, String)>;

pub(super) fn pairs(list: &[(&str, &str)]) -> Headers {
    list.iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Go map assignment: replace an exact-key entry in place, else append.
pub(super) fn merge(mut base: Headers, extra: Headers) -> Headers {
    for (key, value) in extra {
        match base.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => base.push((key, value)),
        }
    }
    base
}

/// Go `browserHeaders` (declaration order).
pub(super) fn browser() -> Headers {
    pairs(&[
        ("sec-ch-ua", SEC_CH_UA),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", r#""Windows""#),
        ("sec-fetch-site", "same-origin"),
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-dest", "empty"),
        ("accept", "*/*"),
        ("accept-language", "en-US,en;q=0.9"),
        ("accept-encoding", "gzip, deflate, br, zstd"),
    ])
}

/// Go `withBrowserHeaders(extra)`.
pub(super) fn with_browser(extra: &[(&str, &str)]) -> Headers {
    merge(browser(), pairs(extra))
}

/// Go `browserHeadersExcept(drop...)`.
pub(super) fn browser_except(drop: &[&str]) -> Headers {
    let mut headers = browser();
    headers.retain(|(k, _)| !drop.contains(&k.as_str()));
    headers
}

/// RIT headers for counter offset `k` (values are server-dependent).
pub(super) fn rit_headers(k: u64) -> Headers {
    vec![
        ("X-HM-Token".into(), format!("{{rit-token n+{k}}}")),
        ("X-HM-N".into(), format!("{{n+{k}}}")),
        ("X-HM-TB".into(), "{tb}".into()),
    ]
}

/// Go `collectSeed` header map: User-Agent and Content-Type, then `hdr`.
pub(super) fn collect_headers(ua: &str, hdr: Headers) -> Headers {
    merge(
        pairs(&[("User-Agent", ua), ("Content-Type", "application/json")]),
        hdr,
    )
}

/// Go `httpRoundTrip` / `doOrderedH1` framing around the request headers.
pub(super) fn h1_wire(headers: Headers, cookie: bool, body: &str, templated: bool) -> Headers {
    let mut wire = vec![("Host".to_owned(), LAB_HOST.to_owned())];
    wire.extend(headers);
    if cookie {
        wire.push(("Cookie".into(), "{session-cookie}".into()));
    }
    if !body.is_empty() {
        let length = if templated {
            "{content-length}".to_owned()
        } else {
            body.len().to_string()
        };
        wire.push(("Content-Length".into(), length));
    }
    wire.push(("Connection".into(), "close".into()));
    wire
}

pub(super) fn lowercase(headers: Headers) -> Headers {
    let mut out: Headers = Vec::new();
    for (k, v) in headers {
        out = merge(out, vec![(k.to_ascii_lowercase(), v)]);
    }
    out
}

/// Go raw h2 framer (`rawH2Conn.request`): Chrome header order, the rest,
/// then `cookie` last.
pub(super) fn h2_raw_wire(headers: Headers, cookie: bool) -> Headers {
    let mut rest = lowercase(headers);
    let mut wire = Vec::new();
    for name in CHROME_H2_HEADER_ORDER {
        if let Some(i) = rest.iter().position(|(k, _)| k == name) {
            wire.push(rest.remove(i));
        }
    }
    wire.extend(rest);
    if cookie {
        wire.push(("cookie".into(), "{session-cookie}".into()));
    }
    wire
}

/// Go `x/net/http2` Transport (`doH2`/`doH2Spec`): lowercased request
/// headers (Go map order), the cookie, then content-length for a body.
pub(super) fn h2_transport_wire(headers: Headers, cookie: bool, body: &str) -> Headers {
    let mut wire = lowercase(headers);
    if cookie {
        wire.push(("cookie".into(), "{session-cookie}".into()));
    }
    if !body.is_empty() {
        wire.push(("content-length".into(), body.len().to_string()));
    }
    wire
}

// --- Step constructors (mirror the Go request helpers) ---

/// Go `session(hello)`: GET /api/session over HTTP/1.1.
pub(super) fn session(hello: Profile) -> Step {
    Step {
        purpose: "session",
        hello,
        mutation: None,
        http: Http::H1,
        method: "GET",
        path: "/api/session",
        headers: h1_wire(with_browser(&[("User-Agent", CHROME_UA)]), false, "", false),
        body: String::new(),
        rit: Rit::None,
        repeat: 1,
        note: None,
    }
}

/// Go `collect(hello, ua, cookie, hdr, body)` (no RIT unless `hdr` has it).
pub(super) fn collect(hello: Profile, ua: &str, hdr: Headers, body: String) -> Step {
    Step {
        purpose: "collect",
        hello,
        mutation: None,
        http: Http::H1,
        method: "POST",
        path: "/api/collect",
        headers: h1_wire(collect_headers(ua, hdr), true, &body, false),
        body,
        rit: Rit::None,
        repeat: 1,
        note: None,
    }
}

/// Go `signedCollectHdr(hello, ua, ..., n+k, body, extra)`.
pub(super) fn signed_collect(
    hello: Profile,
    ua: &str,
    k: u64,
    body: String,
    extra: &[(&str, &str)],
) -> Step {
    let hdr = merge(with_browser(extra), rit_headers(k));
    Step {
        rit: Rit::Sign(k),
        ..collect(hello, ua, hdr, body)
    }
}

/// Go `h2RawEvasion`: session + two RIT-signed collects over the raw framer.
pub(super) fn h2_raw_evasion(profile: H2Profile) -> Vec<Step> {
    let mut steps = vec![Step {
        purpose: "session",
        hello: Profile::Chrome,
        mutation: None,
        http: Http::H2(profile),
        method: "GET",
        path: "/api/session",
        headers: h2_raw_wire(with_browser(&[("user-agent", CHROME_UA)]), false),
        body: String::new(),
        rit: Rit::None,
        repeat: 1,
        note: None,
    }];
    for k in 1..=2 {
        let hdr = merge(
            with_browser(&[
                ("user-agent", CHROME_UA),
                ("content-type", "application/json"),
            ]),
            lowercase(rit_headers(k)),
        );
        steps.push(Step {
            purpose: "collect",
            hello: Profile::Chrome,
            mutation: None,
            http: Http::H2(profile),
            method: "POST",
            path: "/api/collect",
            headers: h2_raw_wire(hdr, true),
            body: coherent_report_body(CHROME_UA),
            rit: Rit::Sign(k),
            repeat: 1,
            note: None,
        });
    }
    steps
}

/// Go `alpsAbsent`/`echAbsent`/`certCompressionAbsent`: HTTP/1.1 session, then
/// one RIT-signed collect over `doH2Spec` with the mutated Chrome_Auto hello.
pub(super) fn stripped_h2(mutation: Mutation) -> Vec<Step> {
    let body = coherent_report_body(CHROME_UA);
    let hdr = merge(with_browser(&[("User-Agent", CHROME_UA)]), rit_headers(1));
    vec![
        session(Profile::Chrome),
        Step {
            purpose: "collect",
            hello: Profile::Chrome,
            mutation: Some(mutation),
            http: Http::H2(H2Profile::GoTransport),
            method: "POST",
            path: "/api/collect",
            headers: h2_transport_wire(hdr, true, &body),
            body,
            rit: Rit::Sign(1),
            repeat: 1,
            note: None,
        },
    ]
}

/// Go `powFastSolve`/`powLaunder`: challenge-band collect, then PoW submits.
pub(super) fn pow_plan(submits: &[Option<&'static str>]) -> Vec<Step> {
    let collect_hdr = with_browser(&[
        ("User-Agent", CHROME_UA),
        ("Content-Type", "application/json"),
    ]);
    let mut steps = vec![
        session(Profile::Firefox),
        Step {
            purpose: "collect",
            hello: Profile::Firefox,
            mutation: None,
            http: Http::H1,
            method: "POST",
            path: "/api/collect",
            headers: h1_wire(collect_hdr, true, POW_BODY, false),
            body: POW_BODY.into(),
            rit: Rit::None,
            repeat: 1,
            note: Some(
                "reads the X-HM-PoW challenge (seed:difficulty:bucket); without one the oracle reports the collect verdict",
            ),
        },
    ];
    for note in submits {
        steps.push(Step {
            purpose: "pow-submit",
            hello: Profile::Firefox,
            mutation: None,
            http: Http::H1,
            method: "POST",
            path: "/api/pow",
            headers: h1_wire(
                pairs(&[
                    ("User-Agent", CHROME_UA),
                    ("Content-Type", "application/json"),
                ]),
                true,
                POW_SOLUTION_BODY,
                true,
            ),
            body: POW_SOLUTION_BODY.into(),
            rit: Rit::None,
            repeat: 1,
            note: *note,
        });
    }
    steps
}

// --- Per-attack plans (one per Go attack function) ---

pub(super) fn tls_static() -> Vec<Step> {
    let mut steps = vec![session(Profile::Chrome)];
    for k in 1..=4 {
        steps.push(signed_collect(
            Profile::Chrome100,
            CHROME_UA,
            k,
            COLLECT_BODY.into(),
            &[],
        ));
    }
    steps
}
pub(super) fn tls_rotate() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        signed_collect(Profile::Chrome, CHROME_UA, 1, COLLECT_BODY.into(), &[]),
        signed_collect(Profile::Firefox, CHROME_UA, 2, COLLECT_BODY.into(), &[]),
    ]
}
pub(super) fn ja4_churn() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        signed_collect(
            Profile::Chrome100,
            CHROME_UA,
            1,
            JS_EVIDENCE_BODY.into(),
            &[],
        ),
        signed_collect(Profile::Firefox, CHROME_UA, 2, JS_EVIDENCE_BODY.into(), &[]),
        signed_collect(Profile::Chrome, CHROME_UA, 3, JS_EVIDENCE_BODY.into(), &[]),
    ]
}
pub(super) fn multi_axis_rotate() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        signed_collect(Profile::Chrome, CHROME_UA, 1, JS_EVIDENCE_BODY.into(), &[]),
        signed_collect(Profile::Firefox, ALT_UA, 2, JS_EVIDENCE_BODY.into(), &[]),
    ]
}
pub(super) fn ua_rotate() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        signed_collect(Profile::Chrome, CHROME_UA, 1, COLLECT_BODY.into(), &[]),
        signed_collect(Profile::Chrome, ALT_UA, 2, COLLECT_BODY.into(), &[]),
    ]
}
pub(super) fn grease_absent_js() -> Vec<Step> {
    vec![
        session(Profile::Stock),
        collect(
            Profile::Stock,
            CHROME_UA,
            with_browser(&[]),
            JS_EVIDENCE_BODY.into(),
        ),
    ]
}
pub(super) fn alps_absent() -> Vec<Step> {
    stripped_h2(Mutation::StripAlps)
}
pub(super) fn ech_absent() -> Vec<Step> {
    stripped_h2(Mutation::StripEch)
}
pub(super) fn cert_compression_absent() -> Vec<Step> {
    stripped_h2(Mutation::StripCertCompression)
}
pub(super) fn pq_absent() -> Vec<Step> {
    vec![
        session(Profile::Chrome100),
        signed_collect(
            Profile::Chrome100,
            CHROME_PQ_UA,
            1,
            coherent_report_body(CHROME_PQ_UA),
            &[("sec-ch-ua", SEC_CH_UA_131)],
        ),
    ]
}
pub(super) fn h2_settings_split() -> Vec<Step> {
    h2_raw_evasion(H2Profile::SettingsSplit)
}
pub(super) fn h2_flow_control() -> Vec<Step> {
    h2_raw_evasion(H2Profile::FlowControlSplit)
}
pub(super) fn h2_priority_absent() -> Vec<Step> {
    h2_raw_evasion(H2Profile::PriorityAbsent)
}
pub(super) fn coherent_ceiling() -> Vec<Step> {
    h2_raw_evasion(H2Profile::Chrome)
}
pub(super) fn h2_protocol_split() -> Vec<Step> {
    let mut steps = vec![Step {
        purpose: "session",
        hello: Profile::Chrome,
        mutation: None,
        http: Http::H2(H2Profile::GoTransport),
        method: "GET",
        path: "/api/session",
        headers: h2_transport_wire(with_browser(&[("User-Agent", CHROME_UA)]), false, ""),
        body: String::new(),
        rit: Rit::None,
        repeat: 1,
        note: None,
    }];
    for k in 1..=2 {
        let body = coherent_report_body(CHROME_UA);
        let hdr = merge(
            with_browser(&[
                ("User-Agent", CHROME_UA),
                ("Content-Type", "application/json"),
            ]),
            rit_headers(k),
        );
        steps.push(Step {
            purpose: "collect",
            hello: Profile::Chrome,
            mutation: None,
            http: Http::H2(H2Profile::GoTransport),
            method: "POST",
            path: "/api/collect",
            headers: h2_transport_wire(hdr, true, &body),
            body,
            rit: Rit::Sign(k),
            repeat: 1,
            note: None,
        });
    }
    steps
}
pub(super) fn browser_tls_h1() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[]),
            coherent_report_body(CHROME_UA),
        ),
    ]
}
pub(super) fn header_order_split() -> Vec<Step> {
    let body = coherent_report_body(CHROME_UA);
    let mut ordered = pairs(&[
        ("user-agent", CHROME_UA),
        ("accept", "*/*"),
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-dest", "empty"),
        ("sec-ch-ua", SEC_CH_UA),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", r#""Windows""#),
        ("accept-encoding", "gzip, deflate, br, zstd"),
        ("accept-language", "en-US,en;q=0.9"),
    ]);
    ordered.extend(lowercase(rit_headers(1)));
    vec![
        session(Profile::Chrome),
        Step {
            purpose: "collect",
            hello: Profile::Chrome,
            mutation: None,
            http: Http::H1Ordered,
            method: "POST",
            path: "/api/collect",
            headers: h1_wire(ordered, true, &body, false),
            body,
            rit: Rit::Sign(1),
            repeat: 1,
            note: None,
        },
    ]
}
pub(super) fn nonbrowser_ua() -> Vec<Step> {
    vec![
        session(Profile::Stock),
        collect(
            Profile::Stock,
            PYTHON_UA,
            Vec::new(),
            r#"{"userAgent":"python-requests/2.31.0","signals":[]}"#.into(),
        ),
    ]
}
pub(super) fn sec_chua_absent() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            browser_except(&["sec-ch-ua", "sec-ch-ua-mobile", "sec-ch-ua-platform"]),
            JS_EVIDENCE_BODY.into(),
        ),
    ]
}
pub(super) fn sec_fetch_absent() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            browser_except(&["sec-fetch-site", "sec-fetch-mode", "sec-fetch-dest"]),
            JS_EVIDENCE_BODY.into(),
        ),
    ]
}
pub(super) fn rit_replay() -> Vec<Step> {
    let hdr = merge(with_browser(&[]), rit_headers(1));
    let first = Step {
        rit: Rit::Sign(1),
        ..collect(Profile::Chrome, CHROME_UA, hdr.clone(), COLLECT_BODY.into())
    };
    let replay = Step {
        rit: Rit::Replay(1),
        ..collect(Profile::Chrome, CHROME_UA, hdr, COLLECT_BODY.into())
    };
    vec![session(Profile::Chrome), first, replay]
}
pub(super) fn rit_tamper() -> Vec<Step> {
    let hdr = merge(with_browser(&[]), rit_headers(1));
    vec![
        session(Profile::Chrome),
        Step {
            rit: Rit::SignOtherBody(1, COLLECT_BODY.into()),
            ..collect(Profile::Chrome, CHROME_UA, hdr, TAMPERED_BODY.into())
        },
    ]
}
pub(super) fn rit_absent() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[]),
            JS_EVIDENCE_BODY.into(),
        ),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[]),
            JS_EVIDENCE_BODY.into(),
        ),
    ]
}
pub(super) fn signal_forgery() -> Vec<Step> {
    vec![
        session(Profile::Firefox),
        collect(
            Profile::Firefox,
            CHROME_UA,
            with_browser(&[]),
            forgery_body(),
        ),
    ]
}
pub(super) fn xff_spoof() -> Vec<Step> {
    let body = format!(
        concat!(
            "{head}",
            r#""fingerprintId":"5p00fed15p00fed15p00fed1","engineVersion":"wasm-1.0.0","advanced":{{"probed":true}},"#,
            r#""environment":{{"probed":true}},"behavior":{{"mouse":{{"samples":32,"velocityStdDev":0.5}},"#,
            r#""events":{{"totalEvents":44}}}},"signals":[]}}"#
        ),
        head = UA_BODY_126
    );
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                ("X-Forwarded-For", "10.20.30.40"),
                ("X-Real-IP", "10.20.30.40"),
            ]),
            body,
        ),
    ]
}
pub(super) fn squid_forward() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                ("Via", "1.1 squid.lab (squid/5.7)"),
                ("X-Cache", "MISS from squid.lab"),
                ("X-Cache-Lookup", "MISS from squid.lab:3128"),
                ("X-Forwarded-For", "203.0.113.40"),
            ]),
            proxy_body(UA_126, "squ1df0rward000001squ1", ""),
        ),
    ]
}
pub(super) fn openvpn_exit() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[("X-Forwarded-For", "203.0.113.88")]),
            proxy_body(
                UA_126,
                "0penvpn0000000010penvpn",
                r#""webrtcHostAddrs":["10.8.0.2","192.168.1.42"],"webrtcPublicAddrs":["198.51.100.77"],"#,
            ),
        ),
    ]
}
pub(super) fn wireguard_hop() -> Vec<Step> {
    let body = proxy_body(UA_126, "w1reguard0000001w1regu", "");
    vec![
        session(Profile::Chrome),
        signed_collect(
            Profile::Chrome,
            CHROME_UA,
            1,
            body.clone(),
            &[("X-Forwarded-For", "203.0.113.10")],
        ),
        signed_collect(
            Profile::Chrome,
            CHROME_UA,
            2,
            body,
            &[("X-Forwarded-For", "198.51.100.20, 203.0.113.10")],
        ),
    ]
}
pub(super) fn tor_exit() -> Vec<Step> {
    let mut steps = vec![
        session(Profile::Firefox),
        signed_collect(
            Profile::Firefox,
            TOR_UA,
            1,
            TOR_BODY.into(),
            &[(
                "X-Forwarded-For",
                "185.220.101.1, 185.220.102.2, 203.0.113.50",
            )],
        ),
    ];
    for (i, ip) in ["198.51.100.11", "203.0.113.22", "192.0.2.33"]
        .into_iter()
        .enumerate()
    {
        steps.push(signed_collect(
            Profile::Firefox,
            TOR_UA,
            i as u64 + 2,
            TOR_BODY.into(),
            &[("X-Forwarded-For", ip)],
        ));
    }
    steps
}
pub(super) fn anon_proxy_chain() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                ("Proxy-Connection", "keep-alive"),
                (
                    "X-Forwarded-For",
                    "203.0.113.1, 198.51.100.2, 192.0.2.3, 203.0.113.99",
                ),
            ]),
            proxy_body(UA_126, "an0npr0xychain0001an0n", ""),
        ),
    ]
}
pub(super) fn elite_anon_proxy() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                (
                    "Forwarded",
                    "for=203.0.113.10;proto=https, for=198.51.100.20;by=proxy.elite.lab",
                ),
                ("X-Forwarded-For", "203.0.113.10, 198.51.100.20"),
            ]),
            proxy_body(UA_126, "el1tean0npr0xy0001el1t", ""),
        ),
    ]
}
pub(super) fn cdn_ip_spoof() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                ("X-Forwarded-For", "203.0.113.50"),
                ("CF-Connecting-IP", "8.8.8.8"),
                ("True-Client-IP", "1.1.1.1"),
                ("X-Client-IP", "9.9.9.9"),
                ("X-Original-Forwarded-For", "8.8.8.8"),
            ]),
            proxy_body(UA_126, "cdn1psp00f00000001cdn1", ""),
        ),
    ]
}
pub(super) fn proxy_ua_rotate() -> Vec<Step> {
    let body1 = proxy_body(CHROME_UA, "pr0xyuar0tate0001pr0x", "");
    let body2 = proxy_body(MAC_UA, "pr0xyuar0tate0001pr0x", "");
    vec![
        session(Profile::Chrome),
        signed_collect(
            Profile::Chrome,
            CHROME_UA,
            1,
            body1,
            &[("X-Forwarded-For", "203.0.113.10")],
        ),
        signed_collect(
            Profile::Chrome,
            MAC_UA,
            2,
            body2.clone(),
            &[("X-Forwarded-For", "198.51.100.20")],
        ),
        signed_collect(
            Profile::Chrome,
            MAC_UA,
            3,
            body2,
            &[("X-Forwarded-For", "192.0.2.30")],
        ),
    ]
}
pub(super) fn fp_churn_proxy() -> Vec<Step> {
    let mut steps = vec![session(Profile::Chrome)];
    for (i, (fp, ip)) in [
        ("fpchurn000000000000001", "203.0.113.11"),
        ("fpchurn000000000000002", "198.51.100.22"),
        ("fpchurn000000000000003", "192.0.2.33"),
    ]
    .into_iter()
    .enumerate()
    {
        steps.push(signed_collect(
            Profile::Chrome,
            CHROME_UA,
            i as u64 + 1,
            proxy_body(UA_126, fp, ""),
            &[("X-Forwarded-For", ip)],
        ));
    }
    steps
}
pub(super) fn stacked_proxy_vpn() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[
                ("Via", "1.1 squid-over-vpn (squid/5.7)"),
                ("X-Cache", "MISS from squid-over-vpn"),
                ("X-Forwarded-For", "203.0.113.77"),
            ]),
            proxy_body(
                UA_126,
                "stackpr0xyvpn00001stac",
                r#""webrtcHostAddrs":["10.8.0.5"],"webrtcPublicAddrs":["198.51.100.88"],"#,
            ),
        ),
    ]
}
pub(super) fn socks_exit_hop() -> Vec<Step> {
    let body = proxy_body(UA_126, "s0cks5exith0p0001s0ck", "");
    vec![
        session(Profile::Chrome),
        signed_collect(
            Profile::Chrome,
            CHROME_UA,
            1,
            body.clone(),
            &[("X-Forwarded-For", "203.0.113.40")],
        ),
        signed_collect(
            Profile::Chrome,
            CHROME_UA,
            2,
            body,
            &[("X-Forwarded-For", "198.51.100.41, 203.0.113.40")],
        ),
    ]
}
pub(super) fn flood() -> Vec<Step> {
    vec![
        session(Profile::Chrome),
        Step {
            repeat: 90,
            ..collect(Profile::Chrome, CHROME_UA, with_browser(&[]), flood_body())
        },
    ]
}
pub(super) fn rotated_proxies(body: String) -> Vec<Step> {
    let mut steps = Vec::new();
    for ip in ["203.0.113.7", "198.51.100.23", "192.0.2.44", "203.0.113.99"] {
        steps.push(session(Profile::Chrome));
        steps.push(collect(
            Profile::Chrome,
            CHROME_UA,
            with_browser(&[("X-Forwarded-For", ip)]),
            body.clone(),
        ));
    }
    steps
}
pub(super) fn distributed() -> Vec<Step> {
    rotated_proxies(distributed_body(
        "deadbeefcafe0001deadbeef",
        r#"{"probed":true}"#,
    ))
}
pub(super) fn privacy_evasion() -> Vec<Step> {
    rotated_proxies(distributed_body(
        "deadbeefcafe0002deadbeef",
        r#"{"probed":true,"adBlock":true,"gpc":true,"doNotTrack":"1"}"#,
    ))
}
pub(super) fn pow_fast_solve() -> Vec<Step> {
    pow_plan(&[Some(
        "nonce from a native SHA-256 solve of the issued challenge, submitted immediately (solve loop not ported)",
    )])
}
pub(super) fn pow_launder() -> Vec<Step> {
    pow_plan(&[
        Some("nonce from a native SHA-256 solve, submitted immediately (solve loop not ported)"),
        Some("same nonce resubmitted after sleeping 2^difficulty / 3e6 s + 250 ms past issuance"),
    ])
}
