use crate::{
    arguments::{self, Command},
    manifest,
};

fn parse(args: &[&str]) -> Result<Command, String> {
    arguments::parse(args.iter().map(|arg| (*arg).to_owned()))
}

#[test]
fn default_and_explicit_dry_run_are_identical_and_declare_unknown_qualification() {
    for framework in ["electron", "tauri", "winit", "dotnet", "qt", "flutter"] {
        let Command::Init(default) = parse(&["init", "--framework", framework]).unwrap() else {
            panic!()
        };
        let Command::Init(explicit) =
            parse(&["init", "--framework", framework, "--dry-run"]).unwrap()
        else {
            panic!()
        };
        assert_eq!(default, explicit);
        assert!(!default.write);
        let bytes = manifest::bytes(&default).unwrap();
        assert_eq!(bytes, manifest::bytes(&explicit).unwrap());
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["qualified"], false);
        assert_eq!(value["integration_status"], "not_implemented");
        assert!(value["target"].is_null());
        assert!(
            value["available_capabilities"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(value["requested_policy"]["sandbox_required"], true);
        assert_eq!(value["requested_policy"]["renderer_native_access"], false);
    }
}

#[test]
fn ambiguous_unknown_missing_or_unbounded_arguments_fail_closed() {
    for args in [
        vec![],
        vec!["doctor"],
        vec!["init"],
        vec!["init", "--framework", "unknown"],
        vec!["init", "--framework", "electron", "--write"],
        vec![
            "init",
            "--framework",
            "electron",
            "--write",
            "--dry-run",
            "--workspace",
            "C:\\work",
        ],
        vec!["init", "--framework", "electron", "--framework", "tauri"],
        vec!["init", "--framework", "electron", "--force"],
        vec!["init", "--framework", "electron", "--target", "arbitrary"],
        vec![
            "init",
            "--framework",
            "electron",
            "--runtime-version",
            "bad version",
        ],
        vec!["init", "--framework", "electron", "--output"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    assert!(
        parse(&[
            "init",
            "--framework",
            "electron",
            "--runtime-version",
            "--dry-run",
            "--write",
            "--workspace",
            "C:\\work"
        ])
        .is_err()
    );
    assert!(arguments::parse(vec!["x".to_owned(); 33]).is_err());
    assert!(arguments::parse(vec!["x".repeat(4097)]).is_err());
    assert!(parse(&["--help"]).is_ok());
    assert!(parse(&["--version"]).is_ok());
}

#[test]
fn output_names_cannot_escape_or_use_dos_device_aliases() {
    for name in [
        "../secret.json",
        "/absolute.json",
        "a/b.json",
        "a\\b.json",
        "a:stream.json",
        ".json",
        "CON.json",
        "con.other.json",
        "NUL.json",
        "LPT9.json",
        "a.json ",
        "a.json.",
        "a.exe",
        "a b.json",
    ] {
        assert!(arguments::validate_output(name).is_err(), "{name}");
    }
    assert!(arguments::validate_output("humanymous-native.json").is_ok());
}

#[test]
fn declared_runtime_and_target_are_requirements_not_observed_support() {
    let Command::Init(init) = parse(&[
        "init",
        "--framework",
        "electron",
        "--target",
        "x86_64-pc-windows-msvc",
        "--runtime-version",
        "42.0.1",
    ])
    .unwrap() else {
        panic!()
    };
    let value = serde_json::to_value(manifest::build(&init)).unwrap();
    assert_eq!(value["runtime_version"], "42.0.1");
    assert_eq!(value["qualified"], false);
    assert_eq!(value["generator"]["package"], "humanymous-native-cli");
    assert!(
        !String::from_utf8(manifest::bytes(&init).unwrap())
            .unwrap()
            .contains("<script")
    );
}
