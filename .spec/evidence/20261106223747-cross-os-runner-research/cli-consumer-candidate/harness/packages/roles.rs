//! Closed source-owned target roles; one Runtime package is not every context.
use super::{Packages, Role};
use serde_json::{Value, json};
use std::path::Path;

impl Role {
    fn spec(
        self,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        match self {
            Self::PlatformLibrary => (
                "humanymous-platform",
                "crates/integrations/humanymous-platform",
                "humanymous_platform",
                "lib",
                "src/lib.rs",
            ),
            Self::RuntimeLibrary => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "humanymous_runtime",
                "lib",
                "src/lib.rs",
            ),
            Self::GatewayNative => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "gateway-native-process",
                "test",
                "tests/gateway/native_process.rs",
            ),
            Self::AuthorityNative => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "authority-native-process",
                "test",
                "tests/authority/native_process.rs",
            ),
            Self::AuditDurableWitness => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "audit-durable-witness",
                "test",
                "tests/audit/durable_witness.rs",
            ),
            Self::AuditKeyBackups => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "audit-key-backups",
                "test",
                "tests/audit/key_backups.rs",
            ),
            Self::AuditKeyCustody => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "audit-key-custody",
                "test",
                "tests/audit/key_custody.rs",
            ),
            Self::CliService => (
                "humanymous-cli",
                "crates/apps/humanymous-cli",
                "cli-service-process",
                "test",
                "src/qualification/tests/cli_service_process.rs",
            ),
            Self::CliRuntimeService => (
                "humanymous-cli",
                "crates/apps/humanymous-cli",
                "cli-runtime-service-process",
                "test",
                "src/qualification/tests/cli_runtime_service_process.rs",
            ),
            Self::GatewayBinary => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "humanymous-runtime",
                "bin",
                "src/gateway/main.rs",
            ),
            Self::AuthorityBinary => (
                "humanymous-runtime",
                "crates/sdk/humanymous-runtime",
                "humanymous-authority",
                "bin",
                "src/authority/main.rs",
            ),
            Self::CliBinary => (
                "humanymous-cli",
                "crates/apps/humanymous-cli",
                "humanymous",
                "bin",
                "src/main.rs",
            ),
            Self::RunnerBinary | Self::RunnerHarness => (
                "humanymous-cli",
                "crates/apps/humanymous-cli",
                "humanymous-native-runner",
                "bin",
                "src/bin/native_runner/main.rs",
            ),
        }
    }

    fn test_profile(self) -> bool {
        !matches!(self, Self::GatewayBinary | Self::AuthorityBinary | Self::CliBinary | Self::RunnerBinary)
    }

    fn required_features(self) -> &'static [&'static str] {
        match self {
            Self::CliService => &["service-pilot"],
            Self::CliRuntimeService => &["runtime-service-pilot"],
            Self::RunnerBinary | Self::RunnerHarness => &["qualification-native-runner"],
            _ => &[],
        }
    }
}

impl Packages {
    pub(crate) fn matches(&self, entry: &Value, role: Role) -> bool {
        let (name, directory, target_name, kind, source) = role.spec();
        let Some(package) = entry["package_id"]
            .as_str()
            .and_then(|id| self.entries.get(id))
        else {
            return false;
        };
        if package["name"] != name
            || package["manifest_path"].as_str().map(Path::new)
                != Some(self.root.join(directory).join("Cargo.toml").as_path())
            || entry["target"] != target_name
            || entry["kind"] != json!([kind])
            || entry["profile_test"] != role.test_profile()
            || entry["source_path"].as_str().map(Path::new)
                != Some(self.root.join(directory).join(source).as_path())
        {
            return false;
        }
        let Some(features) = entry["features"].as_array() else {
            return false;
        };
        if features.iter().any(|value| value.as_str().is_none())
            || role.required_features().iter().any(|required| !features.iter().any(|value| value.as_str() == Some(*required)))
        {
            return false;
        }
        let Some(targets) = package["targets"].as_array() else {
            return false;
        };
        let found: Vec<_> = targets
            .iter()
            .filter(|target| target["name"] == entry["target"] && target["kind"] == entry["kind"])
            .collect();
        found.len() == 1
            && found[0]["src_path"].as_str().map(Path::new)
                == Some(self.root.join(directory).join(source).as_path())
            && role.required_features().iter().all(|required| {
                found[0]["required-features"].as_array().is_some_and(|declared| {
                    declared.iter().any(|value| value.as_str() == Some(*required))
                })
            })
    }

    /// Join an actual compiler ledger row to a file; never process/host authority.
    /// Actor dispatch still requires its separately owned source-bound admission.
    pub(crate) fn verify_runner_artifact(&self, entry: &Value, executable: &Path) -> std::io::Result<()> {
        let selected = executable.canonicalize()?;
        let recorded = entry["executable"].as_str().map(Path::new)
            .ok_or_else(|| std::io::Error::other("runner artifact path absent"))?.canonicalize()?;
        if self.matches(entry, Role::RunnerHarness) || !self.matches(entry, Role::RunnerBinary) || selected != recorded
            || entry["sha256"] != crate::native_runner::evidence::digest(&selected)?
        {
            return Err(std::io::Error::other("runner compiler role/artifact mismatch"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fixture_entry(role: Role) -> Value {
        let packages = Self::fixture();
        let (name, directory, target, kind, source) = role.spec();
        let package = packages.entries.values().find(|package| package["name"] == name).unwrap();
        json!({"package_id":package["id"],"target":target,"kind":[kind],"source_path":packages.root.join(directory).join(source),"profile_test":role.test_profile(),"features":role.required_features()})
    }

    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        let root = std::env::current_dir().unwrap();
        let mut records = std::collections::BTreeMap::<&str, Value>::new();
        for role in [
            Role::PlatformLibrary,
            Role::RuntimeLibrary,
            Role::GatewayNative,
            Role::AuthorityNative,
            Role::AuditDurableWitness,
            Role::AuditKeyBackups,
            Role::AuditKeyCustody,
            Role::CliService,
            Role::CliRuntimeService,
            Role::GatewayBinary,
            Role::AuthorityBinary,
            Role::CliBinary,
            Role::RunnerBinary,
        ] {
            let (name, directory, target, kind, source) = role.spec();
            let id = match name {
                "humanymous-platform" => {
                    "path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0"
                }
                "humanymous-runtime" => {
                    "path+file:///C:/workspaces/humanymous/crates/sdk/humanymous-runtime#0.1.0"
                }
                _ => "opaque-humanymous-cli",
            };
            let record = records.entry(name).or_insert_with(|| json!({"id":id,"name":name,"version":"0.1.0","manifest_path":root.join(directory).join("Cargo.toml"),"source":null,"targets":[]}));
            record["targets"].as_array_mut().unwrap().push(
                json!({"name":target,"kind":[kind],"src_path":root.join(directory).join(source),"required-features":role.required_features()}),
            );
        }
        let records: Vec<_> = records.into_values().collect();
        let members: Vec<_> = records.iter().map(|record| record["id"].clone()).collect();
        Self::from_metadata(
            &json!({"workspace_root":root,"workspace_members":members,"packages":records}),
            root,
        )
        .unwrap()
    }
}
