//! Source-only correction tests. Frozen bits are analytic/source-derived;
//! mock authority adapters are explicitly NOT cryptographic qualification.
use super::*;
use super::{numeric::*, features::FeatureMonitor, lifecycle::*, controller::Controller};
use super::gates::{assess, temporal_ids, Policy};
use crate::model::inference::{MlpBundle, Prediction};
use sha2::{Digest, Sha256};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/model/correction-numeric.json"))).unwrap()
}
fn f32_bits(value: &serde_json::Value, key: &str, actual: f32) {
    if let Some(expected) = value[key].as_str() { assert_eq!(actual.to_bits(), u32::from_str_radix(expected, 16).unwrap()); }
}
fn f64_bits(value: &serde_json::Value, key: &str, actual: f64) {
    if let Some(expected) = value[key].as_str() { assert_eq!(actual.to_bits(), u64::from_str_radix(expected, 16).unwrap()); }
}

#[test]
fn frozen_source_derived_aci_bits_and_bot_noop() {
    let f = fixture(); let a = &f["aci"];
    let mut controller = Calibration::new(a["budget"].as_f64().unwrap() as f32,
        a["theta"].as_f64().unwrap() as f32, a["gamma"].as_f64().unwrap() as f32).unwrap();
    for observation in a["observations"].as_array().unwrap() {
        controller.observe(observation["human"].as_bool().unwrap(), observation["p"].as_f64().unwrap() as f32).unwrap();
        f32_bits(observation, "theta_bits", controller.theta);
        f32_bits(observation, "fp_ema_bits", controller.fp_ema);
        assert_eq!(controller.humans, observation["humans"].as_u64().unwrap());
        assert_eq!(controller.bots, observation["bots"].as_u64().unwrap());
    }
}

#[test]
fn frozen_page_hinkley_order_and_reset() {
    let f = fixture(); let p = &f["page_hinkley"];
    let mut detector = PageHinkley::new(p["delta"].as_f64().unwrap(), p["lambda"].as_f64().unwrap()).unwrap();
    for observation in p["observations"].as_array().unwrap() {
        assert_eq!(detector.update(observation["x"].as_f64().unwrap()).unwrap(), observation["alarm"].as_bool().unwrap());
        f64_bits(observation, "mean_bits", detector.mean);
        f64_bits(observation, "cumulative_bits", detector.cumulative);
    }
    detector.reset(); assert_eq!(detector.n, 0); assert!(!detector.alarm);
    assert_eq!(detector.delta, 0.25); assert_eq!(detector.lambda, 0.5);
}

#[test]
fn frozen_pass_fraction_and_solve_ema_then_anomaly_refusal() {
    let f = fixture(); let p = &f["pass"];
    let mut guard = PassGuard::new(p["fraction"].as_f64().unwrap() as f32, p["baseline"].as_f64().unwrap() as f32).unwrap();
    assert_eq!(guard.admit_count(600).unwrap(), 600);
    guard.observe(true).unwrap(); f32_bits(p, "one_solved_ema_bits", guard.ema);
    for _ in 0..5000 { guard.observe(true).unwrap(); }
    assert!(guard.anomalous()); assert_eq!(guard.admit_count(600).unwrap(), 0);
    assert_eq!(guard.admit_count(100_001), Err(Error::Capacity));
}

#[test]
fn frozen_shadow_float32_delta_before_float64_aggregate() {
    let f = fixture(); let s = &f["shadow"]; let mut shadow = Shadow::default();
    assert!(shadow.mean_delta().is_none()); assert!(shadow.agreement().is_none());
    for pair in s["pairs"].as_array().unwrap() {
        shadow.observe(Prediction { p_bot: pair["a"].as_f64().unwrap() as f32, abstain: pair["aa"].as_bool().unwrap() },
            Prediction { p_bot: pair["s"].as_f64().unwrap() as f32, abstain: pair["sa"].as_bool().unwrap() }, 0.5).unwrap();
    }
    assert_eq!(shadow.n, 4); assert_eq!(shadow.both, 2);
    assert_eq!(shadow.shadow_more, 1); assert_eq!(shadow.active_more, 1);
    f64_bits(s, "mean_delta_bits", shadow.mean_delta().unwrap());
    f64_bits(s, "agreement_bits", shadow.agreement().unwrap());
}

#[test]
fn psi_identity_exact_and_invalid_shapes_fail_closed() {
    let f = fixture(); let p = &f["psi_identity"];
    f64_bits(p, "expected_bits", psi(&[0.25, 0.75], &[0.25, 0.75]).unwrap());
    assert_eq!(psi(&[], &[]), Err(Error::Shape));
    assert_eq!(psi(&[0.5], &[0.5, 0.5]), Err(Error::Shape));
    assert_eq!(psi(&[f64::NAN], &[0.5]), Err(Error::Nonfinite));
    // Qualitative nonzero-ln check, NOT a cross-language tolerance waiver.
    assert!(psi(&[0.25, 0.75], &[0.75, 0.25]).unwrap() > 0.25);
}

#[test]
fn numeric_nonfinite_capacity_and_late_failures_preserve_state() {
    let mut c = Calibration::new(0.25, 0.5, 0.25).unwrap(); let before = c.clone();
    assert_eq!(c.observe(true, f32::NAN), Err(Error::Nonfinite)); assert_eq!(c, before);
    c.humans = MAX_OBSERVATIONS; let before = c.clone();
    assert_eq!(c.observe(true, 1.0), Err(Error::Capacity)); assert_eq!(c, before);
    let mut p = PageHinkley::new(0.25, 0.5).unwrap(); let before = p.clone();
    assert_eq!(p.update(f64::INFINITY), Err(Error::Nonfinite)); assert_eq!(p, before);
    let mut shadow = Shadow::default(); let before = shadow.clone();
    assert_eq!(shadow.observe(Prediction { p_bot: 1.1, abstain: false }, Prediction::abstained(), 0.5), Err(Error::Nonfinite));
    assert_eq!(shadow, before);
}

#[test]
fn frozen_reference_window_closes_once_and_nan_is_transactional() {
    let mut monitor = FeatureMonitor::new(8).unwrap();
    for i in 0..8 { let mut x = [0.0; 27]; x[0] = i as f32; assert_eq!(monitor.observe(&x).unwrap(), None); }
    assert!(monitor.ready()); let before = monitor.clone();
    let mut invalid = [0.0; 27]; invalid[26] = f32::NAN;
    assert_eq!(monitor.observe(&invalid), Err(Error::Nonfinite)); assert_eq!(monitor, before);
    for i in 0..7 { let mut x = [0.0; 27]; x[0] = i as f32; assert_eq!(monitor.observe(&x).unwrap(), None); }
    let mut last = [0.0; 27]; last[0] = 7.0;
    assert_eq!(monitor.observe(&last).unwrap().unwrap().to_bits(), 0.0_f64.to_bits());
    assert_eq!(monitor.observe(&[0.0; 26]), Err(Error::Shape));
    assert_eq!(FeatureMonitor::new(4097).unwrap_err(), Error::Capacity);
}

#[test]
fn degenerate_reference_blindspot_is_preserved_not_overclaimed() {
    let mut monitor = FeatureMonitor::new(2).unwrap();
    monitor.observe(&[0.0; 27]).unwrap(); monitor.observe(&[0.0; 27]).unwrap();
    assert_eq!(monitor.observe(&[100.0; 27]).unwrap(), None);
    // Original repeated edges put both constant reference and larger current
    // values in bucket7. Do not silently redesign this detector under freeze.
    assert_eq!(monitor.observe(&[100.0; 27]).unwrap(), Some(0.0));
}

#[test]
fn weak_labels_quarantine_while_legacy_mapping_remains_explicit() {
    for weak in [Outcome::ChallengeFailed, Outcome::ChallengeAbandoned] {
        assert_eq!(legacy_label(weak), Label::Bot); assert_eq!(reviewed_label(weak), Label::Ambiguous);
    }
    assert_eq!(reviewed_label(Outcome::PassSolved), Label::Human);
    assert_eq!(reviewed_label(Outcome::CatalogBot), Label::Bot);
    assert_eq!(reviewed_label(Outcome::Unknown), Label::Ambiguous);
}

#[test]
fn controller_uses_post_update_theta_and_snapshots_do_not_increment_events() {
    let mut controller = Controller::new("synthetic", 0.25, 0.5, 0.25, 8).unwrap();
    controller.observe_reviewed_outcome(Outcome::PassSolved, 0.5, false).unwrap();
    assert_eq!(controller.calibration.theta, 0.6875);
    assert_eq!(controller.drift.oracle.mean.to_bits(), 0.0_f64.to_bits());
    controller.drift.mimic.alarm = true; controller.drift.update_psi(0.5).unwrap();
    let before = controller.clone(); let a = controller.snapshot(); let b = controller.snapshot();
    assert_eq!(a, b); assert_eq!(controller, before); assert!(!a.activation);
    controller.observe_reviewed_outcome(Outcome::ChallengeAbandoned, 0.5, true).unwrap();
    assert_eq!(controller.calibration.humans, 1); assert_eq!(controller.calibration.bots, 0);
}

#[test]
fn controller_composite_failure_rolls_back_mimic_and_calibration() {
    let mut c = Controller::new("synthetic", 0.25, 0.5, 0.25, 8).unwrap();
    c.drift.oracle.n = MAX_OBSERVATIONS; let before = c.clone();
    assert_eq!(c.observe_reviewed_outcome(Outcome::CatalogBot, 0.75, true), Err(Error::Capacity));
    assert_eq!(c, before);
    let before = c.clone(); assert_eq!(c.observe_features(&[f32::NAN; 27]), Err(Error::Nonfinite)); assert_eq!(c, before);
}

#[test]
fn canary_strict_boundary_priority_one_shot_and_count_regression() {
    let mut c = Canary::new(0.25, 20, true, true).unwrap(); c.arm(100).unwrap();
    assert_eq!(c.evaluate(119, 0.5, false, false).unwrap(), None);
    assert_eq!(c.evaluate(120, 0.5, true, true).unwrap(), Some(TripReason::HumanFp));
    assert_eq!(c.evaluate(121, 0.5, true, true).unwrap(), None);
    assert_eq!(c.state, CanaryState::Tripped);
    c.arm(200).unwrap(); let before = c.clone();
    assert_eq!(c.evaluate(199, 0.0, false, false), Err(Error::ClockRegression)); assert_eq!(c, before);
    assert_eq!(c.evaluate(220, 0.25, false, false).unwrap(), None); assert_eq!(c.state, CanaryState::Graduated);
    c.arm(220).unwrap(); assert_eq!(c.evaluate(220, 0.0, true, true).unwrap(), Some(TripReason::Drift));
}

fn bundle(version: &str) -> Vec<u8> {
    let mut w1 = vec![vec![0.0; 27]]; w1[0][1] = 8.0;
    let mut mean = vec![0.0; 27]; mean[1] = 0.5;
    let mut std = vec![1.0; 27]; std[1] = 0.25;
    serde_json::to_vec(&MlpBundle { version: version.into(), schema_hash: crate::behavior::features::schema_hash(),
        feature_dim: 27, hidden: 1, mean, std,
        w1, b1: vec![8.0], w2: vec![1.0], b2: -8.0,
        cal_a: 1.0, cal_b: 0.0 }).unwrap()
}
struct MockSignature;
impl SignatureVerifier for MockSignature {
    fn verify(&self, _: &[u8], signature: &[u8; 64]) -> Result<ModelDigest, Error> {
        if signature == &[7; 64] { Ok([9; 32]) } else { Err(Error::Signature) }
    }
}
struct MockAuthority(ModelDigest);
impl ApprovalAuthority for MockAuthority {
    fn verify(&self, digest: ModelDigest, proof: &[u8]) -> Result<String, Error> {
        if digest != self.0 { return Err(Error::Authority); }
        match proof { b"alice" => Ok("alice".into()), b"bob" => Ok("bob".into()), _ => Err(Error::Authority) }
    }
}
fn stage(ledger: &mut Ledger, version: &str) {
    let bytes = bundle(version); let digest = Sha256::digest(&bytes).into();
    ledger.stage(&bytes, digest, StagePolicy::RequireSignature, Some(&[7; 64]), Some(&MockSignature)).unwrap();
    let gate = assess(&bytes, &samples(0), &samples(1000), Policy::default()).unwrap();
    ledger.attach_gate(&gate).unwrap();
}
fn promote(ledger: &mut Ledger, now: u64) -> Receipt {
    let request = ledger.preview(Action::PromoteOffline, now + 100).unwrap();
    ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), now).unwrap()
}

#[test]
fn stage_exact_bytes_digest_signature_schema_and_failed_replacement_are_atomic() {
    let mut ledger = Ledger::default(); stage(&mut ledger, "v1"); let before = ledger.clone();
    assert_eq!(ledger.staged().unwrap().version, "v1"); assert!(ledger.generation() > 0);
    let bytes = bundle("v2"); let digest = Sha256::digest(&bytes).into();
    assert_eq!(ledger.stage(&bytes, [0; 32], StagePolicy::UnsignedOffline, None, None), Err(Error::Digest));
    assert_eq!(ledger, before);
    assert_eq!(ledger.stage(&bytes, digest, StagePolicy::RequireSignature, Some(&[8; 64]), Some(&MockSignature)), Err(Error::Signature));
    assert_eq!(ledger, before);
    let mut malformed: MlpBundle = serde_json::from_slice(&bytes).unwrap(); malformed.schema_hash = "stale".into();
    let malformed = serde_json::to_vec(&malformed).unwrap(); let digest = Sha256::digest(&malformed).into();
    assert_eq!(ledger.stage(&malformed, digest, StagePolicy::UnsignedOffline, None, None), Err(Error::Schema));
    assert_eq!(ledger, before); assert!(ledger.selected().is_none());
}

#[test]
fn unsigned_data_stages_but_never_produces_promotion_request() {
    let mut ledger = Ledger::default(); let bytes = bundle("unsigned");
    ledger.stage(&bytes, Sha256::digest(&bytes).into(), StagePolicy::UnsignedOffline, None, None).unwrap();
    assert_eq!(ledger.preview(Action::PromoteOffline, 100), Err(Error::Unsigned));
    assert!(ledger.selected().is_none());
}

#[test]
fn dual_control_scope_stale_generation_expiry_and_replay_refuse_without_mutation() {
    let mut ledger = Ledger::default(); stage(&mut ledger, "v1");
    let request = ledger.preview(Action::PromoteOffline, 100).unwrap(); let before = ledger.clone();
    assert_eq!(ledger.commit(&request, [b"alice", b"alice"], &MockAuthority(request.digest()), 1), Err(Error::SamePrincipal));
    assert_eq!(ledger, before);
    assert_eq!(ledger.commit(&request, [b"alice", b"bob"], &MockAuthority([0; 32]), 1), Err(Error::Authority));
    assert_eq!(ledger, before);
    assert_eq!(ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), 100), Err(Error::Expired));
    assert_eq!(ledger, before);
    stage(&mut ledger, "v2"); let before = ledger.clone();
    assert_eq!(ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), 1), Err(Error::Generation));
    assert_eq!(ledger, before);
    let receipt = promote(&mut ledger, 1); assert!(!receipt.activation);
    let before = ledger.clone();
    assert_eq!(ledger.commit(&receipt.binding, [b"alice", b"bob"], &MockAuthority(receipt.binding.digest()), 2), Err(Error::NoStaged));
    assert_eq!(ledger, before);
}

#[test]
fn offline_promotion_retains_one_previous_and_rollback_reverts_to_none_without_runtime() {
    let mut ledger = Ledger::default(); stage(&mut ledger, "v1"); promote(&mut ledger, 10);
    stage(&mut ledger, "v2"); promote(&mut ledger, 20);
    let request = ledger.preview(Action::RollbackOffline, 100).unwrap();
    let receipt = ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), 30).unwrap();
    assert!(!receipt.activation); assert_eq!(ledger.selected().unwrap().version, "v1");
    let request = ledger.preview(Action::RollbackOffline, 100).unwrap();
    ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), 40).unwrap();
    assert!(ledger.selected().is_none());
    assert_eq!(ledger.preview(Action::RollbackOffline, 100), Err(Error::NoPrevious));
}

#[test]
fn authority_clock_regression_is_fail_closed_and_staging_never_selects() {
    let mut ledger = Ledger::default(); stage(&mut ledger, "v1"); assert!(ledger.selected().is_none());
    promote(&mut ledger, 10); stage(&mut ledger, "v2");
    let request = ledger.preview(Action::PromoteOffline, 100).unwrap(); let before = ledger.clone();
    assert_eq!(ledger.commit(&request, [b"alice", b"bob"], &MockAuthority(request.digest()), 9), Err(Error::ClockRegression));
    assert_eq!(ledger, before);
}

fn samples(first: i64) -> Vec<crate::model::training::record::Sample> {
    use crate::{signals::report::BehaviorSummary, model::training::record::{Record, Limits}};
    (0..100).map(|i| {
        let label = u8::from(i % 2 == 0); let mut behavior = BehaviorSummary::default();
        behavior.mouse.mean_curvature = if label == 1 { 0.75 } else { 0.25 };
        Record { label, behavior, ts: Some(first+i),
            cohort: Some(if i%4==1 { "synthetic-a" } else { "synthetic-b" }.into()),
            source: Some(if label==1 { "catalog" } else { "pass" }.into()) }
            .sample(&crate::behavior::features::schema_hash(), Limits::default()).unwrap()
    }).collect()
}

#[test]
fn gate_default_quality_counts_aut_bits_and_exact_model_evidence_binding() {
    let raw = bundle("gate"); let report = assess(&raw, &samples(0), &samples(1000), Policy::default()).unwrap();
    assert!(report.allowed()); assert_eq!(report.counts.humans, 50); assert_eq!(report.counts.bots, 50);
    assert_eq!(report.counts.human_fp, 0); assert_eq!(report.counts.bot_tp, 50);
    let fixed = fixture(); let expected = &fixed["quality_counts"];
    f32_bits(expected, "aut_bits", report.aut);
    f32_bits(expected, "human_fpr_bits", report.counts.human_fpr().unwrap());
    f32_bits(expected, "bot_tpr_bits", report.counts.bot_tpr().unwrap());
    let mut ledger = Ledger::default();
    ledger.stage(&raw, Sha256::digest(&raw).into(), StagePolicy::RequireSignature, Some(&[7; 64]), Some(&MockSignature)).unwrap();
    assert_eq!(ledger.preview(Action::PromoteOffline, 100), Err(Error::Missing));
    ledger.attach_gate(&report).unwrap();
    let binding = ledger.preview(Action::PromoteOffline, 100).unwrap();
    assert_eq!(binding.evidence, Some(report.evidence()));
    let different = assess(&bundle("other"), &samples(0), &samples(1000), Policy::default()).unwrap();
    let before = ledger.clone(); assert_eq!(ledger.attach_gate(&different), Err(Error::Digest)); assert_eq!(ledger, before);
}

#[test]
fn quality_cannot_hide_missing_cohort_or_poisoned_minority_in_aggregate() {
    let raw = bundle("quality"); let train = samples(0); let mut test = samples(1000);
    for sample in &mut test { if sample.record.label == 0 { sample.record.cohort = Some("synthetic-a".into()); } }
    let missing = assess(&raw, &train, &test, Policy::default()).unwrap();
    assert!(!missing.allowed()); assert!(missing.missing_cohorts.iter().any(|s| s == "synthetic-b"));
    test = samples(1000);
    for sample in &mut test {
        if sample.record.label == 0 && sample.record.cohort.as_deref() == Some("synthetic-b") {
            sample.record.behavior.mouse.mean_curvature = 0.75;
            *sample = sample.record.clone().sample(&crate::behavior::features::schema_hash(), crate::model::training::record::Limits::default()).unwrap();
        }
    }
    let poisoned = assess(&raw, &train, &test, Policy::default()).unwrap();
    assert!(!poisoned.allowed()); assert!(poisoned.exceeded_cohorts.iter().any(|s| s == "synthetic-b"));
    let mut ledger = Ledger::default(); stage(&mut ledger, "quality"); let before = ledger.clone();
    assert_eq!(ledger.attach_gate(&poisoned), Err(Error::Support)); assert_eq!(ledger, before);
}

#[test]
fn temporal_ties_missing_class_and_quarantine_refuse_not_zero_success() {
    let all = samples(0); let (train, test) = temporal_ids(&all, 0.25).unwrap();
    assert_eq!(train.len(), 75); assert_eq!(test.len(), 25); assert_eq!(test[0], 75);
    let mut tied = samples(0); tied[75].record.ts = Some(74);
    assert_eq!(temporal_ids(&tied, 0.25), Err(Error::Time));
    assert_eq!(temporal_ids(&all, f64::NAN), Err(Error::Config));
    let train = samples(0); let mut test = samples(1000);
    test.retain(|s| s.record.label == 1);
    assert_eq!(assess(&bundle("quality"), &train, &test, Policy::default()).unwrap_err(), Error::Support);
    test = samples(1000); test[0].record.source = None;
    assert_eq!(assess(&bundle("quality"), &train, &test, Policy::default()).unwrap_err(), Error::Authority);
}

#[test]
fn cohort_rate_is_float32_then_compared_as_float64_like_original_gate() {
    let train = samples(0); let mut test = samples(1000); let mut human = 0;
    for sample in &mut test {
        if sample.record.label == 0 {
            sample.record.cohort = Some(if human < 20 { "synthetic-a" } else { "synthetic-b" }.into());
            if human < 2 { sample.record.behavior.mouse.mean_curvature = 0.75; }
            *sample = sample.record.clone().sample(&crate::behavior::features::schema_hash(), crate::model::training::record::Limits::default()).unwrap();
            human += 1;
        }
    }
    let raw = bundle("boundary");
    let report = assess(&raw, &train, &test, Policy { max_human_fpr: 0.1, ..Policy::default() }).unwrap();
    assert!(!report.allowed()); assert_eq!(report.exceeded_cohorts, vec!["synthetic-a"]);
    let fixed = fixture(); let expected = &fixed["quality_boundary"];
    f32_bits(expected, "rate_f32_bits", 2.0_f32 / 20.0);
    f64_bits(expected, "policy_f64_bits", 0.1);
    // A second explicitly different policy proves the original widening boundary,
    // not a tolerance waiver or a change to the configured production policy.
    let equal_policy = assess(&raw, &train, &test,
        Policy { max_human_fpr: f64::from(0.1_f32), ..Policy::default() }).unwrap();
    assert!(equal_policy.allowed()); assert_ne!(report.evidence(), equal_policy.evidence());
}

#[test]
fn composed_shadow_pass_poisoning_and_canary_emit_data_without_activation() {
    let mut c = Controller::new("synthetic", 0.25, 0.5, 0.25, 8).unwrap();
    c.observe_shadow(Prediction { p_bot: 0.25, abstain: false }, Prediction { p_bot: 0.75, abstain: false }).unwrap();
    assert_eq!(c.snapshot().shadow.shadow_more, 1);
    for _ in 0..5000 { c.observe_pass(true).unwrap(); }
    assert!(c.snapshot().pass_anomaly); assert_eq!(c.pass.admit_count(600).unwrap(), 0);
    c.arm_canary(Canary::new(0.1, 100, false, false).unwrap()).unwrap();
    for i in 1..=20 {
        let proposal = c.observe_reviewed_outcome(Outcome::PassSolved, 1.0, false).unwrap();
        assert!(!proposal.activation);
        assert_eq!(proposal.rollback, if i == 20 { Some(TripReason::HumanFp) } else { None });
    }
    assert_eq!(c.observe_reviewed_outcome(Outcome::PassSolved, 1.0, false).unwrap().rollback, None);
    assert!(!c.snapshot().activation);
}
