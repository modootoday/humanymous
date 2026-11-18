//! Differential parity against outputs captured from the frozen Go oracle at
//! 981d25c (see the fixture's `provenance`). Expectations are never
//! regenerated to hide drift. Every captured value is compared exactly,
//! except three reported differences: the owner-accepted fail-closed
//! `Unknown` action cells, invalid UTF-8 route files (Go accepts, Rust
//! refuses) and OS error text for an unreadable file.
use super::super::*;
use super::TempDir;
use serde_json::Value;
use std::sync::OnceLock;

fn golden() -> &'static Value {
    static GOLDEN: OnceLock<Value> = OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/gateway/route-policy-go-golden.json"
        ))
        .expect("golden fixture")
    })
}

fn entries(section: &str, name: &str) -> &'static [Value] {
    golden()["outputs"][section][name]
        .as_array()
        .expect("array")
}

fn text(value: &Value) -> &str {
    value.as_str().expect("string")
}

fn string_map(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .expect("object")
        .iter()
        .map(|(key, value)| (key.clone(), text(value).to_owned()))
        .collect()
}

fn assert_policy(actual: RoutePolicy, expected: &Value, label: &str) {
    let fields = [
        ("name", Value::from(actual.name)),
        ("inject", actual.inject.into()),
        ("enforce", actual.enforce.into()),
        ("failClosed", actual.fail_closed.into()),
        ("syncScore", actual.sync_score.into()),
        ("attestFloor", actual.attest_floor.into()),
        ("modeName", actual.mode_name().into()),
    ];
    for (key, value) in &fields {
        assert_eq!(value, &expected[*key], "{label}: {key}");
    }
    assert_eq!(expected.as_object().unwrap().len(), fields.len(), "{label}");
}

#[test]
fn presets_normalization_and_segment_matching_match_go() {
    let presets = entries("gate", "presets");
    assert_eq!(presets.len(), 13);
    for case in presets {
        let name = text(&case["in"]);
        assert_policy(preset_by_name(name), &case["policy"], name);
    }
    let normalize = entries("gate", "normalize");
    assert_eq!(normalize.len(), 54);
    for case in normalize {
        let input = text(&case["in"]);
        assert_eq!(normalize_match_path(input), text(&case["out"]), "{input:?}");
    }
    let segment = entries("gate", "segment");
    assert_eq!(segment.len(), 13 * 13);
    for case in segment {
        let (path, prefix) = (text(&case["path"]), text(&case["prefix"]));
        assert_eq!(
            segment_match(path, prefix),
            case["match"].as_bool().unwrap(),
            "{path:?} {prefix:?}"
        );
    }
}

#[test]
fn route_resolution_matches_go() {
    let tables: BTreeMap<&str, BTreeMap<String, String>> = golden()["inputs"]["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| (text(&table["name"]), string_map(&table["routes"])))
        .collect();
    let resolve = entries("gate", "resolve");
    assert_eq!(resolve.len(), 14 * 2 * 43);
    let mut ties = 0;
    for case in resolve {
        let table = RouteTable {
            routes: tables[text(&case["table"])].clone(),
            global_monitor: case["globalMonitor"].as_bool().unwrap(),
        };
        let path = text(&case["path"]);
        let label = format!("{} {} {path:?}", case["table"], case["globalMonitor"]);
        let actual = table.resolve(path);
        if case["tie"].as_bool().unwrap() {
            // Go picks among equal-length matches by map order; Rust's
            // byte-order pick must be one of the tied presets.
            ties += 1;
            let normalized = normalize_match_path(path);
            let best = table
                .routes
                .keys()
                .filter(|prefix| segment_match(&normalized, prefix))
                .map(String::len)
                .max()
                .unwrap();
            assert!(
                table
                    .routes
                    .iter()
                    .any(|(prefix, preset)| prefix.len() == best
                        && segment_match(&normalized, prefix)
                        && preset_by_name(preset).name == actual.name),
                "{label}"
            );
            continue;
        }
        assert_policy(actual, &case["policy"], &label);
    }
    assert_eq!(ties, 4, "only the slashes table ties");
}

fn verdict_of(name: &str) -> Verdict {
    match name {
        "allow" => Verdict::Allow,
        "challenge" => Verdict::Challenge,
        "deny" => Verdict::Deny,
        "none" => Verdict::Unknown,
        other => panic!("unknown verdict {other}"),
    }
}

/// Every cell equals Go except `Unknown` on a safe method of a route that is
/// neither fail-closed nor sync-scored, where Go passed and Rust challenges
/// (owner decisions 2026-11-16 and 2026-11-18 C1).
#[test]
fn edge_actions_match_go_except_the_accepted_fail_closed_cells() {
    let actions = entries("gate", "actions");
    assert_eq!(actions.len(), 4 * 5 * 2);
    let mut differences = Vec::new();
    for case in actions {
        let verdict = verdict_of(text(&case["verdict"]));
        let route = preset_by_name(text(&case["preset"]));
        let unsafe_method = case["unsafe"].as_bool().unwrap();
        let actual = edge_action(verdict, unsafe_method);
        let go = (text(&case["appliedEventType"]), text(&case["action"]));
        if (actual.event_type, actual.action) == go {
            continue;
        }
        assert_eq!(verdict, Verdict::Unknown, "{case}");
        assert!(
            !unsafe_method && !route.fail_closed && !route.sync_score,
            "{case}"
        );
        assert_eq!(go, (EVENT_ENF_ALLOW, "pass"), "{case}");
        assert_eq!(
            (actual.event_type, actual.action),
            (EVENT_ENF_CHALLENGE_ISSUED, "challenge_pow"),
            "{case}"
        );
        differences.push(route.name);
    }
    assert_eq!(differences, ["balanced", "monitor", "off"]);
}

#[test]
fn engine_verdict_and_method_classification_match_go() {
    for case in entries("gate", "mapVerdict") {
        let input = text(&case["in"]);
        assert_eq!(
            Verdict::from_engine(input).as_str(),
            text(&case["verdict"]),
            "{input:?}"
        );
    }
    for case in entries("gate", "methods") {
        let input = text(&case["in"]);
        assert_eq!(
            is_unsafe_method(input),
            case["unsafe"].as_bool().unwrap(),
            "{input:?}"
        );
    }
}

/// Rebuild a route file body exactly as the capture program did.
fn body_of(case: &Value) -> Vec<u8> {
    let mut body = match case.get("hex").and_then(Value::as_str) {
        Some(hex_text) => hex::decode(hex_text).unwrap(),
        None => text(&case["body"]).as_bytes().to_vec(),
    };
    if let Some(long) = case.get("append_long") {
        let length = usize::try_from(long["len"].as_u64().unwrap()).unwrap();
        let carriage = long["cr"].as_bool().unwrap_or(false);
        body.push(b'/');
        body.extend(std::iter::repeat_n(
            b'a',
            length - 1 - 7 - usize::from(carriage),
        ));
        body.extend_from_slice(b" strict");
        if carriage {
            body.push(b'\r');
        }
        if long["newline"].as_bool().unwrap() {
            body.push(b'\n');
        }
    }
    body
}

#[test]
fn route_files_load_like_go() {
    let inputs = golden()["inputs"]["route_files"].as_array().unwrap();
    let outputs = entries("cmdGate", "routeFiles");
    assert_eq!(inputs.len(), outputs.len());
    let directory = TempDir::new("parity");
    for (input, expected) in inputs.iter().zip(outputs) {
        let name = text(&input["name"]);
        assert_eq!(name, text(&expected["name"]));
        let path = directory.write(&body_of(input));
        let actual = load_routes(&path);
        if name == "invalid_utf8" {
            // Reported difference: Go accepted the invalid byte as a prefix.
            assert_eq!(string_map(&expected["ok"]).len(), 1);
            assert!(matches!(
                actual,
                Err(RoutesError::NotUtf8 { line_number: 1 })
            ));
            continue;
        }
        match (actual, expected["ok"].is_object()) {
            (Ok(routes), true) => assert_eq!(routes, string_map(&expected["ok"]), "{name}"),
            (Err(error), false) => assert_eq!(
                error.to_string(),
                text(&expected["err"]).replace("{path}", &path.display().to_string()),
                "{name}"
            ),
            (actual, _) => panic!("{name}: Go {expected}, Rust {actual:?}"),
        }
    }
    // OS error text is the platform's, not Go's; only refusal is compared.
    assert!(text(&golden()["outputs"]["cmdGate"]["missingFileErr"]).starts_with("open {path}"));
    assert!(matches!(
        load_routes(&path_missing(&directory)),
        Err(RoutesError::Io(_))
    ));
}

fn path_missing(directory: &TempDir) -> std::path::PathBuf {
    directory.write(b"").with_file_name("missing.conf")
}

#[test]
fn attested_route_preconditions_match_go() {
    let tables: BTreeMap<&str, BTreeMap<String, String>> = golden()["inputs"]["attested"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| (text(&table["name"]), string_map(&table["routes"])))
        .collect();
    let cases = entries("cmdGate", "attested");
    assert_eq!(cases.len(), tables.len() * 4);
    for case in cases {
        let name = text(&case["name"]);
        let shared = case["sharedTokenKey"].as_bool().unwrap();
        let verifier = case["hasVerifier"].as_bool().unwrap();
        let label = format!("{name} shared={shared} verifier={verifier}");
        match validate_attested_routes(&tables[name], shared, verifier) {
            Ok(mut warnings) => {
                warnings.sort();
                let expected: Vec<&str> = case["warnings"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{label}: Go refused"))
                    .iter()
                    .map(text)
                    .collect();
                assert_eq!(warnings, expected, "{label}");
            }
            // With two attested prefixes Go names either one (map order);
            // both captured runs named "/a", which is also Rust's pick.
            Err(error) => assert_eq!(error.to_string(), text(&case["err"]), "{label}"),
        }
    }
}
