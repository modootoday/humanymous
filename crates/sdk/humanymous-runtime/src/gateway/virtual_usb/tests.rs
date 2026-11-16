//! Ports of legacy internal/gate/admin_vusb_test.go (baseline 981d25c) plus
//! the bounded-file and consistency boundaries.
use super::*;
use serde_json::{Value, json};
use std::path::PathBuf;

struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn root() -> Root {
    let mut id = [0u8; 8];
    getrandom::fill(&mut id).unwrap();
    let path = std::env::temp_dir().join(format!("hmn-vusb-{}", hex::encode(id)));
    fs::create_dir(&path).unwrap();
    Root(path)
}

fn body() -> Value {
    json!({
        "schemaVersion": "humanymous.virtual-usb-ladder/v1",
        "canonical": true,
        "engines": ["chromium", "firefox"],
        "order": ["external_input_virtual", "external_input_dom_virtual",
                  "external_input_vusb", "external_input_dom_vusb"],
        "measured": 8, "pass": 7, "residual": 1, "failures": [],
        "ime": {"schemaVersion": "humanymous.ime-composition-vusb/v1",
                "order": ["ko-KR", "zh-CN", "ja-JP"],
                "expected": 6, "measured": 6, "pass": 6, "failures": []}
    })
}

fn write(root: &Path, run: &str, body: &Value) {
    let dir = root.join(run);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("ladder-result.json"),
        serde_json::to_vec(body).unwrap(),
    )
    .unwrap();
}

#[test]
fn disabled_by_default() {
    let v = view(None);
    assert!(!v.enabled && v.runs.is_empty());
    let json = serde_json::to_value(&v).unwrap();
    assert_eq!(json["enabled"], false);
}

#[test]
fn reads_only_bounded_terminal_summary() {
    let r = root();
    write(&r.0, "run-1", &body());
    let v = view(Some(&r.0));
    assert_eq!(
        (v.enabled, v.healthy, v.verification),
        (true, Some(true), Some("schema-only"))
    );
    assert_eq!(v.runs.len(), 1);
    assert!(v.runs[0].result.canonical && v.runs[0].result.measured == 8);
}

#[test]
fn rejects_unknown_fields_and_schema_violations() {
    let cases: [fn(&mut Value); 6] = [
        |b| b["rawDevicePaths"] = json!(["/dev/input/event0"]),
        |b| b["measured"] = json!(9),
        |b| b["engines"] = json!(["chromium"]),
        |b| b["ime"]["order"] = json!(["ja-JP", "zh-CN", "ko-KR"]),
        |b| b["ime"]["pass"] = json!(7),
        |b| b["failures"] = json!(["x"]),
    ];
    for mutate in cases {
        let r = root();
        let mut b = body();
        mutate(&mut b);
        write(&r.0, "run-1", &b);
        let v = view(Some(&r.0));
        assert_eq!((v.healthy, v.runs.len()), (Some(false), 0), "{b}");
    }
}

#[test]
fn trailing_json_and_oversize_files_fail_closed() {
    let r = root();
    write(&r.0, "run-1", &body());
    let path = r.0.join("run-1/ladder-result.json");
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b" {}");
    fs::write(&path, &bytes).unwrap();
    assert_eq!(read(&r.0), Err(Invalid::Schema));
    fs::write(&path, vec![b' '; MAX_RESULT_BYTES as usize + 1]).unwrap();
    assert_eq!(read(&r.0), Err(Invalid::File));
}

#[test]
fn non_canonical_partial_results_are_accepted() {
    let r = root();
    let mut b = body();
    b["canonical"] = json!(false);
    b["pass"] = json!(3);
    b["residual"] = json!(1);
    b["failures"] = json!(["external_input_vusb"]);
    write(&r.0, "run-1", &b);
    assert_eq!(read(&r.0).unwrap()[0].result.pass, 3);
}

#[test]
fn missing_root_and_empty_runs() {
    let r = root();
    assert_eq!(read(&r.0.join("absent")), Err(Invalid::Root));
    fs::create_dir(r.0.join("empty-run")).unwrap();
    assert_eq!(read(&r.0), Ok(Vec::new()));
}

#[cfg(unix)]
#[test]
fn symlinked_results_and_roots_are_refused() {
    let r = root();
    write(&r.0, "real", &body());
    let link_root = r.0.join("link-root");
    std::os::unix::fs::symlink(r.0.join("real"), &link_root).unwrap();
    assert_eq!(read(&link_root), Err(Invalid::Root));
    let r2 = root();
    fs::create_dir(r2.0.join("run-1")).unwrap();
    std::os::unix::fs::symlink(
        r.0.join("real/ladder-result.json"),
        r2.0.join("run-1/ladder-result.json"),
    )
    .unwrap();
    assert_eq!(read(&r2.0), Err(Invalid::File));
}
