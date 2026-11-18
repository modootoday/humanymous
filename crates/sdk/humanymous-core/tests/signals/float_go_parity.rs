//! Differential parity of the Go float formatting helpers against outputs
//! captured from Go 1.26.3 (`fmt` `%v`, `%.1f` and `encoding/json`) in the
//! pinned oracle image; provenance is in the fixture.
use humanymous_core::signals::wire::{format_fixed_go, format_float_go, json_float_go};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Stream {
    name: String,
    count: usize,
    sha256: String,
}

#[derive(Deserialize)]
struct Golden {
    streams: Vec<Stream>,
    cases: Vec<String>,
}

fn golden() -> Golden {
    serde_json::from_str(include_str!("fixtures/float-go-golden.json")).expect("float golden")
}

/// The capture line for one value: bits, `%v`, `%.1f`, `json.Marshal`.
fn line(value: f64) -> String {
    format!(
        "{:016x}\t{}\t{}\t{}\n",
        value.to_bits(),
        format_float_go(value),
        format_fixed_go(value, 1),
        json_float_go(value).expect("finite corpus value")
    )
}

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

const SEED: u64 = 0x5eed_b1f1_0a7c_2026;

/// Go `math.Ldexp(float64(k), -m)` for k <= 2^53 and 1 <= m <= 52 (exact).
fn dyadic(k: u64, m: u32) -> f64 {
    k as f64 * f64::from_bits(u64::from(1023 - m) << 52)
}

fn bits_range(random: &mut SplitMix, low: f64, high: f64) -> f64 {
    let (low, high) = (low.to_bits(), high.to_bits());
    f64::from_bits(low + random.next() % (high - low + 1))
}

fn families() -> Vec<f64> {
    let powers_of_two = (-1074_i32..=1023).map(|exponent| {
        if exponent >= -1022 {
            f64::from_bits(u64::try_from(exponent + 1023).unwrap() << 52)
        } else {
            f64::from_bits(1 << (exponent + 1074))
        }
    });
    let powers_of_ten = (-323_i32..=308).map(|exponent| format!("1e{exponent}").parse().unwrap());
    powers_of_two.chain(powers_of_ten).collect()
}

fn seeded() -> Vec<f64> {
    let mut random = SplitMix(SEED);
    let mut values = Vec::with_capacity(140_000);
    for _ in 0..40_000 {
        let mut bits = random.next();
        if (bits >> 52) & 0x7ff == 0x7ff {
            bits ^= 1 << 52;
        }
        values.push(f64::from_bits(bits));
    }
    for _ in 0..20_000 {
        values.push(bits_range(&mut random, 0.0, 2.0));
    }
    for _ in 0..20_000 {
        values.push(bits_range(&mut random, 10.0, 95.0));
    }
    for _ in 0..20_000 {
        let m = 1 + (random.next() % 52) as u32;
        let k = random.next() % ((1_u64 << (m + 1)) + 1);
        values.push(dyadic(k, m));
    }
    for _ in 0..20_000 {
        let m = 1 + (random.next() % 46) as u32;
        let k = (10_u64 << m) + random.next() % ((85_u64 << m) + 1);
        values.push(dyadic(k, m));
    }
    for _ in 0..20_000 {
        let digits = 1 + (random.next() % 17) as u32;
        let mantissa = random.next() % 10_u64.pow(digits);
        let exponent = (random.next() % 14) as i64 - 10;
        values.push(format!("{mantissa}e{exponent}").parse().unwrap());
    }
    values
}

fn tiehunt() -> Vec<f64> {
    let mut random = SplitMix(SEED + 1);
    (0..500_000)
        .map(|index| {
            let m = 10 + (random.next() % 15) as u32;
            let mut k = random.next() % ((1_u64 << (m + 1)) + 1);
            if index % 2 == 1 {
                k = (10_u64 << m) + random.next() % ((85_u64 << m) + 1);
            }
            dyadic(k, m)
        })
        .collect()
}

#[test]
fn regenerated_streams_hash_to_go_captured_outputs() {
    let golden = golden();
    let names: Vec<&str> = golden.streams.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["families", "seeded", "tiehunt"]);
    let mut total = 0;
    for stream in &golden.streams {
        let values = match stream.name.as_str() {
            "families" => families(),
            "seeded" => seeded(),
            _ => tiehunt(),
        };
        assert_eq!(values.len(), stream.count, "{}", stream.name);
        let mut hasher = Sha256::new();
        for value in values {
            hasher.update(line(value).as_bytes());
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(digest, stream.sha256, "{}", stream.name);
        total += stream.count;
    }
    assert_eq!(total, 642_730);
}

#[test]
fn explicit_edge_and_tie_cases_match_go_exactly() {
    let golden = golden();
    assert_eq!(golden.cases.len(), 3000);
    for case in &golden.cases {
        let bits = u64::from_str_radix(&case[..16], 16).expect("hex bits");
        assert_eq!(line(f64::from_bits(bits)), format!("{case}\n"));
    }
}
