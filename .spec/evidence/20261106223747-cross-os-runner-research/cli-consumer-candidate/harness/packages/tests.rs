use super::*;
use serde_json::json;
const PLATFORM: &str =
    "path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0";
const RUNTIME: &str = "path+file:///C:/workspaces/humanymous/crates/sdk/humanymous-runtime#0.1.0";

#[test]
fn short_id_requires_exact_member_name_manifest_and_target_join() {
    let packages = Packages::fixture();
    let entry = Packages::fixture_entry(Role::PlatformLibrary);
    assert!(packages.matches(&entry, Role::PlatformLibrary));
    for (field, value) in [
        ("package_id", json!("unknown#humanymous-platform@0.1.0")),
        ("package_id", json!(RUNTIME)),
        ("target", json!("wrong")),
        ("kind", json!(["bin"])),
    ] {
        let mut wrong = entry.clone();
        wrong[field] = value;
        assert!(!packages.matches(&wrong, Role::PlatformLibrary));
    }
    for (field, value) in [
        ("name", json!("humanymous-platform-counterfeit")),
        (
            "manifest_path",
            json!(packages.root.join("counterfeit/Cargo.toml")),
        ),
    ] {
        let mut wrong = Packages::fixture();
        wrong.entries.get_mut(PLATFORM).unwrap()[field] = value;
        assert!(!wrong.matches(&entry, Role::PlatformLibrary));
    }
    let mut nonmember = Packages::fixture();
    nonmember.entries.remove(PLATFORM);
    assert!(!nonmember.matches(&entry, Role::PlatformLibrary));
}

#[test]
fn metadata_nonmember_duplicate_registry_and_missing_version_cannot_assign_role() {
    let packages = Packages::fixture();
    let records: Vec<_> = packages.entries.values().cloned().collect();
    let members: Vec<_> = packages.entries.keys().cloned().collect();
    let metadata =
        json!({"workspace_root":packages.root,"workspace_members":members,"packages":records});
    let entry = Packages::fixture_entry(Role::PlatformLibrary);
    let mut nonmember = metadata.clone();
    nonmember["workspace_members"]
        .as_array_mut()
        .unwrap()
        .retain(|member| member != PLATFORM);
    assert!(
        !Packages::from_metadata(&nonmember, packages.root.clone())
            .unwrap()
            .matches(&entry, Role::PlatformLibrary)
    );
    let index = records
        .iter()
        .position(|record| record["id"] == PLATFORM)
        .unwrap();
    for (field, value) in [
        ("source", json!("registry+counterfeit")),
        ("version", json!("")),
        ("manifest_path", json!("relative/Cargo.toml")),
    ] {
        let mut wrong = metadata.clone();
        wrong["packages"][index][field] = value;
        assert!(Packages::from_metadata(&wrong, packages.root.clone()).is_err());
    }
    let mut duplicate = metadata.clone();
    duplicate["packages"]
        .as_array_mut()
        .unwrap()
        .push(records[index].clone());
    assert!(Packages::from_metadata(&duplicate, packages.root.clone()).is_err());
    let mut unknown = metadata;
    unknown["workspace_members"]
        .as_array_mut()
        .unwrap()
        .push(json!("absent opaque ID"));
    assert!(Packages::from_metadata(&unknown, packages.root).is_err());
}

#[test]
fn shared_runtime_id_requires_exact_context_target_kind_and_source() {
    let packages = Packages::fixture();
    for (name, role) in [
        ("gateway-native-process", Role::GatewayNative),
        ("authority-native-process", Role::AuthorityNative),
        ("audit-durable-witness", Role::AuditDurableWitness),
        ("audit-key-backups", Role::AuditKeyBackups),
        ("audit-key-custody", Role::AuditKeyCustody),
    ] {
        let entry = Packages::fixture_entry(role);
        assert!(packages.matches(&entry, role));
        let mut wrong_kind = entry.clone();
        wrong_kind["kind"] = json!(["bin"]);
        assert!(!packages.matches(&wrong_kind, role));
        for other in [
            Role::RuntimeLibrary,
            Role::GatewayNative,
            Role::AuthorityNative,
            Role::AuditDurableWitness,
            Role::AuditKeyBackups,
            Role::AuditKeyCustody,
        ] {
            assert_eq!(packages.matches(&entry, other), role == other);
        }
        let mut wrong = Packages::fixture();
        let targets = wrong.entries.get_mut(RUNTIME).unwrap()["targets"]
            .as_array_mut()
            .unwrap();
        let index = targets
            .iter()
            .position(|target| target["name"] == name)
            .unwrap();
        targets[index]["src_path"] = json!(
            wrong
                .root
                .join("crates/sdk/humanymous-runtime/tests/network/counterfeit.rs")
        );
        assert!(!wrong.matches(&entry, role));
        let mut duplicate = Packages::fixture();
        let targets = duplicate.entries.get_mut(RUNTIME).unwrap()["targets"]
            .as_array_mut()
            .unwrap();
        let repeated = targets
            .iter()
            .find(|target| target["name"] == name)
            .unwrap()
            .clone();
        targets.push(repeated);
        assert!(!duplicate.matches(&entry, role));
    }
    let library = Packages::fixture_entry(Role::RuntimeLibrary);
    assert!(packages.matches(&library, Role::RuntimeLibrary));
    assert!(!packages.matches(&library, Role::AuthorityNative));
    assert!(!packages.matches(&library, Role::GatewayNative));
    let gateway_bin = Packages::fixture_entry(Role::GatewayBinary);
    let authority_bin = Packages::fixture_entry(Role::AuthorityBinary);
    assert!(packages.matches(&gateway_bin, Role::GatewayBinary));
    assert!(!packages.matches(&gateway_bin, Role::AuthorityBinary));
    assert!(packages.matches(&authority_bin, Role::AuthorityBinary));
    assert!(!packages.matches(&authority_bin, Role::GatewayBinary));
    for name in [
        "native_process",
        "durable_witness",
        "key_custody",
        "network_tls_parity",
        "capture_hosted_transport",
    ] {
        assert!(!packages.matches(
            &json!({"package_id":RUNTIME,"target":name,"kind":["test"]}),
            Role::AuthorityNative
        ));
    }
    // Same CLI package and bin-kind are insufficient: normal actor and unit
    // harness differ by the ACTUAL compiler profile, with exact feature/source.
    let packages = Packages::fixture();
    for role in [Role::RunnerBinary, Role::RunnerHarness, Role::CliBinary, Role::CliService, Role::CliRuntimeService] {
        let entry = Packages::fixture_entry(role);
        assert!(packages.matches(&entry, role));
        for other in [Role::RunnerBinary, Role::RunnerHarness, Role::CliBinary, Role::CliService, Role::CliRuntimeService] {
            assert_eq!(packages.matches(&entry, other), role == other);
        }
        for (key, value) in [
            ("source_path", json!(packages.root.join("crates/apps/humanymous-cli/src/lib.rs"))),
            ("profile_test", json!(!entry["profile_test"].as_bool().unwrap())),
            ("package_id", json!("counterfeit-humanymous-cli")),
        ] {
            let mut wrong = entry.clone();
            wrong[key] = value;
            assert!(!packages.matches(&wrong, role));
        }
        if role != Role::CliBinary {
            let mut missing = entry.clone();
            missing["features"] = json!([]);
            assert!(!packages.matches(&missing, role));
        }
    }
    let mut runner = Packages::fixture_entry(Role::RunnerBinary);
    runner["features"] = json!(["qualification-lab-runner"]);
    assert!(!packages.matches(&runner, Role::RunnerBinary));
    let mut wrong_source = Packages::fixture();
    let targets = wrong_source.entries.get_mut("opaque-humanymous-cli").unwrap()["targets"].as_array_mut().unwrap();
    let runner = targets.iter_mut().find(|target| target["name"] == "humanymous-native-runner").unwrap();
    runner["src_path"] = json!(wrong_source.root.join("crates/apps/humanymous-cli/src/main.rs"));
    assert!(!wrong_source.matches(&Packages::fixture_entry(Role::RunnerBinary), Role::RunnerBinary));
    let directory = crate::native_runner::fixture::Directory::new().unwrap();
    let executable = directory.path().join("synthetic-artifact");
    std::fs::write(&executable, b"synthetic artifact bytes; never executed").unwrap();
    let selected = Packages::fixture_entry(Role::RunnerBinary);
    let cargo = json!({"reason":"compiler-artifact","package_id":selected["package_id"],"target":{"name":selected["target"],"kind":selected["kind"],"src_path":selected["source_path"]},"profile":{"test":false},"features":selected["features"],"executable":executable});
    let parsed = super::super::artifacts(&cargo.to_string(), false).unwrap();
    assert_eq!(parsed[0]["profile_test"], false);
    assert_eq!(parsed[0]["source_path"], selected["source_path"]);
    packages.verify_runner_artifact(&parsed[0], &executable).unwrap();
    let mut wrong_hash = parsed[0].clone();
    wrong_hash["sha256"] = json!("0".repeat(64));
    assert!(packages.verify_runner_artifact(&wrong_hash, &executable).is_err());
    let mut harness = parsed[0].clone();
    harness["profile_test"] = json!(true);
    assert!(packages.matches(&harness, Role::RunnerHarness));
    assert!(packages.verify_runner_artifact(&harness, &executable).is_err());
    let mut missing = cargo;
    missing.as_object_mut().unwrap().remove("features");
    assert!(super::super::artifacts(&missing.to_string(), false).is_err());
}
