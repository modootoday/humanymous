use super::*;
use crate::network::intel::snapshot::valid_name;
use std::collections::BTreeSet;

#[test]
fn registry_names_are_unique_valid_and_cover_every_category() {
    let names: BTreeSet<_> = REGISTRY.iter().map(|s| s.name).collect();
    assert_eq!(names.len(), REGISTRY.len());
    assert!(
        REGISTRY
            .iter()
            .all(|s| valid_name(s.name) && !s.licence.is_empty())
    );
    for category in IntelCategory::ALL {
        assert!(
            REGISTRY.iter().any(|s| s.category == category),
            "{category:?}"
        );
    }
    assert_eq!(find("aws-ip-ranges").unwrap().format, Format::AwsJson);
    assert!(find("unknown").is_none());
}

#[test]
fn only_git_sources_have_submodule_paths_and_all_urls_are_https() {
    for spec in REGISTRY {
        match spec.origin {
            SourceOrigin::Git { repository, files } => {
                assert_eq!(
                    spec.submodule_path().unwrap(),
                    format!("third_party/ip-intel/{}", spec.name)
                );
                assert!(repository.starts_with("https://") && !files.is_empty());
            }
            SourceOrigin::Http { urls } => {
                assert!(spec.submodule_path().is_none());
                assert!(!urls.is_empty() && urls.iter().all(|u| u.starts_with("https://")));
            }
            SourceOrigin::OperatorUrl { discovery } => {
                assert!(spec.submodule_path().is_none() && discovery.starts_with("https://"));
            }
        }
    }
}

#[test]
fn normalize_sorts_dedups_and_masks() {
    let prefixes = normalize(
        Format::Netset,
        b"198.51.100.0/24\n192.0.2.9/24\n192.0.2.0/24\n2001:db8::1/32\n",
    )
    .unwrap();
    let text: Vec<String> = prefixes.iter().map(ToString::to_string).collect();
    assert_eq!(text, ["192.0.2.0/24", "198.51.100.0/24", "2001:db8::/32"]);
}

#[test]
fn normalize_never_returns_an_empty_or_unbounded_list() {
    assert_eq!(
        normalize(Format::Netset, b"# only comments\n"),
        Err(SourceError::Empty)
    );
    assert_eq!(
        normalize(Format::Netset, &vec![b'#'; MAX_SOURCE_BYTES + 1]),
        Err(SourceError::TooLarge)
    );
    assert_eq!(
        normalize(Format::Netset, &[0xff, 0xfe]),
        Err(SourceError::Malformed { line: 0 })
    );
    assert_eq!(
        SourceError::Prefix { line: 7 }.to_string(),
        "source_prefix:7"
    );
}
