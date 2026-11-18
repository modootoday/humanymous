use super::*;
use std::net::{Ipv4Addr, Ipv6Addr};

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
    for broad in [
        "0.0.0.0/0",
        "::/0",
        "0.0.0.0/7",
        "2000::/15",
        "2600::/16",
        "2600::/18",
    ] {
        let snapshot = datacenter(vec![prefix(broad)]);
        assert_eq!(encode(&snapshot), Err(SnapshotError::TooBroad), "{broad}");
        assert_eq!(
            check_breadth(IntelCategory::Datacenter, &[prefix(broad)]),
            Err(SnapshotError::TooBroad)
        );
    }
    let at_floor = datacenter(vec![prefix("3.0.0.0/8"), prefix("2600::/19")]);
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
    // IPv6 cap 2^112: sixteen /20s reach it exactly, a seventeenth exceeds it.
    let slash20 = |n: u16| prefix(&format!("{:x}::/20", 0x2400 + (n << 4)));
    assert!(encode(&datacenter((0..16).map(slash20).collect())).is_ok());
    assert_eq!(
        encode(&datacenter((0..17).map(slash20).collect())),
        Err(SnapshotError::Coverage)
    );
    // Shadow categories are not capped.
    assert_eq!(check_breadth(IntelCategory::TorExit, &over), Ok(()));
}

/// Disjoint, aligned blocks of a `width`-bit family starting at `base` whose
/// sizes sum to exactly `total`, none shorter than `/shortest`, as
/// `(network, length)` pairs.
fn blocks(width: u8, base: u128, total: u128, shortest: u8) -> Vec<(u128, u8)> {
    let mut out = Vec::new();
    let mut next = base;
    let mut rest = total;
    for len in shortest..=width {
        let size = 1u128 << (width - len);
        let count = if len == shortest {
            rest / size
        } else {
            (rest / size) & 1
        };
        for _ in 0..count {
            out.push((next, len));
            next += size;
        }
        rest -= count * size;
    }
    assert_eq!(rest, 0);
    out
}

fn v6_blocks(base: u128, total: u128, shortest: u8) -> Vec<Prefix> {
    blocks(128, base, total, shortest)
        .into_iter()
        .map(|(net, len)| prefix(&format!("{}/{len}", Ipv6Addr::from(net))))
        .collect()
}

fn v4_blocks(base: u32, total: u64, shortest: u8) -> Vec<Prefix> {
    blocks(32, base.into(), total.into(), shortest)
        .into_iter()
        .map(|(net, len)| {
            let net = u32::try_from(net).unwrap();
            prefix(&format!("{}/{len}", Ipv4Addr::from(net)))
        })
        .collect()
}

#[test]
fn carrier_scale_ipv6_aggregates_are_refused() {
    // 2600::/12 (Comcast, Verizon, T-Mobile and other North American
    // residential and mobile carriers) split into sixteen /16s: every piece is
    // below the /19 floor.
    let carriers: Vec<Prefix> = (0..16u16)
        .map(|n| prefix(&format!("{:x}::/16", 0x2600 + n)))
        .collect();
    assert_eq!(
        check_breadth(IntelCategory::Datacenter, &carriers),
        Err(SnapshotError::TooBroad)
    );
    assert_eq!(encode(&datacenter(carriers)), Err(SnapshotError::TooBroad));
    // The same /12 split into floor-legal /20s is refused by the coverage cap.
    let base = 0x2600u128 << 112;
    let at_floor: Vec<Prefix> = (0..256u128)
        .map(|n| prefix(&format!("{}/20", Ipv6Addr::from(base + (n << 108)))))
        .collect();
    assert_eq!(
        check_breadth(IntelCategory::Datacenter, &at_floor),
        Err(SnapshotError::Coverage)
    );
}

/// Measured 2026-11-18 from the pinned X4BNet checkout
/// (`c60005a45e8b93dbdaa5d5d750de81a76a9d6a0f`, `output/datacenter/ipv6.txt`):
/// shortest prefix /20, merged coverage exactly this many addresses (2^109.76).
const X4BNET_V6_COVERAGE: u128 = 1_096_521_313_767_921_542_387_388_511_158_272;
/// Same checkout, `output/datacenter/ipv4.txt`: shortest /8, merged coverage.
const X4BNET_V4_COVERAGE: u64 = 408_109_015;

#[test]
fn measured_x4bnet_breadth_fits_with_headroom() {
    // A fixture with the measured stats: same shortest prefixes, same merged
    // coverage, so the bounds are tested without the submodule.
    let mut fixture = v6_blocks(0x2400u128 << 112, X4BNET_V6_COVERAGE, 20);
    assert_eq!(fixture.iter().map(Prefix::len).min(), Some(20));
    fixture.extend(v4_blocks(0x0300_0000, X4BNET_V4_COVERAGE, 8));
    assert_eq!(
        PrefixSet::new(fixture.iter().copied()).coverage(),
        (X4BNET_V4_COVERAGE, X4BNET_V6_COVERAGE)
    );
    assert_eq!(check_breadth(IntelCategory::Datacenter, &fixture), Ok(()));
    // Headroom, checked at compile time: measured IPv6 coverage is under a
    // quarter of its cap, IPv4 under half, and the IPv6 floor sits below the
    // shortest measured prefix.
    const {
        assert!(X4BNET_V6_COVERAGE * 4 < SCORING_MAX_V6);
        assert!(X4BNET_V4_COVERAGE * 2 < SCORING_MAX_V4);
        assert!(SCORING_FLOOR.v6 < 20 && SCORING_FLOOR.v4 <= 8);
    }
}

/// Reads the real pinned list. Ignored by default because the submodule is
/// optional in a checkout; an explicit `--ignored` run without the data fails
/// with the reason instead of reporting a pass.
#[test]
#[ignore = "needs the initialized third_party/ip-intel/x4bnet-datacenter submodule"]
fn pinned_x4bnet_datacenter_list_passes_the_breadth_guard() {
    let spec = source::find("x4bnet-datacenter").unwrap();
    let source::SourceOrigin::Git { files, .. } = spec.origin else {
        panic!("x4bnet-datacenter is a git source");
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(spec.submodule_path().unwrap());
    let mut bytes = Vec::new();
    for file in files {
        let path = root.join(file);
        match std::fs::read(&path) {
            Ok(body) => bytes.extend(body),
            Err(error) => panic!(
                "SKIPPED, not passed: pinned X4BNet data absent at {} ({error}); run \
                 git submodule update --init third_party/ip-intel/x4bnet-datacenter",
                path.display()
            ),
        }
        bytes.push(b'\n');
    }
    let prefixes = source::normalize(spec.format, &bytes).unwrap();
    assert_eq!(check_breadth(spec.category, &prefixes), Ok(()));
    // The pinned data is exactly what the fixture above measures.
    let shortest_v6 = prefixes
        .iter()
        .filter(|p| p.network().is_ipv6())
        .map(Prefix::len)
        .min();
    assert_eq!(shortest_v6, Some(20));
    assert_eq!(
        PrefixSet::new(prefixes.iter().copied()).coverage(),
        (X4BNET_V4_COVERAGE, X4BNET_V6_COVERAGE)
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
