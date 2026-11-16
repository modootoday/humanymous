//! Ports of legacy cmd/server/external_input_receipt_test.go (baseline
//! 981d25c) plus label and publication boundaries.
use super::*;
use humanymous_core::signals::observation::{Signal, Verdict};
use serde_json::json;

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn dir() -> Dir {
    let mut id = [0u8; 8];
    getrandom::fill(&mut id).unwrap();
    let path = std::env::temp_dir().join(format!("hmn-ext-{}", hex::encode(id)));
    fs::create_dir(&path).unwrap();
    Dir(path)
}

const DETECTOR: &[u8] = b"\x00asm-test-detector";
const LABEL: &str = "external-input/run-20260727/external_input_virtual";

fn fixture() -> (Dir, Dir, Writer) {
    let web = dir();
    fs::write(web.0.join("detector.wasm"), DETECTOR).unwrap();
    let receipts = dir();
    let writer = Writer::configure(Some(&receipts.0), &web.0)
        .unwrap()
        .unwrap();
    (web, receipts, writer)
}

fn report() -> SessionReport {
    let mut report = SessionReport {
        session_id: "sensitive-session-id".into(),
        ..SessionReport::default()
    };
    report.client.session_id = "sensitive-session-id".into();
    report.client.engine_version = "detector-test-v1".into();
    report.client.signals = Some(vec![Signal::new(
        "l1.navigator.webdriver",
        json!(false),
        Verdict::Ok,
        1.0,
        Source::Wasm,
        "fixture",
    )]);
    report
}

fn result() -> ScoreResult {
    ScoreResult {
        risk_score: 74.3,
        verdict: "CHALLENGE".into(),
        hard_rule_fired: "HR-12".into(),
        policy_version: "1.0.0".into(),
        ..ScoreResult::default()
    }
}

#[test]
fn receipt_binds_canonical_core_evidence() {
    let (_web, receipts, writer) = fixture();
    let report = report();
    writer
        .write(LABEL, "sensitive-session-id", &result(), &report)
        .unwrap();
    let raw = fs::read_to_string(receipts.0.join(RECEIPT_FILE)).unwrap();
    assert!(
        !raw.contains("sensitive-session-id"),
        "raw session id leaked"
    );
    assert!(raw.ends_with('\n'));
    let receipt: Receipt = serde_json::from_str(&raw).unwrap();
    let report_sha = hex::encode(Sha256::digest(serde_json::to_vec(&report).unwrap()));
    assert_eq!(receipt.schema_version, RECEIPT_SCHEMA);
    assert_eq!(receipt.run_label, LABEL);
    assert_eq!(receipt.run_id, "run-20260727");
    assert_eq!(receipt.profile_id, "external_input_virtual");
    assert_eq!(
        receipt.session_ref,
        hex::encode(&Sha256::digest(b"sensitive-session-id")[..8])
    );
    assert_eq!(
        receipt.detector_wasm_hash,
        hex::encode(Sha256::digest(DETECTOR))
    );
    assert_eq!(receipt.detector_path, DETECTOR_PATH);
    assert_eq!(receipt.report_sha256, report_sha);
    assert_eq!(receipt.wasm_signal_count, 1);
    assert_eq!(receipt.engine_version, "detector-test-v1");
    assert!(!receipt.score_recomputed && receipt.server_authoritative);
    let trace = [
        TRACE_SCHEMA,
        &receipt.run_label,
        &receipt.session_ref,
        "74.3",
        "CHALLENGE",
        "HR-12",
        "1.0.0",
        &receipt.detector_wasm_hash,
        "detector-test-v1",
        "1",
        &report_sha,
    ]
    .join("\0");
    assert_eq!(
        receipt.score_trace_sha256,
        hex::encode(Sha256::digest(trace.as_bytes()))
    );
    assert_eq!(
        writer.write(LABEL, "another-session", &result(), &report),
        Err(Refusal::Write),
        "exclusive publication never overwrites"
    );
    assert_eq!(
        fs::read_to_string(receipts.0.join(RECEIPT_FILE)).unwrap(),
        raw
    );
}

#[test]
fn fails_closed_without_wasm_evidence() {
    let (_web, receipts, writer) = fixture();
    let mut js = report();
    js.client.signals.as_mut().unwrap()[0].collected = Source::Js;
    assert_eq!(
        writer.write(LABEL, "session", &result(), &js),
        Err(Refusal::NoWasmEvidence)
    );
    let mut unversioned = report();
    unversioned.client.engine_version.clear();
    assert_eq!(
        writer.write(LABEL, "session", &result(), &unversioned),
        Err(Refusal::NoWasmEvidence)
    );
    assert!(!receipts.0.join(RECEIPT_FILE).exists());
}

#[test]
fn configuration_is_explicit() {
    let web = dir();
    assert!(Writer::configure(None, &web.0).unwrap().is_none());
    assert!(
        Writer::configure(Some(Path::new("")), &web.0)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        Writer::configure(Some(Path::new("relative")), &web.0).unwrap_err(),
        Refusal::Relative
    );
    let receipts = dir();
    assert_eq!(
        Writer::configure(Some(&receipts.0), &web.0).unwrap_err(),
        Refusal::Detector,
        "no channel without detector.wasm"
    );
    fs::write(web.0.join("detector.wasm"), b"").unwrap();
    assert_eq!(
        Writer::configure(Some(&receipts.0), &web.0).unwrap_err(),
        Refusal::Detector,
        "empty detector refused"
    );
    assert_eq!(
        Writer::configure(Some(&receipts.0.join("missing")), &web.0).unwrap_err(),
        Refusal::Directory
    );
    #[cfg(unix)]
    {
        fs::write(web.0.join("detector.wasm"), DETECTOR).unwrap();
        let link = web.0.join("link");
        std::os::unix::fs::symlink(&receipts.0, &link).unwrap();
        assert_eq!(
            Writer::configure(Some(&link), &web.0).unwrap_err(),
            Refusal::Directory,
            "symlinked receipt directory refused"
        );
    }
}

#[test]
fn receipt_channel_writes_only_the_receipt_file() {
    let (_web, receipts, writer) = fixture();
    writer
        .write(
            "external-input/run-secret/external_input_virtual",
            "sensitive-session-id",
            &result(),
            &report(),
        )
        .unwrap();
    let names: Vec<_> = fs::read_dir(&receipts.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![RECEIPT_FILE.to_owned()],
        "no temporary file left"
    );
}

#[test]
fn labels_outside_the_run_shape_are_ignored_or_refused() {
    let (_web, receipts, writer) = fixture();
    for ignored in ["", "other/run/profile", "external-input/run id/p"] {
        assert_eq!(writer.write(ignored, "s", &result(), &report()), Ok(()));
    }
    for refused in [
        "external-input/run-only",
        "external-input//profile",
        "external-input/run/",
        "external-input/a/b/c",
    ] {
        assert_eq!(
            writer.write(refused, "s", &result(), &report()),
            Err(Refusal::Label),
            "{refused}"
        );
    }
    assert!(!receipts.0.join(RECEIPT_FILE).exists());
}
