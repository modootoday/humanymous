use super::*;

fn rule(action: RuleAction) -> Rule {
    let (min_similarity, min_recurrence, min_subject_repeats) = floor_for(action);
    Rule {
        action,
        min_similarity,
        min_recurrence,
        min_subject_repeats,
    }
}

fn standard() -> Policy {
    Policy {
        version: POLICY_VERSION,
        window_secs: 24 * 3600,
        rules: vec![
            rule(RuleAction::Challenge),
            rule(RuleAction::TemporaryBlock),
            rule(RuleAction::PermanentBanProposal),
        ],
    }
}

#[test]
fn floors_are_monotonic_in_severity() {
    let actions = [
        RuleAction::Challenge,
        RuleAction::TemporaryBlock,
        RuleAction::PermanentBanProposal,
    ];
    for pair in actions.windows(2) {
        let (low, high) = (floor_for(pair[0]), floor_for(pair[1]));
        assert!(low.0 < high.0 && low.1 < high.1 && low.2 < high.2);
    }
}

#[test]
fn similarity_alone_reaches_at_most_challenge() {
    let policy = standard();
    assert_eq!(
        policy.evaluate(Some(1.0), 0, 0).expect("eval"),
        ShadowOutcome::Challenge
    );
    assert_eq!(
        policy.evaluate(Some(1.0), 1000, 0).expect("eval"),
        ShadowOutcome::Challenge
    );
}

#[test]
fn block_needs_recurrence_and_subject_repetition() {
    let policy = standard();
    assert_eq!(
        policy.evaluate(Some(0.93), 3, 2).expect("eval"),
        ShadowOutcome::TemporaryBlock
    );
    assert_eq!(
        policy.evaluate(Some(0.99), 10, 3).expect("eval"),
        ShadowOutcome::PermanentBanProposal
    );
}

#[test]
fn no_cluster_or_low_similarity_is_no_change() {
    let policy = standard();
    assert_eq!(
        policy.evaluate(None, 99, 99).expect("eval"),
        ShadowOutcome::NoChange
    );
    assert_eq!(
        policy.evaluate(Some(0.5), 99, 99).expect("eval"),
        ShadowOutcome::NoChange
    );
}

#[test]
fn rules_below_floors_are_refused() {
    let cases = [
        (
            RuleAction::Challenge,
            0.79,
            0,
            0,
            PolicyRefusal::SimilarityFloor,
        ),
        (
            RuleAction::TemporaryBlock,
            0.95,
            2,
            2,
            PolicyRefusal::RecurrenceFloor,
        ),
        (
            RuleAction::TemporaryBlock,
            0.95,
            3,
            1,
            PolicyRefusal::SubjectFloor,
        ),
        (
            RuleAction::TemporaryBlock,
            0.85,
            9,
            9,
            PolicyRefusal::SimilarityFloor,
        ),
        (
            RuleAction::PermanentBanProposal,
            0.99,
            9,
            9,
            PolicyRefusal::RecurrenceFloor,
        ),
        (
            RuleAction::PermanentBanProposal,
            0.99,
            99,
            2,
            PolicyRefusal::SubjectFloor,
        ),
    ];
    for (action, similarity, recurrence, subject, refusal) in cases {
        let mut policy = standard();
        policy.rules = vec![Rule {
            action,
            min_similarity: similarity,
            min_recurrence: recurrence,
            min_subject_repeats: subject,
        }];
        assert_eq!(policy.validate(), Err(refusal.into()), "{action:?}");
    }
}

#[test]
fn malformed_policies_are_refused() {
    let mut policy = standard();
    policy.version = 2;
    assert_eq!(policy.validate(), Err(PolicyRefusal::Version.into()));
    let mut policy = standard();
    policy.rules.clear();
    assert_eq!(policy.validate(), Err(PolicyRefusal::RuleCount.into()));
    let mut policy = standard();
    policy.rules = vec![rule(RuleAction::Challenge); MAX_RULES + 1];
    assert_eq!(policy.validate(), Err(PolicyRefusal::RuleCount.into()));
    for window in [MIN_WINDOW_SECS - 1, MAX_WINDOW_SECS + 1] {
        let mut policy = standard();
        policy.window_secs = window;
        assert_eq!(policy.validate(), Err(PolicyRefusal::Window.into()));
    }
    let mut policy = standard();
    policy.rules[0].min_similarity = f64::NAN;
    assert_eq!(policy.validate(), Err(SimilarityError::Nonfinite));
    let mut policy = standard();
    policy.rules[0].min_similarity = 1.5;
    assert_eq!(policy.validate(), Err(SimilarityError::Range));
}

#[test]
fn json_round_trip_and_unknown_fields() {
    let policy = standard();
    let text = serde_json::to_string(&policy).expect("encode");
    assert_eq!(Policy::from_json(&text).expect("decode"), policy);
    let extra = text.replacen('{', "{\"far_from_human\":true,", 1);
    assert_eq!(Policy::from_json(&extra), Err(PolicyRefusal::Syntax.into()));
    assert_eq!(
        Policy::from_json("{\"version\":1"),
        Err(PolicyRefusal::Syntax.into())
    );
    let bad_action = text.replacen("\"challenge\"", "\"allow_if_far\"", 1);
    assert_eq!(
        Policy::from_json(&bad_action),
        Err(PolicyRefusal::Syntax.into())
    );
}

#[test]
fn invalid_similarity_input_is_refused() {
    let policy = standard();
    assert_eq!(
        policy.evaluate(Some(f64::NAN), 0, 0),
        Err(SimilarityError::Nonfinite)
    );
    assert_eq!(
        policy.evaluate(Some(1.1), 0, 0),
        Err(SimilarityError::Range)
    );
}

#[test]
fn digest_is_stable_and_sensitive() {
    let policy = standard();
    assert_eq!(policy.digest(), standard().digest());
    assert_eq!(policy.digest().len(), 64);
    let mut changed = standard();
    changed.window_secs += 1;
    assert_ne!(policy.digest(), changed.digest());
}
