use super::*;
use crate::model::similarity::{POLICY_VERSION, Rule, RuleAction, floor_for};

fn policy() -> Policy {
    let rules = [
        RuleAction::Challenge,
        RuleAction::TemporaryBlock,
        RuleAction::PermanentBanProposal,
    ]
    .into_iter()
    .map(|action| {
        let (min_similarity, min_recurrence, min_subject_repeats) = floor_for(action);
        Rule {
            action,
            min_similarity,
            min_recurrence,
            min_subject_repeats,
        }
    })
    .collect();
    Policy {
        version: POLICY_VERSION,
        window_secs: 3600,
        rules,
    }
}

fn sample(similarity: Option<f64>, actual: ActualVerdict, events: u32) -> Sample {
    Sample {
        similarity,
        recurrence: 0,
        subject_repeats: 0,
        actual,
        events,
    }
}

#[test]
fn latency_buckets_have_closed_edges() {
    assert_eq!(LatencyBucket::of(0), LatencyBucket::UpTo16);
    assert_eq!(LatencyBucket::of(16), LatencyBucket::UpTo16);
    assert_eq!(LatencyBucket::of(17), LatencyBucket::UpTo64);
    assert_eq!(LatencyBucket::of(256), LatencyBucket::UpTo256);
    assert_eq!(LatencyBucket::of(257), LatencyBucket::Over256);
}

#[test]
fn ledger_counts_agreement_direction_and_latency() {
    let mut ledger = ShadowLedger::new(policy()).expect("ledger");
    ledger
        .observe(sample(Some(0.95), ActualVerdict::Allow, 10))
        .expect("stricter");
    ledger
        .observe(sample(Some(0.95), ActualVerdict::Challenge, 40))
        .expect("agree");
    ledger
        .observe(sample(None, ActualVerdict::Block, 500))
        .expect("looser");
    ledger
        .observe(sample(None, ActualVerdict::Allow, 500))
        .expect("agree");
    let stats = ledger.stats();
    assert_eq!(stats.n, 4);
    assert_eq!(stats.outcomes, [2, 2, 0, 0]);
    assert_eq!((stats.stricter, stats.agree, stats.looser), (1, 2, 1));
    assert_eq!(stats.latency, [1, 1, 0, 0]);
    assert_eq!(stats.rejected, 0);
}

#[test]
fn invalid_sample_counts_only_as_rejected() {
    let mut ledger = ShadowLedger::new(policy()).expect("ledger");
    assert_eq!(
        ledger.observe(sample(Some(2.0), ActualVerdict::Allow, 1)),
        Err(SimilarityError::Range)
    );
    let stats = ledger.stats();
    assert_eq!((stats.n, stats.rejected), (0, 1));
}

#[test]
fn ledger_refuses_an_invalid_policy() {
    let mut bad = policy();
    bad.rules.clear();
    assert!(ShadowLedger::new(bad).is_err());
}

#[test]
fn preview_matches_a_live_ledger_and_survives_bad_samples() {
    let samples = [
        sample(Some(0.97), ActualVerdict::Allow, 8),
        Sample {
            similarity: Some(0.97),
            recurrence: 12,
            subject_repeats: 4,
            actual: ActualVerdict::Block,
            events: 300,
        },
        sample(Some(f64::INFINITY), ActualVerdict::Allow, 1),
    ];
    let stats = preview(&policy(), &samples).expect("preview");
    assert_eq!(stats.n, 2);
    assert_eq!(stats.rejected, 1);
    assert_eq!(stats.outcomes, [0, 1, 0, 1]);
    let mut ledger = ShadowLedger::new(policy()).expect("ledger");
    for sample in &samples {
        let _ = ledger.observe(*sample);
    }
    assert_eq!(ledger.stats(), stats);
    assert_eq!(ledger.policy_digest(), policy().digest());
}
