//! Roadmap 2d "session-bound real browser collector": evaluate the required
//! `collector-parity` and `missing-signals-closed` cases from what the real
//! browser-product run observed (the collect entity captured at the Gate
//! frontend, both proxies' wire counters, the authenticated offline audit and
//! the Gate's post-browser missing-signal controls). The run holds two session
//! epochs; `browser_epochs` feeds `collector-parity` the complete (hooks
//! opted in) epoch and `missing-signals-closed` the stripped one.
//!
//! The Go oracle is measured at 981d25c, never guessed:
//! - `internal/signals/signals.go` `ClientReport`/`Signal` JSON field sets;
//! - `internal/detect/{l1_static,l2_fingerprint,l3_integrity}.go`: the 17 L1-L3
//!   client signal IDs, of which only `l1.navigator.webdriver` is emitted
//!   unconditionally;
//! - `web/js/loader.js`: the posted report always carried `guard.reported=true`,
//!   a probed `environment`, a `fingerprintId` and the behavior summary;
//! - `cmd/server/handlers.go`/`collect_pipeline.go` and
//!   `internal/gate/control_collect.go`: the report's `sessionId` is the server
//!   session, never the client's;
//! - `cmd/server/rit.go`: a replayed or re-bound signed request is
//!   `l5.rit.stale_replay`/`l5.rit.header_tampered`, a tokenless one
//!   `l5.rit.absent`; `internal/gate/control_collect.go` HR-29 refuses a
//!   replayed beacon with 409;
//! - `internal/gate/enforce.go`: no evidence on an unsafe method is
//!   `challenge_pow`, never `pass`.
//!
//! A case passes only with an empty gap list; every gap is a finite label.
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const GO_ORACLE: &str = "981d25c";
/// `internal/signals/signals.go` ClientReport JSON names (no omitempty).
const GO_REPORT_KEYS: [&str; 10] = [
    "sessionId",
    "userAgent",
    "uaClientHints",
    "signals",
    "behavior",
    "environment",
    "guard",
    "advanced",
    "fingerprintId",
    "engineVersion",
];
/// `internal/signals/signals.go` Signal JSON names; the last two may be omitted.
const GO_SIGNAL_KEYS: [&str; 10] = [
    "id",
    "layer",
    "value",
    "verdict",
    "weight",
    "score",
    "confidence",
    "collected",
    "expectedHuman",
    "notes",
];
const GO_SIGNAL_REQUIRED: usize = 8;
/// Every client signal ID in `internal/detect/l{1,2,3}_*.go` at the oracle.
const GO_CLIENT_SIGNALS: [&str; 17] = [
    "l1.artifact.phantom",
    "l1.artifact.playwright",
    "l1.artifact.selenium",
    "l1.cdp.proxy_leak",
    "l1.cdp.runtime_enable",
    "l1.navigator.languages_empty",
    "l1.navigator.plugins_empty",
    "l1.navigator.webdriver",
    "l1.permissions.mismatch",
    "l1.ua.headless_token",
    "l1.window.chrome_missing",
    "l1.window.outer_eq_inner",
    "l2.hardware.devicememory",
    "l2.screen.metrics",
    "l2.webgl.swiftshader",
    "l3.integrity.iframe_recheck",
    "l3.integrity.native_tostring",
];
/// Emitted by the Go detector whatever the browser observed.
const GO_UNCONDITIONAL: [&str; 1] = ["l1.navigator.webdriver"];
/// Accepted (HTTP status, runtime error code) pairs for one control.
type Refusals = &'static [(u64, Option<&'static str>)];
/// Post-browser controls (test/lab/runtime/src/browser_controls.rs), in order,
/// with the only refusals that count as failing closed. The replay/foreign
/// alternatives are the runtime's deadline/expiry checks, which run first.
const CONTROLS: [(&str, Refusals); 5] = [
    ("fresh-session-bootstrap", &[(200, None)]),
    (
        "fresh-session-required-collector",
        &[(422, Some("required_capability"))],
    ),
    (
        "replayed-collect",
        &[
            (409, Some("counter_replay_or_gap")),
            (403, Some("deadline")),
            (401, Some("session")),
        ],
    ),
    (
        "foreign-session-collect",
        &[(403, Some("request_integrity")), (403, Some("deadline"))],
    ),
    (
        "foreign-session-required-collector-after-replay",
        &[(422, Some("required_capability"))],
    ),
];
const CONTROLS_SCHEMA: &str = "humanymous.browser-missing-signal-controls/1";
const REPORT_MAX: usize = 1_048_576;

/// Standard padded base64, as the Gate actor encodes captured entities.
fn base64(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let value = |b: u8| -> Option<u32> {
        Some(match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let chunks = bytes.chunks(4).count();
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let pad = chunk.iter().rev().take_while(|b| **b == b'=').count();
        if pad > 2 || (pad > 0 && index + 1 != chunks) {
            return None;
        }
        let mut word = 0u32;
        for (at, b) in chunk.iter().enumerate() {
            word <<= 6;
            if at < 4 - pad {
                word |= value(*b)?;
            } else if *b != b'=' {
                return None;
            }
        }
        let decoded = word.to_be_bytes();
        out.extend_from_slice(&decoded[1..4 - pad]);
    }
    Some(out)
}

/// The single collect entity the real browser posted, verified against its
/// digest and length and decoded with duplicate/size refusal.
pub(super) fn captured(gate: &Value) -> Option<Value> {
    let [one] = gate["collector_reports"].as_array()?.as_slice() else {
        return None;
    };
    let bytes = base64(one["base64"].as_str()?)?;
    if one["bytes"].as_u64() != Some(bytes.len() as u64)
        || one["sha256"].as_str() != Some(&format!("{:x}", Sha256::digest(&bytes)))
    {
        return None;
    }
    decode_strict_json(&bytes, REPORT_MAX).ok()
}

fn count(hits: &Value, key: &str) -> u64 {
    hits[key].as_u64().unwrap_or(0)
}

/// Response keys both proxies recorded for one route.
fn responses<'a>(hits: &'a Value, path: &str) -> Vec<(&'a String, u64)> {
    let prefix = format!("response:{path}:");
    hits.as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(key, n)| (key, n.as_u64().unwrap_or(0)))
        .filter(|(_, n)| *n > 0)
        .collect()
}

fn both(gate: &Value, actor: &Value, key: &str, n: u64) -> bool {
    count(&gate["proxy_hits"], key) == n && count(&actor["proxy_hits"], key) == n
}

fn route(fixture: &Value, key: &str) -> String {
    fixture["routes"][key]
        .as_str()
        .unwrap_or("\u{0}")
        .to_owned()
}

/// Shape/content parity of one report against the Go oracle field sets.
fn report_gaps(report: &Value, gaps: &mut Vec<String>) {
    let Some(object) = report.as_object() else {
        gaps.push("report_not_object".into());
        return;
    };
    if object.len() != GO_REPORT_KEYS.len()
        || GO_REPORT_KEYS.iter().any(|key| !object.contains_key(*key))
    {
        gaps.push("report_keys_differ_from_go".into());
    }
    let signals = report["signals"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut ids = std::collections::BTreeSet::new();
    for signal in signals {
        let keys = signal.as_object();
        if !keys.is_some_and(|keys| {
            keys.keys()
                .all(|key| GO_SIGNAL_KEYS.contains(&key.as_str()))
                && GO_SIGNAL_KEYS[..GO_SIGNAL_REQUIRED]
                    .iter()
                    .all(|key| keys.contains_key(*key))
        }) {
            gaps.push("signal_shape_differs_from_go".into());
        }
        let id = signal["id"].as_str().unwrap_or("");
        if !GO_CLIENT_SIGNALS.contains(&id) {
            gaps.push("signal_outside_go_l1_l3_set".into());
        }
        if !ids.insert(id) {
            gaps.push("duplicate_signal".into());
        }
        if signal["collected"] != "wasm" {
            gaps.push("signal_not_wasm_collected".into());
        }
    }
    for id in GO_UNCONDITIONAL {
        if !ids.contains(id) {
            gaps.push(format!("go_unconditional_signal_absent:{id}"));
        }
    }
    if !report["engineVersion"]
        .as_str()
        .is_some_and(|v| v.len() > "rust-wasm-".len() && v.starts_with("rust-wasm-"))
    {
        gaps.push("engine_version_not_rust_wasm".into());
    }
    // web/js/loader.js populated these planes on every posted report.
    if report["guard"]["reported"] != true {
        gaps.push("guard_not_reported".into());
    }
    if report["environment"]["probed"] != true {
        gaps.push("environment_not_probed".into());
    }
    if !report["fingerprintId"]
        .as_str()
        .is_some_and(|v| !v.is_empty())
    {
        gaps.push("fingerprint_id_absent".into());
    }
    if !report["behavior"].is_object() {
        gaps.push("behavior_plane_absent".into());
    }
    if report["advanced"].as_object().is_none() {
        gaps.push("advanced_plane_absent".into());
    }
}

fn controls(gate: &Value) -> (Vec<String>, Value) {
    let ledger = &gate["missing_signal_controls"];
    let rows = ledger["rows"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut gaps = Vec::new();
    if ledger["schema"] != CONTROLS_SCHEMA
        || ledger["captured_collect_requests"] != 1
        || ledger["fresh_session_distinct"] != true
        || rows.len() != CONTROLS.len()
    {
        gaps.push("controls_ledger_incomplete".into());
    }
    let mut observed = Vec::new();
    for (index, (id, allowed)) in CONTROLS.iter().enumerate() {
        let row = rows.get(index).unwrap_or(&Value::Null);
        let refused = row["id"] == *id
            && row["failure"].is_null()
            && row["ack"] == false
            && allowed.iter().any(|(status, code)| {
                row["http"].as_u64() == Some(*status) && row["error"].as_str() == *code
            });
        if !refused {
            gaps.push(format!("control:{id}"));
        }
        observed.push(json!({"id":id,"http":row["http"],"error":row["error"],
            "ack":row["ack"],"failure":row["failure"],"expected":refused}));
    }
    (gaps, json!(observed))
}

/// `collector-parity`: the real browser's Rust WASM report carries the Go
/// collector's signal set and planes, is bound to the issuing session, and the
/// Gate admits it on a required-collector route for that session only.
pub(super) fn parity(gate: &Value, actor: &Value, expected: &Value, fixture: &Value) -> Value {
    let mut gaps = Vec::new();
    let report = captured(gate);
    match &report {
        Some(report) => report_gaps(report, &mut gaps),
        None => gaps.push("collect_entity_unavailable".into()),
    }
    let collect = route(fixture, "collect");
    let collect_ok = format!("response:{collect}:200:ack_true");
    let collected = expected["audit_records"].as_array().is_some_and(|records| {
        gate["audit_records"] == expected["audit_records"]
            && records
                .iter()
                .filter(|r| r["event_type"] == "runtime.collect" && r["verdict"] == "COLLECTED")
                .count()
                == 1
    });
    if !(both(gate, actor, &collect_ok, 1) && both(gate, actor, &collect, 1) && collected) {
        gaps.push("collect_not_bound_to_issuing_session".into());
    }
    // The issuing session must be admitted (a consumed, acknowledged decision)
    // on a required-collector route; a 422 or an unacknowledged reply is not.
    let required = route(fixture, "required_collector");
    let admitted = |hits: &Value| {
        responses(hits, &required)
            .iter()
            .any(|(key, _)| key.ends_with(":ack_true") && !key.contains(":422:"))
    };
    if !(admitted(&gate["proxy_hits"]) && admitted(&actor["proxy_hits"])) {
        gaps.push("issuing_session_not_admitted_on_required_collector_route".into());
    }
    let (control_gaps, observed) = controls(gate);
    if control_gaps
        .iter()
        .any(|gap| gap == "control:fresh-session-required-collector")
        || control_gaps.contains(&"controls_ledger_incomplete".to_owned())
    {
        gaps.push("foreign_session_not_refused".into());
    }
    gaps.sort();
    gaps.dedup();
    json!({"go_oracle":GO_ORACLE,"passed":gaps.is_empty(),"gaps":gaps,
        "signal_ids":report.as_ref().map(|r| r["signals"].as_array().into_iter().flatten()
            .map(|s| s["id"].clone()).collect::<Vec<_>>()),
        "engine_version":report.as_ref().map(|r| r["engineVersion"].clone()),
        "controls":observed})
}

/// `missing-signals-closed`: absent, stripped and replayed/foreign-session
/// collector evidence never admits; nothing reaches the origin or the audit.
pub(super) fn missing_signals(
    gate: &Value,
    actor: &Value,
    expected: &Value,
    fixture: &Value,
) -> Value {
    let mut gaps = Vec::new();
    // Absent: every pre-report effect was a consumed CHALLENGE (Go: unknown
    // verdict on an unsafe method is challenge_pow).
    let effects = expected["effect_requests"].as_u64().unwrap_or(0);
    let effect = route(fixture, "effect");
    let challenged = gate["audit_records"].as_array().is_some_and(|records| {
        effects > 0
            && records.len() as u64 > effects
            && records[..effects as usize]
                .iter()
                .all(|r| r["event_type"] == "runtime.decision" && r["verdict"] == "CHALLENGE")
    });
    if !(challenged
        && both(
            gate,
            actor,
            &format!("response:{effect}:403:ack_true"),
            effects,
        ))
    {
        gaps.push("absent_report_not_challenged".into());
    }
    // Stripped: the real report lacked complete provenance and the required
    // route refused it before counter consumption.
    match captured(gate) {
        Some(report)
            if report["guard"]["reported"] != true
                || !report["engineVersion"]
                    .as_str()
                    .is_some_and(|v| v.starts_with("rust-wasm-")) => {}
        Some(_) => gaps.push("stripped_report_not_exercised".into()),
        None => gaps.push("collect_entity_unavailable".into()),
    }
    let required = route(fixture, "required_collector");
    let refused = format!("response:{required}:422:ack_false");
    let only = |hits: &Value| responses(hits, &required) == vec![(&refused, 1)];
    let refused_key = refused.clone();
    if !(only(&gate["proxy_hits"]) && only(&actor["proxy_hits"]))
        || !both(gate, actor, &refused_key, 1)
        || actor["status"] != "passed"
    {
        gaps.push("stripped_report_not_refused".into());
    }
    let (control_gaps, observed) = controls(gate);
    gaps.extend(control_gaps);
    if gate["origin_effects"] != 0
        || gate["origin_effects"] != expected["origin_effects"]
        || gate["audit_sequence"] != expected["audit_sequence"]
        || gate["audit_records"] != expected["audit_records"]
    {
        gaps.push("origin_or_audit_changed".into());
    }
    gaps.sort();
    gaps.dedup();
    json!({"go_oracle":GO_ORACLE,"passed":gaps.is_empty(),"gaps":gaps,"controls":observed,
        "go_behaviour":{
            "absent":"internal/gate/enforce.go: unknown verdict on unsafe method -> challenge_pow",
            "stripped":"web/js/loader.js always posted guard.reported=true; runtime refuses incomplete provenance with 422 before consumption",
            "replayed":"cmd/server/rit.go l5.rit.stale_replay; internal/gate/control_collect.go HR-29 409",
            "foreign_session":"cmd/server/rit.go l5.rit.header_tampered (HMAC binds session); server owns sessionId"}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../../crates/collector/pilot/fixture.json"
        ))
        .unwrap()
    }

    fn encode(bytes: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    /// The Go-loader-complete report shape; today's Rust snapshot differs.
    fn go_complete_report() -> Value {
        json!({"sessionId":"","userAgent":"Mozilla/5.0 Firefox","uaClientHints":{},
            "signals":[{"id":"l1.navigator.webdriver","layer":"L1","value":true,"verdict":"BOT",
                "weight":0.0,"score":0.0,"confidence":1.0,"collected":"wasm"}],
            "behavior":{},"environment":{"probed":true},"guard":{"reported":true},
            "advanced":{"probed":true},"fingerprintId":"fp","engineVersion":"rust-wasm-0.1.0"})
    }

    fn rust_snapshot_report() -> Value {
        let mut report = go_complete_report();
        report["guard"]["reported"] = json!(false);
        report["environment"]["probed"] = json!(false);
        report["advanced"]["probed"] = json!(false);
        report["fingerprintId"] = json!("");
        report
    }

    fn entity(report: &Value) -> Value {
        let bytes = serde_json::to_vec(report).unwrap();
        json!({"sha256":format!("{:x}", Sha256::digest(&bytes)),"bytes":bytes.len(),
            "base64":encode(&bytes),"oversize":false})
    }

    fn rows(observed: [(u64, &str); 5]) -> Value {
        json!({"schema":CONTROLS_SCHEMA,"captured_collect_requests":1,"fresh_session_distinct":true,
            "rows":CONTROLS.iter().zip(observed).map(|((id,_),(http,error))| json!({"id":id,
                "http":http,"error":if error.is_empty(){Value::Null}else{json!(error)},
                "ack":false,"failure":null})).collect::<Vec<_>>()})
    }

    /// One epoch's view of the fixture flow: a refused (422) required route is
    /// the stripped epoch's expectation, an admitted one the complete epoch's.
    /// Controls are refused either way (the browser_epochs tests cover the
    /// whole two-epoch run).
    fn run(report: &Value, required_key: &str) -> (Value, Value, Value) {
        let fixture = fixture();
        let epoch = if required_key.contains(":422:") {
            "stripped"
        } else {
            "complete"
        };
        let expected =
            crate::browser_expectations::from_fixture(&fixture).unwrap()["epochs"][epoch].clone();
        let mut hits = json!({
            "/_humanymous/browser/collect":1,
            "response:/_humanymous/browser/collect:200:ack_true":1,
            "response:/effect:403:ack_true":2,
        });
        hits[required_key] = json!(1);
        let gate = json!({"proxy_hits":hits,"collector_reports":[entity(report)],
            "audit_records":expected["audit_records"],"audit_sequence":expected["audit_sequence"],
            "origin_effects":0,"missing_signal_controls":rows([(200,""),(422,"required_capability"),
                (409,"counter_replay_or_gap"),(403,"request_integrity"),(422,"required_capability")])});
        let actor = json!({"status":"passed","proxy_hits":hits});
        (gate, actor, expected)
    }

    /// The Go tables name exactly what the Rust collector and wire declare.
    #[test]
    fn go_oracle_tables_match_the_rust_collector_and_wire_sources() {
        let detection = [
            include_str!("../../../../../crates/collector/src/detection/static_client.rs"),
            include_str!("../../../../../crates/collector/src/detection/fingerprint.rs"),
            include_str!("../../../../../crates/collector/src/detection/integrity.rs"),
        ]
        .concat();
        let mut declared = std::collections::BTreeSet::new();
        for piece in detection.split('"').skip(1).step_by(2) {
            if piece.len() > 3
                && matches!(&piece[..3], "l1." | "l2." | "l3.")
                && piece
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'.' || b == b'_' || b.is_ascii_digit())
            {
                declared.insert(piece);
            }
        }
        assert_eq!(declared, GO_CLIENT_SIGNALS.into_iter().collect());
        let wire =
            include_str!("../../../../../crates/sdk/humanymous-core/src/signals/report/client.rs");
        let start = wire.find("pub struct ClientReport {").unwrap();
        let block = &wire[start..start + wire[start..].find("\n}").unwrap()];
        let renames: Vec<&str> = block
            .split("rename = \"")
            .skip(1)
            .map(|rest| &rest[..rest.find('"').unwrap()])
            .collect();
        assert_eq!(renames, GO_REPORT_KEYS);
    }

    /// The runner's control ledger is the Gate actor's, verbatim and in order.
    #[test]
    fn control_ledger_matches_the_gate_actor_source() {
        let source = include_str!("../../../runtime/src/browser_controls.rs");
        let start = source.find("pub const CONTROLS: &[&str] = &[").unwrap();
        let block = &source[start..];
        let block = &block[block.find('[').unwrap()..block.find("];").unwrap()];
        let declared: Vec<&str> = block
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\","))
            .collect();
        assert_eq!(declared, CONTROLS.map(|(id, _)| id));
        assert!(source.contains(&format!("pub const SCHEMA: &str = \"{CONTROLS_SCHEMA}\";")));
    }

    #[test]
    fn base64_round_trips_and_refuses_malformed_text() {
        for bytes in [&b""[..], b"{", b"{}", b"{\"a\":1}", &[0, 255, 7, 9]] {
            assert_eq!(base64(&encode(bytes)).unwrap(), bytes);
        }
        for bad in ["e30", "e3=0", "e$0=", "e30=e30=", "===="] {
            assert!(base64(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn todays_rust_snapshot_fails_parity_with_named_gaps_and_closes_missing_signals() {
        let (gate, actor, expected) = run(
            &rust_snapshot_report(),
            "response:/requires-collector:422:ack_false",
        );
        let parity = parity(&gate, &actor, &expected, &fixture());
        assert_eq!(parity["passed"], false);
        assert_eq!(
            parity["gaps"],
            json!([
                "environment_not_probed",
                "fingerprint_id_absent",
                "guard_not_reported",
                "issuing_session_not_admitted_on_required_collector_route"
            ])
        );
        let missing = missing_signals(&gate, &actor, &expected, &fixture());
        assert_eq!(missing["gaps"], json!([]), "{missing}");
        assert_eq!(missing["passed"], true);
    }

    #[test]
    fn complete_go_shaped_report_admitted_for_its_session_passes_parity_only() {
        let (gate, actor, expected) = run(
            &go_complete_report(),
            "response:/requires-collector:403:ack_true",
        );
        let parity = parity(&gate, &actor, &expected, &fixture());
        assert_eq!(parity["gaps"], json!([]), "{parity}");
        // The same run did not exercise a stripped report, so it cannot also
        // claim the missing-signal case.
        let missing = missing_signals(&gate, &actor, &expected, &fixture());
        let gaps = missing["gaps"].as_array().unwrap();
        assert!(gaps.contains(&json!("stripped_report_not_exercised")));
        assert!(gaps.contains(&json!("stripped_report_not_refused")));
    }

    #[test]
    fn every_unrefused_control_or_side_effect_is_a_named_gap() {
        let base = run(
            &rust_snapshot_report(),
            "response:/requires-collector:422:ack_false",
        );
        let check = |gate: &Value| missing_signals(gate, &base.1, &base.2, &fixture());
        for (index, http, error) in [
            (1, 200, ""),
            (2, 200, ""),
            (3, 409, "counter_replay_or_gap"),
            (4, 403, "required_capability"),
            (0, 503, "browser_profile_unavailable"),
        ] {
            let mut gate = base.0.clone();
            gate["missing_signal_controls"]["rows"][index]["http"] = json!(http);
            gate["missing_signal_controls"]["rows"][index]["error"] = if error.is_empty() {
                Value::Null
            } else {
                json!(error)
            };
            let gaps = check(&gate)["gaps"].clone();
            assert!(
                gaps.as_array()
                    .unwrap()
                    .contains(&json!(format!("control:{}", CONTROLS[index].0))),
                "{gaps}"
            );
        }
        let mut acked = base.0.clone();
        acked["missing_signal_controls"]["rows"][2]["ack"] = json!(true);
        assert_eq!(check(&acked)["passed"], false);
        let mut failed = base.0.clone();
        failed["missing_signal_controls"]["rows"][3]["failure"] = json!("connect");
        assert_eq!(check(&failed)["passed"], false);
        let mut same = base.0.clone();
        same["missing_signal_controls"]["fresh_session_distinct"] = json!(false);
        assert!(
            check(&same)["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("controls_ledger_incomplete"))
        );
        let mut effect = base.0.clone();
        effect["origin_effects"] = json!(1);
        assert!(
            check(&effect)["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("origin_or_audit_changed"))
        );
        let mut audit = base.0.clone();
        audit["audit_sequence"] = json!(99);
        assert!(
            check(&audit)["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("origin_or_audit_changed"))
        );
        let mut allowed = base.0.clone();
        allowed["audit_records"][0]["verdict"] = json!("ALLOW");
        assert!(
            check(&allowed)["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("absent_report_not_challenged"))
        );
        let mut admitted = base.0.clone();
        admitted["proxy_hits"]["response:/requires-collector:200:ack_true"] = json!(1);
        assert!(
            check(&admitted)["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("stripped_report_not_refused"))
        );
        let mut browser = base.1.clone();
        browser["status"] = json!("failed");
        assert_eq!(
            missing_signals(&base.0, &browser, &base.2, &fixture())["passed"],
            false
        );
    }

    #[test]
    fn tampered_missing_or_foreign_report_entities_are_never_evaluated() {
        let (gate, actor, expected) = run(
            &go_complete_report(),
            "response:/requires-collector:403:ack_true",
        );
        let mut digest = gate.clone();
        digest["collector_reports"][0]["sha256"] = json!("00");
        let mut two = gate.clone();
        let one = two["collector_reports"][0].clone();
        two["collector_reports"].as_array_mut().unwrap().push(one);
        let mut none = gate.clone();
        none["collector_reports"] = json!([]);
        for bad in [digest, two, none] {
            let result = parity(&bad, &actor, &expected, &fixture());
            assert!(
                result["gaps"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("collect_entity_unavailable"))
            );
        }
        for (field, value, gap) in [
            (
                "engineVersion",
                json!("wasm-1.0.0"),
                "engine_version_not_rust_wasm",
            ),
            ("extra", json!(1), "report_keys_differ_from_go"),
            (
                "signals",
                json!([{"id":"l5.rit.ok","layer":"L5","value":true,"verdict":"OK","weight":0.0,
                    "score":0.0,"confidence":1.0,"collected":"server"}]),
                "signal_outside_go_l1_l3_set",
            ),
        ] {
            let mut report = go_complete_report();
            report[field] = value;
            let (gate, actor, expected) = run(&report, "response:/requires-collector:403:ack_true");
            let gaps = parity(&gate, &actor, &expected, &fixture())["gaps"].clone();
            assert!(
                gaps.as_array().unwrap().contains(&json!(gap)),
                "{field}: {gaps}"
            );
        }
        let mut doubled = go_complete_report();
        let first = doubled["signals"][0].clone();
        doubled["signals"].as_array_mut().unwrap().push(first);
        let (gate, actor, expected) = run(&doubled, "response:/requires-collector:403:ack_true");
        assert!(
            parity(&gate, &actor, &expected, &fixture())["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("duplicate_signal"))
        );
    }
}
