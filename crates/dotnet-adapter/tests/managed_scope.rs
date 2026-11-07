//! The human-approved exception is exactly three authored C# source paths.
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

fn sources(root: &Path, directory: &Path, found: &mut BTreeSet<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        assert!(
            !entry.file_type().unwrap().is_symlink(),
            "source tree symlink is not approved"
        );
        if path.is_dir() {
            sources(root, &path, found);
        } else if path.extension().is_some_and(|x| x == "cs") {
            found.insert(path.strip_prefix(root).unwrap().to_owned());
        }
    }
}
#[test]
fn exact_three_managed_sources_and_no_hidden_build_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dotnet");
    let mut actual = BTreeSet::new();
    sources(&root, &root, &mut actual);
    let expected: BTreeSet<PathBuf> = [
        "src/Humanymous.Native/NativeBinding.cs",
        "src/Humanymous.Native/HumanymousMiddleware.cs",
        "pilot/Program.cs",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    assert_eq!(actual, expected);
    for source in &actual {
        let text = std::fs::read_to_string(root.join(source)).unwrap();
        for unapproved in [
            "NativeLibrary.Load",
            "NativeLibrary.Free",
            "unsafe {",
            "unsafe{",
        ] {
            assert!(
                !text.contains(unapproved),
                "unapproved loader/unsafe implementation: {}",
                source.display()
            );
        }
    }
    for project in [
        "src/Humanymous.Native/Humanymous.Native.csproj",
        "pilot/Humanymous.HostPilot.csproj",
        "Directory.Build.props",
    ] {
        let text = std::fs::read_to_string(root.join(project)).unwrap();
        for forbidden in ["<UsingTask", "<Exec ", "<WriteCodeFragment"] {
            assert!(
                !text.contains(forbidden),
                "extra source/executable generation is not approved: {project}"
            );
        }
        if project.ends_with(".csproj") {
            assert!(text.contains("<EnableDefaultCompileItems>false</EnableDefaultCompileItems>"));
            let actual: Vec<_> = text
                .split("<Compile ")
                .skip(1)
                .map(|tail| tail.split("/>").next().unwrap().trim())
                .collect();
            let expected = if project.starts_with("src/") {
                vec![
                    "Include=\"NativeBinding.cs\"",
                    "Include=\"HumanymousMiddleware.cs\"",
                ]
            } else {
                vec!["Include=\"Program.cs\""]
            };
            assert_eq!(
                actual, expected,
                "only explicit approved compile inputs are allowed"
            );
        } else {
            assert!(!text.contains("<Compile "));
        }
    }
}
