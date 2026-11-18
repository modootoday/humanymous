use super::*;
use humanymous_runtime::network::intel::IntelCategory;
use std::cell::RefCell;

const NOW: u64 = 1_800_000_000;
const SHA: &str = "c60005a45e8b93dbdaa5d5d750de81a76a9d6a0f";

/// Serves fixed bodies by URL and records every request; no network.
#[derive(Default)]
struct FakeFetch {
    bodies: BTreeMap<&'static str, &'static [u8]>,
    requested: RefCell<Vec<String>>,
}
impl Fetch for FakeFetch {
    fn fetch(&self, url: &str, limit: usize) -> Result<Vec<u8>, &'static str> {
        assert_eq!(limit, MAX_SOURCE_BYTES);
        self.requested.borrow_mut().push(url.into());
        self.bodies
            .get(url)
            .map(|b| b.to_vec())
            .ok_or("fetch_failed")
    }
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-ip-intel-cli-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A submodule-shaped checkout: `.git` file pointing at a separate git dir.
fn checkout(root: &Path, name: &str, head: &str, files: &[(&str, &str)]) {
    let path = root.join("third_party/ip-intel").join(name);
    let git_dir = root.join(".git/modules").join(name);
    fs::create_dir_all(&git_dir).unwrap();
    fs::create_dir_all(&path).unwrap();
    fs::write(git_dir.join("HEAD"), format!("{head}\n")).unwrap();
    for (file, text) in files {
        let file = path.join(file);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    fs::write(
        path.join(".git"),
        format!("gitdir: ../../../.git/modules/{name}\n"),
    )
    .unwrap();
}

fn plan(root: &Path, sources: &[&str]) -> UpdatePlan {
    UpdatePlan {
        output: root.join("out/ip-intel.json"),
        repository: root.into(),
        sources: sources.iter().map(|s| (*s).into()).collect(),
        ..Default::default()
    }
}

#[test]
fn writes_a_parseable_snapshot_with_git_commit_and_fetch_time() {
    let dir = TempDir::new("write");
    fs::create_dir_all(dir.0.join("out")).unwrap();
    checkout(
        &dir.0,
        "x4bnet-datacenter",
        SHA,
        &[
            ("output/datacenter/ipv4.txt", "192.0.2.0/24\n"),
            ("output/datacenter/ipv6.txt", "2001:db8::/32\n"),
        ],
    );
    let fetch = FakeFetch {
        bodies: BTreeMap::from([(
            "https://check.torproject.org/torbulkexitlist",
            b"198.51.100.7\n".as_slice(),
        )]),
        ..Default::default()
    };
    let report = update(
        &plan(&dir.0, &["x4bnet-datacenter", "tor-bulk-exit"]),
        &fetch,
        NOW,
    )
    .unwrap();
    assert!(!report.partial);
    assert_eq!(report.prefixes, 3);
    let x4b = report
        .sources
        .iter()
        .find(|s| s.name == "x4bnet-datacenter")
        .unwrap();
    assert_eq!(x4b.commit.as_deref(), Some(SHA));
    let bytes = fs::read(dir.0.join("out/ip-intel.json")).unwrap();
    let parsed = snapshot::parse(&bytes, NOW).unwrap();
    let tor = parsed
        .sources
        .iter()
        .find(|s| s.name == "tor-bulk-exit")
        .unwrap();
    assert_eq!(tor.category, IntelCategory::TorExit);
    assert_eq!(
        tor.origin,
        Origin::Http {
            url: "https://check.torproject.org/torbulkexitlist".into(),
            fetched_at: NOW
        }
    );
    let summary = inspect(&dir.0.join("out/ip-intel.json"), NOW).unwrap();
    assert_eq!(summary["prefixes"], 3);
    assert!(
        fs::read_dir(dir.0.join("out")).unwrap().count() == 1,
        "no temporary file is left behind"
    );
}

#[test]
fn uninitialized_submodule_and_offline_sources_are_visible_not_empty() {
    let dir = TempDir::new("missing");
    fs::create_dir_all(dir.0.join("out")).unwrap();
    fs::create_dir_all(dir.0.join("third_party/ip-intel/firehol-level1")).unwrap();
    let input = dir.0.join("aws.json");
    fs::write(&input, br#"{"prefixes":[{"ip_prefix":"192.0.2.0/24"}]}"#).unwrap();
    let mut plan = plan(
        &dir.0,
        &[
            "firehol-level1",
            "aws-ip-ranges",
            "gcp-cloud",
            "azure-service-tags",
        ],
    );
    plan.offline = true;
    plan.inputs.insert("aws-ip-ranges".into(), input);
    let fetch = FakeFetch::default();
    let report = update(&plan, &fetch, NOW).unwrap();
    assert!(fetch.requested.borrow().is_empty(), "offline never fetches");
    assert!(report.partial);
    let detail = |name: &str| {
        let line = report.sources.iter().find(|s| s.name == name).unwrap();
        (line.status, line.detail.clone())
    };
    assert_eq!(detail("aws-ip-ranges"), (SourceStatus::Ok, String::new()));
    assert_eq!(
        detail("firehol-level1"),
        (SourceStatus::Missing, "submodule_not_initialized".into())
    );
    assert_eq!(
        detail("gcp-cloud"),
        (SourceStatus::Missing, "offline".into())
    );
    assert_eq!(
        detail("azure-service-tags"),
        (SourceStatus::Missing, "operator_url_required".into())
    );
    let parsed = snapshot::parse(&fs::read(&plan.output).unwrap(), NOW).unwrap();
    assert_eq!(parsed.sources.len(), 4);
}

#[test]
fn malformed_upstream_is_invalid_and_no_success_writes_nothing() {
    let dir = TempDir::new("invalid");
    let fetch = FakeFetch {
        bodies: BTreeMap::from([(
            "https://www.gstatic.com/ipranges/cloud.json",
            b"<html>blocked</html>".as_slice(),
        )]),
        ..Default::default()
    };
    let plan = plan(&dir.0, &["gcp-cloud"]);
    let error = update(&plan, &fetch, NOW).unwrap_err();
    assert!(error.to_string().contains("no source succeeded"));
    assert!(!plan.output.exists());
    let mut unknown = plan.clone();
    unknown.sources = vec!["not-a-source".into()];
    assert!(update(&unknown, &fetch, NOW).is_err());
    let mut unknown_input = plan.clone();
    unknown_input.inputs.insert("nope".into(), dir.0.join("x"));
    assert!(update(&unknown_input, &fetch, NOW).is_err());
}

#[test]
fn checkout_commit_reads_detached_loose_and_packed_heads() {
    let dir = TempDir::new("commit");
    assert_eq!(checkout_commit(&dir.0.join("absent")), None);
    checkout(&dir.0, "detached", SHA, &[]);
    assert_eq!(
        checkout_commit(&dir.0.join("third_party/ip-intel/detached")).as_deref(),
        Some(SHA)
    );
    checkout(&dir.0, "loose", "ref: refs/heads/main", &[]);
    let git_dir = dir.0.join(".git/modules/loose");
    fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
    fs::write(git_dir.join("refs/heads/main"), format!("{SHA}\n")).unwrap();
    assert_eq!(
        checkout_commit(&dir.0.join("third_party/ip-intel/loose")).as_deref(),
        Some(SHA)
    );
    checkout(&dir.0, "packed", "ref: refs/heads/main", &[]);
    fs::write(
        dir.0.join(".git/modules/packed/packed-refs"),
        format!("# pack-refs with: peeled\n{SHA} refs/heads/main\n"),
    )
    .unwrap();
    assert_eq!(
        checkout_commit(&dir.0.join("third_party/ip-intel/packed")).as_deref(),
        Some(SHA)
    );
    checkout(&dir.0, "garbage", "not-a-sha", &[]);
    assert_eq!(
        checkout_commit(&dir.0.join("third_party/ip-intel/garbage")),
        None
    );
}

#[test]
fn registry_listing_marks_only_datacenter_as_scoring() {
    let listing = registry();
    let scoring: Vec<_> = listing
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["feeds_scoring"] == true)
        .map(|s| s["category"].clone())
        .collect();
    assert!(scoring.iter().all(|c| c == "datacenter"));
    assert!(
        listing
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["origin"]["git"]["path"] == "third_party/ip-intel/firehol-level1")
    );
}
