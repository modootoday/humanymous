//! Unit coverage for legacy cmd/server/rit.go (baseline 981d25c had no direct
//! Go test). Golden values were produced by Go 1.25 running the baseline's
//! `ritCanonical`, `integrity.SessionKey`/`Seed` and the `verifyRIT` HMAC.
use super::*;

const MASTER: &[u8] = b"golden-master-key";
const SID: &str = "sid-1";
const BODY: &[u8] = br#"{"a":1}"#;
const NOW: i64 = 1_700_000_005;
const GOLDEN_TOKEN: &str = "fZTiizWofKbIdQBWMpfme-h3Pwk2mpAtErb854HQmn4";

fn presented<'a>(token: &'a str, counter: &'a str, bucket: &'a str) -> Presented<'a> {
    Presented {
        token,
        counter,
        bucket,
    }
}
fn check(last: u64, p: Presented<'_>, body: &[u8]) -> Outcome {
    verify(MASTER, SID, last, p, NOW, body, || {
        panic!("grace must not be consulted")
    })
    .unwrap()
}

#[test]
fn canonical_matches_go_golden() {
    assert_eq!(
        canonical(SID, 4, 170_000_000, BODY),
        "sid-1\n4\n170000000\n015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
    assert_eq!(
        canonical("", 0, 0, b""),
        "\n0\n0\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn issued_seed_matches_go_golden_and_refuses_missing_inputs() {
    assert_eq!(
        issue_seed(MASTER, SID, 3).unwrap(),
        IssuedSeed {
            seed: "Uct8PTkgRtPiiP9NQO5EYEQUlaGhD1saEASEXIXIavU".into(),
            counter: 3,
            window: 10,
        }
    );
    assert_eq!(issue_seed(b"", SID, 3), Err(Refusal::MasterKey));
    assert_eq!(issue_seed(MASTER, "", 3), Err(Refusal::Session));
}

#[test]
fn token_matches_go_golden() {
    let seed = seed(&session_key(MASTER, SID), 3);
    assert_eq!(
        token(&seed, &canonical(SID, 4, 170_000_000, BODY)),
        GOLDEN_TOKEN
    );
}

#[test]
fn within_bucket_is_symmetric_with_tolerance() {
    assert!(within_bucket(10, 10, 1));
    assert!(within_bucket(10, 11, 1) && within_bucket(11, 10, 1));
    assert!(!within_bucket(10, 12, 1) && !within_bucket(12, 10, 1));
    assert!(within_bucket(0, u64::MAX, u64::MAX));
    assert!(!within_bucket(0, u64::MAX, 1));
}

#[test]
fn counter_parsing_follows_go_parse_uint() {
    assert_eq!(parse_counter("4"), 4);
    assert_eq!(parse_counter("05"), 5);
    assert_eq!(parse_counter("18446744073709551615"), u64::MAX);
    for bad in [
        "",
        "+5",
        "-1",
        " 4",
        "4 ",
        "1_0",
        "0x10",
        "18446744073709551616",
    ] {
        assert_eq!(parse_counter(bad), 0, "{bad:?}");
    }
}

#[test]
fn valid_request_verifies() {
    assert_eq!(
        check(3, presented(GOLDEN_TOKEN, "4", "170000000"), BODY),
        Outcome::Valid
    );
    // Adjacent buckets are within tolerance.
    let seed = seed(&session_key(MASTER, SID), 3);
    for bucket in [169_999_999u64, 170_000_001] {
        let t = token(&seed, &canonical(SID, 4, bucket, BODY));
        assert_eq!(
            check(3, presented(&t, "4", &bucket.to_string()), BODY),
            Outcome::Valid
        );
    }
}

#[test]
fn first_tokenless_request_claims_the_grace_once() {
    let mut claimed = false;
    let mut claim = || !std::mem::replace(&mut claimed, true);
    let empty = Presented::default();
    assert_eq!(
        verify(MASTER, SID, 0, empty, NOW, BODY, &mut claim),
        Ok(Outcome::Bootstrap)
    );
    assert_eq!(
        verify(MASTER, SID, 0, empty, NOW, BODY, &mut claim),
        Ok(Outcome::Absent)
    );
}

#[test]
fn tokenless_request_after_signing_is_absent_without_grace() {
    assert_eq!(check(3, Presented::default(), BODY), Outcome::Absent);
}

#[test]
fn out_of_sequence_counter_is_stale() {
    assert_eq!(
        check(3, presented(GOLDEN_TOKEN, "3", "170000000"), BODY),
        Outcome::StaleCounter(3)
    );
    assert_eq!(
        check(3, presented(GOLDEN_TOKEN, "+4", "170000000"), BODY),
        Outcome::StaleCounter(0)
    );
    // Go's uint64 `lastN+1` wraps.
    assert_eq!(
        check(u64::MAX, presented(GOLDEN_TOKEN, "1", "170000000"), BODY),
        Outcome::StaleCounter(1)
    );
}

#[test]
fn bucket_outside_tolerance_is_stale() {
    for bucket in ["169999998", "170000002", ""] {
        assert!(matches!(
            check(3, presented(GOLDEN_TOKEN, "4", bucket), BODY),
            Outcome::StaleBucket(_)
        ));
    }
}

#[test]
fn any_tampering_fails_the_single_hmac_check() {
    assert_eq!(
        check(3, presented(GOLDEN_TOKEN, "4", "170000000"), br#"{"a":2}"#),
        Outcome::Tampered
    );
    assert_eq!(
        check(3, presented("fZTi", "4", "170000000"), BODY),
        Outcome::Tampered
    );
    assert_eq!(
        verify(
            MASTER,
            "sid-2",
            3,
            presented(GOLDEN_TOKEN, "4", "170000000"),
            NOW,
            BODY,
            || false
        ),
        Ok(Outcome::Tampered)
    );
}

#[test]
fn verify_refuses_missing_inputs() {
    let p = presented(GOLDEN_TOKEN, "4", "170000000");
    assert_eq!(
        verify(b"", SID, 3, p, NOW, BODY, || true),
        Err(Refusal::MasterKey)
    );
    assert_eq!(
        verify(MASTER, "", 3, p, NOW, BODY, || true),
        Err(Refusal::Session)
    );
}
