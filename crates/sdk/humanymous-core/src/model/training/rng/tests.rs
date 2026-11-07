//! Source-only tests: retained Go OUTPUT goldens versus Rust-DERIVED state evidence.
//! No mixed-stream Go internal state was observed. Reference and kernel share the
//! audited tables; this differential model is not an independent Go execution.
use super::{DrawSource, GoRng, MAX_DRAWS, MAX_SAMPLES, RngError, normal, shuffle, source, tables};
use crate::behavior::features::{FEATURE_DIM, schema_hash};
use crate::model::inference::MlpBundle;

const SEEDS: [i64; 7] = [0, 1, -1, 2_147_483_647, -2_147_483_648, i64::MIN, i64::MAX];

/// Slow Rust-derived model: full-width modular multiplication for seeding and
/// physical vector rotation for feedback, instead of the production quotient
/// seed recurrence and circular storage. Raw Go internal snapshots are unavailable.
struct Reference {
    queue: Vec<u64>,
    draws: u64,
}

impl Reference {
    fn new(seed: i64) -> Self {
        let mut x = i128::from(seed).rem_euclid(2_147_483_647);
        if x == 0 {
            x = 89_482_311;
        }
        let mut step = || {
            x = (x * 48_271) % 2_147_483_647;
            x as u64
        };
        for _ in 0..20 {
            step();
        }
        let mut words = [0_u64; 607];
        for (index, word) in words.iter_mut().enumerate() {
            *word = (step() << 40) ^ (step() << 20) ^ step() ^ tables::COOKED[index] as u64;
        }
        let queue = (0..607)
            .map(|offset| words[(333 + 607 - offset) % 607])
            .collect();
        Self { queue, draws: 0 }
    }

    fn canonical_words(&self) -> [u64; 607] {
        let feed = (334 + 607 - (self.draws % 607) as usize) % 607;
        let next_feed = (feed + 606) % 607;
        let mut words = [0; 607];
        for (offset, value) in self.queue.iter().enumerate() {
            words[(next_feed + 607 - offset) % 607] = *value;
        }
        words
    }

    fn assert_state(&self, rng: &GoRng) {
        let state = rng.snapshot();
        assert_eq!(
            state.words,
            self.canonical_words(),
            "Rust-derived feedback state"
        );
        assert_eq!(state.draws, self.draws);
        assert_eq!(state.tap, (607 - (self.draws % 607) as usize) % 607);
        assert_eq!(state.feed, (334 + 607 - (self.draws % 607) as usize) % 607);
    }

    fn shuffle_reference(&mut self, order: &mut [usize]) {
        for bound in (2..=order.len()).rev() {
            let modulus = 1_u128 << 32;
            let threshold = modulus % bound as u128;
            let other = loop {
                let word = ((self.draw_u64().unwrap() & ((1_u64 << 63) - 1)) >> 31) as u128;
                let product = word * bound as u128;
                if product % modulus >= threshold {
                    break (product / modulus) as usize;
                }
            };
            order.swap(bound - 1, other);
        }
    }
}

impl DrawSource for Reference {
    fn draw_u64(&mut self) -> Result<u64, RngError> {
        let value = self.queue[0].wrapping_add(self.queue[334]);
        self.queue.rotate_left(1);
        self.queue[606] = value;
        self.draws += 1;
        Ok(value)
    }
}

/// Check each primitive state transition even inside normal rejection/tail loops.
struct Compared {
    actual: GoRng,
    reference: Reference,
}

impl DrawSource for Compared {
    fn draw_u64(&mut self) -> Result<u64, RngError> {
        let actual = self.actual.uint64()?;
        let expected = self.reference.draw_u64()?;
        assert_eq!(actual, expected, "Rust-derived primitive comparison");
        self.reference.assert_state(&self.actual);
        Ok(actual)
    }
}

struct Tape {
    words: Vec<u64>,
    consumed: usize,
}

impl Tape {
    fn new(words: &[u64]) -> Self {
        Self {
            words: words.to_vec(),
            consumed: 0,
        }
    }
}

impl DrawSource for Tape {
    fn draw_u64(&mut self) -> Result<u64, RngError> {
        let value = *self.words.get(self.consumed).ok_or(RngError::DrawLimit)?;
        self.consumed += 1;
        Ok(value)
    }
}

fn uint32_word(value: u32) -> u64 {
    u64::from(value) << 31
}

fn base(hidden: usize) -> MlpBundle {
    MlpBundle {
        version: "offline-test".into(),
        schema_hash: schema_hash(),
        feature_dim: FEATURE_DIM as i64,
        hidden: hidden as i64,
        mean: vec![0.0; FEATURE_DIM],
        std: vec![1.0; FEATURE_DIM],
        w1: vec![vec![0.25; FEATURE_DIM]; hidden],
        b1: vec![-0.5; hidden],
        w2: vec![0.75; hidden],
        b2: -0.125,
        cal_a: 1.0,
        cal_b: 0.0,
    }
}

/// Parse only comma-delimited numeric constants from immutable upstream DATA.
/// These text fixtures are never compiled, interpreted or executed as Go.
fn table_tokens<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    let body = text
        .split_once(marker)
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0;
    body.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect()
}

#[test]
fn translated_tables_match_unchanged_go1263_source_literals() {
    let rng = include_str!("../../../../tests/model/fixtures/go1263-rng-source.txt");
    let normal = include_str!("../../../../tests/model/fixtures/go1263-normal-source.txt");
    let cooked: Vec<i64> = table_tokens(rng, "rngCooked [rngLen]int64 = [...]int64{")
        .into_iter()
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(cooked.as_slice(), tables::COOKED.as_slice());
    let kn: Vec<u32> = table_tokens(normal, "var kn = [128]uint32{")
        .into_iter()
        .map(|value| u32::from_str_radix(value.strip_prefix("0x").unwrap(), 16).unwrap())
        .collect();
    assert_eq!(kn.as_slice(), tables::KN.as_slice());
    for (marker, actual) in [
        ("var wn = [128]float32{", tables::WN),
        ("var fn = [128]float32{", tables::FN),
    ] {
        let expected: Vec<u32> = table_tokens(normal, marker)
            .into_iter()
            .map(|value| value.parse::<f32>().unwrap().to_bits())
            .collect();
        assert_eq!(expected.len(), 128);
        assert_eq!(expected, actual.map(f32::to_bits).to_vec());
    }
}

#[test]
fn retained_go1263_seed0_int63_output_goldens() {
    // Unmodified regress_test.go Int63 section, each method reseeded to zero.
    let expected: [i64; 20] = [
        8717895732742165505,
        2259404117704393152,
        6050128673802995827,
        501233450539197794,
        3390393562759376202,
        2669985732393126063,
        1774932891286980153,
        6044372234677422456,
        8274930044578894929,
        1543572285742637646,
        2661732831099943416,
        8325060299420976708,
        7837839688282259259,
        2518412263346885298,
        5617773211005988520,
        2339563716805116249,
        7144924247938981575,
        161231572858529631,
        7259475919510918339,
        7373105480197164748,
    ];
    let mut rng = GoRng::new(0, 20).unwrap();
    for value in expected {
        assert_eq!(rng.int63().unwrap(), value);
    }
    assert_eq!(rng.snapshot().draws, 20);
}

#[test]
fn retained_go1263_seed0_normal_output_goldens() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/model/fixtures/go1263-normal-seed0.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["status"],
        "RETAINED_UPSTREAM_EXPECTATIONS_NOT_EXECUTED"
    );
    let expected = fixture["expected_f64_decimal"].as_array().unwrap();
    assert_eq!(expected.len(), 20);
    let mut rng = GoRng::new(0, 4096).unwrap();
    for (index, decimal) in expected.iter().enumerate() {
        let expected: f64 = decimal.as_str().unwrap().parse().unwrap();
        assert_eq!(
            rng.norm_float64().unwrap().to_bits(),
            expected.to_bits(),
            "normal {index}"
        );
    }
}

#[test]
fn rust_derived_seed_and_feedback_states_match_across_ring_wraps() {
    for seed in SEEDS {
        let mut compared = Compared {
            actual: GoRng::new(seed, 4096).unwrap(),
            reference: Reference::new(seed),
        };
        compared.reference.assert_state(&compared.actual);
        for _ in 0..2048 {
            compared.draw_u64().unwrap();
        }
    }
}

#[test]
fn rust_derived_normal_and_shuffle_mixed_stream_checks_every_primitive_draw() {
    for seed in SEEDS {
        let mut compared = Compared {
            actual: GoRng::new(seed, 4096).unwrap(),
            reference: Reference::new(seed),
        };
        for _ in 0..56 {
            normal::sample(&mut compared).unwrap();
        }
        let mut order: Vec<usize> = (0..7).collect();
        for _ in 0..3 {
            shuffle::apply(&mut compared, &mut order).unwrap();
        }
        assert!(compared.actual.snapshot().draws >= 56 + 18);
    }
}

#[test]
fn rust_derived_cold_warm_initialization_and_successive_epoch_orders() {
    for seed in SEEDS {
        for hidden in [1, 2, 16] {
            for warm in [false, true] {
                let mut actual = GoRng::new(seed, 4096).unwrap();
                let mut reference = Reference::new(seed);
                let base = base(hidden);
                let weights = actual.initialize(hidden, warm.then_some(&base)).unwrap();
                if warm {
                    assert_eq!(weights.w1, base.w1);
                    assert_eq!(weights.b1, base.b1);
                    assert_eq!(weights.w2, base.w2);
                    assert_eq!(weights.b2, base.b2);
                    assert_eq!(actual.snapshot().draws, 0);
                } else {
                    let scale1 = (2.0_f64 / FEATURE_DIM as f64).sqrt() as f32;
                    let scale2 = (2.0_f64 / hidden as f64).sqrt() as f32;
                    for h in 0..hidden {
                        for input in 0..FEATURE_DIM {
                            let expected = normal::sample(&mut reference).unwrap() as f32 * scale1;
                            assert_eq!(weights.w1[h][input].to_bits(), expected.to_bits());
                        }
                        let expected = normal::sample(&mut reference).unwrap() as f32 * scale2;
                        assert_eq!(weights.w2[h].to_bits(), expected.to_bits());
                    }
                    assert_eq!(weights.b1, vec![0.0; hidden]);
                    assert_eq!(weights.b2, 0.0);
                }
                reference.assert_state(&actual);
                let mut order: Vec<usize> = (0..7).collect();
                let mut expected_order = order.clone();
                for _ in 0..3 {
                    actual.shuffle(&mut order).unwrap();
                    reference.shuffle_reference(&mut expected_order);
                    assert_eq!(order, expected_order, "Rust-derived in-place epoch order");
                    reference.assert_state(&actual);
                }
            }
        }
    }
}

#[test]
fn requested_warm_state_is_validated_before_any_draw_or_copy_alias() {
    let mut rng = GoRng::new(1, 4096).unwrap();
    let before = rng.snapshot();
    let good = base(2);
    let mut malformed = good.clone();
    malformed.w1[0].pop();
    assert_eq!(rng.initialize(2, Some(&malformed)), Err(RngError::Shape));
    assert_eq!(rng.initialize(1, Some(&good)), Err(RngError::Shape));
    malformed = good.clone();
    malformed.feature_dim += 1;
    assert_eq!(rng.initialize(2, Some(&malformed)), Err(RngError::Shape));
    malformed = good.clone();
    malformed.schema_hash = "stale".into();
    assert_eq!(rng.initialize(2, Some(&malformed)), Err(RngError::Schema));
    malformed = good.clone();
    malformed.w2[0] = f32::NAN;
    assert_eq!(
        rng.initialize(2, Some(&malformed)),
        Err(RngError::Nonfinite)
    );
    assert_eq!(rng.snapshot(), before);
    let mut copied = rng.initialize(2, Some(&good)).unwrap();
    copied.w1[0][0] = 99.0;
    assert_eq!(good.w1[0][0], 0.25);
    assert_eq!(rng.snapshot(), before);
}

#[test]
fn empty_singleton_and_seed_modulus_cases_preserve_draw_contract() {
    let rng = GoRng::new(0, 10).unwrap();
    assert!(rng == GoRng::new(2_147_483_647, 10).unwrap());
    assert!(rng == GoRng::new(-2_147_483_647, 10).unwrap());
    assert!(rng == GoRng::new(89_482_311, 10).unwrap());
    let mut rng = rng;
    let before = rng.snapshot();
    rng.shuffle(&mut []).unwrap();
    rng.shuffle(&mut [42]).unwrap();
    assert_eq!(rng.snapshot(), before);
}

#[test]
fn budget_shape_and_size_errors_preserve_public_state_and_order() {
    assert!(matches!(GoRng::new(0, 0), Err(RngError::InvalidBudget)));
    assert!(matches!(
        GoRng::new(0, MAX_DRAWS + 1),
        Err(RngError::InvalidBudget)
    ));
    let mut rng = GoRng::new(0, 1).unwrap();
    let before = rng.snapshot();
    assert_eq!(rng.initialize(0, None), Err(RngError::Shape));
    assert_eq!(rng.initialize(257, None), Err(RngError::Shape));
    assert_eq!(rng.initialize(1, None), Err(RngError::DrawLimit));
    let mut order: Vec<usize> = (0..7).collect();
    assert_eq!(rng.shuffle(&mut order), Err(RngError::DrawLimit));
    assert_eq!(order, (0..7).collect::<Vec<_>>());
    let mut too_large = vec![0; MAX_SAMPLES + 1];
    assert_eq!(rng.shuffle(&mut too_large), Err(RngError::SampleLimit));
    assert_eq!(rng.snapshot(), before);
    rng.norm_float64().unwrap();
    let exhausted = rng.snapshot();
    assert_eq!(rng.norm_float64(), Err(RngError::DrawLimit));
    assert_eq!(rng.float64(), Err(RngError::DrawLimit));
    assert_eq!(rng.snapshot(), exhausted);
}

#[test]
fn injected_float64_rounding_retry_and_shuffle_rejection_consume_extra_draws() {
    let mut uniform = Tape::new(&[(1_u64 << 63) - 1, 0]);
    assert_eq!(source::uniform(&mut uniform).unwrap(), 0.0);
    assert_eq!(uniform.consumed, 2);
    let mut shuffle = Tape::new(&[uint32_word(0), uint32_word(u32::MAX)]);
    assert_eq!(shuffle::bounded(&mut shuffle, 3).unwrap(), 2);
    assert_eq!(shuffle.consumed, 2);
    let mut empty = Tape::new(&[]);
    assert_eq!(shuffle::bounded(&mut empty, 0), Err(RngError::SampleLimit));
    assert_eq!(empty.consumed, 0);
}

#[test]
fn injected_normal_fast_slow_accept_and_rejection_paths_are_distinct() {
    let mut fast = Tape::new(&[uint32_word(2)]);
    assert_eq!(
        normal::sample(&mut fast).unwrap(),
        2.0 * f64::from(tables::WN[2])
    );
    assert_eq!(fast.consumed, 1);
    let j = 0x77d6657f_u32;
    let mut slow = Tape::new(&[uint32_word(j), 0]);
    assert_eq!(
        normal::sample(&mut slow).unwrap(),
        f64::from(j) * f64::from(tables::WN[127])
    );
    assert_eq!(slow.consumed, 2);
    let mut rejected = Tape::new(&[uint32_word(0x7fffffff), 1_u64 << 62, uint32_word(2)]);
    assert_eq!(
        normal::sample(&mut rejected).unwrap(),
        2.0 * f64::from(tables::WN[2])
    );
    assert_eq!(rejected.consumed, 3);
}

#[test]
fn injected_normal_tail_sign_rejection_and_finite_tape_exhaustion() {
    let x = -0.5_f64.ln() * (1.0 / normal::RN);
    for (j, expected) in [(0x7fffff80, normal::RN + x), (0x80000000, -normal::RN - x)] {
        let mut tape = Tape::new(&[uint32_word(j), 1_u64 << 62, 1_u64 << 62]);
        assert_eq!(
            normal::sample(&mut tape).unwrap().to_bits(),
            expected.to_bits()
        );
        assert_eq!(tape.consumed, 3);
    }
    let mut rejected = Tape::new(&[
        uint32_word(0x7fffff80),
        1_u64 << 57,
        1_u64 << 62,
        1_u64 << 62,
        1_u64 << 62,
    ]);
    assert_eq!(
        normal::sample(&mut rejected).unwrap().to_bits(),
        (normal::RN + x).to_bits()
    );
    assert_eq!(rejected.consumed, 5);
    let mut exhausted = Tape::new(&[uint32_word(0x7fffff80)]);
    assert_eq!(normal::sample(&mut exhausted), Err(RngError::DrawLimit));
}
