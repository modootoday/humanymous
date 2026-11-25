//! Human-attested `human-control` receipts for the human tester hosting
//! profile (`deployments/lab/compose.human-test.yaml`).
//!
//! A person performs the runbook steps with their own browser, keyboard and
//! assistive technology and fills a record by hand. This command only checks
//! that record against the closed schema and binds its digest into a receipt
//! labelled `human-attested`. It never drives a browser, never fills in or
//! infers a step outcome, and refuses to run without an interactive terminal
//! or inside CI, so it cannot be wired into an automated pipeline by accident.
//! The receipt is not a measured product run: it never reports `passed`, never
//! sets `full_product_qualified` and never clears the runner's
//! `required_unavailable` entry; that join stays an owner decision.
use crate::docker::Result;
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, IsTerminal, Write},
    path::Path,
};

pub const RECORD_SCHEMA: &str = "humanymous.human-control-record/1";
pub const RECEIPT_SCHEMA: &str = "humanymous.lab-human-control-receipt/1";
const RECORD_MAX: usize = 64 * 1024;
const NOTE_MAX: usize = 280;
const FIELD_MAX: usize = 64;
const CONFIRMATION: &str = "I performed these steps myself";

/// Every step the runbook defines, with the container-lab assertion it
/// informs. All of them must appear exactly once so none is silently omitted.
pub const STEPS: &[(&str, Option<&str>)] = &[
    ("ca-trusted", None),
    ("gate-reached", None),
    ("ordinary-pass", Some("human-allow-control")),
    ("pass-keyboard-only", Some("keyboard-challenge")),
    ("pass-screen-reader", Some("assistive-challenge")),
    ("pass-zero-pointer", Some("zero-motor-control")),
];
const OUTCOMES: &[&str] = &["passed", "failed", "blocked", "not_run"];
const INPUTS: &[&str] = &[
    "keyboard",
    "pointer",
    "touch",
    "screen-reader",
    "magnifier",
    "switch",
    "voice",
];
/// Environment variables that mark an automated runner.
const AUTOMATION_MARKERS: &[&str] = &["CI", "GITHUB_ACTIONS", "TF_BUILD", "BUILDKITE", "GITLAB_CI"];

/// Short free text with no contact details or links.
fn plain(value: &Value, max: usize, allow_empty: bool) -> bool {
    value.as_str().is_some_and(|text| {
        (allow_empty || !text.trim().is_empty())
            && text.chars().count() <= max
            && !text.chars().any(char::is_control)
            && !text.contains('@')
            && !text.contains("://")
    })
}

fn exact_keys(object: &Map<String, Value>, required: &[&str], optional: &[&str]) -> bool {
    required.iter().all(|key| object.contains_key(*key))
        && object
            .keys()
            .all(|key| required.contains(&key.as_str()) || optional.contains(&key.as_str()))
}

fn date(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        let bytes = text.as_bytes();
        bytes.len() == 10
            && bytes.iter().enumerate().all(|(index, byte)| match index {
                4 | 7 => *byte == b'-',
                _ => byte.is_ascii_digit(),
            })
            && matches!(text[5..7].parse::<u8>(), Ok(1..=12))
            && matches!(text[8..10].parse::<u8>(), Ok(1..=31))
    })
}

fn sha(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 40
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn environment(value: &Value) -> std::result::Result<(), &'static str> {
    let object = value.as_object().ok_or("environment must be an object")?;
    if !exact_keys(
        object,
        &["os", "browser", "assistive_technology", "input_methods"],
        &[],
    ) {
        return Err(
            "environment fields are closed: os, browser, assistive_technology, input_methods",
        );
    }
    if !plain(&object["os"], FIELD_MAX, false) || !plain(&object["browser"], FIELD_MAX, false) {
        return Err("os and browser are short product/version text");
    }
    let tools = object["assistive_technology"]
        .as_array()
        .ok_or("assistive_technology must be an array")?;
    if tools.len() > 4
        || !tools.iter().all(|tool| {
            tool.as_object().is_some_and(|tool| {
                exact_keys(tool, &["name", "version"], &[])
                    && plain(&tool["name"], FIELD_MAX, false)
                    && plain(&tool["version"], FIELD_MAX, false)
            })
        })
    {
        return Err("assistive_technology entries are {name, version}, at most four");
    }
    let inputs = object["input_methods"]
        .as_array()
        .ok_or("input_methods must be an array")?;
    let mut seen = std::collections::BTreeSet::new();
    if inputs.is_empty()
        || !inputs.iter().all(|input| {
            input
                .as_str()
                .is_some_and(|input| INPUTS.contains(&input) && seen.insert(input))
        })
    {
        return Err("input_methods must be distinct known input methods");
    }
    Ok(())
}

/// Validate a hand-filled record. Returns the steps in canonical order.
pub fn validate(record: &Value) -> std::result::Result<Vec<(String, String)>, &'static str> {
    let object = record.as_object().ok_or("record must be a JSON object")?;
    if !exact_keys(
        object,
        &[
            "schema",
            "date",
            "source_sha",
            "venue",
            "environment",
            "steps",
        ],
        &[],
    ) {
        return Err(
            "record fields are closed: schema, date, source_sha, venue, environment, steps",
        );
    }
    if object["schema"] != RECORD_SCHEMA {
        return Err("record schema must be humanymous.human-control-record/1");
    }
    if !date(&object["date"]) {
        return Err("date must be YYYY-MM-DD (no time of day)");
    }
    if !sha(&object["source_sha"]) {
        return Err("source_sha must be the 40-character lowercase commit the lab was built from");
    }
    if !matches!(object["venue"].as_str(), Some("local" | "provider-fronted")) {
        return Err("venue must be local or provider-fronted");
    }
    environment(&object["environment"])?;
    let steps = object["steps"].as_array().ok_or("steps must be an array")?;
    let mut outcomes = std::collections::BTreeMap::new();
    for step in steps {
        let step = step.as_object().ok_or("each step must be an object")?;
        if !exact_keys(step, &["id", "outcome"], &["note"]) {
            return Err("step fields are closed: id, outcome, optional note");
        }
        let id = step["id"].as_str().ok_or("step id must be text")?;
        if !STEPS.iter().any(|(known, _)| *known == id) {
            return Err("unknown step id");
        }
        let outcome = step["outcome"]
            .as_str()
            .ok_or("step outcome must be text")?;
        if !OUTCOMES.contains(&outcome) {
            return Err("outcome must be passed, failed, blocked or not_run");
        }
        if step
            .get("note")
            .is_some_and(|note| !plain(note, NOTE_MAX, true))
        {
            return Err("notes are at most 280 characters with no contact details or links");
        }
        if outcomes.insert(id, outcome).is_some() {
            return Err("duplicate step id");
        }
    }
    STEPS
        .iter()
        .map(|(id, _)| {
            outcomes
                .get(id)
                .map(|outcome| ((*id).to_owned(), (*outcome).to_owned()))
                .ok_or("every runbook step must be recorded, use not_run when skipped")
        })
        .collect()
}

/// Build the receipt for an already validated record and its exact bytes.
pub fn receipt(record: &Value, bytes: &[u8]) -> Result<Value> {
    let steps = validate(record)?;
    let venue = record["venue"].as_str().ok_or("venue")?;
    let complete = steps.iter().all(|(_, outcome)| outcome == "passed");
    let mut assertions = Map::new();
    let mut listed = Vec::new();
    for (id, outcome) in &steps {
        let assertion = STEPS
            .iter()
            .find(|(known, _)| known == id)
            .and_then(|(_, assertion)| *assertion);
        if let Some(assertion) = assertion {
            assertions.insert(assertion.into(), json!(outcome));
        }
        listed.push(json!({"id":id,"outcome":outcome,"assertion":assertion}));
    }
    let provider = (venue == "provider-fronted").then_some(
        "Cloudflare terminates and re-originates TLS; raw TLS/H2 observations are provider facts, not Gate facts",
    );
    Ok(json!({
        "schema": RECEIPT_SCHEMA,
        "case": "human-control",
        "attestation": "human-attested",
        "produced_by": "human-record",
        "automated": false,
        "automation_baseline": false,
        "evidence_label": format!("human-attested/{venue}"),
        "venue": venue,
        "provider_note": provider,
        "date": record["date"],
        "source_sha": record["source_sha"],
        "record_sha256": hex::encode(Sha256::digest(bytes)),
        "environment": record["environment"],
        "steps": listed,
        "assertions": assertions,
        "status": if complete { "attested_complete" } else { "attested_incomplete" },
        "full_product_qualified": false,
        "release_ready": false,
        "clears_required_unavailable": false,
        "limits": [
            "a person's attestation, not a measured or reproducible product run",
            "the edge TLS terminator owns the browser handshake; no raw TLS/H2 Gate claim",
            "unwitnessed Gate; authority/Core absent",
            "one tester, one environment; not a population or accessibility certification"
        ]
    }))
}

fn automation_marker(lookup: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    AUTOMATION_MARKERS
        .iter()
        .copied()
        .find(|name| lookup(name).is_some_and(|value| !value.is_empty()))
}

pub fn run(args: &[String]) -> Result<i32> {
    let (mut record, mut emit) = (None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--record" => record = rest.next(),
            "--emit" => emit = rest.next(),
            _ => return Err("usage: human-control-receipt --record PATH --emit PATH".into()),
        }
    }
    let (Some(record), Some(emit)) = (record, emit) else {
        return Err("usage: human-control-receipt --record PATH --emit PATH".into());
    };
    if let Some(marker) = automation_marker(|name| std::env::var(name).ok()) {
        return Err(format!("refused under automation marker {marker}: human-control receipts are human-attested only").into());
    }
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Err(
            "refused without an interactive terminal: the tester confirms in person".into(),
        );
    }
    let emit = Path::new(emit);
    if emit.exists() {
        return Err("refusing to overwrite an existing receipt".into());
    }
    let bytes = fs::read(record)?;
    let value = decode_strict_json(&bytes, RECORD_MAX)
        .map_err(|_| "record is not strict JSON (duplicate keys or over 64 KiB)")?;
    let receipt = receipt(&value, &bytes)?;
    let mut stderr = std::io::stderr();
    writeln!(
        stderr,
        "Record valid: {} at venue {}.\nType exactly \"{CONFIRMATION}\" to attest:",
        receipt["status"], receipt["venue"]
    )?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if line.trim_end_matches(['\r', '\n']) != CONFIRMATION {
        return Err("attestation not confirmed; no receipt written".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(emit)?;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    println!(
        "{}",
        json!({"emitted":emit.display().to_string(),"status":receipt["status"],"attestation":"human-attested"})
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../human-control-record.example.json");

    fn example() -> Value {
        decode_strict_json(EXAMPLE.as_bytes(), RECORD_MAX).unwrap()
    }

    fn all_passed() -> Value {
        let mut record = example();
        for step in record["steps"].as_array_mut().unwrap() {
            step["outcome"] = json!("passed");
        }
        record
    }

    #[test]
    fn shipped_example_is_valid_and_reports_nothing_run() {
        let record = example();
        let steps = validate(&record).unwrap();
        assert_eq!(steps.len(), STEPS.len());
        let receipt = receipt(&record, EXAMPLE.as_bytes()).unwrap();
        assert_eq!(receipt["status"], "attested_incomplete");
        assert_eq!(receipt["assertions"]["keyboard-challenge"], "not_run");
    }

    #[test]
    fn receipt_is_labelled_human_attested_and_never_qualifies() {
        let record = all_passed();
        let bytes = serde_json::to_vec(&record).unwrap();
        let receipt = receipt(&record, &bytes).unwrap();
        assert_eq!(receipt["schema"], RECEIPT_SCHEMA);
        assert_eq!(receipt["case"], "human-control");
        assert_eq!(receipt["attestation"], "human-attested");
        assert_eq!(receipt["produced_by"], "human-record");
        assert_eq!(receipt["automated"], false);
        assert_eq!(receipt["automation_baseline"], false);
        assert_eq!(receipt["status"], "attested_complete");
        assert_ne!(receipt["status"], "passed");
        assert_eq!(receipt["full_product_qualified"], false);
        assert_eq!(receipt["release_ready"], false);
        assert_eq!(receipt["clears_required_unavailable"], false);
        assert_eq!(receipt["evidence_label"], "human-attested/local");
        assert_eq!(receipt["provider_note"], Value::Null);
        assert_eq!(
            receipt["record_sha256"],
            hex::encode(Sha256::digest(&bytes))
        );
        for assertion in [
            "human-allow-control",
            "keyboard-challenge",
            "assistive-challenge",
            "zero-motor-control",
        ] {
            assert_eq!(receipt["assertions"][assertion], "passed");
        }
    }

    #[test]
    fn provider_fronted_venue_carries_the_provider_fact_label() {
        let mut record = all_passed();
        record["venue"] = json!("provider-fronted");
        let receipt = receipt(&record, b"bytes").unwrap();
        assert_eq!(receipt["evidence_label"], "human-attested/provider-fronted");
        assert!(
            receipt["provider_note"]
                .as_str()
                .unwrap()
                .contains("provider facts")
        );
    }

    #[test]
    fn notes_never_reach_the_receipt() {
        let mut record = all_passed();
        record["steps"][0]["note"] = json!("Edge showed the padlock.");
        let receipt = receipt(&record, b"bytes").unwrap();
        assert!(!receipt.to_string().contains("padlock"));
    }

    #[test]
    fn closed_schema_refuses_extra_or_personal_fields() {
        let mut record = example();
        record["tester_name"] = json!("someone");
        assert!(validate(&record).is_err());
        let mut record = example();
        record["environment"]["email"] = json!("x");
        assert!(validate(&record).is_err());
        let mut record = example();
        record["steps"][0]["tester"] = json!("x");
        assert!(validate(&record).is_err());
    }

    #[test]
    fn contact_details_and_links_are_refused_in_text() {
        for note in [
            "mail me at someone@example.test",
            "see https://example.test",
            "line\nbreak",
        ] {
            let mut record = example();
            record["steps"][1]["note"] = json!(note);
            assert!(validate(&record).is_err(), "{note}");
        }
        let mut record = example();
        record["steps"][1]["note"] = json!("x".repeat(NOTE_MAX + 1));
        assert!(validate(&record).is_err());
        let mut record = example();
        record["environment"]["browser"] = json!("someone@example.test");
        assert!(validate(&record).is_err());
    }

    #[test]
    fn every_step_exactly_once_with_a_known_outcome() {
        let mut record = example();
        record["steps"].as_array_mut().unwrap().pop();
        assert!(validate(&record).is_err());
        let mut record = example();
        let first = record["steps"][0].clone();
        record["steps"].as_array_mut().unwrap().push(first);
        assert!(validate(&record).is_err());
        let mut record = example();
        record["steps"][0]["id"] = json!("invented-step");
        assert!(validate(&record).is_err());
        let mut record = example();
        record["steps"][0]["outcome"] = json!("ok");
        assert!(validate(&record).is_err());
    }

    #[test]
    fn header_fields_are_checked() {
        for (field, value) in [
            ("schema", json!("humanymous.human-control-record/2")),
            ("date", json!("2026-11-17T10:00:00Z")),
            ("date", json!("2026-13-01")),
            ("source_sha", json!("ABC")),
            ("venue", json!("cloud")),
        ] {
            let mut record = example();
            record[field] = value;
            assert!(validate(&record).is_err(), "{field}");
        }
        let mut record = example();
        record["environment"]["input_methods"] = json!([]);
        assert!(validate(&record).is_err());
        let mut record = example();
        record["environment"]["input_methods"] = json!(["keyboard", "keyboard"]);
        assert!(validate(&record).is_err());
        let mut record = example();
        record["environment"]["assistive_technology"] = json!([{"name":"NVDA"}]);
        assert!(validate(&record).is_err());
    }

    #[test]
    fn duplicate_json_keys_are_refused() {
        assert!(decode_strict_json(br#"{"venue":"local","venue":"local"}"#, RECORD_MAX).is_err());
    }

    #[test]
    fn automation_markers_refuse_the_command() {
        assert_eq!(automation_marker(|_| None), None);
        assert_eq!(
            automation_marker(|name| (name == "GITHUB_ACTIONS").then(|| "true".into())),
            Some("GITHUB_ACTIONS")
        );
        assert_eq!(
            automation_marker(|name| (name == "CI").then(|| "1".into())),
            Some("CI")
        );
        assert_eq!(automation_marker(|_| Some(String::new())), None);
    }

    #[test]
    fn run_refuses_usage_and_automation_before_reading_anything() {
        assert!(run(&["--record".into()]).is_err());
        assert!(run(&["--bogus".into()]).is_err());
    }
}
