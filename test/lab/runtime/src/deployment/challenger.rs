//! Network-none challenger for the deployed installation (roadmap 2h, lab
//! scope). It pins the public key the deployed runtime generated inside the
//! manifest volume, issues every nonce itself and verifies each relayed
//! answer against what it knows independently: the documented installation,
//! the expected configuration digest and the shipped runtime binary digest
//! (its own image copy). The runner is only transport and is not trusted.
//!
//! Protocol (JSON lines): first in `{"enrollment":DOC,"installation":ID,
//! "configuration_sha256":HEX}`; then for each row out
//! `{"role":"challenge","row":ID,"nonce":HEX}` and in `{"answer":ATTESTATION}`
//! (or `{"answer":null}` when the target refused); finally out the report.
use super::{CHALLENGER_SCHEMA, emit, read_line};
use humanymous_runtime::gateway::attestation::{
    self, Attestation, ENROLLMENT_SCHEMA, Expected, Refusal,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, BufReader};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const RUNTIME: &str = "/usr/local/bin/humanymous-runtime";

/// Relayed rows in order with the outcome the challenger must observe.
pub const ROWS: &[(&str, Option<Refusal>)] = &[
    ("deployed-answer-verified", None),
    ("stale-answer-refused", Some(Refusal::NonceMismatch)),
    ("fresh-answer-after-stale-verified", None),
    (
        "foreign-installation-key-refused",
        Some(Refusal::UnpinnedKey),
    ),
    (
        "changed-configuration-refused",
        Some(Refusal::ConfigurationMismatch),
    ),
];
/// Row the challenger derives itself from the last verified answer.
pub const TAMPERED: &str = "tampered-signature-refused";

fn outcome(result: std::result::Result<(), Refusal>) -> &'static str {
    match result {
        Ok(()) => "accepted",
        Err(refusal) => refusal.code(),
    }
}

fn nonce() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn pinned(enrollment: &Value) -> Result<[u8; 32]> {
    let public = enrollment["public_key"].as_str().ok_or("public key")?;
    if enrollment["schema"] != ENROLLMENT_SCHEMA || public.len() != 64 {
        return Err("enrollment schema".into());
    }
    let mut key = [0u8; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&public[index * 2..index * 2 + 2], 16)?;
    }
    Ok(key)
}

fn row(id: &str, expected: Option<Refusal>, observed: &str) -> Value {
    let expected = expected.map_or("accepted", Refusal::code);
    json!({"id":id,"status":if observed == expected {"passed"} else {"failed"},
        "expected":expected,"observed":observed})
}

fn execute(input: &mut impl BufRead) -> Result<Value> {
    let first = read_line(input, 65_536)?;
    let enrollment = &first["enrollment"];
    let key = pinned(enrollment)?;
    let installation = first["installation"].as_str().ok_or("installation")?;
    let configuration_sha256 = first["configuration_sha256"]
        .as_str()
        .ok_or("configuration digest")?;
    let binary_sha256 = format!("{:x}", Sha256::digest(std::fs::read(RUNTIME)?));
    let enrolled = enrollment["installation"] == installation
        && enrollment["configuration_sha256"] == configuration_sha256;
    let mut rows = vec![json!({"id":"enrollment-binds-deployed-configuration",
        "status":if enrolled {"passed"} else {"failed"}})];
    let mut previous: Option<(Attestation, String)> = None;
    let mut nonces = Vec::new();
    for (id, expected) in ROWS {
        let fresh = nonce()?;
        nonces.push(fresh.clone());
        emit(&json!({"role":"challenge","row":id,"nonce":fresh}))?;
        let relayed = read_line(input, 65_536)?;
        let answer: Option<Attestation> = serde_json::from_value(relayed["answer"].clone()).ok();
        let check = |attestation: &Attestation| {
            attestation::verify(
                &key,
                &Expected {
                    installation,
                    binary_sha256: &binary_sha256,
                    configuration_sha256,
                    nonce: &fresh,
                },
                attestation,
            )
        };
        let observed = answer.as_ref().map_or("no_answer", |a| outcome(check(a)));
        rows.push(row(id, *expected, observed));
        if observed == "accepted"
            && let Some(answer) = answer
        {
            // The tampered row uses the most recent genuine verified answer.
            if id.starts_with("fresh-answer") {
                let mut forged = answer.clone();
                let flipped = if forged.signature.starts_with('0') {
                    "1"
                } else {
                    "0"
                };
                forged.signature.replace_range(0..1, flipped);
                rows.push(row(
                    TAMPERED,
                    Some(Refusal::Signature),
                    outcome(check(&forged)),
                ));
            }
            previous = Some((answer, fresh));
        }
    }
    let distinct = {
        let mut sorted = nonces.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len() == nonces.len()
    };
    let passed = distinct
        && rows.len() == ROWS.len() + 2
        && rows.iter().all(|row| row["status"] == "passed");
    Ok(
        json!({"schema":CHALLENGER_SCHEMA,"role":"report","status":if passed {"passed"} else {"failed"},
        "pinned_public_key":enrollment["public_key"],"binary_sha256":binary_sha256,
        "configuration_sha256":configuration_sha256,"installation":installation,
        "distinct_nonces":distinct,"nonces":nonces.len(),"cases":rows,
        "last_verified_nonce":previous.map(|(_, nonce)| nonce),
        "key_generated_by":"deployed runtime image inside the manifest volume",
        "private_key_exported":false,"full_product_qualified":false}),
    )
}

pub fn run() {
    let mut input = BufReader::new(io::stdin());
    let report = execute(&mut input).unwrap_or_else(|error| {
        json!({"schema":CHALLENGER_SCHEMA,"role":"report","status":"failed","cases":[],
            "error":error.to_string(),"full_product_qualified":false})
    });
    let passed = report["status"] == "passed";
    emit(&report).expect("challenger report output");
    if !passed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_runtime::gateway::attestation::{Statement, public_key, sign};

    #[test]
    fn rows_pass_only_on_the_exact_expected_outcome() {
        assert_eq!(row("a", None, "accepted")["status"], "passed");
        assert_eq!(row("a", None, "nonce_mismatch")["status"], "failed");
        assert_eq!(
            row("a", Some(Refusal::UnpinnedKey), "unpinned_key")["status"],
            "passed"
        );
        assert_eq!(
            row("a", Some(Refusal::UnpinnedKey), "no_answer")["status"],
            "failed"
        );
        assert_eq!(ROWS.len(), 5);
    }

    #[test]
    fn pinning_reads_only_a_well_formed_enrollment() {
        let seed = [7u8; 32];
        let public = public_key(&seed);
        let enrollment = json!({"schema":ENROLLMENT_SCHEMA,"public_key":public});
        let key = pinned(&enrollment).unwrap();
        let statement = Statement {
            installation: "example-installation".into(),
            binary_sha256: "a".repeat(64),
            configuration_sha256: "b".repeat(64),
            nonce: "c".repeat(64),
            public_key: public.clone(),
        };
        let signed = sign(&seed, statement).unwrap();
        let expected = Expected {
            installation: "example-installation",
            binary_sha256: &"a".repeat(64),
            configuration_sha256: &"b".repeat(64),
            nonce: &"c".repeat(64),
        };
        assert_eq!(attestation::verify(&key, &expected, &signed), Ok(()));
        assert!(pinned(&json!({"schema":"other","public_key":public})).is_err());
        assert!(pinned(&json!({"schema":ENROLLMENT_SCHEMA,"public_key":"ab"})).is_err());
    }
}
