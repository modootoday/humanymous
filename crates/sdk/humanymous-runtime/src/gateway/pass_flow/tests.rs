//! Ports of legacy cmd/server/pass_handler_test.go at baseline 981d25c plus
//! flow tests for pass_handler.go and pass_scoring.go.
use super::scoring::*;
use super::*;
use crate::gateway::token::MIN_KEY;
use humanymous_core::signals::observation::{Source, Verdict, lookup};
use std::{collections::HashMap, path::Path};

const KEY: &[u8] = b"test-master-key";
const START: Duration = Duration::from_secs(1_800_000_000);

struct TestClock {
    now: Mutex<(Duration, Duration)>,
}

impl TestClock {
    fn advance(&self, by: Duration) {
        let mut now = self.now.lock().unwrap();
        now.0 += by;
        now.1 += by;
    }
}

impl Clock for TestClock {
    fn wall(&self) -> Duration {
        self.now.lock().unwrap().0
    }
    fn monotonic(&self) -> Duration {
        self.now.lock().unwrap().1
    }
}

#[derive(Default)]
struct MemoryReports {
    reports: Mutex<HashMap<String, SessionReport>>,
    ensured: Mutex<Vec<String>>,
}

impl MemoryReports {
    fn signal_ids(&self, session: &str) -> Vec<String> {
        self.get(session)
            .network
            .signals
            .unwrap_or_default()
            .into_iter()
            .map(|signal| signal.id)
            .collect()
    }
    fn set_risk(&self, session: &str, risk: f64) {
        let mut report = self.get(session);
        report.scoring.risk_score = risk;
        self.store_scored(session, report);
    }
}

impl Reports for MemoryReports {
    fn ensure(&self, session: &str) {
        self.ensured.lock().unwrap().push(session.to_owned());
    }
    fn get(&self, session: &str) -> SessionReport {
        self.reports
            .lock()
            .unwrap()
            .get(session)
            .cloned()
            .unwrap_or_default()
    }
    fn store_scored(&self, session: &str, report: SessionReport) {
        self.reports
            .lock()
            .unwrap()
            .insert(session.to_owned(), report);
    }
}

struct Fixture {
    flow: PassFlow,
    clock: Arc<TestClock>,
    reports: Arc<MemoryReports>,
    fed: Arc<Mutex<Vec<bool>>>,
    hub: Arc<Hub>,
    step_up: Arc<TokenKey>,
}

fn web_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web")
}

fn fixture(step_up: bool) -> Fixture {
    let clock = Arc::new(TestClock {
        now: Mutex::new((START, Duration::from_secs(1))),
    });
    let reports = Arc::new(MemoryReports::default());
    let fed = Arc::new(Mutex::new(Vec::new()));
    let hub = Hub::new(64, 64);
    let key = Arc::new(TokenKey::new(vec![7; MIN_KEY]).unwrap());
    let sink = fed.clone();
    let flow = PassFlow::new(
        KEY.to_vec(),
        web_dir(),
        Dependencies {
            engine: Arc::new(Engine::default()),
            reports: reports.clone(),
            clock: clock.clone(),
            feed: Arc::new(move |_, solved| sink.lock().unwrap().push(solved)),
            hub: Some(hub.clone()),
            step_up: step_up.then(|| key.clone()),
        },
    );
    Fixture {
        flow,
        clock,
        reports,
        fed,
        hub,
        step_up: key,
    }
}

fn cookie(session: &str) -> String {
    format!("hsid={session}")
}

fn call(
    fixture: &Fixture,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: &[u8],
    context: PassContext<'_>,
) -> Response {
    let cookie = session.map(cookie);
    let request = Request {
        method,
        path,
        host: "example.test",
        cookie: cookie.as_deref(),
        body,
        ..Request::default()
    };
    fixture.flow.handle(&request, &context).unwrap()
}

fn json_of(response: &Response) -> Value {
    serde_json::from_slice(response.bytes().unwrap()).unwrap()
}

const FP: PassContext<'static> = PassContext {
    fingerprint: "t13d1516h2_8daaf6152771|203.0.113",
    redteam: None,
};

fn new_instance(fixture: &Fixture, session: &str) -> Value {
    let response = call(fixture, "GET", NEW_PATH, Some(session), b"", FP);
    assert_eq!(response.status, 200);
    json_of(&response)
}

fn pay_pow(fixture: &Fixture, session: &str, issued: &Value) -> bool {
    let pow = &issued["preflight"]["pow"];
    let solution = proof_of_work::solve(
        pow["seed"].as_str().unwrap(),
        pow["difficulty"].as_i64().unwrap() as i32,
        1 << 26,
    )
    .unwrap();
    let body = json!({
        "bucket": pow["bucket"],
        "nonce": solution,
        "challengeNonce": issued["challengeNonce"],
    });
    let response = call(
        fixture,
        "POST",
        POW_PATH,
        Some(session),
        body.to_string().as_bytes(),
        FP,
    );
    json_of(&response)["ok"] == true
}

fn proof(fixture: &Fixture, session: &str, issued: &Value, salt: f64) -> PassProof {
    let state = fixture
        .flow
        .store()
        .with_session(session, Duration::ZERO, |state| state.clone())
        .unwrap();
    PassProof {
        bucket: state.bucket,
        challenge_nonce: issued["challengeNonce"].as_str().unwrap().to_owned(),
        offsets: pass::solution_offsets(
            KEY,
            session,
            state.bucket,
            state.instance,
            state.difficulty as i32,
        ),
        key_durs: vec![131.4 + salt, 88.9, 210.2, 123.7],
        ..PassProof::default()
    }
}

fn submit(fixture: &Fixture, session: &str, proof: &PassProof, context: PassContext<'_>) -> Value {
    let body = serde_json::to_vec(proof).unwrap();
    let response = call(fixture, "POST", SOLVE_PATH, Some(session), &body, context);
    assert_eq!(response.status, 200);
    json_of(&response)
}

// --- Go pass_handler_test.go ports -----------------------------------------

#[test]
fn pow_instance_binding() {
    let bucket = START.as_secs() / proof_of_work::WINDOW;
    let challenge =
        proof_of_work::issue(KEY, &pow_session("sess-xyz", "instance-aaaa"), 12, bucket);
    let solution = proof_of_work::solve(&challenge.seed, 12, 1 << 24).unwrap();
    assert!(proof_of_work::verify(
        KEY,
        &pow_session("sess-xyz", "instance-aaaa"),
        12,
        bucket,
        bucket,
        &solution
    ));
    assert!(!proof_of_work::verify(
        KEY,
        &pow_session("sess-xyz", "instance-bbbb"),
        12,
        bucket,
        bucket,
        &solution
    ));
    assert_eq!(pow_session("s", "n"), "s|pass|n");
}

#[test]
fn pass_signals_are_registered_with_legacy_weights() {
    for (id, weight) in [
        (SOLVED_SIGNAL, 0.0),
        (VELOCITY_SIGNAL, 25.0),
        (FLOOD_SIGNAL, 45.0),
    ] {
        assert_eq!(lookup(id).unwrap().weight, weight, "{id}");
    }
    for id in BEHAVIOR_SIGNALS {
        assert!(lookup(id).is_some(), "{id} must already be registered");
    }
    let solved = solved_signal();
    assert_eq!(
        (solved.verdict, solved.collected, solved.value),
        (Verdict::Ok, Source::Server, Value::Bool(true))
    );
    assert!(velocity_signal(Level::Normal).is_none());
    let soft = velocity_signal(Level::Soft).unwrap();
    assert_eq!(
        (soft.id.as_str(), soft.verdict),
        (VELOCITY_SIGNAL, Verdict::Suspicious)
    );
    let hard = velocity_signal(Level::Hard).unwrap();
    assert_eq!(
        (hard.id.as_str(), hard.verdict),
        (FLOOD_SIGNAL, Verdict::Bot)
    );
}

#[test]
fn velocity_meter_thresholds() {
    let meter = Meter::new(VELOCITY_WINDOW, VELOCITY_SOFT, VELOCITY_HARD);
    let now = Duration::from_secs(5);
    let levels: Vec<Level> = (0..8).map(|_| meter.observe("k", now).unwrap()).collect();
    assert_eq!(
        levels[1],
        Level::Normal,
        "a human's 2nd attempt stays normal"
    );
    assert_eq!(levels[3], Level::Soft, "4th observation is elevated");
    assert_eq!(levels[7], Level::Hard, "8th observation is flood");
    // The window self-clears: no lockout.
    assert_eq!(
        meter.observe("k", now + VELOCITY_WINDOW).unwrap(),
        Level::Normal
    );
}

#[test]
fn attest_issuance_budget() {
    let meter = Meter::new(ATTEST_WINDOW, ATTEST_SOFT, ATTEST_HARD);
    let now = Duration::from_secs(5);
    assert_ne!(meter.observe("fp", now).unwrap(), Level::Hard);
    assert_ne!(meter.observe("fp", now).unwrap(), Level::Hard);
    assert_eq!(meter.observe("fp", now).unwrap(), Level::Hard);
}

#[test]
fn behavior_timing_tells() {
    let human = [137.3, 89.7, 211.4, 123.8];
    let machine = [7.0, 5.0, 9.0, 6.0];
    assert!(!all_integer_ms(&human));
    assert!(all_integer_ms(&machine));
    assert!(mean(&human) >= 15.0);
    assert!(mean(&machine) < 15.0);
    assert!(stddev(&human) >= 6.0);
    assert!(stddev(&machine) < 6.0);
    assert!(!all_integer_ms(&[]));
    assert_eq!((mean(&[]), stddev(&[])), (0.0, 0.0));
}

// --- pass_scoring.go ---------------------------------------------------------

#[test]
fn behavior_signals_are_soft_deduplicated_and_ordered() {
    let human = PassProof {
        key_durs: vec![137.3, 89.7, 211.4, 123.8],
        durations: vec![16.7, 17.2, 15.9, 18.3, 16.1],
        ..PassProof::default()
    };
    assert!(behavior_signals(&human).is_empty());
    let machine = PassProof {
        key_durs: vec![7.0, 5.0, 9.0],
        durations: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        pointer_type: "touch".into(),
        ..PassProof::default()
    };
    let signals = behavior_signals(&machine);
    let ids: Vec<&str> = signals.iter().map(|signal| signal.id.as_str()).collect();
    assert_eq!(ids, BEHAVIOR_SIGNALS);
    assert_eq!(signals[0].verdict, Verdict::Bot);
    assert_eq!(signals[0].confidence, 0.7);
    assert_eq!(signals[2].notes, "zero sub-ms jitter (synthetic timing)");
    let pointer_only = PassProof {
        durations: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        ..PassProof::default()
    };
    assert_eq!(
        behavior_signals(&pointer_only)[0].notes,
        "zero sub-ms jitter (synthetic pointer timing)"
    );
    let jittery_phone = PassProof {
        pressures: vec![0.4],
        motion: vec![0.11, 0.19, 0.07],
        ..PassProof::default()
    };
    assert!(behavior_signals(&jittery_phone).is_empty());
}

#[test]
fn difficulty_label_and_fingerprint_match_go() {
    for (risk, want) in [
        (0.0, 0),
        (30.0, 0),
        (34.9, 0),
        (35.0, 1),
        (45.0, 1),
        (60.0, 2),
        (75.0, 3),
        (100.0, 3),
        (f64::NAN, 0),
    ] {
        assert_eq!(difficulty(None, risk), want, "risk {risk}");
    }
    assert_eq!(difficulty(Some("bezier"), 0.0), 3);
    assert_eq!(difficulty(Some(""), 0.0), 0);
    assert_eq!(pow_difficulty(0, Level::Normal), 14);
    assert_eq!(pow_difficulty(1, Level::Soft), 17);
    assert_eq!(pow_difficulty(3, Level::Hard), 20);
    assert_eq!(label(80.0, Some("bezier")), "bezier");
    assert_eq!(label(70.0, None), "bot");
    assert_eq!(label(29.9, Some("")), "human");
    assert_eq!(label(30.0, None), "unknown");
    assert!(is_bot_label("bot") && is_bot_label("bezier"));
    assert!(!is_bot_label("human") && !is_bot_label("unknown"));
    assert_eq!(fingerprint("t13d", "203.0.113.9"), "t13d|203.0.113");
    assert_eq!(fingerprint("", "2001:db8::1"), "|2001:db8::1");
}

// --- pass_handler.go flow ----------------------------------------------------

#[test]
fn page_serves_pass_html_and_issues_a_session() {
    let fixture = fixture(false);
    let response = call(&fixture, "GET", PAGE_PATH, None, b"", FP);
    assert_eq!(response.status, 200);
    let Body::File(path) = &response.body else {
        panic!("expected the pass page");
    };
    assert_eq!(path, &web_dir().join("pass.html"));
    assert!(path.is_file());
    let set = response.header("Set-Cookie").unwrap();
    let session = set
        .strip_prefix("hsid=")
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    assert_eq!(*fixture.reports.ensured.lock().unwrap(), [session]);
    let again = call(&fixture, "GET", PAGE_PATH, Some("abc"), b"", FP);
    assert!(again.header("Set-Cookie").is_none());
    let other = Request {
        path: "/api/other",
        ..Request::default()
    };
    assert!(fixture.flow.handle(&other, &FP).is_none());
}

#[test]
fn new_issues_a_fresh_instance_every_call() {
    let fixture = fixture(false);
    let first = new_instance(&fixture, "sess-a");
    assert_eq!(first["ok"], true);
    assert_eq!(first["difficulty"], 0);
    assert_eq!(first["triesLeft"], PASS_MAX_TRIES);
    assert_eq!(first["expiresInS"], 60);
    assert_eq!(first["bucket"], START.as_secs() / PASS_WINDOW_SECS);
    assert_eq!(first["challenge"]["n"], 9);
    assert_eq!(first["challengeNonce"].as_str().unwrap().len(), 36);
    assert_eq!(first["preflight"]["pow"]["difficulty"], 14);
    assert_eq!(first["preflight"]["attestRequired"], false);
    let second = new_instance(&fixture, "sess-a");
    assert_ne!(first["challengeNonce"], second["challengeNonce"]);
    assert_ne!(
        first["preflight"]["pow"]["seed"],
        second["preflight"]["pow"]["seed"]
    );
    let state = fixture
        .flow
        .store()
        .with_session("sess-a", Duration::ZERO, |state| state.clone())
        .unwrap();
    assert_eq!(state.instance, 2);
    // A cookie-less call issues a session.
    let fresh = call(&fixture, "GET", NEW_PATH, None, b"", FP);
    assert!(fresh.header("Set-Cookie").is_some());
}

#[test]
fn new_scales_difficulty_with_risk_and_red_team() {
    let fixture = fixture(false);
    fixture.reports.set_risk("risky", 61.0);
    let issued = new_instance(&fixture, "risky");
    assert_eq!(issued["difficulty"], 2);
    assert_eq!(issued["challenge"]["n"], 13);
    assert_eq!(issued["preflight"]["pow"]["difficulty"], 16);
    let red = call(
        &fixture,
        "GET",
        NEW_PATH,
        Some("red"),
        b"",
        PassContext {
            redteam: Some("bezier"),
            ..FP
        },
    );
    assert_eq!(json_of(&red)["difficulty"], 3);
}

#[test]
fn solve_clears_and_grants_the_trust_upgrade() {
    let fixture = fixture(true);
    let issued = new_instance(&fixture, "sess-ok");
    assert!(pay_pow(&fixture, "sess-ok", &issued));
    let proof = proof(&fixture, "sess-ok", &issued, 0.0);
    let out = submit(&fixture, "sess-ok", &proof, FP);
    assert_eq!(out["ok"], true, "{out}");
    let stored = fixture.reports.get("sess-ok");
    assert_eq!(out["verdict"], stored.scoring.verdict.as_str());
    assert_eq!(out["riskScore"], stored.scoring.risk_score);
    assert!(
        fixture
            .reports
            .signal_ids("sess-ok")
            .contains(&SOLVED_SIGNAL.to_owned())
    );
    let receipt = out["stepUpReceipt"].as_str().unwrap();
    assert!(
        fixture
            .step_up
            .verify_receipt(receipt, "sess-ok", START.as_secs())
            .is_ok()
    );
    assert_eq!(*fixture.fed.lock().unwrap(), [true]);
    let metrics = fixture.flow.store().metrics();
    assert_eq!(
        metrics
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>(),
        [("human|pass|d0".to_owned(), 1)]
    );
    let (events, _) = fixture.hub.subscribe(0);
    assert_eq!(events.last().unwrap().kind, EVENT_ATTEMPT);
    assert!(events.last().unwrap().data.contains("\"solved\":true"));
    // Re-poll re-issues a receipt without re-solving.
    let again = submit(&fixture, "sess-ok", &proof, FP);
    assert_eq!(again["alreadySolved"], true);
    assert!(again["stepUpReceipt"].is_string());
}

#[test]
fn standalone_solve_omits_the_receipt() {
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "solo");
    assert!(pay_pow(&fixture, "solo", &issued));
    let out = submit(&fixture, "solo", &proof(&fixture, "solo", &issued, 0.0), FP);
    assert_eq!(out["ok"], true);
    assert!(out.get("stepUpReceipt").is_none());
    let again = submit(&fixture, "solo", &proof(&fixture, "solo", &issued, 0.0), FP);
    assert_eq!(again, json!({"ok": true, "alreadySolved": true}));
}

#[test]
fn solve_requires_the_instance_nonce_and_the_crypto_axis() {
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "sess-b");
    let mut stale = proof(&fixture, "sess-b", &issued, 0.0);
    stale.challenge_nonce = "a-different-nonce".into();
    let out = submit(&fixture, "sess-b", &stale, FP);
    assert_eq!(out["reason"], "stale or missing challenge");
    assert_eq!(out["triesLeft"], 3);
    let unpaid = proof(&fixture, "sess-b", &issued, 0.0);
    let out = submit(&fixture, "sess-b", &unpaid, FP);
    assert_eq!(out["reason"], "no cryptographic preflight");
    assert_eq!(*fixture.fed.lock().unwrap(), [false, false]);
    // A PoW for another instance does not carry over.
    let replacement = new_instance(&fixture, "sess-b");
    let body = json!({"bucket": 0, "nonce": "x", "challengeNonce": issued["challengeNonce"]});
    let response = call(
        &fixture,
        "POST",
        POW_PATH,
        Some("sess-b"),
        body.to_string().as_bytes(),
        FP,
    );
    assert_eq!(json_of(&response)["ok"], false);
    assert!(pay_pow(&fixture, "sess-b", &replacement));
}

#[test]
fn retry_cap_requires_a_new_instance() {
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "sess-c");
    let mut wrong = proof(&fixture, "sess-c", &issued, 0.0);
    wrong.challenge_nonce = "stale".into();
    for left in (0..PASS_MAX_TRIES).rev() {
        assert_eq!(submit(&fixture, "sess-c", &wrong, FP)["triesLeft"], left);
    }
    let capped = submit(&fixture, "sess-c", &wrong, FP);
    assert_eq!(
        capped,
        json!({"ok": false, "reason": "retry cap reached", "triesLeft": 0})
    );
    new_instance(&fixture, "sess-c");
    let reset = fixture
        .flow
        .store()
        .with_session("sess-c", Duration::ZERO, |state| state.tries)
        .unwrap();
    assert_eq!(reset, 0);
}

#[test]
fn misaligned_and_expired_are_reported_honestly() {
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "sess-d");
    assert!(pay_pow(&fixture, "sess-d", &issued));
    let mut wrong = proof(&fixture, "sess-d", &issued, 0.0);
    wrong.offsets = wrong.offsets.iter().map(|offset| offset + 1).collect();
    let out = submit(&fixture, "sess-d", &wrong, FP);
    assert_eq!(out["reason"], "the keys are not all in the slot");
    assert_eq!(out["triesLeft"], 3);
    // Go does not feed a misaligned attempt.
    assert!(fixture.fed.lock().unwrap().is_empty());
    fixture.clock.advance(Duration::from_secs(
        PASS_WINDOW_SECS * (pass::MAX_BUCKET_AGE + 1),
    ));
    let late = submit(
        &fixture,
        "sess-d",
        &proof(&fixture, "sess-d", &issued, 0.0),
        FP,
    );
    assert_eq!(late["expired"], true);
    assert_eq!(late["ok"], false);
    assert!(late.get("triesLeft").is_none());
    let metrics = fixture.flow.store().metrics();
    assert_eq!(
        metrics.values().sum::<u64>(),
        1,
        "expiry is not counted as a failure"
    );
}

#[test]
fn a_captured_trace_is_admissible_once() {
    let fixture = fixture(false);
    let mut traces = Vec::new();
    for session in ["sess-e", "sess-f"] {
        let issued = new_instance(&fixture, session);
        assert!(pay_pow(&fixture, session, &issued));
        traces.push(proof(&fixture, session, &issued, 0.0));
    }
    assert_eq!(submit(&fixture, "sess-e", &traces[0], FP)["ok"], true);
    let replay = submit(&fixture, "sess-f", &traces[1], FP);
    assert_eq!(replay["reason"], "duplicate interaction trace");
    let mut huge = traces[1].clone();
    huge.key_durs = vec![1e300];
    assert_eq!(
        submit(&fixture, "sess-f", &huge, FP)["reason"],
        "invalid interaction trace"
    );
}

#[test]
fn machine_like_keyboard_trace_still_clears_but_raises_risk() {
    // realEventOK is not ported: a sparse, fast, regular keyboard-only trace
    // (as some switch and assistive devices produce) is never rejected for it.
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "sess-at");
    assert!(pay_pow(&fixture, "sess-at", &issued));
    let mut proof = proof(&fixture, "sess-at", &issued, 0.0);
    proof.key_durs = vec![7.0, 5.0, 9.0, 6.0];
    proof.trusted = false;
    let out = submit(&fixture, "sess-at", &proof, FP);
    assert_eq!(out["ok"], true, "{out}");
    let ids = fixture.reports.signal_ids("sess-at");
    for id in [
        "l4.key.machine_speed",
        "l4.key.flight_std",
        "l4.event.perfect_timing",
        SOLVED_SIGNAL,
    ] {
        assert!(ids.contains(&id.to_owned()), "{id} in {ids:?}");
    }
}

#[test]
fn velocity_raises_risk_once_per_level_and_requires_attestation() {
    let fixture = fixture(false);
    for _ in 0..3 {
        new_instance(&fixture, "fast");
    }
    assert!(fixture.reports.signal_ids("fast").is_empty());
    let fourth = new_instance(&fixture, "fast");
    assert_eq!(fourth["preflight"]["attestRequired"], true);
    assert_eq!(fourth["preflight"]["pow"]["difficulty"], 14 + 2);
    new_instance(&fixture, "fast");
    assert_eq!(fixture.reports.signal_ids("fast"), [VELOCITY_SIGNAL]);
    for _ in 0..3 {
        new_instance(&fixture, "fast");
    }
    assert_eq!(
        fixture.reports.signal_ids("fast"),
        [VELOCITY_SIGNAL, FLOOD_SIGNAL]
    );
    let stored = fixture.reports.get("fast");
    assert!(stored.scoring.risk_score > 0.0);
    // Window self-clears; the session-level flag decays with it.
    fixture.clock.advance(VELOCITY_WINDOW);
    let calm = new_instance(&fixture, "fast");
    assert_eq!(calm["preflight"]["attestRequired"], false);
}

#[test]
fn shared_fingerprint_velocity_taxes_pow_without_requiring_attestation() {
    let fixture = fixture(false);
    for index in 0..3 {
        new_instance(&fixture, &format!("rot-{index}"));
    }
    let fourth = new_instance(&fixture, "rot-3");
    assert_eq!(fourth["preflight"]["attestRequired"], false);
    assert_eq!(fourth["preflight"]["pow"]["difficulty"], 16);
    assert_eq!(fixture.reports.signal_ids("rot-3"), [VELOCITY_SIGNAL]);
}

#[test]
fn attestation_is_bound_budgeted_and_enforced() {
    let fixture = fixture(false);
    for _ in 0..4 {
        new_instance(&fixture, "att");
    }
    let issued = new_instance(&fixture, "att");
    assert_eq!(issued["preflight"]["attestRequired"], true);
    let stale = call(
        &fixture,
        "POST",
        ATTEST_PATH,
        Some("att"),
        br#"{"challengeNonce":"nope"}"#,
        FP,
    );
    assert_eq!(
        json_of(&stale),
        json!({"ok": false, "reason": "stale challenge"})
    );
    let body = json!({"challengeNonce": issued["challengeNonce"]}).to_string();
    let token = json_of(&call(
        &fixture,
        "POST",
        ATTEST_PATH,
        Some("att"),
        body.as_bytes(),
        FP,
    ));
    assert_eq!(token["ok"], true);
    let token = token["token"].as_str().unwrap().to_owned();

    // The solve without the token is refused even with PoW paid. Paying a
    // velocity-raised PoW is slow, so mark the crypto axis directly.
    let mark_paid = || {
        fixture
            .flow
            .store()
            .with_session("att", Duration::ZERO, |state| state.pow_ok = true)
            .unwrap()
    };
    mark_paid();
    let mut proof = proof(&fixture, "att", &issued, 0.0);
    assert_eq!(
        submit(&fixture, "att", &proof, FP)["reason"],
        "attestation required"
    );
    proof.attest_token = token;
    assert_eq!(submit(&fixture, "att", &proof, FP)["ok"], true);

    // Issuance budget per fingerprint: the third issuance is denied.
    let second = json_of(&call(
        &fixture,
        "POST",
        ATTEST_PATH,
        Some("att"),
        body.as_bytes(),
        FP,
    ));
    assert_eq!(second["ok"], true);
    let third = json_of(&call(
        &fixture,
        "POST",
        ATTEST_PATH,
        Some("att"),
        body.as_bytes(),
        FP,
    ));
    assert_eq!(
        third,
        json!({"ok": false, "reason": "attestation budget exhausted — retry shortly"})
    );
    fixture.clock.advance(ATTEST_WINDOW);
    let later = json_of(&call(
        &fixture,
        "POST",
        ATTEST_PATH,
        Some("att"),
        body.as_bytes(),
        FP,
    ));
    assert_eq!(later["ok"], true, "the budget window self-clears");
}

#[test]
fn transport_errors_match_go() {
    let fixture = fixture(false);
    for path in [POW_PATH, ATTEST_PATH, SOLVE_PATH] {
        let get = call(&fixture, "GET", path, Some("s"), b"{}", FP);
        assert_eq!(
            (get.status, get.bytes().unwrap()),
            (405, &b"POST only\n"[..])
        );
        let anonymous = call(&fixture, "POST", path, None, b"{}", FP);
        assert_eq!(
            (anonymous.status, anonymous.bytes().unwrap()),
            (400, &b"no session\n"[..])
        );
        let bad = call(&fixture, "POST", path, Some("s"), b"{", FP);
        assert_eq!(
            (bad.status, bad.bytes().unwrap()),
            (400, &b"bad body\n"[..])
        );
    }
    let oversized = vec![b' '; MAX_SMALL_BODY + 1];
    assert_eq!(
        call(&fixture, "POST", POW_PATH, Some("s"), &oversized, FP).status,
        400
    );
    let long = "x".repeat(200);
    assert_eq!(
        call(&fixture, "GET", NEW_PATH, Some(&long), b"", FP).status,
        400
    );
}

#[test]
fn malformed_red_team_label_is_refused_before_a_try() {
    let fixture = fixture(false);
    let issued = new_instance(&fixture, "lbl");
    let body = serde_json::to_vec(&proof(&fixture, "lbl", &issued, 0.0)).unwrap();
    let response = call(
        &fixture,
        "POST",
        SOLVE_PATH,
        Some("lbl"),
        &body,
        PassContext {
            redteam: Some("bad|label"),
            ..FP
        },
    );
    assert_eq!(response.status, 400);
    let tries = fixture
        .flow
        .store()
        .with_session("lbl", Duration::ZERO, |state| state.tries)
        .unwrap();
    assert_eq!(tries, 0);
}

#[test]
fn kpi_aggregates_bypass_and_human_floor() {
    let store = PassStore::new();
    for (label, difficulty, passed) in [
        ("human", 0, true),
        ("human", 0, false),
        ("bezier", 3, false),
        ("bezier", 3, false),
        ("bezier", 3, true),
        ("bot", 1, false),
        ("unknown", 1, true),
    ] {
        store.record(label, difficulty, passed).unwrap();
    }
    let report = kpi(&store.metrics());
    assert_eq!(report["botAttempts"], 4);
    assert_eq!(report["bypassRate"], 0.25);
    assert_eq!(
        report["perStrategy"],
        json!({"bezier": 1.0 / 3.0, "bot": 0.0})
    );
    assert_eq!(report["humanPassRate"], 0.5);
    assert_eq!(report["humanAttempts"], 2);
    assert_eq!(report["unknownPassRate"], 1.0);
    assert_eq!(
        report["perDifficulty"],
        json!({"d0": 0.5, "d1": 0.5, "d3": 1.0 / 3.0})
    );
    assert_eq!(report["raw"]["bezier|fail|d3"], 2);
    let empty = kpi(&BTreeMap::new());
    assert_eq!(empty["bypassRate"], 0.0);
    assert_eq!(empty["humanAttempts"], 0);
    let fixture = fixture(false);
    let served = call(&fixture, "GET", KPI_PATH, None, b"", FP);
    assert_eq!(json_of(&served)["botAttempts"], 0);
}

#[test]
fn meter_clamps_out_of_order_clock_reads() {
    let meter = Meter::new(VELOCITY_WINDOW, VELOCITY_SOFT, VELOCITY_HARD);
    assert_eq!(
        meter.observe("k", Duration::from_secs(10)).unwrap(),
        Level::Normal
    );
    assert_eq!(
        meter.observe("k", Duration::from_secs(9)).unwrap(),
        Level::Normal
    );
}

#[test]
fn correction_feed_emits_traces_only_for_solves() {
    let traces = Arc::new(Mutex::new(Vec::new()));
    let sink = traces.clone();
    let feed = correction_feed(
        None,
        Arc::new(|_| Prediction::abstained()),
        Arc::new(move |trace| sink.lock().unwrap().push(trace)),
    );
    let report = SessionReport::default();
    feed(&report, false);
    assert!(traces.lock().unwrap().is_empty());
    feed(&report, true);
    let traces = traces.lock().unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!((traces[0].label, traces[0].source), (0, "pass"));
}
