//! Real listener dispatch of the legacy surfaces; not a browser e2e run.
use super::*;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::{application::Application, configuration::Configuration};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Clone, Default)]
struct Audit(Arc<Mutex<Vec<Record>>>);
impl DurableAudit for Audit {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        self.0.lock().unwrap().push(record);
        Ok(())
    }
}

fn web_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web")
        .canonicalize()
        .unwrap()
}

fn policy(pass: bool, demo: bool, static_assets: bool) -> LegacyPolicy {
    LegacyPolicy {
        web_dir: web_dir(),
        pass,
        demo,
        static_assets,
    }
}

fn configuration(legacy: Option<LegacyPolicy>) -> Configuration {
    let mut config = crate::gateway::tests::config();
    config.legacy = legacy;
    config
}

/// Bind the real application listener; returns its address and audit.
async fn serve(legacy: Option<LegacyPolicy>) -> (SocketAddr, Audit, tokio::task::JoinHandle<()>) {
    let audit = Audit::default();
    let app = Application::new(
        configuration(legacy),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _ = app.serve(listener).await;
    });
    (address, audit, task)
}

struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}
impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

/// One raw HTTP/1.1 exchange shaped like a browser navigation: no
/// Content-Length on bodyless requests unless `fields` adds one.
async fn exchange(address: SocketAddr, method: &str, target: &str, fields: &str) -> Reply {
    exchange_body(address, method, target, fields, b"").await
}

async fn exchange_body(
    address: SocketAddr,
    method: &str,
    target: &str,
    fields: &str,
    body: &[u8],
) -> Reply {
    let mut socket = TcpStream::connect(address).await.unwrap();
    let mut frame = format!(
        "{method} {target} HTTP/1.1\r\nHost: owned.test\r\nUser-Agent: Mozilla/5.0 Chrome/126.0\r\nAccept: */*\r\n{fields}"
    )
    .into_bytes();
    if !body.is_empty() {
        frame.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    frame.extend_from_slice(b"\r\n");
    frame.extend_from_slice(body);
    socket.write_all(&frame).await.unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let status = head[9..12].parse().unwrap();
    Reply {
        status,
        head,
        body: bytes[end + 4..].to_vec(),
    }
}

fn error_code(reply: &Reply) -> String {
    reply.json()["error"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn legacy_absent_keeps_prior_gateway_behaviour() {
    let (address, audit, task) = serve(None).await;
    for path in ["/pass", "/api/pass/new", "/demo", "/static/js/demo.js"] {
        let bare = exchange(address, "GET", path, "").await;
        assert_eq!(
            (bare.status, error_code(&bare).as_str()),
            (400, "missing_integer"),
            "{path}"
        );
        let framed = exchange(address, "GET", path, "Content-Length: 0\r\n").await;
        assert_eq!(
            (framed.status, error_code(&framed).as_str()),
            (409, "route_inventory"),
            "{path}"
        );
    }
    let other = exchange(address, "GET", "/other", "").await;
    assert_eq!(
        other.status, 400,
        "non-legacy bodyless framing stays refused"
    );
    assert!(audit.0.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn browser_get_pass_serves_the_page_with_session_and_security_headers() {
    let (address, audit, task) = serve(Some(policy(true, false, false))).await;
    let page = exchange(address, "GET", "/pass", "").await;
    assert_eq!(page.status, 200, "{}", page.head);
    assert_eq!(
        page.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let cookie = page.header("Set-Cookie").unwrap();
    assert!(
        cookie.starts_with("hsid=") && cookie.ends_with("; Path=/; HttpOnly; Secure; SameSite=Lax")
    );
    assert_eq!(page.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(
        page.header("Referrer-Policy"),
        Some("strict-origin-when-cross-origin")
    );
    assert!(
        page.header("Content-Security-Policy-Report-Only")
            .unwrap()
            .contains("'wasm-unsafe-eval'")
    );
    assert_eq!(
        page.body,
        std::fs::read(web_dir().join("pass.html")).unwrap()
    );
    assert_eq!(
        page.header("Content-Length"),
        Some(page.body.len().to_string().as_str())
    );
    let head = exchange(address, "HEAD", "/pass", "").await;
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());
    // Public sessionless surface: no gateway decision is written.
    assert!(audit.0.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn pass_api_issues_a_challenge_and_enforces_post_only_routes() {
    let (address, _, task) = serve(Some(policy(true, false, false))).await;
    let fresh = exchange(address, "GET", "/api/pass/new", "").await;
    assert_eq!(fresh.status, 200, "{}", fresh.head);
    let session = fresh.header("Set-Cookie").unwrap();
    let session = session.split(';').next().unwrap().to_owned();
    let issued = fresh.json();
    assert_eq!(issued["ok"], true);
    assert_eq!(
        issued["triesLeft"],
        crate::gateway::pass_store::PASS_MAX_TRIES
    );
    assert!(issued["challenge"].is_object());
    assert!(!issued["challengeNonce"].as_str().unwrap().is_empty());
    assert!(issued["preflight"]["pow"].is_object());
    let cookie = format!("Cookie: {session}\r\n");
    let again = exchange(address, "GET", "/api/pass/new", &cookie).await;
    assert_eq!(again.header("Set-Cookie"), None, "the session is reused");
    assert_ne!(again.json()["challengeNonce"], issued["challengeNonce"]);
    for path in ["/api/pass/pow", "/api/pass/attest", "/api/pass/solve"] {
        let refused = exchange(address, "GET", path, &cookie).await;
        assert_eq!(refused.status, 405, "{path}");
        assert_eq!(refused.body, b"POST only\n");
    }
    let stale = exchange_body(
        address,
        "POST",
        "/api/pass/attest",
        &format!("{cookie}Content-Type: application/json\r\n"),
        br#"{"challengeNonce":"00"}"#,
    )
    .await;
    assert_eq!(stale.status, 200);
    assert_eq!(stale.json()["reason"], "stale challenge");
    let kpi = exchange(address, "GET", "/api/pass/kpi", "").await;
    assert_eq!(kpi.status, 200);
    assert!(kpi.json().is_object());
    task.abort();
}

#[tokio::test]
async fn demo_and_static_follow_the_go_paths() {
    let (address, _, task) = serve(Some(policy(false, true, true))).await;
    let demo = exchange(address, "GET", "/demo", "").await;
    assert_eq!(demo.status, 200);
    assert!(demo.header("Set-Cookie").unwrap().starts_with("hsid="));
    assert_eq!(
        demo.body,
        std::fs::read(web_dir().join("demo.html")).unwrap()
    );
    let script = exchange(address, "GET", "/static/js/demo.js?v=1", "").await;
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("Content-Type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(
        script.body,
        std::fs::read(web_dir().join("js/demo.js")).unwrap()
    );
    let head = exchange(address, "HEAD", "/static/js/demo.js", "").await;
    assert_eq!((head.status, head.body.len()), (200, 0));
    for missing in [
        "/static/",
        "/static/js/",
        "/static/js/absent.js",
        "/static/js//demo.js",
    ] {
        let reply = exchange(address, "GET", missing, "").await;
        assert_eq!(reply.status, 404, "{missing}");
        assert_eq!(reply.body, b"404 page not found\n");
    }
    // Traversal never reaches the dispatcher: the strict parser refuses it.
    let traversal = exchange(address, "GET", "/static/../Cargo.toml", "").await;
    assert_eq!(traversal.status, 400);
    task.abort();
}

#[tokio::test]
async fn disabled_and_unmounted_surfaces_keep_gateway_behaviour() {
    let (address, _, task) = serve(Some(policy(true, false, false))).await;
    let demo = exchange(address, "GET", "/demo", "").await;
    assert_eq!(
        (demo.status, error_code(&demo).as_str()),
        (400, "missing_integer")
    );
    for path in [
        "/demo",
        "/static/js/demo.js",
        "/playground",
        "/__hmn/admin/console",
        "/other",
    ] {
        let reply = exchange(address, "GET", path, "Content-Length: 0\r\n").await;
        assert_eq!(
            (reply.status, error_code(&reply).as_str()),
            (409, "route_inventory"),
            "{path}"
        );
    }
    // The exact authority binding still applies to mounted surfaces.
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(b"GET /pass HTTP/1.1\r\nHost: attacker.test\r\n\r\n")
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 403"));
    task.abort();
}

#[test]
fn configuration_refuses_ambiguous_or_incomplete_legacy_policy() {
    assert!(
        configuration(Some(policy(true, true, true)))
            .validate()
            .is_ok()
    );
    let mut relative = policy(true, false, false);
    relative.web_dir = PathBuf::from("web");
    assert!(configuration(Some(relative)).validate().is_err());
    assert!(
        configuration(Some(policy(false, false, false)))
            .validate()
            .is_err()
    );
    let mut colliding = configuration(Some(policy(true, false, false)));
    colliding.routes[0].path = "/api/pass/solve".into();
    assert!(colliding.validate().is_err());
    let mut disabled = configuration(Some(policy(false, true, false)));
    disabled.routes[0].path = "/api/pass/solve".into();
    assert!(disabled.validate().is_ok(), "only mounted paths collide");
    let mut empty = policy(true, false, false);
    empty.web_dir = web_dir().join("js");
    assert!(
        Application::new(
            configuration(Some(empty)),
            Default::default(),
            Box::new(Audit::default())
        )
        .is_err(),
        "a web directory without pass.html refuses startup"
    );
    let parsed: Result<LegacyPolicy, _> =
        serde_json::from_str(r#"{"web_dir":"/w","pass":true,"console":true}"#);
    assert!(parsed.is_err(), "the console cannot be mounted here");
}

#[test]
fn pass_context_uses_the_peer_and_the_unavailable_ja4_form() {
    assert_eq!(ja4_stable(None), "");
    assert_eq!(
        scoring::fingerprint(&ja4_stable(None), "127.0.0.1"),
        "|127.0.0"
    );
    let hello = ClientHello::default();
    let stable = ja4_stable(Some(&hello));
    let (full, _) = ja4(&hello);
    assert_eq!(stable.matches('_').count(), 1);
    assert!(full.starts_with(&format!("{stable}_")));
}

#[test]
fn memory_reports_are_bounded() {
    let reports = MemoryReports::new(1);
    reports.ensure("a");
    reports.ensure("b");
    let mut scored = SessionReport::default();
    scored.scoring.risk_score = 42.0;
    reports.store_scored("b", scored.clone());
    assert_eq!(reports.get("b").scoring.risk_score, 0.0);
    reports.store_scored("a", scored);
    assert_eq!(reports.get("a").scoring.risk_score, 42.0);
}

/// Step-up recovery through the application decision path (Go
/// internal/gate control.go handleStepUp and gates.go attestation floor).
mod step_up {
    use super::*;
    use crate::gateway::application::{ATTEST_FLOOR_RULE, Decision};
    use crate::gateway::tests::{collected, issue, request, sign};
    use humanymous_core::challenge::{pass, proof_of_work};
    use std::net::Ipv4Addr;

    const PEER: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn attested_configuration(pass: bool) -> Configuration {
        let mut config = configuration(Some(policy(pass, !pass, false)));
        let route: crate::gateway::configuration::Route = serde_json::from_value(
            serde_json::json!({"method":"POST","path":"/attested","attest_floor":true,
                "required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}),
        )
        .unwrap();
        config.routes.push(route);
        config
    }

    struct Lab {
        app: Application,
        config: Configuration,
        audit: Audit,
    }

    fn lab() -> Lab {
        let audit = Audit::default();
        let config = attested_configuration(true);
        assert!(config.validate().is_ok());
        let app = Application::new(
            attested_configuration(true),
            Default::default(),
            Box::new(audit.clone()),
        )
        .unwrap();
        Lab { app, config, audit }
    }

    fn browser(
        method: &str,
        target: &str,
        cookie: Option<&str>,
        extra: &[(&str, &str)],
        body: &[u8],
    ) -> http::Request {
        let mut headers: Vec<(String, String)> = vec![
            ("Host".into(), "owned.test".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 Chrome/126.0 Safari/537.36".into(),
            ),
            ("sec-ch-ua".into(), "Chromium".into()),
        ];
        if let Some(cookie) = cookie {
            headers.push(("Cookie".into(), cookie.into()));
        }
        for (name, value) in extra {
            headers.push(((*name).into(), (*value).into()));
        }
        if !body.is_empty() {
            headers.push(("Content-Length".into(), body.len().to_string()));
        }
        http::Request {
            method: method.into(),
            target: target.into(),
            headers,
            body: body.to_vec(),
        }
    }

    struct Parsed {
        status: u16,
        cookies: Vec<String>,
        body: Value,
    }

    fn legacy(lab: &mut Lab, req: &http::Request, peer: Option<IpAddr>) -> Parsed {
        let mut ack = None;
        let Decision::Legacy(bytes) = lab
            .app
            .decide_observed(req, || Ok(now_ms()), &mut ack, None, peer)
            .unwrap_or_else(|_| panic!("legacy refused"))
        else {
            panic!("not a legacy response");
        };
        let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
        let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
        let cookies = head
            .split("\r\n")
            .filter_map(|line| line.strip_prefix("Set-Cookie: "))
            .map(|value| value.split(';').next().unwrap().to_owned())
            .collect();
        Parsed {
            status: head[9..12].parse().unwrap(),
            cookies,
            body: serde_json::from_slice(&bytes[end + 4..]).unwrap_or(Value::Null),
        }
    }

    /// Issue a Pass instance; returns the `hsid=...` cookie and the instance.
    fn instance(lab: &mut Lab, cookie: Option<&str>) -> (String, Value) {
        let reply = legacy(
            lab,
            &browser("GET", "/api/pass/new", cookie, &[], b""),
            PEER,
        );
        assert_eq!(reply.status, 200);
        let cookie = cookie
            .map(str::to_owned)
            .or_else(|| reply.cookies.into_iter().next())
            .unwrap();
        (cookie, reply.body)
    }

    fn pow(lab: &mut Lab, cookie: &str, issued: &Value) -> bool {
        let challenge = &issued["preflight"]["pow"];
        let nonce = proof_of_work::solve(
            challenge["seed"].as_str().unwrap(),
            challenge["difficulty"].as_i64().unwrap() as i32,
            1 << 24,
        )
        .unwrap();
        let body = serde_json::to_vec(&serde_json::json!({"bucket":challenge["bucket"],
            "nonce":nonce,"challengeNonce":issued["challengeNonce"]}))
        .unwrap();
        legacy(
            lab,
            &browser("POST", "/api/pass/pow", Some(cookie), &[], &body),
            PEER,
        )
        .body["ok"]
            == true
    }

    /// Keyboard-lane solve from the public scene only: no motor trace.
    fn solve_body(issued: &Value) -> Vec<u8> {
        let scene: pass::Challenge = serde_json::from_value(issued["challenge"].clone()).unwrap();
        let offsets: Vec<i64> = scene
            .rows
            .iter()
            .map(|row| (scene.center as i64 - row.key_index as i64).rem_euclid(scene.n as i64))
            .collect();
        serde_json::to_vec(&serde_json::json!({"bucket":issued["bucket"],
            "challengeNonce":issued["challengeNonce"],"offsets":offsets}))
        .unwrap()
    }

    fn solve(lab: &mut Lab, cookie: &str, body: &[u8]) -> Value {
        legacy(
            lab,
            &browser("POST", "/api/pass/solve", Some(cookie), &[], body),
            PEER,
        )
        .body
    }

    fn redeem(lab: &mut Lab, cookie: &str, receipt: Option<&str>) -> Parsed {
        let extra: Vec<(&str, &str)> = receipt
            .map(|receipt| vec![("X-Hmn-Stepup-Receipt", receipt)])
            .unwrap_or_default();
        legacy(
            lab,
            &browser("POST", "/__hmn/stepup", Some(cookie), &extra, b"{}"),
            PEER,
        )
    }

    /// One collected RIT session's signed attested request.
    fn attested(
        lab: &mut Lab,
        cookie: Option<&str>,
        user_agent: &str,
        peer: Option<IpAddr>,
        collect: bool,
    ) -> Result<Decision, Rejection> {
        let now = now_ms();
        let issued = issue(&mut lab.app, now);
        if collect {
            collected(&mut lab.app, &lab.config, &issued, now);
        }
        let mut req = request("POST", "/attested", b"effect".to_vec());
        for header in &mut req.headers {
            if header.0 == "User-Agent" {
                header.1 = user_agent.into();
            }
        }
        if let Some(cookie) = cookie {
            req.headers.push(("Cookie".into(), cookie.into()));
        }
        sign(
            &lab.config,
            &mut req,
            issued["session"].as_str().unwrap(),
            issued["key"].as_str().unwrap(),
            if collect { 2 } else { 1 },
            now,
        );
        let mut ack = None;
        lab.app
            .decide_observed(&req, || Ok(now), &mut ack, None, peer)
    }

    const UA: &str = "Mozilla/5.0 Chrome/126.0 Safari/537.36";

    fn challenged(decision: Result<Decision, Rejection>) -> bool {
        matches!(decision, Ok(Decision::Local(403, body))
            if serde_json::from_slice::<Value>(&body).unwrap()["verdict"] == "CHALLENGE")
    }

    #[test]
    fn solved_pass_redeems_a_socket_bound_proof_that_admits_the_attested_route() {
        let mut lab = lab();
        // A scoring ALLOW on the attested route is enforced as a challenge.
        assert!(challenged(attested(&mut lab, None, UA, PEER, true)));
        {
            let records = lab.audit.0.lock().unwrap();
            let last = records.last().unwrap();
            assert_eq!(last.verdict, "ALLOW");
            assert_eq!(last.enforcement_action, "CHALLENGE");
            assert_eq!(
                last.triggered_rules,
                Some(vec![ATTEST_FLOOR_RULE.to_owned()])
            );
        }
        let (cookie, issued) = instance(&mut lab, None);
        assert!(cookie.starts_with("hsid="));
        // Missing proof of work: refused before any placement check, no receipt.
        let refused = solve(&mut lab, &cookie, &solve_body(&issued));
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["reason"], "no cryptographic preflight");
        assert!(refused.get("stepUpReceipt").is_none());
        assert!(pow(&mut lab, &cookie, &issued));
        let solved = solve(&mut lab, &cookie, &solve_body(&issued));
        assert_eq!(solved["ok"], true, "{solved}");
        let receipt = solved["stepUpReceipt"].as_str().unwrap().to_owned();
        // Redemption refusals grant nothing.
        assert_eq!(redeem(&mut lab, &cookie, None).status, 403);
        let mut forged = receipt.clone();
        let last = forged.pop().unwrap();
        forged.push(if last == 'A' { 'B' } else { 'A' });
        assert_eq!(redeem(&mut lab, &cookie, Some(&forged)).status, 403);
        let (other, _) = instance(&mut lab, None);
        assert_ne!(other, cookie);
        assert_eq!(redeem(&mut lab, &other, Some(&receipt)).status, 403);
        let get = legacy(
            &mut lab,
            &browser(
                "GET",
                "/__hmn/stepup",
                Some(&cookie),
                &[("X-Hmn-Stepup-Receipt", &receipt)],
                b"",
            ),
            PEER,
        );
        assert_eq!(get.status, 405);
        let no_peer = legacy(
            &mut lab,
            &browser(
                "POST",
                "/__hmn/stepup",
                Some(&cookie),
                &[("X-Hmn-Stepup-Receipt", &receipt)],
                b"{}",
            ),
            None,
        );
        assert_eq!(no_peer.status, 403);
        // Genuine redemption.
        let redeemed = redeem(&mut lab, &cookie, Some(&receipt));
        assert_eq!(redeemed.status, 200);
        let proof = redeemed
            .cookies
            .into_iter()
            .find(|c| c.starts_with("hmn_su="))
            .unwrap();
        let both = format!("{cookie}; {proof}");
        assert!(matches!(
            attested(&mut lab, Some(&both), UA, PEER, true),
            Ok(Decision::Origin { .. })
        ));
        assert_eq!(
            lab.audit
                .0
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .enforcement_action,
            "ALLOW"
        );
        // Lifted to another header fingerprint, another session, or no peer.
        assert!(challenged(attested(
            &mut lab,
            Some(&both),
            "Other/1.0",
            PEER,
            true
        )));
        let foreign = format!("{other}; {proof}");
        assert!(challenged(attested(
            &mut lab,
            Some(&foreign),
            UA,
            PEER,
            true
        )));
        assert!(challenged(attested(&mut lab, Some(&both), UA, None, true)));
        assert!(challenged(attested(&mut lab, Some(&proof), UA, PEER, true)));
        // A proof never upgrades a scoring CHALLENGE (missing client, HR-10).
        assert!(challenged(attested(&mut lab, Some(&both), UA, PEER, false)));
        assert_eq!(
            lab.audit.0.lock().unwrap().last().unwrap().verdict,
            "CHALLENGE"
        );
        // Replaying the solved body against a fresh instance is stale.
        let body = solve_body(&issued);
        instance(&mut lab, Some(&cookie));
        let replayed = solve(&mut lab, &cookie, &body);
        assert_eq!(replayed["ok"], false);
        assert_eq!(replayed["reason"], "stale or missing challenge");
        // Positive control after every negative.
        assert!(matches!(
            attested(&mut lab, Some(&both), UA, PEER, true),
            Ok(Decision::Origin { .. })
        ));
    }

    #[test]
    fn expired_receipt_and_proof_are_refused() {
        let lab = lab();
        let surfaces = lab.app.legacy().unwrap();
        let key = &surfaces.step_up.as_ref().unwrap().key;
        let now = now_ms() / 1000;
        let receipt = key.issue_receipt("session", now).unwrap();
        assert!(
            key.verify_receipt(&receipt, "session", now + token::RECEIPT_SKEW)
                .is_ok()
        );
        assert!(
            key.verify_receipt(&receipt, "session", now + token::RECEIPT_SKEW + 1)
                .is_err()
        );
        let step_up = surfaces.step_up.as_ref().unwrap();
        let req = browser("POST", "/attested", None, &[], b"");
        let bound = bind(&req, PEER).unwrap();
        let epoch = step_up.epochs.current(now);
        let fresh = key
            .issue_step_up("session", &bound, &epoch, now + 60)
            .unwrap();
        let stale = key
            .issue_step_up("session", &bound, &epoch, now - 1)
            .unwrap();
        for (proof, valid) in [(fresh, true), (stale, false)] {
            let req = browser(
                "POST",
                "/attested",
                Some(&format!("hsid=session; hmn_su={proof}")),
                &[],
                b"",
            );
            assert_eq!(surfaces.step_up_valid(&req, PEER, now), valid);
        }
    }

    #[test]
    fn attest_floor_needs_the_native_pass_surface_and_is_reported() {
        let config = attested_configuration(true);
        assert!(config.validate().is_ok());
        assert_eq!(config.readiness()["routes"][1]["attest_floor"], true);
        assert!(
            config.readiness()["routes"][0]
                .get("attest_floor")
                .is_none()
        );
        assert!(attested_configuration(false).validate().is_err());
        let mut absent = attested_configuration(true);
        absent.legacy = None;
        assert!(absent.validate().is_err());
        let mut browser_profile = attested_configuration(true);
        browser_profile.routes[1].rit_profile =
            crate::gateway::configuration::RitProfile::BrowserAuthOnly;
        browser_profile.browser = Some(
            serde_json::from_value(serde_json::json!({"origin":"http://owned.test"})).unwrap(),
        );
        assert!(browser_profile.validate().is_err());
        assert!(surface_path(STEP_UP_PATH));
        assert!(claims(&policy(true, false, false), STEP_UP_PATH));
        assert!(!claims(&policy(false, true, true), STEP_UP_PATH));
        let surfaces = Surfaces::new(&policy(false, true, false), &Engine::default(), 4)
            .unwrap_or_else(|_| panic!("demo surfaces"));
        assert!(!surfaces.mounted(STEP_UP_PATH));
        assert!(!surfaces.step_up_valid(
            &browser("POST", "/x", Some("hsid=a; hmn_su=b"), &[], b""),
            PEER,
            0
        ));
    }
}
