//! B1 differential replay of the Go Pass handlers plus the accessible-default
//! and fail-closed behavior of the Rust flow.
use super::*;
use crate::gateway::request::ja4_stable;
use crate::network::http::client_subnet;
use std::collections::{BTreeMap, VecDeque};
use time::OffsetDateTime;

const HOUR: Duration = Duration::from_secs(3600);

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/pass-flow-go-golden.json"
    ))
    .unwrap()
}

fn at(unix: i64, mono: Duration) -> Now {
    Now {
        wall: Timestamp(OffsetDateTime::from_unix_timestamp(unix).unwrap()),
        mono,
    }
}

fn flow(constants: &Value, step_up: bool, motor: MotorEvidence) -> PassFlow {
    mount(Config {
        master_key: constants["masterKey"].as_str().unwrap().as_bytes().to_vec(),
        web_dir: PathBuf::from("web"),
        step_up: step_up.then(|| {
            TokenKey::new(constants["stepUpKey"].as_str().unwrap().as_bytes().to_vec()).unwrap()
        }),
        motor_evidence: motor,
    })
    .unwrap()
}

fn queued(nonces: Vec<String>) -> impl FnMut() -> Result<PassNonce, Refusal> + Send + 'static {
    let mut queue: VecDeque<String> = nonces.into();
    move || {
        let hex = queue
            .pop_front()
            .expect("more /new calls than recorded nonces");
        let bytes: [u8; 18] = hex::decode(hex).unwrap().try_into().unwrap();
        Ok(PassNonce::from_entropy(bytes))
    }
}

fn fixed_nonces() -> impl FnMut() -> Result<PassNonce, Refusal> + Send + 'static {
    let mut counter = 0_u8;
    move || {
        counter = counter.wrapping_add(1);
        Ok(PassNonce::from_entropy([counter; 18]))
    }
}

/// Numbers compared as f64: Go writes `0`, serde writes `0.0`.
fn normalize(value: Value) -> Value {
    match value {
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, normalize(v))).collect())
        }
        other => other,
    }
}

fn body(response: &Response) -> &[u8] {
    match &response.body {
        Body::Bytes(bytes) => bytes,
        _ => panic!("expected a byte body"),
    }
}

fn header<'r>(response: &'r Response, name: &str) -> Option<&'r str> {
    response
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn body_json(response: &Response) -> Value {
    serde_json::from_slice(body(response)).unwrap()
}

struct Harness {
    store: CollectorStore,
    engine: Engine,
    hub: Option<Arc<Hub>>,
}

impl Harness {
    fn new() -> Self {
        Self {
            store: CollectorStore::new(HOUR),
            engine: Engine::default(),
            hub: None,
        }
    }
    fn backend(&mut self) -> StoreBackend<'_> {
        StoreBackend {
            store: &mut self.store,
            engine: &self.engine,
            hub: self.hub.as_ref(),
        }
    }
    fn signal_ids(&self, session: &str) -> Vec<String> {
        self.store
            .get(session)
            .and_then(|report| report.network.signals)
            .unwrap_or_default()
            .into_iter()
            .map(|signal| signal.id)
            .collect()
    }
}

fn compare_scoring(report: &SessionReport, go: &Value, context: &str) {
    assert_eq!(
        report.scoring.risk_score,
        go["riskScore"].as_f64().unwrap(),
        "{context}"
    );
    assert_eq!(
        report.scoring.verdict,
        go["verdict"].as_str().unwrap(),
        "{context}"
    );
    assert_eq!(
        report.scoring.hard_rule_fired,
        go["hardRuleFired"].as_str().unwrap(),
        "{context}"
    );
}

/// Legacy `passFP` from the Rust request helpers equals the Go value for
/// public IPv4 and IPv6 peers without a ClientHello.
#[test]
fn fingerprint_matches_go_golden() {
    let golden = golden();
    for case in golden["outputs"]["fingerprint"].as_array().unwrap() {
        let remote = case["remote"].as_str().unwrap();
        assert_eq!(
            crate::gateway::pass_scoring::fingerprint(
                &ja4_stable(None),
                &client_subnet(remote, &[], &[])
            ),
            case["fingerprint"].as_str().unwrap(),
            "{remote}"
        );
    }
}

/// B1 differential, legacy TestPassVelocitySignalsRegistered wiring and the
/// R336/R337 governor: per-session and per-fingerprint levels, the flagged
/// level, the folded velocity and flood signals and the frozen-engine score
/// after every observation equal Go.
#[test]
fn velocity_fusion_matches_go_golden() {
    let golden = golden();
    let constants = &golden["outputs"]["constants"];
    let mut flow = flow(constants, false, MotorEvidence::Off);
    let mut harness = Harness::new();
    let mut created = BTreeMap::new();
    let steps = golden["outputs"]["velocity"].as_array().unwrap();
    assert!(steps.len() >= 16);
    for case in steps {
        let step = &case["step"];
        let session = step["sid"].as_str().unwrap();
        let offset = step["offsetMs"].as_u64().unwrap();
        let now = at(
            1_795_000_000 + i64::try_from(offset / 1000).unwrap(),
            Duration::from_millis(offset),
        );
        let fingerprint = case["fingerprint"].as_str().unwrap();
        let levels = flow
            .apply_velocity(&mut harness.backend(), session, fingerprint, now)
            .unwrap();
        let context = format!("{step}");
        assert_eq!(
            (u64::from(levels.0), u64::from(levels.1)),
            (
                case["level"].as_u64().unwrap(),
                case["sessionLevel"].as_u64().unwrap()
            ),
            "{context}"
        );
        let flagged = case["velLevel"].as_i64().unwrap();
        if flagged >= 0 {
            created.insert(session.to_owned(), ());
            let level = flow
                .store
                .with_session(session, now.mono, |state| state.velocity_level)
                .unwrap();
            assert_eq!(i64::from(level), flagged, "{context}");
        }
        assert_eq!(flow.store.session_count(), created.len(), "{context}");
        let report = harness.store.get(session);
        assert_eq!(
            report.is_some(),
            case["stored"].as_bool().unwrap(),
            "{context}"
        );
        let expected: Vec<Signal> = serde_json::from_value::<Option<_>>(case["signals"].clone())
            .unwrap()
            .unwrap_or_default();
        let report = report.unwrap_or_default();
        assert_eq!(
            report.network.signals.clone().unwrap_or_default(),
            expected,
            "{context}"
        );
        compare_scoring(&report, &case["scoring"], &context);
    }
}

/// B1 differential replay of the Go handler transcripts through
/// `app.routes()`: page-less Pass scenarios (instance issue, PoW preflight,
/// attestation issuance budget, solve rejections in Go order, retry cap,
/// step-up receipts, duplicate traces, the real-event filter and behavior
/// tells in Go parity mode, KPIs) produce the same status, content type and
/// JSON, and the same stored signals and frozen-engine scores per session.
#[test]
fn handler_scenarios_match_go_golden() {
    let golden = golden();
    let constants = &golden["outputs"]["constants"];
    let scenarios = golden["outputs"]["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 8);
    let mut replayed = 0;
    for scenario in scenarios {
        let name = scenario["name"].as_str().unwrap();
        let nonces: Vec<String> = serde_json::from_value(scenario["nonces"].clone()).unwrap();
        let mut flow = flow(
            constants,
            scenario["stepUp"].as_bool().unwrap(),
            MotorEvidence::GoParity,
        )
        .with_nonce_source(queued(nonces));
        let mut harness = Harness::new();
        let steps = scenario["steps"].as_array().unwrap();
        let base = steps[0]["nowUnix"].as_i64().unwrap();
        for (index, step) in steps.iter().enumerate() {
            let unix = step["nowUnix"].as_i64().unwrap();
            let now = at(
                unix,
                Duration::from_secs(u64::try_from(unix - base).unwrap())
                    + Duration::from_millis(index as u64),
            );
            let cookie = step["cookie"].as_str().unwrap();
            let red_team = step["step"]["redteam"].as_str();
            let request = Request {
                method: step["method"].as_str().unwrap(),
                path: step["path"].as_str().unwrap(),
                cookie: (!cookie.is_empty()).then_some(cookie),
                red_team,
                fingerprint: step["fingerprint"].as_str().unwrap(),
                body: step["body"].as_str().unwrap().as_bytes(),
            };
            let response = flow.handle(&mut harness.backend(), &request, now).unwrap();
            let context = format!("{name} step {index}: {}", step["step"]);
            assert_eq!(
                u64::from(response.status),
                step["status"].as_u64().unwrap(),
                "{context}"
            );
            assert_eq!(
                header(&response, "Content-Type"),
                step["contentType"].as_str(),
                "{context}"
            );
            assert_eq!(
                header(&response, "Set-Cookie").is_some(),
                step["setCookie"].as_bool().unwrap(),
                "{context}"
            );
            let go = step["response"].as_str().unwrap();
            if step["contentType"] == "application/json" {
                assert_eq!(
                    normalize(body_json(&response)),
                    normalize(serde_json::from_str(go).unwrap()),
                    "{context}"
                );
            } else {
                assert_eq!(body(&response), go.as_bytes(), "{context}");
            }
            replayed += 1;
        }
        let finals = scenario["finals"].as_object().unwrap();
        for (session, go) in finals {
            let report = harness.store.get(session);
            assert_eq!(
                report.is_some(),
                go["stored"].as_bool().unwrap(),
                "{name} {session}"
            );
            let expected: Vec<Signal> = serde_json::from_value::<Option<_>>(go["signals"].clone())
                .unwrap()
                .unwrap_or_default();
            let report = report.unwrap_or_default();
            assert_eq!(
                report.network.signals.clone().unwrap_or_default(),
                expected,
                "{name} {session}"
            );
            compare_scoring(&report, &go["scoring"], &format!("{name} {session}"));
        }
    }
    assert!(replayed >= 70, "replayed {replayed} Go requests");
}

fn post<'a>(path: &'a str, cookie: &'a str, body: &'a [u8]) -> Request<'a> {
    Request {
        method: "POST",
        path,
        cookie: Some(cookie),
        red_team: None,
        fingerprint: "|198.51.100",
        body,
    }
}

fn get<'a>(path: &'a str, cookie: Option<&'a str>) -> Request<'a> {
    Request {
        method: "GET",
        path,
        cookie,
        red_team: None,
        fingerprint: "|198.51.100",
        body: b"",
    }
}

/// Issue an instance and pay its proof of work; returns the `/new` reply.
fn issue_and_pay(flow: &mut PassFlow, harness: &mut Harness, cookie: &str, now: Now) -> Value {
    let issued = body_json(
        &flow
            .handle(&mut harness.backend(), &get(NEW_PATH, Some(cookie)), now)
            .unwrap(),
    );
    let pow = &issued["preflight"]["pow"];
    let solution = proof_of_work::solve(
        pow["seed"].as_str().unwrap(),
        i32::try_from(pow["difficulty"].as_i64().unwrap()).unwrap(),
        1 << 26,
    )
    .unwrap();
    let paid = json!({
        "bucket": pow["bucket"],
        "nonce": solution,
        "challengeNonce": issued["challengeNonce"],
    })
    .to_string();
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(POW_PATH, cookie, paid.as_bytes()),
            now,
        )
        .unwrap();
    assert_eq!(body_json(&reply), json!({"ok": true}));
    issued
}

fn solution(issued: &Value) -> Vec<i64> {
    let challenge = &issued["challenge"];
    let n = challenge["n"].as_i64().unwrap();
    let center = challenge["center"].as_i64().unwrap();
    challenge["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (center - row["keyIndex"].as_i64().unwrap()).rem_euclid(n))
        .collect()
}

fn mounted(motor: MotorEvidence) -> PassFlow {
    mount(Config {
        master_key: b"lane-c-unit-master-key-000000001".to_vec(),
        web_dir: PathBuf::from("/srv/web"),
        step_up: None,
        motor_evidence: motor,
    })
    .unwrap()
    .with_nonce_source(fixed_nonces())
}

/// Accessibility default: fast, perfectly regular, untrusted-flagged
/// keyboard input (a switch or other assistive device) clears Pass and adds
/// no behavior tell; Go parity mode rejects the same input (untrusted
/// events). Gating on motor timing stays off unless the owner decides.
#[test]
fn motor_evidence_is_off_by_default_and_never_gates_pass() {
    assert_eq!(MotorEvidence::default(), MotorEvidence::Off);
    let proof = |issued: &Value| {
        json!({
            "challengeNonce": issued["challengeNonce"],
            "offsets": solution(issued),
            "trusted": false,
            "keys": 3,
            "keyDurs": [5, 5, 5, 5],
        })
        .to_string()
    };
    let now = at(1_795_000_000, Duration::from_secs(1));
    let mut accessible = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let issued = issue_and_pay(&mut accessible, &mut harness, "hsid=switch-user", now);
    let reply = body_json(
        &accessible
            .handle(
                &mut harness.backend(),
                &post(SOLVE_PATH, "hsid=switch-user", proof(&issued).as_bytes()),
                now,
            )
            .unwrap(),
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert_eq!(harness.signal_ids("switch-user"), ["l7.pass.solved"]);

    let mut parity = mounted(MotorEvidence::GoParity);
    let mut harness = Harness::new();
    let issued = issue_and_pay(&mut parity, &mut harness, "hsid=switch-user", now);
    let reply = body_json(
        &parity
            .handle(
                &mut harness.backend(),
                &post(SOLVE_PATH, "hsid=switch-user", proof(&issued).as_bytes()),
                now,
            )
            .unwrap(),
    );
    assert_eq!(reply["reason"], json!("untrusted events"), "{reply}");
}

/// Expiry is reported as expiry (audit ACC-1), never as a wrong answer, and
/// is not counted as a failed KPI attempt; the live feed sees `expired`.
#[test]
fn expired_instance_reports_expiry_not_failure() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let hub = Hub::new(0, 0);
    harness.hub = Some(Arc::clone(&hub));
    let issued_at = at(1_795_000_000, Duration::from_secs(1));
    let issued = issue_and_pay(&mut flow, &mut harness, "hsid=slow-user-123", issued_at);
    let proof = json!({
        "challengeNonce": issued["challengeNonce"],
        "offsets": solution(&issued),
        "keys": 3,
        "keyDurs": [900.5, 1400.25, 2100.75],
    })
    .to_string();
    let late = at(1_795_000_000 + 21 * 30, Duration::from_secs(1 + 21 * 30));
    let reply = body_json(
        &flow
            .handle(
                &mut harness.backend(),
                &post(SOLVE_PATH, "hsid=slow-user-123", proof.as_bytes()),
                late,
            )
            .unwrap(),
    );
    assert_eq!(
        reply,
        json!({"ok": false, "expired": true,
               "reason": "this check expired — press New puzzle to start a fresh one"})
    );
    assert!(flow.store().metrics().is_empty());
    let (events, _subscription) = hub.subscribe(0);
    let last = events.last().unwrap();
    assert_eq!(last.kind, "pass.attempt");
    assert_eq!(
        serde_json::from_str::<Value>(&last.data).unwrap(),
        json!({"sid": "slow-use", "solved": false, "reason": "expired"})
    );
}

/// Legacy TestAttestIssuanceBudget wired into `/api/pass/attest`: the
/// per-fingerprint budget issues two tokens and refuses the third, across
/// sessions sharing the fingerprint, and self-clears after the window.
#[test]
fn attest_issuance_budget_is_per_fingerprint_and_self_clearing() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let mut answers = Vec::new();
    for cookie in ["hsid=a1", "hsid=a2", "hsid=a3", "hsid=a4"] {
        let issued = body_json(
            &flow
                .handle(&mut harness.backend(), &get(NEW_PATH, Some(cookie)), now)
                .unwrap(),
        );
        let body = json!({"challengeNonce": issued["challengeNonce"]}).to_string();
        let reply = body_json(
            &flow
                .handle(
                    &mut harness.backend(),
                    &post(ATTEST_PATH, cookie, body.as_bytes()),
                    now,
                )
                .unwrap(),
        );
        answers.push(reply["ok"].as_bool().unwrap());
        if cookie == "hsid=a4" {
            let later = at(1_795_000_031, Duration::from_secs(32));
            let reply = body_json(
                &flow
                    .handle(
                        &mut harness.backend(),
                        &post(ATTEST_PATH, cookie, body.as_bytes()),
                        later,
                    )
                    .unwrap(),
            );
            assert_eq!(reply["ok"], json!(true), "window self-clears: {reply}");
        }
    }
    assert_eq!(answers, [true, true, false, false]);
}

/// The page serves the accessible `pass.html` and issues the session cookie
/// only when absent; the checked-in page is keyboard and screen-reader
/// operable and drives every Pass endpoint.
#[test]
fn page_serves_accessible_pass_html_with_session() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let fresh = flow
        .handle(&mut harness.backend(), &get(PAGE_PATH, None), now)
        .unwrap();
    assert_eq!(fresh.status, 200);
    let cookie = header(&fresh, "Set-Cookie").unwrap();
    assert!(cookie.starts_with("hsid=") && cookie.contains("HttpOnly"));
    assert!(
        matches!(&fresh.body, Body::File(path) if path == &PathBuf::from("/srv/web/pass.html"))
    );
    assert_eq!(harness.store.list(10).len(), 1);
    let returning = flow
        .handle(
            &mut harness.backend(),
            &get(PAGE_PATH, Some("hsid=back")),
            now,
        )
        .unwrap();
    assert_eq!(header(&returning, "Set-Cookie"), None);
    assert!(harness.store.get("back").is_some());

    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web/pass.html"),
    )
    .unwrap();
    for needle in [
        "tabindex=\"0\"",
        "role=\"status\"",
        "aria-live=\"polite\"",
        "keydown",
        NEW_PATH,
        POW_PATH,
        ATTEST_PATH,
        SOLVE_PATH,
    ] {
        assert!(page.contains(needle), "pass.html lacks {needle}");
    }
}

/// Fail-closed refusals and the documented decoding differences.
#[test]
fn refusals_and_decoding_differences_are_pinned() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    assert!(
        flow.handle(&mut harness.backend(), &get("/api/pass/other", None), now)
            .is_none()
    );
    let mut bad_label = get(NEW_PATH, Some("hsid=r1"));
    bad_label.red_team = Some("two words");
    let reply = flow
        .handle(&mut harness.backend(), &bad_label, now)
        .unwrap();
    assert_eq!(
        (reply.status, body(&reply)),
        (400, &b"bad red-team label\n"[..])
    );
    let long = format!("hsid={}", "s".repeat(129));
    let reply = flow
        .handle(&mut harness.backend(), &get(NEW_PATH, Some(&long)), now)
        .unwrap();
    assert_eq!((reply.status, body(&reply)), (400, &b"bad session\n"[..]));

    let issued = body_json(
        &flow
            .handle(&mut harness.backend(), &get(NEW_PATH, Some("hsid=d1")), now)
            .unwrap(),
    );
    let nonce = issued["challengeNonce"].as_str().unwrap();
    let exact = format!(r#"{{"challengeNonce":"{nonce}"}} trailing"#);
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(ATTEST_PATH, "hsid=d1", exact.as_bytes()),
            now,
        )
        .unwrap();
    assert_eq!(body_json(&reply)["ok"], json!(true), "first value decodes");
    let folded = format!(r#"{{"ChallengeNonce":"{nonce}"}}"#);
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(ATTEST_PATH, "hsid=d1", folded.as_bytes()),
            now,
        )
        .unwrap();
    assert_eq!(
        body_json(&reply),
        json!({"ok": false, "reason": "stale challenge"}),
        "field names are exact"
    );
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(SOLVE_PATH, "hsid=d1", b"null"),
            now,
        )
        .unwrap();
    assert_eq!(
        body_json(&reply)["reason"],
        json!("stale or missing challenge"),
        "null decodes to the empty proof"
    );
    let oversized = format!(
        r#"{{"challengeNonce":"{}"}}"#,
        "x".repeat(MAX_PREFLIGHT_BODY)
    );
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(ATTEST_PATH, "hsid=d1", oversized.as_bytes()),
            now,
        )
        .unwrap();
    assert_eq!((reply.status, body(&reply)), (400, &b"bad body\n"[..]));
    let kpi = flow
        .handle(&mut harness.backend(), &get(KPI_PATH, None), now)
        .unwrap();
    assert_eq!(body_json(&kpi)["humanAttempts"], json!(1));
    assert_eq!(session_cookie_value(Some("a=b; hsid=\"q1\"")), Some("q1"));
    assert_eq!(
        session_cookie_value(Some("hsid=bad\\v; hsid=ok")),
        Some("ok")
    );
    assert_eq!(session_cookie_value(Some("hsid=")), None);
}

/// A trace sample outside the int64 range cannot be fingerprinted and is
/// rejected as unusable after the alignment check.
#[test]
fn unusable_trace_is_rejected_after_alignment() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let issued = issue_and_pay(&mut flow, &mut harness, "hsid=u1", now);
    let proof = json!({
        "challengeNonce": issued["challengeNonce"],
        "offsets": solution(&issued),
        "keyDurs": [1e300],
    })
    .to_string();
    let reply = body_json(
        &flow
            .handle(
                &mut harness.backend(),
                &post(SOLVE_PATH, "hsid=u1", proof.as_bytes()),
                now,
            )
            .unwrap(),
    );
    assert_eq!(
        reply,
        json!({"ok": false, "reason": "unusable interaction trace", "triesLeft": 3})
    );
}

/// A store backend whose frozen-engine call can be made to fail.
struct Flaky<'a> {
    inner: StoreBackend<'a>,
    fail: bool,
}

impl Backend for Flaky<'_> {
    fn ensure(&mut self, session: &str, now: Timestamp) {
        self.inner.ensure(session, now);
    }
    fn report(&self, session: &str) -> Option<SessionReport> {
        self.inner.report(session)
    }
    fn store_scored(&mut self, session: &str, report: SessionReport, now: Timestamp) {
        self.inner.store_scored(session, report, now);
    }
    fn score(&mut self, report: &mut SessionReport) -> Option<ScoreResult> {
        if self.fail {
            None
        } else {
            self.inner.score(report)
        }
    }
}

fn unavailable(response: &Response) -> (u16, &[u8]) {
    (response.status, body(response))
}

const UNAVAILABLE: (u16, &[u8]) = (503, b"pass unavailable\n");

/// A scoring failure on a solve fails closed with 503 and spends neither
/// the solve nor the interaction trace: the same proof clears once scoring
/// is back.
#[test]
fn scoring_failure_on_solve_fails_closed_and_keeps_the_trace() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let issued = issue_and_pay(&mut flow, &mut harness, "hsid=sc1", now);
    let proof = json!({
        "challengeNonce": issued["challengeNonce"],
        "offsets": solution(&issued),
        "keyDurs": [90, 110, 95],
    })
    .to_string();
    let solve = post(SOLVE_PATH, "hsid=sc1", proof.as_bytes());
    let failed = flow
        .handle(
            &mut Flaky {
                inner: harness.backend(),
                fail: true,
            },
            &solve,
            now,
        )
        .unwrap();
    assert_eq!(unavailable(&failed), UNAVAILABLE);
    assert!(harness.signal_ids("sc1").is_empty());
    let cleared = body_json(&flow.handle(&mut harness.backend(), &solve, now).unwrap());
    assert_eq!(cleared["ok"], json!(true), "{cleared}");
    assert_eq!(harness.signal_ids("sc1"), ["l7.pass.solved"]);
}

/// A scoring failure on a velocity escalation fails closed with 503 and
/// does not commit the level, so the next request still adds the signal.
#[test]
fn scoring_failure_on_velocity_fails_closed_and_keeps_the_escalation() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let new = get(NEW_PATH, Some("hsid=v1"));
    let mut refused = false;
    for _ in 0..VELOCITY_HARD {
        let reply = flow
            .handle(
                &mut Flaky {
                    inner: harness.backend(),
                    fail: true,
                },
                &new,
                now,
            )
            .unwrap();
        if reply.status == 503 {
            assert_eq!(unavailable(&reply), UNAVAILABLE);
            refused = true;
            break;
        }
        assert_eq!(reply.status, 200);
    }
    assert!(refused, "velocity never escalated");
    assert!(harness.signal_ids("v1").is_empty());
    let reply = flow.handle(&mut harness.backend(), &new, now).unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(harness.signal_ids("v1"), ["l7.pass.velocity"]);
}

/// A limiter refusal (here an over-long limiter key) fails closed with 503
/// on both the velocity and the attestation limiter.
#[test]
fn limiter_failure_fails_closed() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    let long = "f".repeat(MAX_LIMITER_KEY_BYTES);
    let mut new = get(NEW_PATH, Some("hsid=l1"));
    new.fingerprint = &long;
    let reply = flow.handle(&mut harness.backend(), &new, now).unwrap();
    assert_eq!(unavailable(&reply), UNAVAILABLE);

    let issued = body_json(
        &flow
            .handle(&mut harness.backend(), &get(NEW_PATH, Some("hsid=l1")), now)
            .unwrap(),
    );
    let attest = json!({"challengeNonce": issued["challengeNonce"]}).to_string();
    let mut request = post(ATTEST_PATH, "hsid=l1", attest.as_bytes());
    request.fingerprint = &long;
    let reply = flow.handle(&mut harness.backend(), &request, now).unwrap();
    assert_eq!(unavailable(&reply), UNAVAILABLE);
}

/// A Pass store full of fresh sessions refuses a new session with 503 on
/// issue and on solve instead of serving it unmetered (Go did).
#[test]
fn saturated_store_fails_closed() {
    let mut flow = mounted(MotorEvidence::Off);
    let mut harness = Harness::new();
    let now = at(1_795_000_000, Duration::from_secs(1));
    for index in 0..crate::gateway::pass_store::MAX_PASS_SESSIONS {
        flow.store()
            .with_session(&format!("held-{index}"), now.mono, |_| ())
            .unwrap();
    }
    let reply = flow
        .handle(
            &mut harness.backend(),
            &get(NEW_PATH, Some("hsid=late")),
            now,
        )
        .unwrap();
    assert_eq!(unavailable(&reply), UNAVAILABLE);
    let reply = flow
        .handle(
            &mut harness.backend(),
            &post(SOLVE_PATH, "hsid=late", b"{}"),
            now,
        )
        .unwrap();
    assert_eq!(unavailable(&reply), UNAVAILABLE);
}
