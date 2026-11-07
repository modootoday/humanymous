//! Source-reconstructed regressions, not independently generated Go vectors.
use super::*;

const KEY: &[u8] = b"humanymous-pass-test-master-key-000";

// Legacy TestGenerateDeterministic.
#[test]
fn generate_is_deterministic_and_fixed_sessions_differ() {
    let first = generate(KEY, "sess-1", 100, 0, 1);
    assert_eq!(first, generate(KEY, "sess-1", 100, 0, 1));
    assert_ne!(first, generate(KEY, "sess-2", 100, 0, 1));
}

// Legacy TestDimsScaleWithDifficulty.
#[test]
fn dimensions_scale_with_difficulty() {
    for (difficulty, n) in [9, 11, 13, 15].into_iter().enumerate() {
        let value = generate(KEY, "s", 100, 0, difficulty as i32);
        assert_eq!((value.n, value.center, value.rows.len()), (n, n / 2, ROWS));
    }
}

// Legacy TestSolutionVerifies: sixteen session/difficulty combinations.
#[test]
fn canonical_solutions_verify() {
    for difficulty in 0..=3 {
        for session in ["a", "b", "c", "d"] {
            let offsets = solution_offsets(KEY, session, 100, 0, difficulty);
            assert!(verify(KEY, session, 100, 100, 0, difficulty, &offsets));
        }
    }
}

// Legacy TestWrongOffsetsFail.
#[test]
fn wrong_offsets_and_short_count_fail() {
    let mut offsets = solution_offsets(KEY, "a", 100, 0, 1);
    offsets[0] = (offsets[0] + 1) % 11;
    assert!(!verify(KEY, "a", 100, 100, 0, 1, &offsets));
    assert!(!verify(KEY, "a", 100, 100, 0, 1, &[0, 0]));
}

// Legacy TestStaleBucketRejected, including the inclusive accessibility floor.
#[test]
fn freshness_preserves_inclusive_slow_solver_window() {
    let offsets = solution_offsets(KEY, "a", 100, 0, 1);
    assert!(verify(KEY, "a", 100, 100, 0, 1, &offsets));
    assert!(verify(KEY, "a", 100, 120, 0, 1, &offsets));
    assert!(!verify(KEY, "a", 100, 121, 0, 1, &offsets));
    assert!(!verify(KEY, "a", 100, 99, 0, 1, &offsets));
}

// Legacy TestWargameR322_PassSolutionDeterministic.
#[test]
fn r322_solution_is_deterministic() {
    let first = solution_offsets(KEY, "sess-r322", 50, 1, 1);
    assert_eq!(first.len(), ROWS);
    assert_eq!(first, solution_offsets(KEY, "sess-r322", 50, 1, 1));
}

// Legacy TestWargameR323_PassWrongOffsetRejected.
#[test]
fn r323_misaligned_second_row_fails() {
    let mut offsets = solution_offsets(KEY, "sess-r323", 50, 2, 1);
    offsets[1] = (offsets[1] + 3) % 11;
    assert!(!verify(KEY, "sess-r323", 50, 50, 2, 1, &offsets));
}

// Legacy TestWargameR324_PassCrossSessionSolutionRejected, fixed example only.
#[test]
fn r324_fixed_cross_session_solution_fails() {
    let offsets = solution_offsets(KEY, "sess-A", 50, 0, 1);
    assert!(!verify(KEY, "sess-B", 50, 50, 0, 1, &offsets));
}

// Legacy TestWargameR325_PassCrossInstanceRejected, fixed example only.
#[test]
fn r325_fixed_cross_instance_solution_fails() {
    let offsets = solution_offsets(KEY, "sess", 50, 0, 1);
    assert!(!verify(KEY, "sess", 50, 50, 1, 1, &offsets));
}

// Legacy TestWargameR326_PassTTLAllowsSlowAT.
#[test]
fn r326_twenty_bucket_age_is_accepted() {
    let offsets = solution_offsets(KEY, "slow", 100, 0, 1);
    assert!(verify(KEY, "slow", 100, 120, 0, 1, &offsets));
}

// Legacy TestWargameR327_PassTTLRejectsStale.
#[test]
fn r327_twenty_one_bucket_age_is_rejected() {
    let offsets = solution_offsets(KEY, "stale", 100, 0, 1);
    assert!(!verify(KEY, "stale", 100, 121, 0, 1, &offsets));
}

// Legacy TestWargameR328_PassDifficultyScalesN.
#[test]
fn r328_difficulty_changes_cells_without_timing_or_movement_input() {
    let mut previous = 0;
    for difficulty in 0..=3 {
        let value = generate(KEY, "d", 10, 0, difficulty);
        assert!(value.n > previous);
        previous = value.n;
    }
}

// Legacy TestWargameR329_PassWrongLengthOffsets.
#[test]
fn r329_short_and_long_offset_counts_fail() {
    assert!(!verify(KEY, "a", 1, 1, 0, 1, &[0]));
    assert!(!verify(KEY, "a", 1, 1, 0, 1, &[0, 0, 0, 0]));
}

// Legacy TestWargameR330_PassPublicChallengeHasNoHiddenAnswer.
#[test]
fn r330_marked_positions_and_character_cells_are_public() {
    let value = generate(KEY, "pub", 7, 0, 0);
    assert_eq!(value.rows.len(), ROWS);
    for row in value.rows {
        assert!(row.key_index < value.n);
        assert_eq!(row.chars.len(), value.n);
        assert!(
            row.chars.iter().all(|cell| {
                cell.len() == 1 && generation::ALPHABET.contains(&cell.as_bytes()[0])
            })
        );
    }
}

// Legacy TestWargameR331_PassDifferentDifficultyDifferentSolution: domains differ,
// not a guarantee that three numerical solution offsets are always distinct.
#[test]
fn r331_difficulty_changes_solution_domain() {
    assert_eq!(solution_offsets(KEY, "s", 20, 0, 0).len(), ROWS);
    assert_eq!(solution_offsets(KEY, "s", 20, 0, 3).len(), ROWS);
    assert_ne!(
        generate(KEY, "s", 20, 0, 0).n,
        generate(KEY, "s", 20, 0, 3).n
    );
}

// Legacy TestWargameR332_PassAxisClose. Its no-op wrong-offset branch is
// deliberately strengthened into an unconditional one-cell misalignment check.
#[test]
fn r332_fresh_solution_and_session_binding_have_meaningful_negative() {
    let offsets = solution_offsets(KEY, "close", 9, 0, 1);
    assert!(verify(KEY, "close", 9, 9, 0, 1, &offsets));
    assert!(!verify(KEY, "other", 9, 9, 0, 1, &offsets));
    let mut wrong = offsets;
    wrong[0] = (wrong[0] + 1) % 11;
    assert!(!verify(KEY, "close", 9, 9, 0, 1, &wrong));
}

#[test]
fn difficulty_clamps_preserve_boundary_challenges() {
    assert_eq!(
        generate(KEY, "s", 1, 2, i32::MIN),
        generate(KEY, "s", 1, 2, 0)
    );
    assert_eq!(generate(KEY, "s", 1, 2, -1), generate(KEY, "s", 1, 2, 0));
    assert_eq!(generate(KEY, "s", 1, 2, 4), generate(KEY, "s", 1, 2, 3));
    assert_eq!(
        generate(KEY, "s", 1, 2, i32::MAX),
        generate(KEY, "s", 1, 2, 3)
    );
}

#[test]
fn seed_framing_is_exact_and_delimiters_are_not_reencoded() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(b"key").unwrap();
    mac.update(b"pass|a|b|\x01\x02\x03\x04\x05\x06\x07\x08|\x11\x12\x13\x14\x15\x16\x17\x18");
    let expected: [u8; 32] = mac.finalize().into_bytes().into();
    assert_eq!(
        generation::seed(b"key", "a|b", 0x0102030405060708, 0x1112131415161718),
        expected
    );
    assert_ne!(
        generation::seed(b"other", "a|b", 0x0102030405060708, 0x1112131415161718),
        expected
    );
}

#[test]
fn empty_unicode_and_delimiter_sessions_keep_deterministic_binding() {
    for session in ["", "|", "a|b", "e\u{301}", "🦀"] {
        let challenge = generate(b"", session, u64::MAX, u64::MAX, 3);
        assert_eq!(challenge, generate(b"", session, u64::MAX, u64::MAX, 3));
        let offsets = solution_offsets(b"", session, u64::MAX, u64::MAX, 3);
        assert!(verify(
            b"",
            session,
            u64::MAX,
            u64::MAX,
            u64::MAX,
            3,
            &offsets
        ));
    }
    assert_ne!(
        generation::seed(KEY, "é", 0, 0),
        generation::seed(KEY, "e\u{301}", 0, 0)
    );
}

#[test]
fn seed_binds_bucket_and_instance_bytes_at_unsigned_boundaries() {
    let maximum = generation::seed(KEY, "s", u64::MAX, u64::MAX);
    assert_ne!(maximum, generation::seed(KEY, "s", 0, u64::MAX));
    assert_ne!(maximum, generation::seed(KEY, "s", u64::MAX, 0));
    assert_ne!(
        generation::seed(KEY, "s", 1, 256),
        generation::seed(KEY, "s", 256, 1)
    );
}

#[test]
fn splitmix_wraps_and_uses_only_the_odd_big_endian_seed_prefix() {
    let mut random = generation::SplitMix64::new([255; 32]);
    assert_eq!(random.state, u64::MAX);
    let state = (u128::from(u64::MAX) + 0x9E3779B97F4A7C15) as u64;
    let first = (u128::from(state ^ (state >> 30)) * 0xBF58476D1CE4E5B9) as u64;
    let second = (u128::from(first ^ (first >> 27)) * 0x94D049BB133111EB) as u64;
    assert_eq!(random.next(), second ^ (second >> 31));
    assert_eq!(random.state, state);
    let mut prefix = [0; 32];
    prefix[0] = 1;
    assert_eq!(
        generation::SplitMix64::new(prefix).state,
        0x0100000000000001
    );
    prefix[31] = 255;
    assert_eq!(
        generation::SplitMix64::new(prefix).state,
        0x0100000000000001
    );
}

#[test]
fn zero_index_domain_consumes_no_random_draw() {
    let mut random = generation::SplitMix64::new([0; 32]);
    let state = random.state;
    assert_eq!(random.index(0), 0);
    assert_eq!(random.state, state);
}

#[test]
fn freshness_has_no_unsigned_wrap_or_hidden_short_timer() {
    for age in [0, 19, 20] {
        assert!(fresh(100, 100 + age));
    }
    assert!(!fresh(100, 121));
    assert!(!fresh(100, 99));
    assert!(fresh(u64::MAX - 20, u64::MAX));
    assert!(!fresh(u64::MAX - 21, u64::MAX));
    assert!(!fresh(u64::MAX, 0));
    assert!(!fresh(0, u64::MAX));
}

#[test]
fn negative_offsets_and_complete_rotations_preserve_alignment() {
    for difficulty in 0..=3 {
        let challenge = generate(KEY, "signed", 17, 4, difficulty);
        let canonical = solution_offsets(KEY, "signed", 17, 4, difficulty);
        for rotations in [-3, -1, 0, 1, 3] {
            let offsets: Vec<_> = canonical
                .iter()
                .map(|offset| offset + rotations * challenge.n as i64)
                .collect();
            assert!(verify(KEY, "signed", 17, 17, 4, difficulty, &offsets));
        }
    }
}

#[test]
fn extreme_signed_offsets_match_wrapping_64_bit_normalization() {
    let challenge = generate(KEY, "extreme", 2, 9, 3);
    for offset in [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX] {
        let expected = challenge.rows.iter().all(|row| {
            let wide = row.key_index as i128 + i128::from(offset);
            let wrapped = wide as u64 as i64;
            wrapped.rem_euclid(challenge.n as i64) == challenge.center as i64
        });
        assert_eq!(
            verify(KEY, "extreme", 2, 2, 9, 3, &[offset; ROWS]),
            expected
        );
    }
    let offsets: Vec<i64> = challenge
        .rows
        .iter()
        .map(|row| {
            (0..30)
                .map(|delta| i64::MAX - delta)
                .find(|offset| {
                    let wide = row.key_index as i128 + i128::from(*offset);
                    (wide as u64 as i64).rem_euclid(challenge.n as i64) == challenge.center as i64
                })
                .expect("two complete periods include a wrapping-domain solution")
        })
        .collect();
    assert!(verify(KEY, "extreme", 2, 2, 9, 3, &offsets));
}

#[test]
fn wire_fields_remain_public_and_exactly_named() {
    let value = generate(KEY, "wire", 4, 1, 0);
    let wire = serde_json::to_value(&value).unwrap();
    let mut fields: Vec<_> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(fields, ["center", "n", "rows"]);
    for row in wire["rows"].as_array().unwrap() {
        let mut fields: Vec<_> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        fields.sort_unstable();
        assert_eq!(fields, ["chars", "keyIndex"]);
        assert!(row.get("key_index").is_none());
    }
    assert_eq!(serde_json::from_value::<Challenge>(wire).unwrap(), value);
}

#[test]
fn centered_marked_cells_are_allowed_without_forced_movement() {
    let centered = (0..1024).any(|instance| {
        let value = generate(KEY, "center", 3, instance, 0);
        value.rows.iter().any(|row| row.key_index == value.center)
    });
    assert!(centered, "generation must not exclude the center column");
}
