//! Fixed-corpus canonical A/B observation; no product or allocator qualification.
#![forbid(unsafe_code)]
use humanymous_integrity::request::{
    CanonicalInput, SIGNED_HEADERS, canonical, compute_token, lower_header_map, seed, session_key,
    verify_token,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    hint::black_box,
    process::ExitCode,
    time::{Duration, Instant},
};

const FIXTURES: &str = include_str!("../tests/fixtures/request.json");
const ITERATIONS: usize = 256;
type Implementation = for<'a> fn(CanonicalInput<'a>) -> String;
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
struct Oracle {
    key: String,
    session: String,
    counter: u64,
    bucket: u64,
    headers: BTreeMap<String, String>,
    lowered: BTreeMap<String, String>,
    cookie: String,
    body_hex: String,
    canonical: String,
    session_key: String,
    seed: String,
    token: String,
}
struct Prepared {
    headers: BTreeMap<String, String>,
    cookie: String,
    body: Vec<u8>,
    expected: String,
}
impl Prepared {
    fn input(&self) -> CanonicalInput<'_> {
        CanonicalInput {
            headers: &self.headers,
            cookie_id: &self.cookie,
            body: &self.body,
        }
    }
}
// Frozen pre-optimization oracle, confined to this benchmark example.
fn baseline(input: CanonicalInput<'_>) -> String {
    let mut result = String::new();
    for name in SIGNED_HEADERS {
        result.push_str(name);
        result.push(':');
        let value = input
            .headers
            .get(name)
            .map(String::as_str)
            .unwrap_or_default();
        result.push_str(&value.split_whitespace().collect::<Vec<_>>().join(" "));
        result.push('\n');
    }
    result.push_str("cookie:");
    result.push_str(input.cookie_id);
    result.push_str("\nbody:");
    result.push_str(&hex::encode(Sha256::digest(input.body)));
    result
}
fn prepare() -> Result<Vec<Prepared>> {
    let oracle: Vec<Oracle> =
        serde_json::from_str(FIXTURES).map_err(|_| "fixture decode failed")?;
    if oracle.len() != 81 {
        return Err("oracle count changed");
    }
    let mut prepared = Vec::new();
    for case in oracle {
        let headers = lower_header_map(&case.headers);
        let key = session_key(case.key.as_bytes(), &case.session);
        let derived = seed(&key, case.counter);
        if headers != case.lowered
            || hex::encode(key) != case.session_key
            || hex::encode(derived) != case.seed
        {
            return Err("oracle preparation parity failed");
        }
        let value = Prepared {
            headers,
            cookie: case.cookie,
            body: hex::decode(case.body_hex).map_err(|_| "fixture body decode failed")?,
            expected: case.canonical,
        };
        if canonical(value.input()) != value.expected
            || baseline(value.input()) != value.expected
            || compute_token(&derived, &value.expected, case.counter, case.bucket) != case.token
            || !verify_token(
                &derived,
                &value.expected,
                case.counter,
                case.bucket,
                &case.token,
            )
        {
            return Err("oracle canonical/token parity failed");
        }
        prepared.push(value);
    }
    let whitespace: String = (0..=0x10ffff)
        .filter_map(char::from_u32)
        .filter(|c| c.is_whitespace())
        .collect();
    for header in [
        String::new(),
        whitespace.clone(),
        format!("{whitespace}a{whitespace}b{whitespace}"),
        "a\u{85}b\u{a0}c".into(),
        "a\u{200b}b\u{feff}e\u{301}".into(),
        "x".repeat(8192),
    ] {
        let mut value = Prepared {
            headers: SIGNED_HEADERS
                .into_iter()
                .map(|name| (name.into(), header.clone()))
                .collect(),
            cookie: "raw:\n\u{a0}".into(),
            body: vec![0, 255, 13, 10],
            expected: String::new(),
        };
        value.expected = baseline(value.input());
        let actual = canonical(value.input());
        let token = compute_token(b"fixed-fixture-seed", &value.expected, 7, 11);
        if actual != value.expected
            || !verify_token(b"fixed-fixture-seed", &actual, 7, 11, &token)
            || verify_token(b"fixed-fixture-seed", &actual, 8, 11, &token)
        {
            return Err("extra canonical/token parity failed");
        }
        prepared.push(value);
    }
    Ok(prepared)
}
fn batch(
    implementation: Implementation,
    cases: &[Prepared],
    iterations: usize,
    until: Instant,
) -> Result<(u128, usize)> {
    let implementation = black_box(implementation);
    let started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..iterations {
        if Instant::now() >= until {
            return Err("benchmark deadline exceeded");
        }
        for case in cases {
            let output = black_box(implementation(black_box(case.input())));
            bytes = bytes
                .checked_add(output.len())
                .ok_or("byte count overflow")?;
        }
    }
    Ok((started.elapsed().as_nanos(), black_box(bytes)))
}
fn run() -> Result<()> {
    if std::env::args_os().len() != 1 {
        return Err("this fixed benchmark accepts no arguments");
    }
    let cases = prepare()?;
    let until = Instant::now() + Duration::from_secs(120);
    let mut digest = Sha256::new();
    for case in &cases {
        digest.update((case.expected.len() as u64).to_be_bytes());
        digest.update(case.expected.as_bytes());
    }
    batch(baseline, &cases, 16, until)?;
    batch(canonical, &cases, 16, until)?;
    let mut observations = Vec::new();
    for pair in 0..5 {
        let baseline: Implementation = baseline;
        let candidate: Implementation = canonical;
        let order = if pair % 2 == 0 {
            [("baseline", baseline), ("candidate", candidate)]
        } else {
            [("candidate", candidate), ("baseline", baseline)]
        };
        let mut expected_bytes = None;
        for (variant, implementation) in order {
            let (elapsed_ns, bytes) = batch(implementation, &cases, ITERATIONS, until)?;
            if expected_bytes.is_some_and(|expected| expected != bytes) {
                return Err("timed byte counts disagree");
            }
            expected_bytes = Some(bytes);
            observations.push(json!({"pair":pair,"variant":variant,"elapsed_ns":elapsed_ns,"operations":cases.len()*ITERATIONS,"output_bytes":bytes}));
        }
    }
    println!(
        "{}",
        json!({"schema":"humanymous.canonical-function-comparison/0.1","oracle_cases":81,"extra_cases":cases.len()-81,"fixture_sha256":hex::encode(Sha256::digest(FIXTURES.as_bytes())),"canonical_outputs_sha256":hex::encode(digest.finalize()),"iterations_per_case":ITERATIONS,"warmup_iterations_per_case":16,"pairs":5,"deadline_seconds":120,"timing_scope":"function calls, output destruction, byte counting and one deadline check per corpus iteration; fixture preparation excluded","allocation_count":null,"product_runtime_qualified":false,"observations":observations})
    );
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::FAILURE
        }
    }
}
