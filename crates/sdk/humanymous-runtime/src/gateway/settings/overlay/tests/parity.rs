//! Differential parity against outputs captured from the frozen Go oracle at
//! 981d25c (see the fixture's `provenance`). Expectations are never
//! regenerated to hide drift. Decode error text is this port's own, so only
//! success versus refusal is compared for decoding; all other strings, bytes,
//! classes, effective postures, engine configurations and scores are exact.
use super::super::*;
use super::TempDir;
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use humanymous_core::signals::wire::{format_fixed_go, format_float_go, json_float_go};
use humanymous_platform::testing::assert_json_equivalent;
use serde_json::Value;
use std::sync::OnceLock;

fn golden() -> &'static Value {
    static GOLDEN: OnceLock<Value> = OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/gateway/settings-overlay-go-golden.json"
        ))
        .expect("golden fixture")
    })
}

fn text(value: &Value) -> &str {
    value.as_str().expect("string")
}

fn optional_text(value: &Value) -> Option<&str> {
    value.as_str()
}

fn special(name: &str) -> f64 {
    match name {
        "NaN" => f64::NAN,
        "+Inf" => f64::INFINITY,
        "-Inf" => f64::NEG_INFINITY,
        other => panic!("unknown special {other}"),
    }
}

/// Decode an input case exactly as the capture did, then apply its
/// programmatic non-finite injections (sorted by key).
fn decode_case(case: &Value) -> Result<Overlay, OverlayDecodeError> {
    let bytes = match case.get("hex").and_then(Value::as_str) {
        Some(hex_text) => hex::decode(hex_text).unwrap(),
        None => text(&case["json"]).as_bytes().to_vec(),
    };
    let mut overlay = Overlay::from_json(&bytes)?;
    if let Some(inject) = case.get("inject").and_then(Value::as_object) {
        for (key, value) in inject {
            let value = special(text(value));
            match key.as_str() {
                "challengeAt" => overlay.scoring.get_or_insert_default().challenge_at = Some(value),
                "denyAt" => overlay.scoring.get_or_insert_default().deny_at = Some(value),
                "layerCap" => overlay.scoring.get_or_insert_default().layer_cap = Some(value),
                weight => {
                    let id = weight.strip_prefix("weight:").expect("weight injection");
                    overlay.weight_multipliers.insert(id.into(), value);
                }
            }
        }
    }
    Ok(overlay)
}

fn boot_of(value: &Value) -> BootInput {
    BootInput {
        global_monitor: value["globalMonitor"].as_bool().unwrap_or(false),
        kill_switch: value["killSwitch"].as_bool().unwrap_or(false),
        routes: value["routes"]
            .as_object()
            .map(|routes| {
                routes
                    .iter()
                    .map(|(prefix, preset)| (prefix.clone(), text(preset).to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
        rate_window_sec: value["rateWindowSec"].as_i64().unwrap(),
        rate_soft: value["rateSoft"].as_i64().unwrap(),
        rate_hard: value["rateHard"].as_i64().unwrap(),
        hmac_key: text(&value["key"]).as_bytes().to_vec(),
    }
}

fn overlay_named(name: &str) -> Option<Overlay> {
    if name.is_empty() {
        return None;
    }
    let case = golden()["inputs"]["overlays"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap_or_else(|| panic!("no overlay {name}"));
    Some(decode_case(case).unwrap())
}

/// Compare a Go `{"ok": text}` / `{"err": message}` capture.
fn assert_outcome(actual: Result<Vec<u8>, OverlayEncodeError>, expected: &Value, label: &str) {
    match actual {
        Ok(bytes) => assert_eq!(
            Some(String::from_utf8(bytes).unwrap().as_str()),
            optional_text(&expected["ok"]),
            "{label}"
        ),
        Err(error) => assert_eq!(
            Some(error.to_string().as_str()),
            optional_text(&expected["err"]),
            "{label}"
        ),
    }
}

/// Go captured non-finite numbers as `%v` strings and finite ones as numbers.
fn assert_float(actual: f64, expected: &Value, label: &str) {
    match expected.as_str() {
        Some(spelling) => assert_eq!(format_float_go(actual), spelling, "{label}"),
        None => assert_eq!(
            actual.to_bits(),
            expected.as_f64().unwrap().to_bits(),
            "{label}"
        ),
    }
}

fn assert_engine(engine: &Engine, expected: &Value, label: &str) {
    let policy = &expected["policy"];
    assert_eq!(engine.policy.version, text(&policy["Version"]), "{label}");
    assert_float(engine.policy.challenge_at, &policy["ChallengeAt"], label);
    assert_float(engine.policy.deny_at, &policy["DenyAt"], label);
    assert_float(engine.policy.layer_cap, &policy["LayerCap"], label);
    let rule_modes: BTreeMap<String, String> =
        serde_json::from_value(expected["ruleModes"].clone()).unwrap_or_default();
    assert_eq!(engine.rule_modes, rule_modes, "{label}");
    assert_eq!(
        engine.weight_multipliers.is_empty(),
        expected["weightsNil"].as_bool().unwrap(),
        "{label}"
    );
    let weights = expected["weights"].as_object().cloned().unwrap_or_default();
    assert_eq!(engine.weight_multipliers.len(), weights.len(), "{label}");
    for (id, value) in &weights {
        assert_float(engine.weight_multipliers[id], value, label);
    }
    assert_eq!(
        engine.net_policy.is_none(),
        expected["netNil"].as_bool().unwrap(),
        "{label}"
    );
    let net: Option<BTreeMap<String, String>> =
        serde_json::from_value(expected["net"].clone()).unwrap();
    assert_eq!(engine.net_policy, net, "{label}");
}

#[test]
fn go_float_text_matches_oracle_captures() {
    let floats = golden()["go"]["floats"].as_array().unwrap();
    assert_eq!(floats.len(), 39);
    for entry in floats {
        let value = f64::from_bits(u64::from_str_radix(text(&entry["bits"]), 16).unwrap());
        assert_eq!(format_float_go(value), text(&entry["v"]), "{entry}");
        assert_eq!(format_fixed_go(value, 1), text(&entry["f1"]), "{entry}");
        assert_eq!(
            json_float_go(value).as_deref(),
            optional_text(&entry["json"]),
            "{entry}"
        );
    }
}

#[test]
fn overlay_decode_validate_encode_resolve_classify_and_engine_match_go() {
    let inputs = &golden()["inputs"];
    let cases = inputs["overlays"].as_array().unwrap();
    let outputs = golden()["go"]["overlays"].as_array().unwrap();
    assert_eq!(cases.len(), outputs.len());
    assert_eq!(cases.len(), 192);
    let boots: Vec<(String, BootInput)> = inputs["boots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|boot| (text(&boot["name"]).to_owned(), boot_of(boot)))
        .collect();
    let currents: Vec<(String, Option<Overlay>)> = inputs["currents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| (text(name).to_owned(), overlay_named(text(name))))
        .collect();
    let mut decoded = 0;
    let mut classified = 0;
    for (case, expected) in cases.iter().zip(outputs) {
        let name = text(&case["name"]);
        assert_eq!(name, text(&expected["name"]));
        let overlay = decode_case(case);
        assert_eq!(
            overlay.is_ok(),
            expected["decodeErr"].is_null(),
            "{name}: decode outcome {overlay:?}"
        );
        let Ok(overlay) = overlay else { continue };
        decoded += 1;
        let validation = validate(Some(&overlay))
            .err()
            .map(|error| error.to_string());
        if name.starts_with("multi_") {
            // Go's map iteration picks which invalid gate entry is reported, so
            // both sides must report one of the single-entry refusals.
            let possible: Vec<String> = overlay
                .gates
                .iter()
                .filter_map(|(id, mode)| {
                    let single = Overlay {
                        gates: BTreeMap::from([(id.clone(), mode.clone())]),
                        ..overlay.clone()
                    };
                    validate(Some(&single)).err().map(|error| error.to_string())
                })
                .collect();
            assert!(possible.len() > 1, "{name}: {possible:?}");
            let go = text(&expected["validate"]);
            assert!(possible.iter().any(|message| message == go), "{name}: {go}");
            let rust = validation.expect("multi-entry refusal");
            assert!(possible.contains(&rust), "{name}: {rust}");
        } else {
            assert_eq!(
                validation.as_deref(),
                optional_text(&expected["validate"]),
                "{name}"
            );
        }
        assert_outcome(
            Overlay::canonical_json(Some(&overlay)),
            &expected["canonical"],
            name,
        );
        assert_eq!(
            overlay_digest(Some(&overlay), b"k"),
            text(&expected["digest"]),
            "{name}"
        );
        assert_outcome(overlay.to_json_indent(), &expected["indent"], name);
        let resolved = expected["resolve"].as_array().unwrap();
        assert_eq!(resolved.len(), boots.len());
        for ((boot_name, boot), expected) in boots.iter().zip(resolved) {
            let label = format!("{name}/{boot_name}");
            assert_eq!(boot_name, text(&expected["boot"]));
            let effective = resolve(boot, Some(&overlay));
            assert_outcome(effective.to_json(), &expected["effective"], &label);
            assert_eq!(
                effective.hr_mode("HR-12").as_str(),
                text(&expected["hr12"]),
                "{label}"
            );
            assert_eq!(
                effective.gate_mode("gate.ban").as_str(),
                text(&expected["gateBan"]),
                "{label}"
            );
            assert_eq!(
                effective.gate_mode("gate.nope").as_str(),
                text(&expected["gateUnknown"]),
                "{label}"
            );
            let mut engine = Engine::default();
            configure_engine(&mut engine, &effective);
            assert_engine(&engine, &expected["engine"], &label);
        }
        let classes = expected["classify"].as_array().unwrap();
        let mut classes = classes.iter();
        for (boot_name, boot) in &boots {
            for (current_name, current) in &currents {
                let expected = classes.next().expect("classification");
                assert_eq!(boot_name, text(&expected["boot"]));
                assert_eq!(current_name, text(&expected["current"]));
                let base = resolve(boot, current.as_ref());
                assert_eq!(
                    classify(&base, Some(&overlay)).as_str(),
                    text(&expected["class"]),
                    "{name}/{boot_name}/{current_name}"
                );
                classified += 1;
            }
        }
        assert!(classes.next().is_none());
    }
    assert_eq!((decoded, classified), (157, 1884));
    let nil = &golden()["go"]["nilOverlay"];
    assert_outcome(Overlay::canonical_json(None), &nil["canonical"], "nil");
    assert_eq!(overlay_digest(None, b"k"), text(&nil["digest"]));
    assert!(validate(None).is_ok() && nil["validate"].is_null());
    assert_eq!(
        classify(&resolve(&boots[0].1, None), None).as_str(),
        text(&nil["classify"])
    );
}

#[test]
fn engine_scores_under_resolved_overlays_match_go() {
    let inputs = &golden()["inputs"];
    let boot = boot_of(&inputs["boots"][0]);
    let scored = golden()["go"]["scored"].as_array().unwrap();
    assert_eq!(scored.len(), 32);
    let mut expected = scored.iter();
    for report in inputs["reports"].as_array().unwrap() {
        for overlay_name in inputs["scored"].as_array().unwrap() {
            let expected = expected.next().unwrap();
            assert_eq!(report["name"], expected["report"]);
            assert_eq!(overlay_name, &expected["overlay"]);
            let overlay = overlay_named(text(overlay_name));
            let mut engine = Engine::default();
            configure_engine(&mut engine, &resolve(&boot, overlay.as_ref()));
            let mut session: SessionReport =
                serde_json::from_value(report["report"].clone()).unwrap();
            let (result, trace) = engine.score_with_trace(&mut session).unwrap();
            assert_json_equivalent(&serde_json::to_value(&result).unwrap(), &expected["result"]);
            assert_json_equivalent(&serde_json::to_value(trace).unwrap(), &expected["trace"]);
        }
    }
}

#[test]
fn file_store_scenarios_match_go() {
    let stores = golden()["inputs"]["stores"].as_array().unwrap();
    let outputs = golden()["go"]["stores"].as_array().unwrap();
    assert_eq!(stores.len(), 19);
    for (scenario, expected) in stores.iter().zip(outputs) {
        let name = text(&scenario["name"]);
        assert_eq!(name, text(&expected["name"]));
        let directory = TempDir::new("parity");
        let root = directory.path().join("settings");
        let file = |which: &str| {
            root.join(match which {
                "active" => "settings.overlay.v1.json",
                _ => "settings.overlay.lkg.json",
            })
        };
        let mut store = None;
        let results = expected["results"].as_array().unwrap();
        for (step, expected) in scenario["steps"].as_array().unwrap().iter().zip(results) {
            let label = format!("{name}/{}", text(&step["op"]));
            match text(&step["op"]) {
                "write" => {
                    std::fs::create_dir_all(&root).unwrap();
                    std::fs::write(file(text(&step["file"])), text(&step["bytes"])).unwrap();
                }
                "mkdir" => std::fs::create_dir_all(file(text(&step["file"]))).unwrap(),
                "open" => {
                    let opened = OverlayFileStore::open(&root).unwrap();
                    assert!(expected["err"].is_null(), "{label}");
                    match opened.active() {
                        None => assert!(expected["active"].is_null(), "{label}"),
                        Some(active) => {
                            assert_outcome(active.to_json_indent(), &expected["active"], &label)
                        }
                    }
                    let go_error = optional_text(&expected["loadErr"]);
                    let error = opened.load_error();
                    assert_eq!(error.is_some(), go_error.is_some(), "{label}: {error:?}");
                    match text(&expected["loadKind"]) {
                        "decode" => {
                            assert_eq!(opened.load_error_kind(), Some("decode"), "{label}");
                            let recovered = go_error
                                .unwrap()
                                .starts_with("active corrupt; loaded LKG: ");
                            assert_eq!(
                                error.unwrap().starts_with("active corrupt; loaded LKG: "),
                                recovered,
                                "{label}"
                            );
                        }
                        // Go wraps an OS read failure; its text embeds a temp path.
                        "other" if go_error.is_some_and(|text| text.starts_with("read ")) => {
                            assert_eq!(opened.load_error_kind(), Some("io"), "{label}");
                        }
                        _ => assert_eq!(error.as_deref(), go_error, "{label}"),
                    }
                    store = Some(opened);
                }
                "set" => {
                    let mut overlay = overlay_named(text(&step["overlay"])).unwrap();
                    let outcome = store.as_ref().unwrap().set_active(Some(&mut overlay));
                    assert_eq!(
                        outcome.err().map(|error| error.to_string()).as_deref(),
                        optional_text(&expected["err"]),
                        "{label}"
                    );
                    assert_eq!(overlay.status, text(&expected["status"]), "{label}");
                }
                "clear" => {
                    store.as_ref().unwrap().set_active(None).unwrap();
                    assert!(expected["err"].is_null(), "{label}");
                }
                "files" => {
                    for which in ["active", "lkg"] {
                        let bytes = std::fs::read_to_string(file(which)).ok();
                        assert_eq!(
                            bytes.as_deref(),
                            optional_text(&expected[which]),
                            "{label}/{which}"
                        );
                    }
                }
                other => panic!("unknown step {other}"),
            }
        }
    }
    assert_eq!(
        OverlayFileStore::open("")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        optional_text(&golden()["go"]["emptyDir"])
    );
}

#[test]
fn schema_bytes_match_go_with_sorted_hard_rule_set() {
    let expected = text(&golden()["go"]["schema"]);
    // The oracle builds this one list from a Go map; compare it as a sorted set.
    let marker = "\"integrityCriticalHardRules\":[";
    let start = expected.find(marker).unwrap() + marker.len();
    let end = start + expected[start..].find(']').unwrap();
    let mut rules: Vec<&str> = expected[start..end].split(',').collect();
    rules.sort_unstable();
    let normalized = format!(
        "{}{}{}",
        &expected[..start],
        rules.join(","),
        &expected[end..]
    );
    assert_eq!(String::from_utf8(schema_json()).unwrap(), normalized);
}
