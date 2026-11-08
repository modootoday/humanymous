//! Small test-owned text; the Collector implementation is never a fixture copy.
use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
static SYNTHETIC: OnceLock<Value> = OnceLock::new();
const PATH: &str = "crates/collector/src/browser/transport/runtime_pilot.rs";
const NAME: &str = "browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed";

fn text() -> Vec<u8> {
    (1..=22)
        .map(|line| match line {
            18 => "synthetic origin check\n",
            22 => "synthetic bootstrap check\n",
            _ => "\n",
        })
        .collect::<String>()
        .into_bytes()
}

pub(crate) struct Fixture {
    pub(crate) binding: SourceBinding,
}

impl Fixture {
    pub(crate) fn drift(&self) {
        fs::write(self.binding.root.join(PATH), b"changed source").unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only the uniquely generated test directory, never an external root.
        fs::remove_dir_all(&self.binding.root).unwrap();
    }
}

pub(crate) fn fixture() -> Fixture {
    let bytes = text();
    let profile = SYNTHETIC.get_or_init(|| {
        json!({
        "schema":1,"path":PATH,"name":NAME,
        "sha256":super::super::digest(&bytes),"bytes":bytes.len(),
        "sites":[
            {"line":18,"code":"browser_panic_owned_origin","needle":"synthetic origin"},
            {"line":22,"code":"browser_panic_bootstrap","needle":"synthetic bootstrap"}
        ]
        })
    });
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "hm-source-pin-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join(PATH);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, &bytes).unwrap();
    let selected = SelectedSource {
        path: PATH.into(),
        sha256: super::super::digest(&bytes),
        bytes: bytes.len(),
    };
    let mut binding = SourceBinding::with_profile(&root, &selected, profile).unwrap();
    binding
        .join_compose(&json!({"services":{
        "browser-gate":{"build":{"context":root}},
        "browser-product":{"build":{"context":root}}
        }}))
        .unwrap();
    Fixture { binding }
}

#[test]
fn source_identity_context_and_revalidation_are_required() {
    let fixture = fixture();
    let binding = &fixture.binding;
    let empty_root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "hm-rust-lab-pin-missing-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    fs::create_dir(&empty_root).unwrap();
    let empty = super::super::Snapshot::temporary(empty_root.clone());
    assert!(empty.source_binding().is_err());
    drop(empty);
    assert!(!empty_root.exists());
    let captured = selected(PATH, &text()).unwrap();
    assert_eq!(captured.sha256, binding.selected.sha256);
    assert_eq!(captured.bytes, binding.selected.bytes);
    assert!(selected("foreign.rs", &text()).is_none());
    assert!(binding.refinement_ready());
    assert_eq!(binding.site(22, 10), Some("browser_panic_bootstrap"));
    assert_eq!(binding.site(22, 999), None);
    let unjoined =
        SourceBinding::with_profile(&binding.root, &binding.selected, binding.profile).unwrap();
    assert!(!unjoined.refinement_ready());
    let mut wrong = binding.selected.clone();
    wrong.sha256 = "0".repeat(64);
    assert!(SourceBinding::with_profile(&binding.root, &wrong, binding.profile).is_err());
    wrong = binding.selected.clone();
    wrong.bytes += 1;
    assert!(SourceBinding::with_profile(&binding.root, &wrong, binding.profile).is_err());
    fixture.drift();
    assert!(!binding.refinement_ready());
    assert_eq!(binding.site(22, 10), None);
}

#[test]
fn reader_rejects_missing_oversize_invalid_utf8_and_line_drift() {
    let fixture = fixture();
    let binding = &fixture.binding;
    let path = binding.root.join(PATH);
    let crlf = String::from_utf8(text())
        .unwrap()
        .replace('\n', "\r\n")
        .into_bytes();
    for bytes in [b"\xff".to_vec(), vec![b'x'; MAX_BYTES + 1], crlf] {
        fs::write(&path, bytes).unwrap();
        assert!(binding.revalidate().is_err());
    }
    fs::remove_file(&path).unwrap();
    assert!(binding.revalidate().is_err());
    assert!(read(&binding.root, "../outside.rs").is_err());
    assert!(read(&binding.root, "").is_err());
    assert!(read(Path::new("."), PATH).is_err());
}

#[test]
fn wrong_or_missing_compose_context_revokes_refinement() {
    let mut fixture = fixture();
    assert!(
        fixture
            .binding
            .join_compose(&json!({"services":{}}))
            .is_err()
    );
    assert!(!fixture.binding.refinement_ready());
    let foreign = fixture.binding.root.parent().unwrap();
    let config = json!({"services":{
        "browser-gate":{"build":{"context":fixture.binding.root}},
        "browser-product":{"build":{"context":foreign}}
    }});
    assert!(fixture.binding.join_compose(&config).is_err());
    assert!(!fixture.binding.refinement_ready());
}

#[test]
fn duplicate_or_wrong_line_profiles_are_refused() {
    let fixture = fixture();
    for mutation in 0..3 {
        let mut profile = fixture.binding.profile.clone();
        match mutation {
            0 => {
                let duplicate = profile["sites"][0].clone();
                profile["sites"].as_array_mut().unwrap().push(duplicate);
            }
            1 => profile["sites"][0]["needle"] = json!("not in synthetic text"),
            _ => profile["path"] = json!("../outside.rs"),
        }
        let profile = Box::leak(Box::new(profile));
        assert!(
            SourceBinding::with_profile(&fixture.binding.root, &fixture.binding.selected, profile)
                .is_err()
        );
    }
}

#[test]
fn owned_data_profile_uses_only_reviewed_public_codes() {
    let catalog: Value = serde_json::from_str(include_str!("../../safe-diagnostics.json")).unwrap();
    let sites = profile()["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 33);
    let mut lines = HashSet::new();
    let mut codes = HashSet::new();
    for site in sites {
        assert!(lines.insert(site["line"].as_u64().unwrap()));
        assert!(codes.insert(site["code"].as_str().unwrap()));
        assert!(
            catalog["error_codes"]
                .as_array()
                .unwrap()
                .contains(&site["code"])
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_source_or_parent_is_refused() {
    use std::os::unix::fs::symlink;
    let fixture = fixture();
    let path = fixture.binding.root.join(PATH);
    fs::remove_file(&path).unwrap();
    symlink("/dev/null", &path).unwrap();
    assert!(fixture.binding.revalidate().is_err());
    fs::remove_file(&path).unwrap();
    let parent = path.parent().unwrap();
    fs::remove_dir(parent).unwrap();
    symlink("/tmp", parent).unwrap();
    assert!(fixture.binding.revalidate().is_err());
}

/// Required separate checkout gate; package-only tests do not need Collector.
#[test]
#[ignore = "requires explicit HUMANYMOUS_LAB_SOURCE_ROOT; required checkout gate"]
fn actual_collector_source_matches_reviewed_pin() {
    let root = PathBuf::from(
        std::env::var_os("HUMANYMOUS_LAB_SOURCE_ROOT")
            .expect("explicit frozen checkout source root required"),
    );
    let path = profile()["path"].as_str().unwrap();
    let bytes = read(&root, path).expect("bounded actual source read");
    let record = selected(path, &bytes).expect("selected source record");
    SourceBinding::bind(&root, &record).expect("actual source digest and every line/needle");
}
