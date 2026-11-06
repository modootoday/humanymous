//! Producer-side typo guard across Rust source modules.
use humanymous_signals::observation::lookup;
use std::{fs, path::Path};

fn signal_literals(source: &str) -> impl Iterator<Item = &str> {
    source.match_indices('"').filter_map(|(index, _)| {
        let (literal, _) = source[index + 1..].split_once('"')?;
        let mut parts = literal.splitn(3, '.');
        let layer = parts.next()?;
        let group = parts.next()?;
        let item = parts.next()?;
        let bytes = layer.as_bytes();
        (bytes.len() == 2
            && bytes[0] == b'l'
            && (b'1'..=b'7').contains(&bytes[1])
            && !group.is_empty()
            && !item.is_empty()
            && group
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && item.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.'
            }))
        .then_some(literal)
    })
}

fn scan(path: &Path, found: &mut usize) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().unwrap() != "tests" {
                scan(&path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).unwrap();
            for literal in signal_literals(&source) {
                *found += 1;
                assert!(
                    lookup(literal).is_some(),
                    "unregistered signal {literal} in {}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn escaped_quotes_do_not_disarm_the_producer_scan() {
    let source = r#"let quote = "\""; let id = "l1.navigator.webdriver"; // "l5.rit.ok" "l9.migration.unknown" "l2.incomplete""#;
    assert_eq!(
        signal_literals(source).collect::<Vec<_>>(),
        ["l1.navigator.webdriver", "l5.rit.ok"]
    );
}

#[test]
fn all_rust_producer_signal_ids_resolve() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let parent = manifest.parent().unwrap();
    // The workspace guard owns every crate. An independently packaged library
    // checks its own available source, never unrelated registry packages.
    let own_source = manifest.join("src");
    let crates = if parent.file_name().is_some_and(|name| name == "crates")
        && parent.parent().unwrap().join("Cargo.toml").is_file()
    {
        parent
    } else {
        own_source.as_path()
    };
    let mut found = 0;
    scan(crates, &mut found);
    assert!(found > 0, "producer scan must not pass vacuously");
}
