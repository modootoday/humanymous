//! Port of legacy cmd/server/mlcorrect_ops_test.go `TestHandleMLCorrect_GateAndShape`
//! (baseline 981d25c), plus byte goldens observed 2026-11-18 by calling the Go
//! oracle's `handleMLCorrect` (go1.25.3 linux/amd64) with the same controllers.
use super::*;
use humanymous_core::model::correction::{CanaryBudget, Outcome};

const BEARER: Option<&str> = Some("Bearer secret");

fn text(response: &Response) -> &str {
    std::str::from_utf8(&response.body).unwrap()
}

#[test]
fn handle_ml_correct_gate_and_shape() {
    // unauthorized -> 404 (non-discoverable)
    let controller = Controller::new(0.01, 0.5, 0.01).unwrap();
    let response = handle("secret", None, Some(&controller), None);
    assert_eq!(response.status, 404, "missing bearer must 404");
    assert_eq!(response.body, NOT_FOUND_BODY);
    assert_eq!(response.content_type, "text/plain; charset=utf-8");

    // authorized + controller -> enabled snapshot
    let response = handle("secret", BEARER, Some(&controller), None);
    assert_eq!(response.status, 200);
    assert!(text(&response).contains(r#""enabled":true"#));
    assert!(text(&response).contains("fireThreshold"));

    // authorized but no controller -> disabled marker, not 404
    let response = handle("secret", BEARER, None, None);
    assert_eq!(response.status, 200);
    assert!(text(&response).contains(r#""enabled":false"#));
}

#[test]
fn fresh_and_disabled_documents_match_go_bytes() {
    let controller = Controller::new(0.01, 0.5, 0.01).unwrap();
    let response = handle("secret", BEARER, Some(&controller), None);
    assert_eq!(response.content_type, "application/json");
    assert_eq!(
        text(&response),
        concat!(
            r#"{"activeBundle":{"digest":"","version":""},"#,
            r#""calibration":{"Theta":0.5,"Budget":0.01,"HumanFPEst":0.01,"HumansSeen":0,"BotsSeen":0},"#,
            r#""canary":{"state":"off","humansIntoProbation":0,"probationHumans":0},"#,
            r#""drift":{"Fired":false,"MimicAlarm":false,"OracleAlarm":false,"CovariateShift":false,"MaxPSI":0},"#,
            r#""enabled":true,"fireThreshold":0.5,"passAnomalous":false,"passSolveRate":0.6,"#,
            r#""shadow":{"n":0,"bothScored":0,"meanAbsDelta":0,"fireAgreement":0,"shadowFiresMore":0,"activeFiresMore":0,"shadowAbstained":0}}"#,
            "\n"
        )
    );
    assert_eq!(
        text(&handle("secret", BEARER, None, None)),
        "{\"enabled\":false,\"reason\":\"no -ml-bundle loaded; l4.ml.behavioral abstains\"}\n"
    );
}

#[test]
fn tripped_canary_with_bundle_manager_matches_go_bytes() {
    let controller = Controller::new(0.005, 0.5, 0.0).unwrap();
    controller
        .arm_canary(
            CanaryBudget {
                max_human_fp: 0.10,
                probation_humans: 100_000,
                ..CanaryBudget::default()
            },
            None,
        )
        .unwrap();
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.9, false)
            .unwrap();
    }
    for i in 0..10 {
        controller.observe_pass(i % 3 == 0);
    }
    let bundles = BundleManager::unsigned_lab();
    assert_eq!(
        text(&handle("secret", BEARER, Some(&controller), Some(&bundles))),
        concat!(
            r#"{"activeBundle":{"digest":"","version":"none"},"#,
            r#""calibration":{"Theta":0.8999963,"Budget":0.005,"HumanFPEst":0.0007593573,"HumansSeen":400,"BotsSeen":0},"#,
            r#""canary":{"state":"tripped","reason":"human-FP 0.3357 over canary budget 0.1000","humansIntoProbation":400,"probationHumans":100000},"#,
            r#""drift":{"Fired":false,"MimicAlarm":false,"OracleAlarm":false,"CovariateShift":false,"MaxPSI":0},"#,
            r#""enabled":true,"fireThreshold":0.8999963,"passAnomalous":false,"passSolveRate":0.5808823,"#,
            r#""shadow":{"n":0,"bothScored":0,"meanAbsDelta":0,"fireAgreement":0,"shadowFiresMore":0,"activeFiresMore":0,"shadowAbstained":0}}"#,
            "\n"
        )
    );
}

#[test]
fn anomalous_pass_rate_matches_go_bytes() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    for _ in 0..300 {
        controller.observe_pass(true);
        controller
            .observe_outcome(Outcome::PassSolved, 0.9, false)
            .unwrap();
    }
    assert_eq!(
        text(&handle("secret", BEARER, Some(&controller), None)),
        concat!(
            r#"{"activeBundle":{"digest":"","version":""},"#,
            r#""calibration":{"Theta":0.8999931,"Budget":0.01,"HumanFPEst":0.0035538946,"HumansSeen":300,"BotsSeen":0},"#,
            r#""canary":{"state":"off","humansIntoProbation":0,"probationHumans":0},"#,
            r#""drift":{"Fired":false,"MimicAlarm":false,"OracleAlarm":false,"CovariateShift":false,"MaxPSI":0},"#,
            r#""enabled":true,"fireThreshold":0.8999931,"passAnomalous":true,"passSolveRate":0.98038363,"#,
            r#""shadow":{"n":0,"bothScored":0,"meanAbsDelta":0,"fireAgreement":0,"shadowFiresMore":0,"activeFiresMore":0,"shadowAbstained":0}}"#,
            "\n"
        )
    );
}

#[test]
fn operator_gate_follows_legacy_predicate() {
    // Unconfigured: never authorized, even with an empty presented token.
    assert!(!operator_authorized("", Some("Bearer ")));
    assert!(!operator_authorized("", None));
    assert!(operator_authorized("secret", Some("Bearer secret")));
    assert!(!operator_authorized("secret", Some("Bearer wrong")));
    assert!(!operator_authorized("secret", Some("Bearer  secret")));
    assert!(!operator_authorized("secret", Some("bearer secret")));
    assert!(!operator_authorized("secret", Some("Bearer secret2")));
    // Go trimmed an optional prefix: the bare token matched too.
    assert!(operator_authorized("secret", Some("secret")));
    let disabled = handle("", Some("Bearer "), None, None);
    assert_eq!(disabled.status, 404);
}
