//! IP reputation contract across boundaries: pinned submodules match the
//! registry, every upstream format normalizes its synthetic fixture, and a
//! snapshot classification reaches the frozen report without new tells.
use humanymous_core::{scoring::engine::Engine, signals::report::SessionReport};
use humanymous_runtime::network::{
    intel::{
        IntelCategory, IntelReader,
        snapshot::{self, Origin, Snapshot, SourceRecord, SourceStatus},
        source::{self, Format, REGISTRY, SUBMODULE_ROOT, SourceOrigin},
        store::{IntelStore, StorePolicy},
    },
    report::{Observation, build},
};
use std::{collections::BTreeMap, path::PathBuf};

const NOW: u64 = 1_800_000_000;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ip_intel")
            .join(name),
    )
    .unwrap()
}

/// `.gitmodules` sections under `third_party/ip-intel/` as path -> url.
fn submodules() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(repository_root().join(".gitmodules")).unwrap();
    let mut result = BTreeMap::new();
    let mut path = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with("[submodule ") {
            path = None;
        } else if let Some(value) = line.strip_prefix("path = ") {
            path = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("url = ")
            && let Some(path) = path.take()
            && path.starts_with(&format!("{SUBMODULE_ROOT}/"))
        {
            result.insert(path, value.to_owned());
        }
    }
    result
}

#[test]
fn gitmodules_match_registry_git_sources_exactly() {
    let expected: BTreeMap<_, _> = REGISTRY
        .iter()
        .filter_map(|spec| match spec.origin {
            SourceOrigin::Git { repository, .. } => {
                Some((spec.submodule_path().unwrap(), repository.to_owned()))
            }
            _ => None,
        })
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(submodules(), expected);
}

#[test]
fn every_format_normalizes_its_synthetic_fixture() {
    let cases = [
        (
            Format::AwsJson,
            "aws-ip-ranges.json",
            ["192.0.2.0/25", "2001:db8:a::/48"].as_slice(),
        ),
        (
            Format::GcpJson,
            "gcp-cloud.json",
            &["192.0.2.128/26", "2001:db8:b::/48"],
        ),
        (
            Format::AzureServiceTags,
            "azure-service-tags.json",
            &["192.0.2.192/27", "2001:db8:c::/48"],
        ),
        (
            Format::OracleJson,
            "oracle-cloud.json",
            &["192.0.2.224/28", "2001:db8:d::/48"],
        ),
        (
            Format::Geofeed,
            "geofeed.csv",
            &["192.0.2.240/29", "2001:db8:e::/48"],
        ),
        (
            Format::Netset,
            "netset.txt",
            &["198.51.100.0/24", "203.0.113.7/32", "2001:db8:f::/48"],
        ),
        (
            Format::SpamhausJsonl,
            "spamhaus-drop.jsonl",
            &["198.51.100.0/25", "2001:db8:1::/48"],
        ),
    ];
    for (format, name, expected) in cases {
        let text: Vec<String> = source::normalize(format, &fixture(name))
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(text, expected, "{name}");
    }
}

fn record(name: &str, category: IntelCategory, format: Format, file: &str) -> SourceRecord {
    SourceRecord {
        name: name.into(),
        category,
        licence: "synthetic test data".into(),
        origin: Origin::File { read_at: NOW },
        status: SourceStatus::Ok,
        detail: String::new(),
        prefixes: source::normalize(format, &fixture(file)).unwrap(),
    }
}

fn signal_ids(observation: &Observation) -> Vec<String> {
    build(observation)
        .signals
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn snapshot_classification_feeds_only_the_frozen_datacenter_input() {
    let path = std::env::temp_dir().join(format!(
        "humanymous-ip-intel-it-{}.json",
        std::process::id()
    ));
    let bytes = snapshot::encode(&Snapshot::new(
        NOW,
        vec![
            record(
                "test-aws",
                IntelCategory::Datacenter,
                Format::AwsJson,
                "aws-ip-ranges.json",
            ),
            record(
                "test-tor",
                IntelCategory::TorExit,
                Format::Netset,
                "netset.txt",
            ),
            record(
                "test-drop",
                IntelCategory::Blocklist,
                Format::SpamhausJsonl,
                "spamhaus-drop.jsonl",
            ),
        ],
    ))
    .unwrap();
    std::fs::write(&path, bytes).unwrap();
    let store = IntelStore::open(
        StorePolicy {
            path: path.clone(),
            max_age_s: 86_400,
            expire_after_s: 7 * 86_400,
            categories: IntelCategory::ALL.to_vec(),
        },
        NOW,
    );
    let _ = std::fs::remove_file(&path);

    let datacenter = store.classify(Some("192.0.2.1".parse().unwrap()), NOW);
    assert!(datacenter.datacenter);
    let observed = Observation {
        is_datacenter_ip: datacenter.datacenter,
        ..Default::default()
    };
    assert!(signal_ids(&observed).contains(&"l5.ip.datacenter_asn".to_owned()));

    // Tor exit and blocklist hits are counted but never reach the report.
    let shadow = store.classify(Some("198.51.100.9".parse().unwrap()), NOW);
    assert!(shadow.tor_exit && shadow.blocklist && !shadow.datacenter);
    let clean = Observation::default();
    let shadowed = Observation {
        is_datacenter_ip: shadow.datacenter,
        ..Default::default()
    };
    assert_eq!(signal_ids(&shadowed), signal_ids(&clean));
    assert!(
        !signal_ids(&shadowed)
            .iter()
            .any(|id| id.starts_with("l5.ip."))
    );
    let engine = Engine::default();
    let mut clean_report = SessionReport {
        network: build(&clean),
        ..Default::default()
    };
    let mut shadow_report = SessionReport {
        network: build(&shadowed),
        ..Default::default()
    };
    let clean_score = engine.score(&mut clean_report).unwrap();
    let shadow_score = engine.score(&mut shadow_report).unwrap();
    assert_eq!(
        (shadow_score.verdict, shadow_score.hard_rule_fired),
        (clean_score.verdict, clean_score.hard_rule_fired)
    );
    let status = store.status(NOW);
    assert_eq!(status.shadow.tor_exit_hits, 1);
    assert_eq!(status.shadow.blocklist_hits, 1);
    assert_eq!(status.lookups.datacenter_inputs, 1);
}
