use super::*;
use crate::test_support::Directory;
use crate::{
    arguments::{self, Command},
    storage,
};

fn init(root: &Path) -> crate::arguments::Init {
    let Command::Init(init) = arguments::parse(
        [
            "init",
            "--framework",
            "electron",
            "--write",
            "--workspace",
            root.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap() else {
        panic!()
    };
    init
}

#[test]
fn complete_manifest_is_published_once_and_existing_bytes_never_change() {
    let root = Directory::new();
    let init = init(&root.0);
    let receipt = storage::publish(&init).unwrap();
    assert!(receipt.published && receipt.cleanup_complete);
    assert_eq!(root.count(), 1);
    let path = root.0.join(&init.output);
    let expected = crate::manifest::bytes(&init).unwrap();
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert_eq!(
        storage::publish(&init).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&path).unwrap(), expected);
}

#[test]
fn competing_writers_have_exactly_one_publication_and_no_temporary_links() {
    let root = Directory::new();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let init = init(&root.0);
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                storage::publish(&init)
            })
        })
        .collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|r| r.is_err()).count(), 1);
    assert_eq!(root.count(), 1);
    assert_eq!(
        fs::read(root.0.join("humanymous-native.json")).unwrap(),
        crate::manifest::bytes(&init(&root.0)).unwrap()
    );
}

#[test]
fn failed_staging_never_creates_destination_and_cleans_its_temporary_link() {
    let root = Directory::new();
    let error = publish_with(
        &root.0,
        "humanymous-native.json",
        b"{}",
        |file, _| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        },
        |path| fs::remove_file(path),
    )
    .unwrap_err();
    assert!(error.to_string().contains("staging_cleanup_complete=true"));
    assert_eq!(root.count(), 0);
}

#[test]
fn cleanup_failure_reports_publication_without_rolling_back_or_repeating_it() {
    let root = Directory::new();
    let receipt = publish_with(
        &root.0,
        "humanymous-native.json",
        b"{}",
        |file, bytes| {
            file.write_all(bytes)?;
            file.sync_all()
        },
        |path| {
            let _deny_delete = OpenOptions::new().read(true).share_mode(1).open(path)?;
            fs::remove_file(path)
        },
    )
    .unwrap();
    assert!(receipt.published);
    assert!(!receipt.cleanup_complete);
    assert!(root.0.join(receipt.temporary_file.unwrap()).is_file());
    assert_eq!(
        fs::read(root.0.join("humanymous-native.json")).unwrap(),
        b"{}"
    );
}

#[test]
fn all_ancestors_remain_pinned_against_rename_and_static_junctions_are_denied() {
    let root = Directory::new();
    let child = root.0.join("workspace");
    fs::create_dir(&child).unwrap();
    let handles = pin(&child).unwrap();
    assert!(fs::rename(&child, root.0.join("moved")).is_err());
    drop(handles);
    fs::remove_dir(&child).unwrap();
    let outside = Directory::new();
    let junction = root.0.join("linked");
    junction::create(&outside.0, &junction).unwrap();
    assert!(storage::publish(&init(&junction)).is_err());
    assert_eq!(outside.count(), 0);
    junction::delete(&junction).unwrap();
}

#[test]
fn existing_link_target_and_unsafe_workspace_paths_are_refused() {
    let root = Directory::new();
    let outside = Directory::new();
    let output = root.0.join("humanymous-native.json");
    junction::create(&outside.0, &output).unwrap();
    assert_eq!(
        storage::publish(&init(&root.0)).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(outside.count(), 0);
    junction::delete(&output).unwrap();
    for path in [
        "relative",
        "C:\\work\\..\\other",
        "\\\\server\\share",
        "\\\\?\\C:\\work",
        "C:\\work:stream",
        "C:\\work.\\child",
    ] {
        assert!(pin(Path::new(path)).is_err(), "{path}");
    }
}

#[test]
fn library_publication_requires_explicit_write_and_revalidates_basename() {
    let root = Directory::new();
    let mut selected = init(&root.0);
    selected.write = false;
    assert!(storage::publish(&selected).is_err());
    selected.write = true;
    selected.output = "../outside.json".into();
    assert!(storage::publish(&selected).is_err());
    assert_eq!(root.count(), 0);
}
