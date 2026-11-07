//! Real owned-file observations; no immutable identity or cache authority claimed.
use humanymous_agent_maintenance::selected_inputs::{
    FrozenSelection, Input, Limits, ReadStatus, Root, SelectionError, observe,
};
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-selected-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn selection(&self, files: &[&str], complete: bool, limits: &Limits) -> FrozenSelection {
        FrozenSelection::new(
            vec![Root {
                id: "fixture".into(),
                path: self.0.clone(),
            }],
            files
                .iter()
                .map(|path| Input {
                    root: "fixture".into(),
                    path: (*path).into(),
                })
                .collect(),
            complete,
            limits,
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn binary_and_empty_files_do_not_inherit_canonical_lf_policy() {
    let f = Fixture::new();
    fs::write(f.0.join("binary"), [0, 13, 255, 10]).unwrap();
    fs::write(f.0.join("empty"), []).unwrap();
    let receipt = observe(
        &f.selection(&["binary", "empty"], true, &Limits::default()),
        Limits::default(),
    )
    .unwrap();
    assert!(receipt.content_read_complete && receipt.input_selection_complete);
    assert_eq!(receipt.actual_read_bytes, 4);
    assert_eq!(receipt.files.len(), 2);
    assert!(!receipt.provider_identity_qualified && !receipt.qualified_complete);
    assert!(
        !receipt.immutable_snapshot && !receipt.execution_authorized && !receipt.cache_authorized
    );
}

#[test]
fn unknown_selection_is_not_promoted_by_successful_content_reads() {
    let f = Fixture::new();
    fs::write(f.0.join("a"), b"abc").unwrap();
    let receipt = observe(
        &f.selection(&["a"], false, &Limits::default()),
        Limits::default(),
    )
    .unwrap();
    assert!(receipt.content_read_complete);
    assert!(!receipt.input_selection_complete && !receipt.qualified_complete);
}

#[test]
fn actual_files_share_the_aggregate_budget_without_hidden_probes() {
    let f = Fixture::new();
    fs::write(f.0.join("a"), b"abc").unwrap();
    fs::write(f.0.join("b"), b"def").unwrap();
    let limits = Limits {
        max_total_read_bytes: 5,
        max_file_bytes: 8,
        ..Limits::default()
    };
    let receipt = observe(&f.selection(&["a", "b"], true, &limits), limits).unwrap();
    assert_eq!(receipt.actual_read_bytes, 5);
    assert_eq!(receipt.probe_bytes, 0);
    assert!(receipt.files[0].content.complete());
    assert_eq!(receipt.files[1].content.status, ReadStatus::AggregateLimit);
    assert!(receipt.files[1].content.sha256.is_none());
    assert!(!receipt.content_read_complete);
}

#[test]
fn tighter_observation_counts_reject_a_looser_frozen_selection() {
    let f = Fixture::new();
    let selection = f.selection(&["not-opened-a", "not-opened-b"], true, &Limits::default());
    let result = observe(
        &selection,
        Limits {
            max_files: 1,
            ..Limits::default()
        },
    );
    assert!(matches!(result, Err(SelectionError::CountLimit)));
    let roots = vec![
        Root {
            id: "a".into(),
            path: f.0.clone(),
        },
        Root {
            id: "b".into(),
            path: f.0.join("not-opened-root"),
        },
    ];
    let selected = FrozenSelection::new(
        roots,
        vec![Input {
            root: "a".into(),
            path: "not-opened".into(),
        }],
        true,
        &Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        observe(
            &selected,
            Limits {
                max_roots: 1,
                ..Limits::default()
            }
        ),
        Err(SelectionError::CountLimit)
    ));
}

#[test]
fn missing_or_directory_inputs_remain_explicit_provider_errors() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("directory")).unwrap();
    let receipt = observe(
        &f.selection(&["missing", "directory"], true, &Limits::default()),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(receipt.files.len(), 2);
    assert_eq!(receipt.errors.len(), 2);
    assert_eq!(receipt.actual_read_bytes, 0);
    assert!(
        receipt
            .files
            .iter()
            .all(|file| file.content.status == ReadStatus::ProviderRefused)
    );
    assert!(!receipt.content_read_complete && !receipt.qualified_complete);
}

#[test]
fn selected_file_link_is_refused_without_reading_its_target() {
    let f = Fixture::new();
    let target = f.0.join("target");
    let link = f.0.join("link");
    fs::write(&target, b"not selected through this link").unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, &link)
        .expect("this actual boundary case requires owned symlink creation");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link)
        .expect("this actual boundary case requires owned symlink creation");
    #[cfg(not(any(windows, unix)))]
    panic!("selected-file link boundary is unsupported on this platform");
    let receipt = observe(
        &f.selection(&["link"], true, &Limits::default()),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(receipt.actual_read_bytes, 0);
    assert_eq!(receipt.files[0].content.status, ReadStatus::ProviderRefused);
    assert!(!receipt.content_read_complete && !receipt.qualified_complete);
}
