//! Workflow structure, pinned entrypoints and the aggregate authorization boundary.
use crate::{Result, command, require, source};
use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, path::Path};
use yaml_rust2::{Yaml, YamlLoader};
const REQUIRED: &[&str] = &[
    "policy",
    "native",
    "msrv",
    "quality",
    "browser",
    "browser-product",
    "nginx",
    "rust-images",
    "docker",
    "images",
    "packages",
    "node",
    "advisories",
    "documentation",
    "pages",
];

fn convert(value: &Yaml) -> Result<Value> {
    Ok(match value {
        Yaml::String(text) => json!(text),
        Yaml::Integer(number) => json!(number),
        Yaml::Boolean(boolean) => json!(boolean),
        Yaml::Null => Value::Null,
        Yaml::Real(number) => serde_json::from_str(number).map_err(|e| e.to_string())?,
        Yaml::Array(items) => Value::Array(items.iter().map(convert).collect::<Result<_>>()?),
        Yaml::Hash(items) => {
            let mut result = Map::new();
            for (key, value) in items {
                result.insert(
                    key.as_str().ok_or("non-string YAML key")?.into(),
                    convert(value)?,
                );
            }
            Value::Object(result)
        }
        _ => return Err("unsupported YAML node".into()),
    })
}
pub fn yaml(text: &str) -> Result<Value> {
    let documents = YamlLoader::load_from_str(text).map_err(|e| e.to_string())?;
    require(documents.len() == 1, "one YAML document required")?;
    convert(&documents[0])
}
fn workflows(root: &Path) -> Result<Vec<(String, Value)>> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(root.join(".github/workflows")).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("yml" | "yaml")
        ) {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            result.push((
                path.file_name()
                    .ok_or("missing workflow name")?
                    .to_string_lossy()
                    .into_owned(),
                yaml(&text)?,
            ));
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}
fn strings(value: &Value) -> BTreeSet<String> {
    if let Some(text) = value.as_str() {
        [text.into()].into()
    } else {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    }
}
fn pinned(action: &str) -> bool {
    if action.starts_with("./") {
        return true;
    }
    action.rsplit_once('@').is_some_and(|(name, sha)| {
        name.split('/').count() >= 2
            && sha.len() == 40
            && sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn upload_allowed(name: &str, condition: Option<&str>) -> bool {
    match name {
        "release.yml" => true,
        "collector-component.yml" | "cef-native-component.yml" | "cross-os-probe.yml" => {
            condition == Some("always() && vars.CI_COMPONENT_ARTIFACTS == 'true'")
        }
        "rust-validation.yml" => matches!(
            condition,
            Some(
                "startsWith(github.ref, 'refs/tags/v')"
                    | "always() && startsWith(github.ref, 'refs/tags/v')"
                    | "${{ !cancelled() && github.ref == 'refs/heads/main' && steps.native_build.outcome == 'success' }}"
            )
        ),
        "pages-build.yml" => matches!(
            condition,
            Some(
                "github.ref == 'refs/heads/main'"
                    | "${{ always() && github.ref == 'refs/heads/main' && steps.source_prepare.outcome == 'success' }}"
                    | "${{ always() && github.ref == 'refs/heads/main' && steps.archive_apex.outcome == 'success' && steps.source_finish.outputs.source-unchanged == 'true' }}"
                    | "${{ always() && github.ref == 'refs/heads/main' && steps.archive_subpath.outcome == 'success' && steps.source_finish.outputs.source-unchanged == 'true' }}"
                    | "${{ github.ref == 'refs/heads/main' && steps.source_finish.outcome == 'success' }}"
            )
        ),
        _ => false,
    }
}
fn caller_permissions(job: &Value) -> Result<()> {
    require(
        job["uses"] == "./.github/workflows/rust-validation.yml"
            && job["permissions"] == json!({"contents":"read","actions":"read"}),
        "validation caller must allow the exact read-only reusable permission ceiling",
    )
}
pub fn document(name: &str, doc: &Value) -> Result<()> {
    fn registry_env(value: &Value) -> Result<()> {
        for key in [
            "NODE_AUTH_TOKEN",
            "CARGO_REGISTRY_TOKEN",
            "CARGO_REGISTRIES_CRATES_IO_TOKEN",
            "NPM_TOKEN",
        ] {
            if let Some(value) = value[key].as_str() {
                require(
                    !value.contains("secrets."),
                    "registry secret-token fallback forbidden",
                )?;
            }
        }
        Ok(())
    }
    registry_env(&doc["env"])?;
    require(
        doc["permissions"] == json!({"contents":"read"}),
        format!("{name}: global permissions expanded"),
    )?;
    require(
        doc["on"].get("pull_request_target").is_none(),
        "privileged untrusted PR trigger forbidden",
    )?;
    let jobs = doc["jobs"].as_object().ok_or("missing workflow jobs")?;
    if name == "codeql.yml" {
        let steps = jobs["analyze"]["steps"]
            .as_array()
            .ok_or("CodeQL steps absent")?;
        let pin = "e4fba868fa4b1b91e1fdab776edc8cfbe6e9fb81";
        require(
            steps.len() == 3
                && steps[1]["uses"] == format!("github/codeql-action/init@{pin}")
                && steps[1]["with"]["languages"] == "rust"
                && steps[1]["with"]["build-mode"] == "none"
                && steps[1]["with"]["config-file"].is_null()
                && steps[2]["uses"] == format!("github/codeql-action/analyze@{pin}"),
            "CodeQL must scan current Rust without legacy Go build or exclusions",
        )?;
    }
    if name == "frozen-go-oracle.yml" {
        require(
            doc["on"].as_object().is_some_and(|events| {
                events.len() == 1 && events.contains_key("workflow_dispatch")
            }),
            "historical Go oracle must remain manual only",
        )?;
        for job in jobs.values() {
            let steps = job["steps"].as_array().ok_or("oracle steps absent")?;
            let checkouts: Vec<_> = steps
                .iter()
                .filter(|step| {
                    step["uses"]
                        .as_str()
                        .is_some_and(|action| action.starts_with("actions/checkout@"))
                })
                .collect();
            require(
                checkouts.len() == 1
                    && checkouts[0]["with"]["ref"] == "981d25cf71a5a54855d7e2b4359e832a9d5998e5"
                    && checkouts[0]["with"]["persist-credentials"] == false,
                "historical oracle must not consume current shipping Go source",
            )?;
            require(
                steps.iter().all(|step| step["with"]["cache-to"].is_null()),
                "historical oracle remote cache writes forbidden",
            )?;
        }
    }
    if name == "h2-hosted-venue-validation.yml" {
        // Specfold Cap=1 N06 hosted venue — codex/rust path-filtered push or dispatch; never ACT spoof.
        require(
            doc["on"].as_object().is_some_and(|events| {
                events.len() == 2
                    && events.contains_key("workflow_dispatch")
                    && events.contains_key("push")
            }) && strings(&doc["on"]["push"]["branches"]) == ["codex/rust".into()].into()
                && strings(&doc["on"]["push"]["paths"])
                    == [".github/workflows/h2-hosted-venue-validation.yml".into()].into(),
            "hosted venue Cap must remain Specfold codex/rust path-filtered push + dispatch",
        )?;
        require(
            jobs.len() == 2
                && jobs.contains_key("preflight")
                && jobs.contains_key("cap")
                && jobs["cap"]["needs"] == "preflight"
                && jobs["cap"].get("if").is_none()
                && jobs["cap"].get("continue-on-error").is_none()
                && jobs["preflight"]["runs-on"] == "ubuntu-latest"
                && jobs["cap"]["runs-on"] == "ubuntu-latest"
                && jobs["preflight"]["timeout-minutes"] == 25
                && jobs["cap"]["timeout-minutes"] == 60
                && doc["env"]["DOCKER_BUILD_RECORD_UPLOAD"] == "false"
                && doc["env"]["CARGO_BUILD_JOBS"] == "1"
                && doc["env"]["CARGO_INCREMENTAL"] == "0",
            "hosted venue Cap must stay bounded ubuntu-latest Specfold and preflight gated",
        )?;
        for job in ["preflight", "cap"] {
            require(
                jobs[job]["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["uses"] == "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"
                            && step["with"]["ref"] == "${{ github.sha }}"
                            && step["with"]["persist-credentials"] == false
                    })
                }),
                "hosted venue Cap jobs must check out exact run source without persisted credentials",
            )?;
        }
        let preflight = jobs["preflight"]["steps"]
            .as_array()
            .ok_or("hosted venue Cap preflight steps absent")?;
        for command in [
            "cargo test --locked -p humanymous-lab-runner --bin humanymous-lab-runner -j1",
            "cargo test --locked -p humanymous-delivery -j1",
            "cargo check --locked -p humanymous-runtime-pilot --all-targets -j1",
            "cargo clippy --locked -p humanymous-lab-runner -p humanymous-runtime-pilot -p humanymous-delivery --all-targets -j1 -- -D warnings",
            "cargo run --locked -p humanymous-delivery -- policy",
        ] {
            require(
                preflight.iter().any(|step| {
                    step["run"] == command
                        && step["if"]
                            == "${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
                }),
                "hosted venue Cap current-source preflight required",
            )?;
        }
        let cap_steps = jobs["cap"]["steps"]
            .as_array()
            .ok_or("hosted venue Cap steps absent")?;
        require(
            cap_steps.iter().any(|step| {
                step["run"].as_str().is_some_and(|run| {
                    run.contains("ACT")
                        && run.contains("GITHUB_ACTIONS")
                        && run.contains("RUNNER_OS")
                        && run.contains("RUNNER_ENVIRONMENT")
                        && run.contains("github-hosted")
                })
            }),
            "hosted venue Cap must refuse ACT and confirm github-hosted Linux",
        )?;
        require(
            cap_steps.iter().any(|step| {
                step["id"] == "bundle"
                    && step["run"].as_str().is_some_and(|run| {
                        run.contains(".spec.local/root/evidence/")
                            && run.contains("register-evidence")
                            && run.contains("index.evidence.md")
                    })
            }),
            "hosted venue Cap must prepare a fresh private Specfold evidence bundle",
        )?;
        require(
            cap_steps.iter().any(|step| {
                step["run"].as_str().is_some_and(|run| {
                    run.contains("cargo run --locked -p humanymous-lab-runner")
                        && run.contains("--w2-h2-hosted-venue")
                        && run.contains("--evidence")
                        && run.contains("steps.bundle.outputs.path")
                }) && step.get("if").is_none()
            }),
            "hosted venue Cap must execute exact lab-runner --w2-h2-hosted-venue pilot against Specfold bundle",
        )?;
    }
    if name == "component-validation.yml" {
        let input = &doc["on"]["workflow_dispatch"]["inputs"]["component"];
        require(
            doc["on"]
                .as_object()
                .is_some_and(|triggers| triggers.len() == 2)
                && doc["on"]["push"]["branches"]
                    == json!([
                        "codex/native-component-validation",
                        "codex/native-artifact-validation",
                        "codex/service-component-validation",
                        "codex/header-component-validation"
                    ])
                && input["type"] == "choice"
                && input["default"] == "capture"
                && input["required"] == true
                && input["options"] == json!(["capture", "header", "archive", "service"])
                && jobs.len() == 4
                && jobs["component"]["needs"] == "preflight"
                && jobs["component"]["if"]
                    == "${{ inputs.component != 'service' && inputs.component != 'header' && (github.event_name != 'push' || (github.ref != 'refs/heads/codex/service-component-validation' && github.ref != 'refs/heads/codex/header-component-validation')) }}"
                && jobs["component"].get("continue-on-error").is_none()
                && jobs["preflight"]["timeout-minutes"] == 25
                && jobs["component"]["timeout-minutes"] == 35
                && jobs["component"]["env"]["HUMANYMOUS_NATIVE_COMPONENT"]
                    == "${{ inputs.component || (github.ref == 'refs/heads/codex/native-artifact-validation' && 'archive') || 'capture' }}",
            "native caller must retain closed targets and bounded preflight",
        )?;
        for job in ["preflight", "component", "service", "header"] {
            require(
                jobs[job]["runs-on"]
                    == if job == "service" {
                        "windows-2025"
                    } else {
                        "ubuntu-24.04"
                    }
                    && jobs[job]["steps"].as_array().is_some_and(|steps| {
                        steps.iter().any(|step| {
                            step["uses"]
                                == "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"
                                && step["with"]["ref"] == "${{ github.sha }}"
                                && step["with"]["persist-credentials"] == false
                        })
                    }),
                "native caller requires exact-source hosted checkout",
            )?;
        }
        for (job, condition, minutes, command) in [
            (
                "service",
                "${{ inputs.component == 'service' || (github.event_name == 'push' && github.ref == 'refs/heads/codex/service-component-validation') }}",
                40,
                "cargo run --locked -p humanymous-delivery -- service-component",
            ),
            (
                "header",
                "${{ inputs.component == 'header' || (github.event_name == 'push' && github.ref == 'refs/heads/codex/header-component-validation') }}",
                35,
                "cargo run --locked -p humanymous-delivery -- native-component header",
            ),
        ] {
            require(
                jobs[job]["needs"] == "preflight"
                    && jobs[job]["if"] == condition
                    && jobs[job]["timeout-minutes"] == minutes
                    && jobs[job].get("continue-on-error").is_none()
                    && jobs[job]["steps"].as_array().is_some_and(|steps| {
                        steps.iter().any(|step| {
                            step["run"] == command
                                && step.get("if").is_none()
                                && step.get("continue-on-error").is_none()
                        })
                    }),
                "closed header/service caller or failure propagation changed",
            )?;
        }
        let profile: Value = serde_json::from_str(include_str!(
            "../../../../adapters/native/header-proof-profile.json"
        ))
        .map_err(|_| "header canonical profile invalid")?;
        require(
            jobs["header"]["container"]["image"] == profile["image"]
                && jobs["header"]["container"]["options"]
                    == "--cpus 1 --memory 2g --pids-limit 256 --cap-drop ALL --security-opt no-new-privileges"
                && jobs["component"].get("container").is_none()
                && jobs["service"].get("container").is_none(),
            "header declared compiler venue must remain exact and isolated from other profiles",
        )?;
        let preflight = jobs["preflight"]["steps"]
            .as_array()
            .ok_or("native preflight absent")?;
        for command in [
            "cargo test --locked -p humanymous-delivery -j1",
            "cargo check --locked -p humanymous-runtime -p humanymous-native-abi-export --features humanymous-runtime/capture --all-targets -j1",
            "cargo clippy --locked -p humanymous-delivery -p humanymous-runtime -p humanymous-native-abi-export --features humanymous-runtime/capture --all-targets -j1 -- -D warnings",
            "cargo run --locked -p humanymous-delivery -- policy",
        ] {
            require(
                preflight.iter().any(|step| {
                    step["run"] == command
                        && step["if"]
                            == "${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
                }),
                "native current-source preflight required",
            )?;
        }
        require(jobs["component"]["steps"].as_array().is_some_and(|steps| steps.iter().any(|step| {
            step["run"] == "cargo run --locked -p humanymous-delivery -- native-component \"$HUMANYMOUS_NATIVE_COMPONENT\""
                && step.get("if").is_none()
        })), "native target must pass only through the quoted environment and closed Rust dispatch")?;
    }
    if matches!(
        name,
        "go-validation.yml"
            | "wargame-validation.yml"
            | "browser-product-validation.yml"
            | "mount-probe-validation.yml"
            | "image-validation.yml"
    ) {
        let browser = name == "browser-product-validation.yml";
        let wargame = name == "wargame-validation.yml";
        let mount = name == "mount-probe-validation.yml";
        let image = name == "image-validation.yml";
        let product = if image {
            "image"
        } else if mount {
            "probe"
        } else {
            "product"
        };
        let branch = if image {
            "codex/image-validation"
        } else if mount {
            "codex/mount-probe-validation"
        } else if browser {
            "codex/browser-product-validation"
        } else if wargame {
            "codex/wargame-validation"
        } else {
            "codex/go-validation"
        };
        let product_command = if image {
            "cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- image-candidates"
        } else if mount {
            "cargo run --locked -p humanymous-delivery -- mount-probe"
        } else if browser {
            "cargo run --locked -p humanymous-delivery -- pilot browser-product"
        } else if wargame {
            "cargo run --locked -p humanymous-delivery -- pilot docker"
        } else {
            "cargo run --locked -p humanymous-delivery -- go-product"
        };
        require(
            strings(&doc["on"]["push"]["branches"]) == [branch.into()].into()
                && doc["on"].get("workflow_dispatch").is_some()
                && jobs.len() == 2
                && jobs[product]["needs"] == "preflight"
                && jobs[product].get("if").is_none()
                && jobs[product].get("continue-on-error").is_none()
                && jobs["preflight"]["runs-on"] == "ubuntu-24.04"
                && jobs[product]["runs-on"] == "ubuntu-24.04"
                && jobs["preflight"]["timeout-minutes"] == 25
                && jobs[product]["timeout-minutes"]
                    == if image {
                        35
                    } else if mount {
                        45
                    } else {
                        60
                    },
            "product entrypoint must stay bounded hosted and preflight gated",
        )?;
        for job in ["preflight", product] {
            require(
                jobs[job]["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["uses"] == "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"
                            && step["with"]["ref"] == "${{ github.sha }}"
                            && step["with"]["persist-credentials"] == false
                    })
                }),
                "product jobs must check out the exact run source without persisted credentials",
            )?;
        }
        let preflight = jobs["preflight"]["steps"]
            .as_array()
            .ok_or("Go preflight steps absent")?;
        let mut commands = if image {
            vec![
                "cargo test --locked -p humanymous-delivery -j1",
                "cargo clippy --locked -p humanymous-delivery --all-targets -j1 -- -D warnings",
                "cargo run --locked -p humanymous-delivery -- policy",
            ]
        } else if wargame {
            vec![
                "cargo test --locked -p humanymous-attacker -p humanymous-defender --lib -j1",
                "cargo test --locked -p humanymous-lab-runner --bin humanymous-lab-runner -j1",
                "cargo test --locked -p humanymous-delivery -j1",
                "cargo check --locked -p humanymous-runtime-pilot --all-targets -j1",
                "cargo clippy --locked -p humanymous-attacker -p humanymous-defender -p humanymous-lab-runner -p humanymous-runtime-pilot -p humanymous-delivery --all-targets -j1 -- -D warnings",
                "cargo run --locked -p humanymous-delivery -- policy",
            ]
        } else {
            vec![
                "cargo test --locked -p humanymous-lab-runner --bin humanymous-lab-runner -j1",
                "cargo test --locked -p humanymous-delivery -j1",
                "cargo check --locked -p humanymous-runtime-pilot --all-targets -j1",
                if mount {
                    "cargo clippy --locked -p humanymous-lab-runner -p humanymous-runtime-pilot -p humanymous-delivery --all-targets -j1 -- -D warnings"
                } else {
                    "cargo clippy --locked -p humanymous-adapter-go -p humanymous-lab-runner -p humanymous-runtime-pilot -p humanymous-delivery --all-targets -j1 -- -D warnings"
                },
                "cargo run --locked -p humanymous-delivery -- policy",
            ]
        };
        if !mount && !image && !wargame {
            commands.push("cargo test --locked -p humanymous-adapter-go --lib -j1");
            commands.extend([
                "cargo run --locked -p humanymous-delivery -- facade-profile rust-library",
                "cargo run --locked -p humanymous-delivery -- facade-profile rust-library adapter-go",
                "cargo test --locked -p humanymous-adapter-go --test deployment_contract -j1",
                "cargo test --locked -p humanymous --lib --no-default-features -j1",
                "cargo test --locked -p humanymous --lib --no-default-features --features adapter-go -j1",
                "cargo test --locked -p humanymous --test go_consumer --no-default-features --features adapter-go -j1",
                "cargo clippy --locked -p humanymous --all-targets --no-default-features -j1 -- -D warnings",
                "cargo clippy --locked -p humanymous --all-targets --no-default-features --features adapter-go -j1 -- -D warnings",
            ]);
        }
        if !mount && !image && !wargame {
            for (admission, enabled, id) in [
                (
                    "cargo run --locked -p humanymous-delivery -- facade-profile rust-library",
                    false,
                    "facade_default_admission",
                ),
                (
                    "cargo run --locked -p humanymous-delivery -- facade-profile rust-library adapter-go",
                    true,
                    "facade_go_admission",
                ),
            ] {
                require(
                    preflight
                        .iter()
                        .filter(|step| step["run"] == admission)
                        .count()
                        == 1
                        && preflight.iter().filter(|step| step["id"] == id).count() == 1,
                    "facade admission command or identity missing/duplicated",
                )?;
                let admitted_at = preflight
                    .iter()
                    .position(|step| step["run"] == admission)
                    .ok_or("facade request admission missing")?;
                require(
                    preflight[admitted_at]["id"] == id,
                    "facade admission identity missing",
                )?;
                for (index, step) in preflight.iter().enumerate() {
                    if let Some(run) = step["run"].as_str() {
                        require(
                            !run.contains("--all-features"),
                            "facade feature widening refused",
                        )?;
                        if run.contains("-p humanymous ") {
                            require(
                                commands.contains(&run),
                                "facade build outside closed command profile",
                            )?;
                            if run.contains("--features adapter-go") == enabled {
                                let condition = if enabled {
                                    "${{ !cancelled() && steps.toolchain.outcome == 'success' && steps.facade_go_admission.outcome == 'success' }}"
                                } else {
                                    "${{ !cancelled() && steps.toolchain.outcome == 'success' && steps.facade_default_admission.outcome == 'success' }}"
                                };
                                require(
                                    admitted_at < index && step["if"] == condition,
                                    "every facade build requires prior successful matching admission",
                                )?;
                            }
                        }
                    }
                }
            }
        }
        for command in commands {
            require(
                preflight.iter().any(|step| {
                    let condition = if command.contains("-p humanymous ") {
                        if command.contains("--features adapter-go") {
                            "${{ !cancelled() && steps.toolchain.outcome == 'success' && steps.facade_go_admission.outcome == 'success' }}"
                        } else {
                            "${{ !cancelled() && steps.toolchain.outcome == 'success' && steps.facade_default_admission.outcome == 'success' }}"
                        }
                    } else {
                        "${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
                    };
                    step["run"] == command && step["if"] == condition
                }),
                "Go current-source preflight required",
            )?;
        }
        require(
            jobs[product]["steps"].as_array().is_some_and(|steps| {
                steps
                    .iter()
                    .any(|step| step["run"] == product_command && step.get("if").is_none())
            }),
            "actual owning product profile missing",
        )?;
    }
    if name == "pages-validation.yml" {
        require(
            strings(&doc["on"]["push"]["branches"]) == ["codex/pages-validation".into()].into()
                && doc["on"].get("workflow_dispatch").is_some()
                && jobs.len() == 2
                && jobs["validate"]["uses"] == "./.github/workflows/pages-build.yml"
                && jobs["validate"]["needs"] == "preflight"
                && jobs["validate"].get("if").is_none()
                && jobs["preflight"]["runs-on"] == "ubuntu-24.04",
            "Pages-only caller must remain bounded build-only",
        )?;
        let steps = jobs["preflight"]["steps"]
            .as_array()
            .ok_or("Pages preflight absent")?;
        for command in [
            "cargo test --locked -p humanymous-delivery -j1",
            "cargo clippy --locked -p humanymous-delivery --all-targets -j1 -- -D warnings",
            "cargo run --locked -p humanymous-delivery -- policy",
            "cargo test --locked -p humanymous-pages --lib -j1",
            "cargo clippy --locked -p humanymous-pages --all-targets -j1 -- -D warnings",
        ] {
            require(
                steps.iter().any(|step| {
                    step["run"] == command
                        && step["if"]
                            == "${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
                }),
                "Pages helper preflight cannot be omitted or skipped on test failure",
            )?;
        }
    }
    if name == "pages-build.yml" {
        let steps = jobs["build"]["steps"]
            .as_array()
            .ok_or("Pages build steps absent")?;
        require(
            !steps.iter().any(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|action| action.starts_with("actions/upload-pages-artifact@"))
            }),
            "Pages serving must use the verified Rust archive, not a hidden-file filtering packager",
        )?;
        for (id, previous, command) in [
            ("package_apex", "apex", " -- package --source docs"),
            ("package_subpath", "subpath", " -- package --source docs"),
            (
                "archive_apex",
                "package_apex",
                " -- archive-check --source docs",
            ),
            (
                "archive_subpath",
                "package_subpath",
                " -- archive-check --source docs",
            ),
        ] {
            require(
                steps.iter().any(|step| {
                    step["id"] == id
                        && step["if"]
                            == format!(
                                "${{{{ !cancelled() && steps.{previous}.outcome == 'success' }}}}"
                            )
                        && step["run"].as_str().is_some_and(|run| {
                            run.contains(command) && run.contains("target/delivery-pages")
                        })
                }),
                "Pages serialized archive checks must precede serving",
            )?;
        }
        require(steps.iter().any(|step|step["id"]=="serving_upload" && step["with"]["path"]=="target/delivery-pages/apex/artifact.tar" && step["if"]=="${{ github.ref == 'refs/heads/main' && steps.source_finish.outcome == 'success' }}"),"Pages may only serve exact verified archive bytes")?;
        require(
            steps.iter().any(|step| {
                step["id"] == "serving_roundtrip"
                    && step["run"].as_str().is_some_and(|run| {
                        run.contains(" -- archive-check --source docs")
                            && run.contains("pages-serving-download/artifact.tar")
                    })
            }),
            "Pages transferred archive consumer check missing",
        )?;
        for id in ["apex", "subpath", "bundle_audit"] {
            require(
                steps.iter().any(|step| {
                    step["id"] == id
                        && step["if"]
                            == "${{ !cancelled() && steps.bundle_install.outcome == 'success' }}"
                }),
                "Pages base/audit attempts must remain independent",
            )?;
        }
        require(
            steps.iter().any(|step| {
                step["id"] == "source_finish"
                    && step["if"] == "${{ always() && steps.source_prepare.outcome == 'success' }}"
            }),
            "Pages source/failure evidence missing",
        )?;
    }
    if matches!(name, "ci.yml" | "release.yml") {
        caller_permissions(jobs.get("validate").unwrap_or(&Value::Null))?;
    }
    for (id, job) in jobs {
        require(
            matches!(
                job.get("continue-on-error"),
                None | Some(Value::Bool(false))
            ),
            "validation job failure may not be tolerated",
        )?;
        registry_env(&job["env"])?;
        let permissions = &job["permissions"];
        for (permission, allowed) in [
            ("contents", name == "release.yml" && id == "gh-release"),
            ("packages", name == "release.yml" && id == "images"),
            ("pages", name == "pages.yml" && id == "deploy"),
        ] {
            require(
                permissions[permission] != "write" || allowed,
                format!("unexpected {permission} write: {name}/{id}"),
            )?;
        }
        if permissions["id-token"] == "write" {
            require(
                (name == "release.yml" && ["crates", "npm", "images"].contains(&id.as_str()))
                    || (name == "pages.yml" && id == "deploy"),
                "unexpected OIDC privilege",
            )?;
        }
        for step in job["steps"].as_array().into_iter().flatten() {
            registry_env(&step["env"])?;
            require(
                matches!(
                    step.get("continue-on-error"),
                    None | Some(Value::Bool(false))
                ) || crate::archive_cache::policy::approved_step(name, id, step),
                "validation step failure may not be tolerated",
            )?;
            if let Some(action) = step["uses"].as_str() {
                require(pinned(action), format!("unpinned action {action}"))?;
                if action.starts_with("actions/upload-artifact@")
                    || action.starts_with("actions/upload-pages-artifact@")
                {
                    require(
                        upload_allowed(name, step["if"].as_str()),
                        "free branch automatic artifact upload forbidden",
                    )?;
                }
                if name == "rust-validation.yml" && action.starts_with("actions/cache") {
                    require(
                        crate::archive_cache::policy::approved_step(name, id, step),
                        "public archive cache boundary violated",
                    )?;
                }
            }
            for key in ["NODE_AUTH_TOKEN", "CARGO_REGISTRY_TOKEN", "NPM_TOKEN"] {
                if let Some(value) = step["env"][key].as_str() {
                    require(
                        !value.contains("secrets."),
                        "registry secret-token fallback forbidden",
                    )?;
                }
            }
            if let Some(run) = step["run"].as_str() {
                require(
                    !run.contains("delivery.py")
                        && !run.contains("check_action_pins.py")
                        && !run.contains("test_delivery.py"),
                    "authored delivery validator still uses a legacy script",
                )?;
            }
        }
    }
    if matches!(name, "rust-validation.yml" | "image-validation.yml") {
        let image_steps = jobs[if name == "image-validation.yml" {
            "image"
        } else {
            "rust-images"
        }]["steps"]
            .as_array()
            .ok_or("image steps absent")?;
        let build = image_steps
            .iter()
            .find(|step| step["id"] == "runtime_image_build")
            .ok_or("required image build absent")?;
        require(
            build["with"]["push"] == false
                && build["with"]["load"] == true
                && build["with"]["file"] == "build/rust/runtime-candidate.Dockerfile"
                && build["with"]["tags"] == "humanymous-runtime-candidate:validation",
            "validation image build must remain local and source bounded",
        )?;
        let scan = image_steps
            .iter()
            .find(|step| step["id"] == "runtime_image_scan")
            .ok_or("required typed image scan absent")?;
        require(
            scan["with"]["version"] == "v0.70.0"
                && scan["with"]["format"] == "json"
                && scan["with"]["output"] == "target/delivery-image-scan/trivy.json"
                && scan["with"]["list-all-pkgs"] == "true"
                && scan["with"]["scanners"] == "vuln"
                && scan["with"]["severity"] == "UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL"
                && scan["with"]["ignore-unfixed"] == false
                && scan["with"]["exit-code"] == "1",
            "image scan coverage or vulnerability policy weakened",
        )?;
        require(
            image_steps.iter().any(|step| {
                step["run"] == "cargo run --locked -p humanymous-delivery -- image-scan verify"
                    && step["if"]
                        == "${{ !cancelled() && steps.runtime_image_build.outcome == 'success' }}"
                    && step["env"]["HUMANYMOUS_TRIVY_OUTCOME"]
                        == "${{ steps.runtime_image_scan.outcome }}"
            }),
            "image scan failure/coverage accounting missing",
        )?;
        require(
            doc["env"]["DOCKER_BUILD_RECORD_UPLOAD"] == "false",
            "automatic Docker build-record artifacts must be disabled",
        )?;
        for step in jobs
            .values()
            .filter_map(|job| job["steps"].as_array())
            .flatten()
        {
            let action = step["uses"].as_str().unwrap_or("");
            let disabled = |key: &str| step["with"][key] == false || step["with"][key] == "false";
            let enabled = |key: &str| step["with"][key] == true || step["with"][key] == "true";
            require(
                (!action.starts_with("aquasecurity/trivy-action@") || disabled("cache"))
                    && (!action.starts_with("docker/setup-buildx-action@")
                        || disabled("cache-binary"))
                    && (!action.starts_with("actions/setup-node@")
                        || disabled("package-manager-cache"))
                    && (!action.starts_with("oven-sh/setup-bun@") || enabled("no-cache")),
                format!("implicit action cache writes must be disabled: {action}"),
            )?;
        }
    }
    if name == "rust-validation.yml" {
        require(
            strings(&doc["jobs"]["ready"]["needs"])
                == REQUIRED.iter().map(|id| (*id).into()).collect(),
            "required aggregate coverage changed",
        )?;
        for id in REQUIRED {
            let job = &jobs[*id];
            require(
                (job.get("if").is_none()
                    || (job["if"] == "${{ !cancelled() && needs.policy.result == 'success' }}"
                        && strings(&job["needs"]).contains("policy")))
                    && job.get("continue-on-error").is_none(),
                format!("required job may not be conditional/tolerated: {id}"),
            )?;
        }
        for (id, required) in [
            ("native", "pilot native"),
            ("browser", "pilot browser"),
            ("browser-product", "pilot browser-product"),
            ("nginx", " -- nginx"),
            ("rust-images", "image-candidates"),
            ("docker", "pilot docker"),
            ("packages", " -- package"),
            ("node", " -- node"),
            ("advisories", "audit-closure"),
        ] {
            require(
                jobs[id]["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["run"].as_str().is_some_and(|run| {
                            run.contains("cargo run --locked")
                                && run.contains("scripts/ci/delivery/Cargo.toml")
                                && run.contains(required)
                        }) && (step.get("if").is_none()
                            || (id == "node" && step["if"] == "${{ !cancelled() }}"))
                    })
                }),
                format!("required Rust authority step missing/conditional: {id}"),
            )?;
        }
    }
    if name == "release.yml" {
        require(
            doc["concurrency"]["cancel-in-progress"] == false,
            "release may not cancel",
        )?;
        require(
            doc["on"].get("workflow_dispatch").is_none(),
            "manual publication forbidden",
        )?;
        require(
            jobs["validate"]["uses"] == "./.github/workflows/rust-validation.yml",
            "release must rerun exact-source validation",
        )?;
        require(
            strings(&jobs["images"]["needs"]) == ["readiness".into()].into()
                && strings(&jobs["readiness"]["needs"]) == ["validate".into()].into(),
            "image authorization bypass",
        )?;
        let images: BTreeSet<_> = jobs["images"]["strategy"]["matrix"]["include"]
            .as_array()
            .ok_or("missing image matrix")?
            .iter()
            .filter_map(|v| v["name"].as_str())
            .collect();
        require(
            images == ["runtime"].into(),
            "only the explicit Rust runtime image identity may publish",
        )?;
        require(
            jobs["images"]["strategy"]["matrix"]["include"]
                .as_array()
                .is_some_and(|images| {
                    images.len() == 1
                        && images.iter().all(|image| {
                            matches!(
                                (image["name"].as_str(), image["dockerfile"].as_str()),
                                (
                                    Some("runtime"),
                                    Some("build/rust/runtime-candidate.Dockerfile")
                                )
                            )
                        })
                }),
            "runtime identity may not alias legacy or attacker Dockerfiles",
        )?;
        let steps = jobs["images"]["steps"]
            .as_array()
            .ok_or("release image steps absent")?;
        let guard = steps.iter().position(|step| step["run"] == "cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- image-sources");
        let login = steps.iter().position(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|action| action.starts_with("docker/login-action@"))
        });
        let build = steps
            .iter()
            .find(|step| step["id"] == "build")
            .ok_or("release image build absent")?;
        require(
            guard.zip(login).is_some_and(|(guard, login)| guard < login)
                && steps.iter().any(|step| step["id"] == "image-source"
                    && step["run"] == "cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- image-scan prepare")
                && build["with"]["platforms"] == "linux/amd64,linux/arm64"
                && build["with"]["provenance"] == "mode=max"
                && build["with"]["sbom"] == true
                && build["with"]["build-args"] == "HUMANYMOUS_SOURCE_REV=${{ github.sha }}\nHUMANYMOUS_SOURCE_MANIFEST=${{ steps.image-source.outputs.source-manifest-sha256 }}\n",
            "release image qualification/source/platform/provenance boundary changed",
        )?;
        for id in ["crates", "npm"] {
            require(
                jobs[id]["environment"] == id,
                "registry product environment binding mismatch",
            )?;
            require(
                strings(&jobs[id]["needs"]).contains("readiness"),
                "registry readiness bypass",
            )?;
        }
    }
    Ok(())
}
fn image_parity(docs: &[(String, Value)]) -> Result<()> {
    let full = &docs
        .iter()
        .find(|(name, _)| name == "rust-validation.yml")
        .ok_or("full image workflow absent")?
        .1["jobs"]["rust-images"]["steps"];
    let dedicated = &docs
        .iter()
        .find(|(name, _)| name == "image-validation.yml")
        .ok_or("dedicated image workflow absent")?
        .1["jobs"]["image"]["steps"];
    let suffix = full.as_array().and_then(|steps| steps.get(3..));
    require(
        suffix.is_some() && suffix == dedicated.as_array().and_then(|steps| steps.get(3..)),
        "dedicated image build/scan steps must match full validation",
    )
}
pub fn check(root: &Path) -> Result<()> {
    let docs = workflows(root)?;
    let inventory = crate::read_json(&root.join("scripts/ci/workflows.json"))?;
    catalog(&inventory, &docs)?;
    crate::archive_cache::policy::validate(
        &inventory["public_crate_archive_cache"],
        &docs
            .iter()
            .find(|(file, _)| file == "rust-validation.yml")
            .ok_or("Rust validation workflow missing")?
            .1,
    )?;
    crate::registry::validate_preparations(
        &crate::read_json(&root.join("scripts/ci/registry-prerequisites.json"))?,
        &inventory["planned_publishers"],
        &docs
            .iter()
            .find(|(file, _)| file == "release.yml")
            .ok_or("release workflow missing")?
            .1,
    )?;
    crate::registry::validate_candidates(
        &inventory["publisher_candidates"],
        &docs
            .iter()
            .find(|(file, _)| file == "release.yml")
            .ok_or("release workflow missing")?
            .1,
    )?;
    for (name, doc) in &docs {
        document(name, doc)?;
    }
    image_parity(&docs)?;
    // Only newly authored Rust/workflow sources are owned here; legacy oracles
    // remain distinct. Enforce actual bytes, not a post-hash newline rewrite.
    let paths = command::text(
        root,
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "scripts/ci/delivery",
            ".github/workflows",
        ],
    )?;
    source::lf_files(
        root,
        paths
            .lines()
            .filter(|p| p.ends_with(".rs") || p.ends_with(".yml") || p.ends_with(".toml"))
            .map(str::to_owned),
    )?;
    println!("Rust workflow authorization/LF policy passed");
    Ok(())
}
fn catalog(catalog: &Value, docs: &[(String, Value)]) -> Result<()> {
    require(
        catalog["schema"] == "humanymous.workflow-catalog/1",
        "workflow catalog schema mismatch",
    )?;
    let entries = catalog["workflows"]
        .as_array()
        .ok_or("workflow catalog entries absent")?;
    let mut files = BTreeSet::new();
    let mut names = BTreeSet::new();
    for entry in entries {
        let file = entry["file"].as_str().ok_or("workflow filename absent")?;
        let name = entry["name"]
            .as_str()
            .ok_or("workflow display name absent")?;
        require(
            files.insert(file) && names.insert(name),
            "duplicate workflow identity",
        )?;
        require(
            ["CI / ", "Docs / ", "Release / ", "Legacy / "]
                .iter()
                .any(|prefix| name.starts_with(prefix)),
            "workflow display namespace absent",
        )?;
        let doc = &docs
            .iter()
            .find(|(actual, _)| actual == file)
            .ok_or_else(|| format!("catalog workflow missing: {file}"))?
            .1;
        require(
            doc["name"] == name,
            format!("workflow display drift: {file}"),
        )?;
        let jobs = doc["jobs"].as_object().ok_or("workflow jobs absent")?;
        require(
            jobs.keys().cloned().collect::<BTreeSet<_>>() == strings(&entry["jobs"]),
            format!("workflow job identity drift: {file}"),
        )?;
        require(
            doc["concurrency"] == entry["concurrency"],
            format!("workflow concurrency drift: {file}"),
        )?;
        let mut environments = Map::new();
        for (job, definition) in jobs {
            if let Some(environment) = definition.get("environment") {
                let name = environment
                    .as_str()
                    .or_else(|| environment["name"].as_str())
                    .ok_or("job environment name absent")?;
                environments.insert(job.clone(), json!(name));
            }
        }
        require(
            Value::Object(environments)
                == entry
                    .get("environments")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            format!("workflow environment drift: {file}"),
        )?;
        let mut artifacts = BTreeSet::new();
        for step in jobs
            .values()
            .filter_map(|job| job["steps"].as_array())
            .flatten()
        {
            if step["uses"].as_str().is_some_and(|action| {
                action.starts_with("actions/upload-artifact@")
                    || action.starts_with("actions/upload-pages-artifact@")
            }) {
                let artifact = step["with"]["name"]
                    .as_str()
                    .ok_or("explicit artifact identity required")?;
                require(
                    artifacts.insert(artifact.to_owned()),
                    "duplicate artifact identity within workflow",
                )?;
            }
        }
        require(
            artifacts == strings(&entry["artifacts"]),
            format!("workflow artifact identity drift: {file}"),
        )?;
    }
    require(
        files == docs.iter().map(|(file, _)| file.as_str()).collect() && files.len() == docs.len(),
        "unknown or duplicate workflow outside catalog",
    )
}
pub fn pins(root: &Path) -> Result<()> {
    let mut actions = BTreeSet::new();
    for (_, doc) in workflows(root)? {
        for job in doc["jobs"].as_object().ok_or("missing jobs")?.values() {
            for step in job["steps"].as_array().into_iter().flatten() {
                if let Some(action) = step["uses"].as_str()
                    && !action.starts_with("./")
                {
                    require(pinned(action), "unpinned action")?;
                    actions.insert(action.to_owned());
                }
            }
        }
    }
    let mut reports = Vec::new();
    let total_actions = actions.len();
    for (index, action) in actions.into_iter().enumerate() {
        let (name, sha) = action.rsplit_once('@').ok_or("bad action")?;
        let parts: Vec<_> = name.split('/').collect();
        // These are public upstream resources. Do not attach the repository's
        // installation token: upstream organization IP rules can reject that
        // authenticated identity even though the public commit is readable.
        let body = ureq::get(&format!(
            "https://api.github.com/repos/{}/{}/commits/{sha}",
            parts[0], parts[1]
        ))
        .header("User-Agent", "humanymous-delivery")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("public action commit lookup {}/{total_actions} failed for {action}: {e}; no authenticated fallback or automatic retry", index + 1))?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
        verify_public_commit(&body, sha)?;
        let suffix = if parts.len() > 2 {
            format!("/{}/", parts[2..].join("/"))
        } else {
            "/".into()
        };
        let prefix = format!(
            "https://raw.githubusercontent.com/{}/{}/{sha}{suffix}",
            parts[0], parts[1]
        );
        let content = match ureq::get(&format!("{prefix}action.yml")).call() {
            Ok(mut response) => response
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())?,
            Err(ureq::Error::StatusCode(404)) => ureq::get(&format!("{prefix}action.yaml"))
                .call()
                .map_err(|e| e.to_string())?
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())?,
            Err(error) => return Err(error.to_string()),
        };
        require(
            yaml(&content)?.get("runs").is_some(),
            "action entrypoint missing",
        )?;
        reports.push(json!({"action":action,"commit":sha,"entrypoint":"verified"}));
    }
    println!("{}", json!(reports));
    Ok(())
}

fn verify_public_commit(body: &str, expected_sha: &str) -> Result<()> {
    let commit: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    require(
        commit["sha"] == expected_sha,
        "action commit identity mismatch",
    )
}
pub fn jobs(raw: &str) -> Result<()> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let jobs = value.as_object().ok_or("aggregate must be an object")?;
    require(
        !jobs.is_empty() && jobs.values().all(|v| v["result"] == "success"),
        "failed/skipped/cancelled validation never authorizes release",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_branch_upload_defaults_are_never_enabled_by_always() {
        for name in [
            "pages-build.yml",
            "rust-validation.yml",
            "go-validation.yml",
            "browser-product-validation.yml",
            "component-validation.yml",
            "mount-probe-validation.yml",
            "image-validation.yml",
            "apache-validation.yml",
            "python-validation.yml",
            "dotnet-validation.yml",
        ] {
            assert!(!upload_allowed(name, None));
            assert!(!upload_allowed(name, Some("always()")));
        }
        assert!(upload_allowed(
            "pages-build.yml",
            Some("github.ref == 'refs/heads/main'")
        ));
        assert!(!upload_allowed(
            "pages-build.yml",
            Some("github.ref == 'refs/heads/main' || true")
        ));
        assert!(upload_allowed(
            "collector-component.yml",
            Some("always() && vars.CI_COMPONENT_ARTIFACTS == 'true'")
        ));
    }
    #[test]
    fn image_caller_cannot_publish_or_weaken_inventory_accounting() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "image-validation.yml")
            .unwrap()
            .1;
        assert!(document("image-validation.yml", &original).is_ok());
        for mutation in 0..4 {
            let mut changed = original.clone();
            let steps = changed["jobs"]["image"]["steps"].as_array_mut().unwrap();
            match mutation {
                0 => {
                    steps
                        .iter_mut()
                        .find(|step| step["id"] == "runtime_image_build")
                        .unwrap()["with"]["push"] = json!(true)
                }
                1 => {
                    steps
                        .iter_mut()
                        .find(|step| step["id"] == "runtime_image_scan")
                        .unwrap()["with"]["list-all-pkgs"] = json!("false")
                }
                2 => steps.last_mut().unwrap()["if"] = json!("${{ success() }}"),
                _ => changed["jobs"]["image"]["needs"] = json!([]),
            }
            assert!(document("image-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn dedicated_image_steps_cannot_drift_from_full_validation() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut docs = workflows(&root).unwrap();
        assert!(image_parity(&docs).is_ok());
        let image = &mut docs
            .iter_mut()
            .find(|(file, _)| file == "image-validation.yml")
            .unwrap()
            .1;
        image["jobs"]["image"]["steps"][5]["with"]["build-args"] =
            json!("HUMANYMOUS_SOURCE_REV=wrong");
        assert!(image_parity(&docs).is_err());
    }
    #[test]
    fn mount_diagnostic_cannot_bypass_preflight_or_mask_failure() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "mount-probe-validation.yml")
            .unwrap()
            .1;
        assert!(document("mount-probe-validation.yml", &original).is_ok());
        for mutation in 0..5 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["jobs"]["probe"]["needs"] = json!([]),
                1 => {
                    changed["jobs"]["probe"]["steps"][2]["run"] =
                        json!("cargo run --locked -p humanymous-delivery -- mount-probe || true")
                }
                2 => changed["on"]["push"]["branches"] = json!(["codex/go-validation"]),
                3 => changed["jobs"]["probe"]["timeout-minutes"] = Value::Null,
                _ => changed["permissions"]["id-token"] = json!("write"),
            }
            assert!(document("mount-probe-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn native_caller_rejects_open_inputs_and_success_masking() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "component-validation.yml")
            .unwrap()
            .1;
        assert!(document("component-validation.yml", &original).is_ok());
        for mutation in 0..10 {
            let mut changed = original.clone();
            match mutation {
                0 => {
                    changed["on"]["workflow_dispatch"]["inputs"]["component"]["type"] =
                        json!("string")
                }
                1 => {
                    changed["on"]["workflow_dispatch"]["inputs"]["component"]["options"] =
                        json!(["capture", "header", "runtime"])
                }
                2 => {
                    changed["jobs"]["component"]["steps"][2]["run"] = json!(
                        "cargo run --locked -p humanymous-delivery -- component ${{ inputs.component }}"
                    )
                }
                3 => changed["jobs"]["component"]["needs"] = json!([]),
                4 => changed["jobs"]["component"]["continue-on-error"] = json!(true),
                5 => {
                    changed["jobs"]["component"]["env"]["HUMANYMOUS_NATIVE_COMPONENT"] =
                        json!("${{ inputs.component || 'capture' }}")
                }
                6 => changed["jobs"]["header"]["container"]["image"] = json!("ubuntu:latest"),
                7 => changed["jobs"]["header"]["needs"] = json!([]),
                8 => changed["jobs"]["header"]["container"]["options"] = json!("--privileged"),
                _ => {
                    changed["jobs"]["service"]["steps"][2]["run"] = json!(
                        "cargo run --locked -p humanymous-delivery -- service-component || true"
                    )
                }
            }
            assert!(document("component-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn browser_diagnostic_cannot_promote_or_replace_the_full_gate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "browser-product-validation.yml")
            .unwrap()
            .1;
        assert!(document("browser-product-validation.yml", &original).is_ok());
        for mutation in 0..7 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["jobs"]["product"]["needs"] = json!([]),
                1 => changed["jobs"]["product"]["if"] = json!("${{ always() }}"),
                2 => {
                    changed["jobs"]["product"]["steps"][2]["run"] = json!(
                        "cargo run --locked -p humanymous-delivery -- pilot browser-product || true"
                    )
                }
                3 => changed["jobs"]["product"]["steps"][2]["continue-on-error"] = json!(true),
                4 => changed["jobs"]["product"]["continue-on-error"] = json!(true),
                5 => changed["jobs"]["product"]["steps"][0]["with"]["ref"] = json!("main"),
                _ => changed["jobs"]["product"]["timeout-minutes"] = Value::Null,
            }
            assert!(document("browser-product-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn wargame_caller_cannot_substitute_profile_or_bypass_current_owners() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "wargame-validation.yml")
            .unwrap()
            .1;
        assert!(document("wargame-validation.yml", &original).is_ok());
        for mutation in 0..5 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["jobs"]["product"]["needs"] = json!([]),
                1 => changed["jobs"]["product"]["steps"][2]["run"] = json!("cargo run --locked -p humanymous-delivery -- pilot browser-product"),
                2 => changed["jobs"]["product"]["continue-on-error"] = json!(true),
                3 => changed["jobs"]["preflight"]["steps"].as_array_mut().unwrap().retain(|step|
                    step["run"] != "cargo test --locked -p humanymous-attacker -p humanymous-defender --lib -j1"),
                _ => changed["permissions"]["id-token"] = json!("write"),
            }
            assert!(document("wargame-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn go_product_cannot_bypass_current_source_preflight() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "go-validation.yml")
            .unwrap()
            .1;
        assert!(document("go-validation.yml", &original).is_ok());
        for mutation in 0..2 {
            let mut changed = original.clone();
            if mutation == 0 {
                changed["jobs"]["product"]["needs"] = json!([]);
            } else {
                changed["jobs"]["product"]["if"] = json!("${{ always() }}");
            }
            assert!(document("go-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn facade_consumer_gates_cannot_omit_or_precede_closed_admission() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        for name in ["go-validation.yml", "browser-product-validation.yml"] {
            let original = &docs.iter().find(|(file, _)| file == name).unwrap().1;
            assert!(document(name, original).is_ok());
            let steps = original["jobs"]["preflight"]["steps"].as_array().unwrap();
            for (index, step) in steps.iter().enumerate() {
                if step["run"].as_str().is_some_and(|run| {
                    run.contains("--test deployment_contract")
                        || run.contains("-p humanymous ")
                        || run.contains("-- facade-profile ")
                }) {
                    let mut changed = original.clone();
                    changed["jobs"]["preflight"]["steps"]
                        .as_array_mut()
                        .unwrap()
                        .remove(index);
                    assert!(document(name, &changed).is_err());
                }
            }
            for (index, step) in steps.iter().enumerate() {
                if step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("-p humanymous "))
                {
                    for condition in [
                        Value::Null,
                        json!("${{ !cancelled() && steps.toolchain.outcome == 'success' }}"),
                        json!(
                            "${{ !cancelled() && steps.toolchain.outcome == 'success' && steps.unrelated.outcome == 'success' }}"
                        ),
                    ] {
                        let mut changed = original.clone();
                        changed["jobs"]["preflight"]["steps"][index]["if"] = condition.clone();
                        assert!(document(name, &changed).is_err());
                        let mut changed = original.clone();
                        let mut duplicate = step.clone();
                        duplicate["if"] = condition;
                        changed["jobs"]["preflight"]["steps"]
                            .as_array_mut()
                            .unwrap()
                            .push(duplicate);
                        assert!(document(name, &changed).is_err());
                    }
                }
                if step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("-- facade-profile "))
                {
                    for id in [Value::Null, json!("unrelated")] {
                        let mut changed = original.clone();
                        changed["jobs"]["preflight"]["steps"][index]["id"] = id.clone();
                        assert!(document(name, &changed).is_err());
                        let mut changed = original.clone();
                        let mut duplicate = step.clone();
                        duplicate["id"] = id;
                        changed["jobs"]["preflight"]["steps"]
                            .as_array_mut()
                            .unwrap()
                            .push(duplicate);
                        assert!(document(name, &changed).is_err());
                    }
                    let mut changed = original.clone();
                    changed["jobs"]["preflight"]["steps"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({
                            "id":step["id"],"run":"cargo --version"
                        }));
                    assert!(document(name, &changed).is_err());
                    let mut changed = original.clone();
                    changed["jobs"]["preflight"]["steps"]
                        .as_array_mut()
                        .unwrap()
                        .push(step.clone());
                    assert!(document(name, &changed).is_err());
                }
            }
            let admission = steps.iter().position(|step| step["run"] ==
                "cargo run --locked -p humanymous-delivery -- facade-profile rust-library adapter-go").unwrap();
            let consumer = steps.iter().position(|step| step["run"] ==
                "cargo test --locked -p humanymous --test go_consumer --no-default-features --features adapter-go -j1").unwrap();
            let mut changed = original.clone();
            changed["jobs"]["preflight"]["steps"]
                .as_array_mut()
                .unwrap()
                .swap(admission, consumer);
            assert!(document(name, &changed).is_err());
            let mut changed = original.clone();
            changed["jobs"]["preflight"]["steps"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "run":"cargo check --locked -p humanymous --all-features -j1",
                    "if":"${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
                }));
            assert!(document(name, &changed).is_err());
        }
    }
    #[test]
    fn pages_only_caller_and_base_failure_accounting_cannot_widen() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        let original = &docs
            .iter()
            .find(|(file, _)| file == "pages-validation.yml")
            .unwrap()
            .1;
        assert!(document("pages-validation.yml", original).is_ok());
        let mut changed = original.clone();
        changed["jobs"]["validate"]["uses"] = json!("./.github/workflows/pages.yml");
        assert!(document("pages-validation.yml", &changed).is_err());
        let original = &docs
            .iter()
            .find(|(file, _)| file == "pages-build.yml")
            .unwrap()
            .1;
        let mut changed = original.clone();
        let subpath = changed["jobs"]["build"]["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["id"] == "subpath")
            .unwrap();
        subpath["if"] = json!("${{ success() }}");
        assert!(document("pages-build.yml", &changed).is_err());
    }
    #[test]
    fn image_inventory_cannot_be_empty_or_failure_accounting_skipped() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "rust-validation.yml")
            .unwrap()
            .1;
        assert!(document("rust-validation.yml", &original).is_ok());
        for mutation in 0..3 {
            let mut changed = original.clone();
            let steps = changed["jobs"]["rust-images"]["steps"]
                .as_array_mut()
                .unwrap();
            if mutation == 2 {
                let verify = steps
                    .iter_mut()
                    .find(|step| {
                        step["run"]
                            == "cargo run --locked -p humanymous-delivery -- image-scan verify"
                    })
                    .unwrap();
                verify["if"] = json!("${{ success() }}");
            } else {
                let scan = steps
                    .iter_mut()
                    .find(|step| step["id"] == "runtime_image_scan")
                    .unwrap();
                scan["with"][if mutation == 0 {
                    "list-all-pkgs"
                } else {
                    "format"
                }] = json!(if mutation == 0 { "false" } else { "table" });
            }
            assert!(document("rust-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn workflow_failures_require_literal_fail_closed_settings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        let image = &docs
            .iter()
            .find(|(file, _)| file == "image-validation.yml")
            .unwrap()
            .1;
        assert!(document("image-validation.yml", image).is_ok());
        let mut literal_false = image.clone();
        literal_false["jobs"]["preflight"]["continue-on-error"] = json!(false);
        literal_false["jobs"]["preflight"]["steps"][2]["continue-on-error"] = json!(false);
        assert!(document("image-validation.yml", &literal_false).is_ok());
        for setting in [
            json!(true),
            json!("${{ true }}"),
            json!("${{ false }}"),
            json!("true"),
            json!("false"),
            json!(0),
            json!(1),
            Value::Null,
            json!([]),
            json!({}),
        ] {
            let mut job = image.clone();
            job["jobs"]["preflight"]["continue-on-error"] = setting.clone();
            assert!(document("image-validation.yml", &job).is_err());
            let mut step = image.clone();
            step["jobs"]["preflight"]["steps"][2]["continue-on-error"] = setting.clone();
            assert!(document("image-validation.yml", &step).is_err());
        }
        let cached = &docs
            .iter()
            .find(|(file, _)| file == "rust-validation.yml")
            .unwrap()
            .1;
        assert!(document("rust-validation.yml", cached).is_ok());
        let steps = cached["jobs"]["policy"]["steps"].as_array().unwrap();
        let exceptions: Vec<_> = steps
            .iter()
            .enumerate()
            .filter(|(_, step)| {
                crate::archive_cache::policy::approved_step("rust-validation.yml", "policy", step)
            })
            .map(|(index, _)| index)
            .collect();
        assert_eq!(exceptions.len(), 2);
        for index in exceptions {
            for setting in [json!("${{ true }}"), json!(1), Value::Null] {
                let mut changed = cached.clone();
                changed["jobs"]["policy"]["steps"][index]["continue-on-error"] = setting;
                assert!(document("rust-validation.yml", &changed).is_err());
            }
        }
    }
    #[test]
    fn optional_public_cache_cannot_widen_paths_or_hide_validation_failures() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "rust-validation.yml")
            .unwrap()
            .1;
        assert!(document("rust-validation.yml", &original).is_ok());
        for mutation in 0..3 {
            let mut changed = original.clone();
            let steps = changed["jobs"]["policy"]["steps"].as_array_mut().unwrap();
            let restore = steps
                .iter_mut()
                .find(|step| step["id"] == "public_crates_restore")
                .unwrap();
            match mutation {
                0 => restore["with"]["path"] = json!("~/.cargo"),
                1 => restore["with"]["restore-keys"] = json!("public-crates"),
                _ => restore["if"] = json!("${{ always() }}"),
            }
            assert!(document("rust-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn workflow_catalog_rejects_unknown_duplicate_and_drifted_identities() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = crate::read_json(&root.join("scripts/ci/workflows.json")).unwrap();
        let docs = workflows(&root).unwrap();
        assert!(catalog(&original, &docs).is_ok());
        let mut unknown = docs.clone();
        unknown.push(("unknown.yml".into(), json!({})));
        assert!(catalog(&original, &unknown).is_err());
        let mut duplicate = original.clone();
        duplicate["workflows"][1]["name"] = duplicate["workflows"][0]["name"].clone();
        assert!(catalog(&duplicate, &docs).is_err());
        for mutation in 0..3 {
            let mut changed = docs.clone();
            let doc = &mut changed
                .iter_mut()
                .find(|(file, _)| file == "rust-validation.yml")
                .unwrap()
                .1;
            match mutation {
                0 => doc["name"] = json!("CI / Unexpected"),
                1 => {
                    doc["jobs"].as_object_mut().unwrap().remove("docker");
                }
                _ => {
                    doc["concurrency"] = json!({"group":"all-workflows","cancel-in-progress":true})
                }
            }
            assert!(catalog(&original, &changed).is_err());
        }
    }
    #[test]
    fn naming_catalog_does_not_authorize_manual_release() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut release = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "release.yml")
            .unwrap()
            .1;
        assert!(document("release.yml", &release).is_ok());
        release["on"]["workflow_dispatch"] = Value::Null;
        assert!(document("release.yml", &release).is_err());
    }
    #[test]
    fn rust_release_cannot_restore_go_alias_or_skip_candidate_guard() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        let release = &docs
            .iter()
            .find(|(name, _)| name == "release.yml")
            .unwrap()
            .1;
        assert!(document("release.yml", release).is_ok());
        for mutation in 0..3 {
            let mut changed = release.clone();
            match mutation {
                0 => {
                    changed["jobs"]["images"]["strategy"]["matrix"]["include"] =
                        json!([{"name":"core","dockerfile":"build/core.Dockerfile"}])
                }
                1 => changed["jobs"]["images"]["steps"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|step| {
                        step["name"] != "Recheck source and candidate shipping qualification"
                    }),
                _ => {
                    changed["jobs"]["images"]["steps"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|step| step["id"] == "build")
                        .unwrap()["with"]["platforms"] = json!("linux/amd64")
                }
            }
            assert!(document("release.yml", &changed).is_err());
        }
    }
    #[test]
    fn rust_codeql_cannot_restore_go_or_autobuild() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        let original = &docs
            .iter()
            .find(|(name, _)| name == "codeql.yml")
            .unwrap()
            .1;
        assert!(document("codeql.yml", original).is_ok());
        for mutation in 0..2 {
            let mut changed = original.clone();
            changed["jobs"]["analyze"]["steps"][1]["with"][if mutation == 0 {
                "languages"
            } else {
                "build-mode"
            }] = json!(if mutation == 0 { "go" } else { "autobuild" });
            assert!(document("codeql.yml", &changed).is_err());
        }
    }
    #[test]
    fn historical_oracle_cannot_follow_current_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let docs = workflows(&root).unwrap();
        let mut oracle = docs
            .iter()
            .find(|(name, _)| name == "frozen-go-oracle.yml")
            .unwrap()
            .1
            .clone();
        assert!(document("frozen-go-oracle.yml", &oracle).is_ok());
        oracle["jobs"]["go"]["steps"][0]["with"]["ref"] = json!("main");
        assert!(document("frozen-go-oracle.yml", &oracle).is_err());
    }
    #[test]
    fn hosted_venue_cap_cannot_bypass_specfold_or_act_refuse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root)
            .unwrap()
            .into_iter()
            .find(|(file, _)| file == "h2-hosted-venue-validation.yml")
            .unwrap()
            .1;
        assert!(document("h2-hosted-venue-validation.yml", &original).is_ok());
        for mutation in 0..4 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["on"]["push"]["branches"] = json!(["main"]),
                1 => changed["jobs"]["cap"]["runs-on"] = json!("ubuntu-24.04"),
                2 => {
                    let steps = changed["jobs"]["cap"]["steps"].as_array_mut().unwrap();
                    steps.retain(|step| {
                        !step["run"].as_str().is_some_and(|run| run.contains("ACT"))
                    });
                }
                _ => {
                    let steps = changed["jobs"]["cap"]["steps"].as_array_mut().unwrap();
                    steps.retain(|step| {
                        !step["run"]
                            .as_str()
                            .is_some_and(|run| run.contains("--w2-h2-hosted-venue"))
                    });
                }
            }
            assert!(document("h2-hosted-venue-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn registry_job_cannot_return_to_shared_release_environment() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let original = workflows(&root).unwrap();
        let manifest = crate::read_json(&root.join("scripts/ci/workflows.json")).unwrap();
        let mut changed = original.clone();
        let release = &mut changed
            .iter_mut()
            .find(|(file, _)| file == "release.yml")
            .unwrap()
            .1;
        release["jobs"]["crates"]["environment"] = json!("release");
        assert!(document("release.yml", release).is_err());
        assert!(catalog(&manifest, &changed).is_err());
    }
    #[test]
    fn public_commit_json_requires_exact_identity() {
        let sha = "ed142fd0673e97e23eac54620cfb913e5ce36c25";
        assert!(verify_public_commit(&json!({"sha":sha}).to_string(), sha).is_ok());
        for body in [
            "not json".to_owned(),
            json!({"sha":"moving-tag"}).to_string(),
            json!({"message":"API rate limit exceeded"}).to_string(),
            json!({"sha":null}).to_string(),
        ] {
            assert!(verify_public_commit(&body, sha).is_err());
        }
    }
    #[test]
    fn yaml_boolean_keys_follow_yaml12() {
        assert!(
            yaml("on:\n  workflow_call:\npermissions:\n  contents: read\n")
                .unwrap()
                .get("on")
                .is_some()
        );
    }
    #[test]
    fn moving_action_refs_fail() {
        for action in [
            "actions/checkout@v4",
            "actions/checkout@main",
            "actions/checkout@123",
        ] {
            assert!(!pinned(action));
        }
        assert!(pinned(
            "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"
        ));
    }
    #[test]
    fn every_non_success_aggregate_fails() {
        for result in ["skipped", "cancelled", "failure", "neutral", ""] {
            assert!(jobs(&json!({"docker":{"result":result}}).to_string()).is_err());
        }
        assert!(jobs("{}").is_err());
        assert!(jobs(r#"{"docker":{"result":"success"}}"#).is_ok());
    }
    #[test]
    fn broad_global_permissions_fail() {
        assert!(
            document(
                "ci.yml",
                &json!({"permissions":{"contents":"write"},"jobs":{}})
            )
            .is_err()
        );
    }
    #[test]
    fn reusable_validation_caller_cannot_deny_or_expand_cache_read_permission() {
        let valid = json!({"uses":"./.github/workflows/rust-validation.yml","permissions":{"contents":"read","actions":"read"}});
        assert!(caller_permissions(&valid).is_ok());
        for permissions in [
            json!({"contents":"read"}),
            json!({"contents":"read","actions":"write"}),
            json!({"contents":"read","actions":"read","id-token":"write"}),
        ] {
            let mut invalid = valid.clone();
            invalid["permissions"] = permissions;
            assert!(caller_permissions(&invalid).is_err());
        }
    }
    #[test]
    fn required_docker_aggregate_and_execution_cannot_be_removed_or_skipped() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.github/workflows/rust-validation.yml");
        let original = yaml(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(document("rust-validation.yml", &original).is_ok());
        for mutation in 0..7 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["jobs"]["ready"]["needs"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|value| value != "docker"),
                1 => changed["jobs"]["docker"]["if"] = json!("false"),
                2 => changed["jobs"]["docker"]["steps"] = json!([]),
                3 => changed["jobs"]["docker"]["steps"][0]["continue-on-error"] = json!(true),
                4 => changed["jobs"]["native"]["if"] = json!("${{ !cancelled() }}"),
                5 => changed["jobs"]["native"]["needs"] = json!(["browser"]),
                _ => changed["env"]["DOCKER_BUILD_RECORD_UPLOAD"] = json!("true"),
            }
            assert!(document("rust-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn registry_tokens_at_job_scope_cannot_bypass_step_policy() {
        let workflow = json!({"permissions":{"contents":"read"},"on":{"push":null},"jobs":{"x":{"env":{"NPM_TOKEN":"${{ secrets.NPM_TOKEN }}"}}}});
        assert!(document("ci.yml", &workflow).is_err());
    }
}
