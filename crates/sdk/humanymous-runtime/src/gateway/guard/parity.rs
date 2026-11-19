//! Differential parity against legacy internal/gate/guard.go outputs captured
//! from the frozen Go oracle at 981d25c (fixture `provenance`): spoofScan and
//! stripInbound over `http.Header`, originAuth/originEpoch/originEpochGrace,
//! ValidateOriginAuth and the HR-27b spoofRecord (with smuggleRecord, which
//! shares its record helper). Expectations are never regenerated; each
//! accepted deviation is named and asserted explicitly.
use super::*;
use crate::gateway::smuggle::{Reason, smuggle_record, spoof_record};
use serde_json::Value;
use std::collections::BTreeMap;

fn golden() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/configsign-guard-go-golden.json"
    ))
    .expect("golden fixture");
    fixture["outputs"].clone()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}

/// Header names grouped case-insensitively with values in arrival order, the
/// shape of a Go `http.Header` after canonicalization.
fn grouped(headers: &[(String, String)]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, value) in headers {
        map.entry(name.to_ascii_lowercase())
            .or_default()
            .push(value.clone());
    }
    map
}

/// Go `spoofScan` reads `Header.Get`, the first value only; the Rust scan
/// reports a name when any of its values is non-empty. The only captured case
/// where that differs is a forged value behind an empty first value, which Go
/// missed and Rust reports (fail-closed; Go stripped it either way).
#[test]
fn spoof_scan_and_strip_match_go_capture() {
    let golden = golden();
    let cases = golden["headers"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut headers: Vec<(String, String)> = case["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let found: Vec<String> = spoof_scan(&headers)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let expected = strings(&case["found"]);
        if name == "duplicate_first_empty" {
            assert!(expected.is_empty(), "Go read only the empty first value");
            assert_eq!(found, ["X-Forwarded-For"], "Rust reports the later forgery");
        } else {
            assert_eq!(found, expected, "{name}: spoofScan");
        }
        strip_inbound(&mut headers);
        let remaining: BTreeMap<String, Vec<String>> = case["remaining"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry[0].as_str().unwrap().to_ascii_lowercase(),
                    strings(&entry[1]),
                )
            })
            .collect();
        assert_eq!(grouped(&headers), remaining, "{name}: stripInbound");
    }
}

/// Epochs, grace sets, tokens and origin-side validation equal Go byte for
/// byte. Go accepted any key length; the Rust key refuses fewer than
/// `MIN_ORIGIN_KEY` bytes (owner-accepted fail-closed difference), so the
/// captured short-material rows are asserted as refusals.
#[test]
fn origin_cloak_matches_go_capture() {
    let golden = golden();
    for case in golden["epochs"].as_array().unwrap() {
        let unix = case["unix"].as_i64().unwrap();
        assert_eq!(
            origin_epoch(unix),
            case["epoch"].as_str().unwrap(),
            "{unix}"
        );
        assert_eq!(
            origin_epoch_grace(unix).to_vec(),
            strings(&case["grace"]),
            "{unix}"
        );
    }
    let mut refused = 0;
    for case in golden["cloak"].as_array().unwrap() {
        let material = case["material"].as_str().unwrap().as_bytes().to_vec();
        let short = material.len() < MIN_ORIGIN_KEY;
        match OriginKey::new(material) {
            Ok(origin) => assert_eq!(
                origin.token(case["epoch"].as_str().unwrap()),
                case["hex"].as_str().unwrap()
            ),
            Err(ShortKey) => {
                assert!(short);
                refused += 1;
            }
        }
    }
    assert_eq!(refused, 5);
    let mut verified = 0;
    for case in golden["verify"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let material = case["material"].as_str().unwrap().as_bytes().to_vec();
        let header = case["header"].as_str().unwrap();
        let unix = case["unix"].as_i64().unwrap();
        match OriginKey::new(material) {
            Ok(origin) => {
                assert_eq!(
                    origin.valid(header, unix),
                    case["valid"].as_bool().unwrap(),
                    "{name}"
                );
                verified += 1;
            }
            Err(ShortKey) => assert_eq!(name, "short_material_current"),
        }
    }
    assert_eq!(verified, 8);
}

/// Record fields the legacy builders set (the sink stamps the rest). Go
/// writes the gate verdict vocabulary in lower case (`deny`); every Rust
/// gateway record writes `DENY`, so verdicts compare case-insensitively.
#[test]
fn spoof_and_smuggle_records_match_go_capture() {
    const FIELDS: [&str; 11] = [
        "event_type",
        "actor",
        "tenant_id",
        "session_pseudonym",
        "route_class",
        "triggered_rules",
        "enforcement_action",
        "enforcement_mode",
        "fail_mode",
        "fail_reason",
        "key_id",
    ];
    let golden = golden();
    let cases = golden["records"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let node = case["node"].as_str().unwrap();
        let psn = case["psn"].as_str().unwrap();
        let record = match case["kind"].as_str().unwrap() {
            "spoof" => {
                let found: Vec<&'static str> = strings(&case["found"])
                    .iter()
                    .map(|name| {
                        *INTERNAL_HEADERS
                            .iter()
                            .find(|known| **known == name.as_str())
                            .unwrap()
                    })
                    .collect();
                spoof_record(node, psn, &found)
            }
            "smuggle" => {
                let reason = [
                    Reason::TeClConflict,
                    Reason::AmbiguousLength,
                    Reason::TeNotChunked,
                    Reason::ObsFold,
                ]
                .into_iter()
                .find(|reason| reason.as_str() == case["reason"].as_str().unwrap())
                .unwrap();
                smuggle_record(node, psn, reason)
            }
            other => panic!("unknown record kind {other}"),
        };
        let actual = serde_json::to_value(&record).unwrap();
        let expected = &case["record"];
        for field in FIELDS {
            assert_eq!(
                actual.get(field),
                expected.get(field),
                "{}: {field}",
                case["kind"]
            );
        }
        assert!(
            actual["verdict"]
                .as_str()
                .unwrap()
                .eq_ignore_ascii_case(expected["verdict"].as_str().unwrap())
        );
        assert_eq!(expected["verdict"], "deny");
        assert_eq!(actual["verdict"], "DENY");
    }
}
