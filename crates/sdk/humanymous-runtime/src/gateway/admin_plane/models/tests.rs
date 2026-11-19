//! Differential port of the legacy internal/mlcorrect/bundle.go lifecycle and
//! the `mlserve` serving seam at baseline 981d25c against outputs recorded
//! from the Go oracle (`tests/fixtures/gateway/ml-bundle-go-golden.json`),
//! driven through the admin routes of owner decision #12, plus pins of the
//! audit-before-mutation rule and the detection freeze.
//!
//! Stage steps call [`Models::stage`] (startup staging). Promote and
//! rollback steps go through `AdminPlane::handle`: promote is requested by
//! the operator and committed by a distinct approver, rollback is applied by
//! the operator, each naming the digest it acts on.
use super::*;
use crate::audit::control::Scope;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_plane::PREFIX;
use crate::gateway::ban::Store;
use crate::gateway::http::Request;
use crate::gateway::model_admission::Ed25519Verifier;
use crate::network::history::rate::{Config, Limits};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use humanymous_core::model::correction::digest;
use humanymous_core::scoring::derivation::behavior_signals;
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use std::io;
use std::sync::LazyLock;
use std::time::Duration;

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/ml-bundle-go-golden.json"
    ))
    .unwrap()
});

const KEY: &[u8] = b"lane-j-models-token-key-0123456789";

/// Records appended by the plane; `fail` makes `config.changed` fail.
#[derive(Clone, Default)]
struct Sink {
    records: Arc<Mutex<Vec<Record>>>,
    fail: Arc<Mutex<bool>>,
}

impl DurableAudit for Sink {
    fn append(&mut self, record: Record) -> io::Result<()> {
        if *self.fail.lock().unwrap() && record.event_type == EVENT {
            return Err(io::Error::other("sink down"));
        }
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}

impl Sink {
    fn changes(&self) -> Vec<(String, String)> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.event_type == EVENT)
            .map(|record| {
                (
                    record.enforcement_action.clone(),
                    record.fail_reason.clone(),
                )
            })
            .collect()
    }
}

fn token(role: Role) -> String {
    format!("{}-token-0123456789", role.as_str())
}

fn plane(models: Option<Models>, sink: &Sink) -> AdminPlane {
    let limits = Limits::new(64, 256, 128, Duration::from_secs(3600), 1 << 20, 64).unwrap();
    let config = Config::new(Duration::from_secs(10), 60, 120, limits).unwrap();
    let bans = Arc::new(Mutex::new(Store::new(config, 64).unwrap()));
    let mut plane = AdminPlane::new(
        "n1",
        Scope {
            installation: "installation".into(),
            node: "n1".into(),
        },
        KEY,
        bans,
        Box::new(sink.clone()),
    );
    if let Some(models) = models {
        plane = plane.with_models(models);
    }
    for (role, id) in [
        (Role::Auditor, "aud-1"),
        (Role::Operator, "op-1"),
        (Role::Approver, "appr-1"),
        (Role::Dpo, "dpo-1"),
    ] {
        plane.auth().add(&token(role), id, role).unwrap();
    }
    plane
}

fn clock(step: u64) -> Clock {
    Clock {
        monotonic: Duration::from_secs(5 + step),
        unix: 1_800_000_000 + step,
    }
}

fn request(method: &str, sub: &str, role: Role, body: &str) -> Request {
    Request {
        method: method.into(),
        target: format!("{PREFIX}{sub}"),
        headers: vec![
            ("Host".to_owned(), "admin".to_owned()),
            ("Authorization".into(), format!("Bearer {}", token(role))),
        ],
        body: body.as_bytes().to_vec(),
    }
}

fn body_json(response: &Response) -> Value {
    serde_json::from_slice(&response.body).unwrap()
}

fn digest_body(digest: &str) -> String {
    json!({ "Digest": digest }).to_string()
}

/// Operator request, then approver commit; returns the commit response or
/// the refusal of the request.
fn promote(plane: &AdminPlane, digest: &str, step: u64) -> Response {
    let requested = plane.handle(
        &request(
            "POST",
            "models/promote",
            Role::Operator,
            &digest_body(digest),
        ),
        clock(step),
    );
    if requested.status != 200 {
        return requested;
    }
    let id = body_json(&requested)["approvalId"]
        .as_str()
        .unwrap()
        .to_owned();
    plane.handle(
        &request("POST", &format!("approvals/{id}"), Role::Approver, ""),
        clock(step),
    )
}

fn rollback(plane: &AdminPlane, digest: &str, step: u64) -> Response {
    plane.handle(
        &request(
            "POST",
            "models/rollback",
            Role::Operator,
            &digest_body(digest),
        ),
        clock(step),
    )
}

fn decoded(map: &str, name: &str) -> Vec<u8> {
    STANDARD
        .decode(GOLDEN[map][name].as_str().unwrap())
        .unwrap()
}

fn features() -> Vec<f32> {
    GOLDEN["features_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bits| f32::from_bits(u32::try_from(bits.as_u64().unwrap()).unwrap()))
        .collect()
}

fn class(error: ModelsError) -> &'static str {
    match error {
        ModelsError::Correction(CorrectionError::Digest) => "digest",
        ModelsError::Correction(CorrectionError::Signature) => "signature",
        ModelsError::Correction(CorrectionError::StaleSchema) => "stale_schema",
        ModelsError::Correction(CorrectionError::InvalidBundle) => "invalid",
        ModelsError::Correction(CorrectionError::NoStaged) => "no_staged",
        ModelsError::Correction(CorrectionError::NoPrevious) => "no_previous",
        other => panic!("unexpected stage error {other:?}"),
    }
}

/// Map a promote or rollback response to Go's result class.
fn route_class(response: &Response) -> &'static str {
    match (
        response.status,
        std::str::from_utf8(&response.body).unwrap(),
    ) {
        (200, _) => "ok",
        (409, "no verified candidate staged\n") => "no_staged",
        (409, "no previous bundle to roll back to\n") => "no_previous",
        (status, body) => panic!("unexpected response {status} {body}"),
    }
}

fn replay(name: &str) {
    let scenario = GOLDEN["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scenario| scenario["name"] == name)
        .unwrap();
    let runtime = Runtime::default();
    let models = if scenario["signed"].as_bool().unwrap() {
        let pem = GOLDEN["public_key_pem"].as_str().unwrap();
        let verifier = Ed25519Verifier::from_pkix_pem(pem.as_bytes()).unwrap();
        Models::signed(Box::new(verifier), runtime.clone())
    } else {
        Models::unsigned(runtime.clone())
    };
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    let features = features();
    for (index, step) in scenario["records"].as_array().unwrap().iter().enumerate() {
        let at = format!("{name} step {index}");
        let step_no = index as u64;
        let result = match step["op"].as_str().unwrap() {
            "stage" => {
                let bundle = decoded("bundles", step["bundle"].as_str().unwrap());
                let pin = step["digest"].as_str().filter(|pin| !pin.is_empty());
                let signature = step["sig"].as_str().map(|name| decoded("signatures", name));
                let models = plane.models.as_ref().unwrap();
                match models.stage(&bundle, pin, signature.as_deref()) {
                    Ok(identity) => {
                        assert_eq!(identity.digest, digest(&bundle), "{at}");
                        "ok"
                    }
                    Err(error) => class(error),
                }
            }
            "promote" => {
                let staged = plane.models.as_ref().unwrap().staged().unwrap();
                match staged {
                    // Go `Promote` with nothing staged: the request is refused.
                    None => route_class(&promote(&plane, "0", step_no)),
                    Some(staged) => route_class(&promote(&plane, &staged.digest, step_no)),
                }
            }
            "rollback" => {
                let active = plane.models.as_ref().unwrap().active().unwrap();
                let digest = active.map(|active| active.digest).unwrap_or_default();
                route_class(&rollback(&plane, &digest, step_no))
            }
            other => panic!("unknown op {other}"),
        };
        assert_eq!(result, step["result"].as_str().unwrap(), "{at}");
        let view =
            body_json(&plane.handle(&request("GET", "models", Role::Auditor, ""), clock(step_no)));
        let staged_ok = step["staged_ok"].as_bool().unwrap();
        if staged_ok {
            assert_eq!(view["staged"]["version"], step["staged_version"], "{at}");
            assert_eq!(view["staged"]["digest"], step["staged_digest"], "{at}");
        } else {
            assert_eq!(view["staged"], Value::Null, "{at}");
        }
        assert_eq!(view["active"]["version"], step["active_version"], "{at}");
        assert_eq!(view["active"]["digest"], step["active_digest"], "{at}");
        assert_eq!(
            runtime.current().bundle_version(),
            step["served_version"].as_str().unwrap(),
            "{at}"
        );
        let prediction = runtime.score(&features);
        assert_eq!(
            prediction.abstain,
            step["score_abstain"].as_bool().unwrap(),
            "{at}"
        );
        assert_eq!(
            u64::from(prediction.p_bot.to_bits()),
            step["score_p_bot_bits"].as_u64().unwrap(),
            "{at}"
        );
    }
}

#[test]
fn golden_covers_the_capture() {
    let names: Vec<_> = GOLDEN["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .map(|scenario| scenario["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["unsigned_verification", "signed_verification", "lifecycle"]
    );
    assert_eq!(
        GOLDEN["schema_hash"].as_str().unwrap(),
        humanymous_core::behavior::features::schema_hash()
    );
    assert_eq!(
        GOLDEN["feature_dim"].as_u64().unwrap(),
        humanymous_core::behavior::features::FEATURE_DIM as u64
    );
}

#[test]
fn golden_unsigned_verification() {
    replay("unsigned_verification");
}

#[test]
fn golden_signed_verification() {
    replay("signed_verification");
}

#[test]
fn golden_lifecycle() {
    replay("lifecycle");
}

fn staged_models(runtime: &Runtime) -> (Models, String) {
    let models = Models::unsigned(runtime.clone());
    let bundle = decoded("bundles", "v1");
    let identity = models.stage(&bundle, None, None).unwrap();
    (models, identity.digest)
}

#[test]
fn staging_never_activates_a_model() {
    let runtime = Runtime::default();
    let (models, _) = staged_models(&runtime);
    assert!(models.staged().unwrap().is_some());
    assert_eq!(models.active().unwrap(), None);
    assert_eq!(runtime.current().bundle_version(), "none");
    assert!(runtime.score(&features()).abstain);
}

#[test]
fn promote_needs_a_distinct_approver_and_records_before_serving() {
    let runtime = Runtime::default();
    let (models, digest) = staged_models(&runtime);
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    let requested = plane.handle(
        &request(
            "POST",
            "models/promote",
            Role::Operator,
            &digest_body(&digest),
        ),
        clock(1),
    );
    assert_eq!(requested.status, 200);
    let pending = body_json(&requested);
    assert_eq!(pending["pending"], true);
    assert_eq!(pending["needsRole"], "approver");
    // Requesting changes nothing.
    assert_eq!(runtime.current().bundle_version(), "none");
    let id = pending["approvalId"].as_str().unwrap();
    // The requester cannot approve its own promotion.
    let own = plane.handle(
        &request("POST", &format!("approvals/{id}"), Role::Operator, ""),
        clock(2),
    );
    assert_eq!(own.status, 403);
    assert_eq!(runtime.current().bundle_version(), "none");
    let committed = plane.handle(
        &request("POST", &format!("approvals/{id}"), Role::Approver, ""),
        clock(3),
    );
    assert_eq!(committed.status, 200);
    let committed = body_json(&committed);
    assert_eq!(committed["committed"], KIND_PROMOTE);
    assert_eq!(committed["active"]["version"], "v1");
    assert_eq!(committed["active"]["digest"], digest.as_str());
    assert_eq!(runtime.current().bundle_version(), "v1");
    assert_eq!(
        sink.changes(),
        [(
            KIND_PROMOTE.to_owned(),
            format!(
                "ml.model.promoted version=v1 digest={digest} (dual-control: op-1\u{2192}appr-1)"
            )
        )]
    );
}

#[test]
fn failed_audit_answers_503_and_changes_nothing() {
    let runtime = Runtime::default();
    let (models, digest) = staged_models(&runtime);
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    *sink.fail.lock().unwrap() = true;
    let refused = promote(&plane, &digest, 1);
    assert_eq!(refused.status, 503);
    assert_eq!(refused.body, b"audit unavailable\n");
    assert_eq!(runtime.current().bundle_version(), "none");
    let models = plane.models.as_ref().unwrap();
    assert_eq!(models.active().unwrap(), None);
    assert_eq!(models.staged().unwrap().unwrap().digest, digest);

    *sink.fail.lock().unwrap() = false;
    assert_eq!(promote(&plane, &digest, 2).status, 200);
    assert_eq!(runtime.current().bundle_version(), "v1");
    *sink.fail.lock().unwrap() = true;
    let refused = rollback(&plane, &digest, 3);
    assert_eq!(refused.status, 503);
    assert_eq!(runtime.current().bundle_version(), "v1");
    assert_eq!(models.active().unwrap().unwrap().digest, digest);
    *sink.fail.lock().unwrap() = false;
    let rolled = rollback(&plane, &digest, 4);
    assert_eq!(rolled.status, 200);
    assert_eq!(body_json(&rolled)["active"]["version"], "none");
    assert_eq!(runtime.current().bundle_version(), "none");
    let changes = sink.changes();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[1].0, "ml.model.rollback");
    assert_eq!(
        changes[1].1,
        format!("ml.model.rolled_back from version=v1 digest={digest} to none (abstain)")
    );
}

#[test]
fn poisoned_lifecycle_answers_503_and_records_nothing() {
    let runtime = Runtime::default();
    let (models, digest) = staged_models(&runtime);
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    let requested = plane.handle(
        &request(
            "POST",
            "models/promote",
            Role::Operator,
            &digest_body(&digest),
        ),
        clock(1),
    );
    assert_eq!(requested.status, 200);
    let id = body_json(&requested)["approvalId"]
        .as_str()
        .unwrap()
        .to_owned();
    let models = plane.models.as_ref().unwrap();
    // A transition that panics while holding the lifecycle lock.
    std::thread::scope(|scope| {
        let poisoned = scope.spawn(|| {
            let _held = models.slots.lock().unwrap();
            panic!("transition panicked");
        });
        assert!(poisoned.join().is_err());
    });
    assert!(models.slots.is_poisoned());
    let refused = [
        plane.handle(&request("GET", "models", Role::Auditor, ""), clock(2)),
        plane.handle(
            &request("POST", &format!("approvals/{id}"), Role::Approver, ""),
            clock(3),
        ),
        plane.handle(
            &request(
                "POST",
                "models/promote",
                Role::Operator,
                &digest_body(&digest),
            ),
            clock(4),
        ),
        rollback(&plane, "", 5),
    ];
    for response in &refused {
        assert_eq!(response.status, 503);
        assert_eq!(response.body, b"model lifecycle unavailable\n");
    }
    assert_eq!(
        models.stage(&decoded("bundles", "v2"), None, None),
        Err(ModelsError::Unavailable)
    );
    assert_eq!(models.staged(), Err(ModelsError::Unavailable));
    assert_eq!(models.active(), Err(ModelsError::Unavailable));
    assert_eq!(runtime.current().bundle_version(), "none");
    assert!(sink.changes().is_empty());
}

#[test]
fn digest_mismatch_is_a_conflict_at_request_and_at_commit() {
    let runtime = Runtime::default();
    let (models, digest) = staged_models(&runtime);
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    let wrong = plane.handle(
        &request(
            "POST",
            "models/promote",
            Role::Operator,
            &digest_body(&"0".repeat(64)),
        ),
        clock(1),
    );
    assert_eq!(wrong.status, 409);
    assert_eq!(
        wrong.body,
        b"CAS: digest does not match the staged candidate\n"
    );
    let empty = plane.handle(
        &request("POST", "models/promote", Role::Operator, "{}"),
        clock(1),
    );
    assert_eq!(empty.status, 400);
    assert_eq!(empty.body, b"digest required\n");
    let bad = plane.handle(
        &request("POST", "models/promote", Role::Operator, "{\"Digest\":1}"),
        clock(1),
    );
    assert_eq!(bad.status, 400);

    // A new candidate staged between request and commit invalidates it.
    let requested = plane.handle(
        &request(
            "POST",
            "models/promote",
            Role::Operator,
            &digest_body(&digest),
        ),
        clock(2),
    );
    let id = body_json(&requested)["approvalId"]
        .as_str()
        .unwrap()
        .to_owned();
    let models = plane.models.as_ref().unwrap();
    models.stage(&decoded("bundles", "v2"), None, None).unwrap();
    let stale = plane.handle(
        &request("POST", &format!("approvals/{id}"), Role::Approver, ""),
        clock(3),
    );
    assert_eq!(stale.status, 409);
    assert_eq!(runtime.current().bundle_version(), "none");
    assert!(sink.changes().is_empty());

    // Rollback names the active digest; nothing active means "".
    let rolled = rollback(&plane, &digest, 4);
    assert_eq!(rolled.status, 409);
    assert_eq!(
        rolled.body,
        b"CAS: digest does not match the active model\n"
    );
    let nothing = rollback(&plane, "", 5);
    assert_eq!(nothing.status, 409);
    assert_eq!(nothing.body, b"no previous bundle to roll back to\n");
    assert!(sink.changes().is_empty());
}

#[test]
fn roles_and_unattached_plane() {
    let sink = Sink::default();
    let bare = plane(None, &sink);
    for (method, sub) in [
        ("GET", "models"),
        ("POST", "models/promote"),
        ("POST", "models/rollback"),
    ] {
        let response = bare.handle(&request(method, sub, Role::Operator, "{}"), clock(1));
        assert_eq!(response.status, 404, "{method} {sub}");
    }
    let runtime = Runtime::default();
    let (models, digest) = staged_models(&runtime);
    let plane = plane(Some(models), &sink);
    let view = plane.handle(&request("GET", "models", Role::Auditor, ""), clock(1));
    assert_eq!(view.status, 200);
    assert_eq!(
        body_json(&view),
        json!({
            "active": {"version": "none", "digest": ""},
            "previous": null,
            "signed": false,
            "staged": {"version": "v1", "digest": digest},
        })
    );
    for sub in ["models/promote", "models/rollback"] {
        let response = plane.handle(
            &request("POST", sub, Role::Auditor, &digest_body(&digest)),
            clock(1),
        );
        assert_eq!(response.status, 403, "{sub}");
        assert_eq!(response.body, b"role auditor not permitted\n");
    }
    assert_eq!(runtime.current().bundle_version(), "none");
}

#[test]
fn activation_keeps_every_score_and_verdict() {
    let engine = Engine::default();
    let (models, digest) = staged_models(&engine.model);
    let sink = Sink::default();
    let plane = plane(Some(models), &sink);
    let reports: Vec<SessionReport> = [
        json!({"client": {"userAgent": "Mozilla/5.0 (X11; Linux x86_64) Chrome/131.0",
            "behavior": {"mouse": {"samples": 40, "meanCurvature": 0.02, "velocityStdDev": 0.4,
                "accelEntropy": 2.1, "straightLineFrac": 0.2, "pauseCount": 3},
                "key": {"keystrokes": 12, "meanDwellMs": 90, "dwellStdDevMs": 20,
                "meanFlightMs": 140, "flightStdDevMs": 45},
                "events": {"totalEvents": 80, "clickCount": 3, "minReactionMs": 260},
                "durationS": 9.5}}}),
        json!({"client": {"userAgent": "Mozilla/5.0 (X11; Linux x86_64) Chrome/131.0",
            "behavior": {"mouse": {"samples": 0, "maxTeleportPx": 400},
                "events": {"totalEvents": 4, "noMoveClicks": 2, "clickCount": 2},
                "durationS": 3.0}}}),
        json!({}),
    ]
    .into_iter()
    .map(|value| serde_json::from_value(value).unwrap())
    .collect();
    // (result, trace with zero-probability layer rows set aside, those rows)
    let score = |report: &SessionReport| {
        let mut report = report.clone();
        let (result, trace) = engine.score_with_trace(&mut report).unwrap();
        let mut trace = serde_json::to_value(trace).unwrap();
        let layers = trace["perLayer"].take();
        let (zero, live): (Vec<Value>, Vec<Value>) = layers
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .partition(|row| row["rawProb"] == json!(0.0));
        trace["perLayer"] = Value::Array(live);
        (serde_json::to_value(result).unwrap(), trace, zero)
    };
    // How many reports carry the model's `l4.ml.behavioral` observation.
    let observed = || {
        reports
            .iter()
            .filter(|report| {
                behavior_signals(&report.client.behavior, &engine.model)
                    .iter()
                    .any(|signal| signal.id == "l4.ml.behavioral")
            })
            .count()
    };
    assert_eq!(observed(), 0, "a model ran before the promote");
    let before: Vec<_> = reports.iter().map(score).collect();
    assert_eq!(promote(&plane, &digest, 1).status, 200);
    assert_eq!(engine.model.current().bundle_version(), "v1");
    assert!(!engine.model.score(&features()).abstain);
    // The freeze below is not vacuous: the promoted model ran on the scored
    // reports and its observation reached the explanation as an L4 row.
    assert!(observed() > 0, "the promoted model never ran");
    let after: Vec<_> = reports.iter().map(score).collect();
    let l4_rows = |rows: &Vec<Value>| rows.iter().filter(|row| row["layer"] == "L4").count();
    let l4_before: usize = before.iter().map(|(_, _, zero)| l4_rows(zero)).sum();
    let l4_after: usize = after.iter().map(|(_, _, zero)| l4_rows(zero)).sum();
    assert!(
        l4_after > l4_before,
        "no L4 row from the promoted model: {after:?}"
    );
    for ((result, trace, _), (result_after, trace_after, zero_after)) in before.iter().zip(&after) {
        // Risk, verdict, hard rule, contributors and every hard-rule match
        // are unchanged.
        assert_eq!(
            result, result_after,
            "activation changed a score or verdict"
        );
        assert_eq!(trace, trace_after, "activation changed the explanation");
        // The weight-0 annotation may add an L4 layer row of probability 0
        // to the explanation (legacy: "a live model shifts only the audit
        // trace, never a verdict").
        for row in zero_after {
            assert_eq!(row["cappedProb"], json!(0.0));
        }
    }
    // The model's only output is the existing weight-0 annotation.
    let definition = humanymous_core::signals::observation::lookup("l4.ml.behavioral").unwrap();
    assert_eq!(definition.weight, 0.0);
    assert_eq!(engine.model.fire_threshold(), 0.5);
}
