use super::*;
use crate::network::intel::{
    prefix_set::Prefix,
    snapshot::{SourceRecord, encode},
};
use std::path::Path;

const NOW: u64 = 1_800_000_000;
const DAY: u64 = 86_400;

struct TempFile(PathBuf);
impl TempFile {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-intel-store-{}-{name}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        Self(path)
    }
    fn write(&self, bytes: &[u8]) {
        std::fs::write(&self.0, bytes).unwrap();
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn record(name: &str, category: IntelCategory, prefixes: &[&str]) -> SourceRecord {
    SourceRecord {
        name: name.into(),
        category,
        licence: "synthetic test data".into(),
        origin: Origin::File { read_at: NOW - 60 },
        status: SourceStatus::Ok,
        detail: String::new(),
        prefixes: prefixes
            .iter()
            .map(|p| Prefix::parse_canonical(p).unwrap())
            .collect(),
    }
}

fn snapshot_bytes(generated_at: u64) -> Vec<u8> {
    let mut missing = record("test-missing", IntelCategory::TorExit, &[]);
    missing.status = SourceStatus::Missing;
    missing.detail = "submodule_not_initialized".into();
    encode(&Snapshot::new(
        generated_at,
        vec![
            record(
                "test-datacenter",
                IntelCategory::Datacenter,
                &["192.0.2.0/24", "2001:db8::/32"],
            ),
            record("test-tor", IntelCategory::TorExit, &["198.51.100.7"]),
            record(
                "test-blocklist",
                IntelCategory::Blocklist,
                &["203.0.113.0/24", "198.51.100.0/24"],
            ),
            missing,
        ],
    ))
    .unwrap()
}

fn policy(path: &Path) -> StorePolicy {
    StorePolicy {
        path: path.into(),
        max_age_s: 7 * DAY,
        expire_after_s: 30 * DAY,
        categories: IntelCategory::ALL.to_vec(),
    }
}

fn ip(text: &str) -> Option<IpAddr> {
    Some(text.parse().unwrap())
}

#[test]
fn classifies_every_category_and_counts_shadow_only_separately() {
    let file = TempFile::new("classify");
    file.write(&snapshot_bytes(NOW));
    let store = IntelStore::open(policy(&file.0), NOW);
    assert_eq!(store.freshness(NOW), Freshness::Fresh);
    let dc = store.classify(ip("192.0.2.200"), NOW);
    assert_eq!(
        dc,
        Classification {
            datacenter: true,
            ..Default::default()
        }
    );
    assert!(store.classify(ip("2001:db8:ffff::1"), NOW).datacenter);
    let tor = store.classify(ip("198.51.100.7"), NOW);
    assert!(tor.tor_exit && tor.blocklist && !tor.datacenter);
    assert!(store.classify(ip("203.0.113.1"), NOW).blocklist);
    assert_eq!(
        store.classify(ip("8.8.8.8"), NOW),
        Classification::default()
    );
    assert_eq!(
        store.classify(ip("10.0.0.1"), NOW),
        Classification::default()
    );
    assert_eq!(store.classify(None, NOW), Classification::default());
    let status = store.status(NOW);
    assert!(status.datacenter_scoring_active);
    assert_eq!(status.scoring_categories, [IntelCategory::Datacenter]);
    assert_eq!(
        status.shadow_categories,
        [IntelCategory::TorExit, IntelCategory::Blocklist]
    );
    assert_eq!(
        status.lookups,
        LookupStats {
            lookups: 7,
            unattributed: 1,
            not_classifiable: 1,
            unavailable: 0,
            stale: 0,
            datacenter_inputs: 2,
        }
    );
    assert_eq!(status.shadow.tor_exit_hits, 1);
    assert_eq!(status.shadow.blocklist_hits, 2);
    let missing = status
        .sources
        .iter()
        .find(|s| s.name == "test-missing")
        .unwrap();
    assert_eq!(
        (missing.status, missing.detail.as_str()),
        (SourceStatus::Missing, "submodule_not_initialized")
    );
    let json = serde_json::to_string(&status).unwrap();
    assert!(
        !json.contains("192.0.2.200"),
        "status never carries client addresses"
    );
}

#[test]
fn missing_feed_is_visible_and_denies_nobody() {
    let file = TempFile::new("missing");
    let store = IntelStore::open(policy(&file.0), NOW);
    assert_eq!(
        store.classify(ip("192.0.2.1"), NOW),
        Classification::default()
    );
    let status = store.status(NOW);
    assert_eq!(status.state, Freshness::Missing);
    assert!(!status.datacenter_scoring_active);
    assert_eq!(status.last_reload_error.as_deref(), Some("snapshot_io"));
    assert_eq!((status.reloads_ok, status.reloads_failed), (0, 1));
    assert_eq!(status.lookups.unavailable, 1);
}

#[test]
fn stale_snapshot_still_classifies_and_is_reported_then_expires() {
    let file = TempFile::new("stale");
    file.write(&snapshot_bytes(NOW));
    let store = IntelStore::open(policy(&file.0), NOW);
    let stale_at = NOW + 7 * DAY + 1;
    assert_eq!(store.freshness(stale_at), Freshness::Stale);
    assert!(store.classify(ip("192.0.2.1"), stale_at).datacenter);
    let status = store.status(stale_at);
    assert_eq!(status.state, Freshness::Stale);
    assert_eq!(status.age_s, Some(7 * DAY + 1));
    assert!(status.datacenter_scoring_active);
    assert_eq!(status.lookups.stale, 1);
    let expired_at = NOW + 30 * DAY + 1;
    assert_eq!(
        store.classify(ip("192.0.2.1"), expired_at),
        Classification::default()
    );
    let status = store.status(expired_at);
    assert_eq!(status.state, Freshness::Expired);
    assert!(!status.datacenter_scoring_active);
    assert_eq!(status.lookups.unavailable, 1);
}

#[test]
fn malformed_reload_is_refused_without_state_change() {
    let file = TempFile::new("malformed");
    file.write(&snapshot_bytes(NOW));
    let store = IntelStore::open(policy(&file.0), NOW);
    let before = store.status(NOW).snapshot_sha256;
    for bad in [
        b"{".to_vec(),
        b"{\"format\":\"humanymous-ip-intel/1\",\"generated_at\":1,\"sources\":[]}".to_vec(),
        String::from_utf8(snapshot_bytes(NOW))
            .unwrap()
            .replace("192.0.2.0/24", "192.0.2.1/24")
            .into_bytes(),
        snapshot_bytes(NOW + DAY),
    ] {
        file.write(&bad);
        assert!(store.reload(NOW).is_err());
        assert_eq!(store.status(NOW).snapshot_sha256, before);
        assert!(store.classify(ip("192.0.2.1"), NOW).datacenter);
    }
    let status = store.status(NOW);
    assert_eq!((status.reloads_ok, status.reloads_failed), (1, 4));
    assert_eq!(
        status.last_reload_error.as_deref(),
        Some("snapshot_timestamp")
    );
}

#[test]
fn reload_if_changed_swaps_only_on_change() {
    let file = TempFile::new("swap");
    file.write(&snapshot_bytes(NOW));
    let store = IntelStore::open(policy(&file.0), NOW);
    assert_eq!(store.reload_if_changed(NOW), Ok(false));
    let replacement = encode(&Snapshot::new(
        NOW + 1,
        vec![record(
            "test-datacenter",
            IntelCategory::Datacenter,
            &["198.51.100.0/24"],
        )],
    ))
    .unwrap();
    file.write(&replacement);
    assert_eq!(store.reload_if_changed(NOW + 1), Ok(true));
    assert!(!store.classify(ip("192.0.2.1"), NOW + 1).datacenter);
    assert!(store.classify(ip("198.51.100.1"), NOW + 1).datacenter);
    assert_eq!(store.status(NOW + 1).reloads_ok, 2);
}

#[test]
fn disabled_categories_are_not_loaded() {
    let file = TempFile::new("categories");
    file.write(&snapshot_bytes(NOW));
    let mut policy = policy(&file.0);
    policy.categories = vec![IntelCategory::TorExit];
    let store = IntelStore::open(policy, NOW);
    let both = store.classify(ip("198.51.100.7"), NOW);
    assert!(both.tor_exit && !both.blocklist);
    assert!(!store.classify(ip("192.0.2.1"), NOW).datacenter);
    let status = store.status(NOW);
    assert!(!status.datacenter_scoring_active);
    assert!(status.scoring_categories.is_empty());
}
