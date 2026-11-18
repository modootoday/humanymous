use super::*;

const NOW: u64 = 1_800_000_000;
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn prefix(text: &str) -> Prefix {
    Prefix::parse_canonical(text).unwrap()
}

fn sample() -> Snapshot {
    Snapshot::new(
        NOW,
        vec![
            SourceRecord {
                name: "test-datacenter".into(),
                category: IntelCategory::Datacenter,
                licence: "synthetic test data".into(),
                origin: Origin::Http {
                    url: "https://example.test/ranges.json".into(),
                    fetched_at: NOW - 10,
                },
                status: SourceStatus::Ok,
                detail: String::new(),
                prefixes: vec![
                    prefix("2001:db8::/32"),
                    prefix("192.0.2.0/24"),
                    prefix("192.0.2.0/24"),
                ],
            },
            SourceRecord {
                name: "test-blocklist".into(),
                category: IntelCategory::Blocklist,
                licence: "synthetic test data".into(),
                origin: Origin::Git {
                    path: "third_party/ip-intel/test-blocklist".into(),
                    commit: Some(SHA.into()),
                },
                status: SourceStatus::Ok,
                detail: String::new(),
                prefixes: vec![prefix("198.51.100.0/24")],
            },
            SourceRecord {
                name: "test-missing".into(),
                category: IntelCategory::TorExit,
                licence: "synthetic test data".into(),
                origin: Origin::Git {
                    path: "third_party/ip-intel/test-missing".into(),
                    commit: None,
                },
                status: SourceStatus::Missing,
                detail: "submodule_not_initialized".into(),
                prefixes: vec![],
            },
        ],
    )
}

#[test]
fn encode_parse_round_trip_is_normalized_and_stable() {
    let snapshot = sample();
    assert_eq!(
        snapshot
            .sources
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["test-blocklist", "test-datacenter", "test-missing"]
    );
    assert_eq!(snapshot.prefix_count(), 3);
    let bytes = encode(&snapshot).unwrap();
    assert!(bytes.ends_with(b"\n"));
    let parsed = parse(&bytes, NOW).unwrap();
    assert_eq!(parsed, snapshot);
    assert_eq!(encode(&parsed).unwrap(), bytes);
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("\"192.0.2.0/24\"") && text.contains("\"2001:db8::/32\""));
}

fn mutate(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value = serde_json::to_value(sample()).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn malformed_documents_are_refused_with_static_codes() {
    use SnapshotError::*;
    let cases: Vec<(Vec<u8>, SnapshotError)> = vec![
        (b"not json".to_vec(), Malformed),
        (
            mutate(|v| v["format"] = "humanymous-ip-intel/2".into()),
            Format,
        ),
        (mutate(|v| v["extra"] = 1.into()), Malformed),
        (
            mutate(|v| v["sources"][0]["prefixes"][0] = "198.51.100.1/24".into()),
            Malformed,
        ),
        (
            mutate(|v| v["sources"][0]["prefixes"][0] = "not-a-prefix".into()),
            Malformed,
        ),
        (
            mutate(|v| v["sources"][0]["category"] = "proxy".into()),
            Malformed,
        ),
        (mutate(|v| v["sources"] = serde_json::json!([])), Bounds),
        (
            mutate(|v| v["sources"][1]["name"] = "test-blocklist".into()),
            SourceName,
        ),
        (
            mutate(|v| v["sources"][0]["name"] = "Bad Name".into()),
            SourceName,
        ),
        (mutate(|v| v["sources"][0]["licence"] = "".into()), Text),
        (mutate(|v| v["sources"][0]["licence"] = "a\nb".into()), Text),
        (mutate(|v| v["sources"][0]["detail"] = "x".into()), Status),
        (mutate(|v| v["sources"][2]["detail"] = "".into()), Status),
        (mutate(|v| v["sources"][2]["status"] = "ok".into()), Status),
        (
            mutate(|v| v["sources"][0]["prefixes"] = serde_json::json!([])),
            Status,
        ),
        (
            mutate(|v| v["sources"][0]["origin"]["git"]["commit"] = "ABC".into()),
            Commit,
        ),
        (
            mutate(|v| v["sources"][0]["origin"]["git"]["commit"] = serde_json::Value::Null),
            Commit,
        ),
        (
            mutate(|v| v["generated_at"] = (NOW + MAX_FUTURE_SKEW_S + 1).into()),
            Timestamp,
        ),
        (
            mutate(|v| v["sources"][1]["origin"]["http"]["fetched_at"] = (NOW + 1).into()),
            Timestamp,
        ),
    ];
    for (index, (bytes, expected)) in cases.into_iter().enumerate() {
        assert_eq!(parse(&bytes, NOW), Err(expected), "case {index}");
    }
    assert!(
        parse(
            &mutate(|v| v["generated_at"] = (NOW + MAX_FUTURE_SKEW_S).into()),
            NOW
        )
        .is_ok()
    );
}

#[test]
fn size_bounds_are_checked_before_parsing() {
    assert_eq!(
        parse(&vec![b' '; MAX_SNAPSHOT_BYTES + 1], NOW),
        Err(SnapshotError::TooLarge)
    );
    let mut many = sample();
    many.sources = (0..=MAX_SOURCES)
        .map(|i| SourceRecord {
            name: format!("s{i}"),
            ..sample().sources[0].clone()
        })
        .collect();
    assert_eq!(encode(&many), Err(SnapshotError::Bounds));
}

#[test]
fn source_names_are_restricted() {
    assert!(valid_name("aws-ip-ranges"));
    for bad in ["", "AWS", "a b", "a_b", &"a".repeat(65)] {
        assert!(!valid_name(bad), "{bad}");
    }
}
