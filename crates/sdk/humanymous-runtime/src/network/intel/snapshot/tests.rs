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
                    checkout: Some(Checkout {
                        gitlink: Some(SHA.into()),
                        clean: true,
                        unpinned_override: false,
                    }),
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
                    checkout: None,
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
        // Breadth guard on the scoring category, on every load.
        (
            mutate(|v| v["sources"][1]["prefixes"][0] = "0.0.0.0/0".into()),
            TooBroad,
        ),
        (
            mutate(|v| v["sources"][1]["prefixes"][1] = "::/0".into()),
            TooBroad,
        ),
        (
            mutate(|v| v["sources"][1]["prefixes"][0] = "2.0.0.0/7".into()),
            TooBroad,
        ),
        // Prefixes sorted and unique; sources sorted by name.
        (
            mutate(|v| {
                v["sources"][1]["prefixes"] = serde_json::json!(["2001:db8::/32", "192.0.2.0/24"])
            }),
            Order,
        ),
        (
            mutate(|v| {
                v["sources"][1]["prefixes"] = serde_json::json!(["192.0.2.0/24", "192.0.2.0/24"])
            }),
            Order,
        ),
        (
            mutate(|v| v["sources"][0]["name"] = "zz-last".into()),
            Order,
        ),
        // A registry source keeps its registered category: no promotion by edit.
        (
            mutate(|v| {
                v["sources"][2]["name"] = "tor-bulk-exit".into();
                v["sources"][2]["category"] = "datacenter".into();
            }),
            Category,
        ),
        // Pin record: present with a commit, override exactly when unpinned.
        (
            mutate(|v| v["sources"][0]["origin"]["git"]["checkout"] = serde_json::Value::Null),
            Checkout,
        ),
        (
            mutate(|v| v["sources"][0]["origin"]["git"]["checkout"]["clean"] = false.into()),
            Checkout,
        ),
        (
            mutate(|v| {
                v["sources"][0]["origin"]["git"]["checkout"]["unpinned_override"] = true.into()
            }),
            Checkout,
        ),
        (
            mutate(|v| v["sources"][0]["origin"]["git"]["checkout"]["gitlink"] = "x".into()),
            Checkout,
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
    // An explicitly overridden, unpinned checkout is accepted and recorded.
    let overridden = mutate(|v| {
        v["sources"][0]["origin"]["git"]["checkout"] =
            serde_json::json!({"gitlink": null, "clean": false, "unpinned_override": true})
    });
    assert!(parse(&overridden, NOW).is_ok());
    // A registry source with its own category is accepted.
    assert!(
        parse(
            &mutate(|v| {
                v["sources"][2]["name"] = "tor-bulk-exit".into();
                v["sources"][2]["category"] = "tor_exit".into();
            }),
            NOW
        )
        .is_ok()
    );
}

fn datacenter(prefixes: Vec<Prefix>) -> Snapshot {
    let mut snapshot = sample();
    snapshot.sources[1].prefixes = prefixes;
    snapshot = Snapshot::new(NOW, snapshot.sources);
    snapshot
}

#[test]
fn scoring_floor_refuses_default_routes_and_accepts_the_floor_itself() {
    for broad in ["0.0.0.0/0", "::/0", "0.0.0.0/7", "2000::/15"] {
        let snapshot = datacenter(vec![prefix(broad)]);
        assert_eq!(encode(&snapshot), Err(SnapshotError::TooBroad), "{broad}");
        assert_eq!(
            check_breadth(IntelCategory::Datacenter, &[prefix(broad)]),
            Err(SnapshotError::TooBroad)
        );
    }
    let at_floor = datacenter(vec![prefix("3.0.0.0/8"), prefix("2001::/16")]);
    let bytes = encode(&at_floor).unwrap();
    assert_eq!(parse(&bytes, NOW).unwrap(), at_floor);
    // Shadow-only categories never reach scoring and may carry bogon blocks.
    assert_eq!(
        check_breadth(IntelCategory::Blocklist, &[prefix("224.0.0.0/3")]),
        Ok(())
    );
}

#[test]
fn scoring_coverage_cap_is_enforced_per_category_across_sources() {
    let slash8 = |first: u8| prefix(&format!("{first}.0.0.0/8"));
    let at_cap: Vec<Prefix> = (1..=64).map(slash8).collect();
    assert!(encode(&datacenter(at_cap.clone())).is_ok());
    let over: Vec<Prefix> = (1..=65).map(slash8).collect();
    assert_eq!(
        encode(&datacenter(over.clone())),
        Err(SnapshotError::Coverage)
    );
    // The cap applies to the category union: two sources under it alone,
    // over it together, are refused.
    let mut split = datacenter(over[..40].to_vec());
    let mut second = split.sources[1].clone();
    second.name = "test-datacenter-two".into();
    second.prefixes = over[40..].to_vec();
    split.sources.push(second);
    let split = Snapshot::new(NOW, split.sources);
    assert_eq!(encode(&split), Err(SnapshotError::Coverage));
    let slash16 = |n: u16| prefix(&format!("{:x}::/16", 0x2000 + n));
    assert!(encode(&datacenter((0..16).map(slash16).collect())).is_ok());
    assert_eq!(
        encode(&datacenter((0..17).map(slash16).collect())),
        Err(SnapshotError::Coverage)
    );
    // Shadow categories are not capped.
    assert_eq!(check_breadth(IntelCategory::TorExit, &over), Ok(()));
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
