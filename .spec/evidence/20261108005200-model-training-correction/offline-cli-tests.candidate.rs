//! REVIEW CANDIDATE: private offline_train/tests.rs, no runtime bootstrap needed.
use super::*;

fn args() -> Vec<String> {
    ["--train", "train.jsonl", "--holdout", "holdout.jsonl", "--out", "model.json",
        "--version", "offline-test", "--hidden", "1", "--epochs", "0", "--lr",
        "0.03125", "--seed", "2", "--min-cohort-humans", "1", "--reviewed-input"]
        .into_iter().map(str::to_owned).collect()
}

#[test]
fn explicit_numeric_command_has_no_native_readiness_or_activation_option() {
    let parsed = parse(args()).unwrap();
    assert_eq!(parsed.hidden, 1); assert_eq!(parsed.epochs, 0);
    assert_eq!(parsed.learning_rate.to_bits(), 0.03125_f32.to_bits());
    assert_eq!(parsed.minimum_cohort_humans, 1); assert!(parsed.base.is_none());
    let mut bad = args(); bad.push("--activate".into());
    assert!(matches!(parse(bad), Err(CommandError::Arguments)));
}

#[test]
fn missing_review_duplicate_flag_and_incomplete_numeric_value_refuse() {
    let mut unreviewed = args(); unreviewed.pop();
    assert!(matches!(parse(unreviewed), Err(CommandError::ReviewRequired)));
    let mut duplicate = args(); duplicate.extend(["--hidden".into(), "3".into()]);
    assert!(matches!(parse(duplicate), Err(CommandError::Arguments)));
    let mut malformed = args(); malformed[9] = "not-a-number".into();
    assert!(matches!(parse(malformed), Err(CommandError::Arguments)));
    let mut missing = args(); missing.push("--base".into());
    assert!(matches!(parse(missing), Err(CommandError::Arguments)));
}
