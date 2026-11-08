//! Docker control plane and source-bound case accounting, with owned cleanup.
use crate::{
    docker::{Docker, Result},
    snapshot,
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const OWNER: &str = "rust-product-lab";
fn safe_diagnostics(cases: &[Value]) -> Value {
    let contract: Value = serde_json::from_str(include_str!("../safe-diagnostics.json"))
        .expect("owned safe diagnostic contract");
    let failed: Vec<_> = cases
        .iter()
        .filter(|case| case["status"] == "failed")
        .collect();
    let failures: Vec<_> = failed
        .iter()
        .take(512)
        .map(|case| {
            let id = case["id"]
                .as_str()
                .filter(|id| {
                    !id.is_empty()
                        && id.len() <= 96
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                })
                .unwrap_or("unclassified_case");
            let code = case["error_code"]
                .as_str()
                .filter(|code| {
                    contract["error_codes"].as_array().is_some_and(|allowed| {
                        allowed.iter().any(|value| value.as_str() == Some(*code))
                    })
                })
                .unwrap_or("assertion_failed");
            json!({"case_id":id,"status":"failed","error_code":code})
        })
        .collect();
    json!({"schema":"humanymous.lab-safe-diagnostics/1","failures":failures,"failure_count":failed.len(),"partial":failed.len()>512})
}
mod authority;
mod browser;
mod browser_actor;
mod browser_failure;
pub(crate) mod g4_cef_at;
mod go;
mod go_fixture;
mod go_public;
pub(crate) mod h2_product_binding;
mod mount_probe;
mod public_receipts;
mod requirement;
mod startup_failure;
mod witness;
struct Lab {
    docker: Docker,
    project: String,
    environment: Vec<(String, String)>,
    evidence: PathBuf,
    collector: bool,
    topology_only: bool,
    browser_product: bool,
    source_binding: Option<snapshot::source_pin::SourceBinding>,
    go_product: bool,
    mount_probe: bool,
    authority_node_ipc: bool,
    w2_h2_product_binding: bool,
    w2_h2_hosted_venue: bool,
    g4_cef_at: bool,
    go_component_verified: bool,
    stage: &'static str,
    cases: Vec<Value>,
    components: Value,
    public_receipts: Value,
}
impl Lab {
    fn config_file(&self, relative: &str) -> Result<String> {
        let context = &self
            .environment
            .iter()
            .find(|(name, _)| name == "HM_LAB_BUILD_CONTEXT")
            .ok_or("immutable Compose source context required")?
            .1;
        Ok(std::path::Path::new(context)
            .join(relative)
            .to_string_lossy()
            .into_owned())
    }
    fn compose_args(&self, args: &[&str]) -> Result<Vec<String>> {
        // Authority Cap uses its own compose only (not the full product topology).
        if self.authority_node_ipc {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.authority.yaml")?,
                "--profile".into(),
                "authority".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.w2_h2_product_binding {
            // Cap topology compose is Root-allocated; Cap CLI refuses before Docker when absent.
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.h2-product-binding.yaml")?,
                "--profile".into(),
                "h2-product-binding".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.w2_h2_hosted_venue {
            // Hosted Cap topology; Cap CLI refuse-closed unless actual_hosted_linux.
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.h2-hosted-venue.yaml")?,
                "--profile".into(),
                "h2-hosted-venue".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.g4_cef_at {
            // G4 Cap is host CEF/AT refuse-closed; no Docker compose topology.
            return Err(
                "g4-cef-at Cap has no Docker compose topology; Cap CLI refuse-closed to host CEF/AT prerequisites only; never invent pass_product_clean_room"
                    .into(),
            );
        }
        let mut command = vec![
            "compose".into(),
            "-p".into(),
            self.project.clone(),
            "-f".into(),
            self.config_file("deployments/lab/compose.rust.yaml")?,
            "--profile".into(),
            "controls".into(),
        ];
        if !self.topology_only {
            command.extend([
                "-f".into(),
                self.config_file("deployments/lab/compose.rust.witness.yaml")?,
                "--profile".into(),
                "witness".into(),
            ]);
        }
        if self.collector {
            command.extend([
                "-f".into(),
                self.config_file("deployments/lab/compose.rust.collector.yaml")?,
                "--profile".into(),
                "collector".into(),
            ]);
        }
        if self.browser_product {
            command.extend([
                "-f".into(),
                self.config_file("deployments/lab/compose.rust.browser.yaml")?,
                "--profile".into(),
                "browser-product".into(),
            ]);
        }
        if self.go_product {
            command.extend([
                "-f".into(),
                self.config_file("deployments/lab/compose.rust.go.yaml")?,
                "--profile".into(),
                "go-product".into(),
            ]);
        }
        if self.mount_probe {
            command.extend([
                "-f".into(),
                self.config_file("deployments/lab/compose.rust.mount.yaml")?,
                "--profile".into(),
                "mount-probe".into(),
            ]);
        }
        command.extend(args.iter().map(|s| (*s).to_owned()));
        Ok(command)
    }
    fn compose(&mut self, args: &[&str], required: bool) -> Result<crate::docker::Output> {
        let command = self.compose_args(args)?;
        let result = if args.first() == Some(&"build") {
            self.docker
                .invoke_image_build(&command, &self.environment, required)
        } else {
            self.docker.invoke(&command, &self.environment, required)
        };
        if result.is_err()
            && args.first() == Some(&"build")
            && let Some(stage) = self
                .docker
                .last_failure
                .and_then(|failure| failure.go_build_stage(args.get(1).copied().unwrap_or("")))
        {
            self.stage = stage;
        }
        result
    }
    fn case(&mut self, id: &str, passed: bool, detail: Value) -> Result<()> {
        self.cases
            .push(json!({"id":id,"status":if passed{"passed"}else{"failed"},"detail":detail}));
        if passed {
            Ok(())
        } else {
            Err(format!("assertion failed: {id}").into())
        }
    }
    fn inspect(&mut self, service: &str) -> Result<Value> {
        let id = self
            .compose(&["ps", "--all", "-q", service], true)?
            .stdout
            .trim()
            .to_owned();
        let state: Value = serde_json::from_str(&self.docker.call(&["inspect", &id])?)?;
        let container = state[0].clone();
        self.case(
            &format!("owned-container-{service}"),
            container["Config"]["Labels"]["com.docker.compose.project"] == self.project,
            json!(id),
        )?;
        snapshot::write(
            &self.evidence,
            &format!("{service}-container.json"),
            &container,
        )?;
        let host = &container["HostConfig"];
        let mounts_allowed = if service.starts_with("witness-")
            || matches!(
                service,
                "browser-gate" | "go-gate" | "mount-writer-volume" | "mount-witness-volume"
            ) {
            let destinations: &[&str] = if matches!(
                service,
                "witness-writer" | "browser-gate" | "go-gate" | "mount-writer-volume"
            ) {
                &["/writer/keys", "/writer/wal"]
            } else if service == "witness-key-denied" {
                &["/witness/keys"]
            } else {
                &["/witness/keys", "/witness/head"]
            };
            container["Mounts"].as_array().is_some_and(|mounts| {
                mounts.len() == destinations.len()
                    && mounts.iter().all(|m| {
                        m["Type"] == "volume"
                            && destinations.contains(&m["Destination"].as_str().unwrap_or(""))
                    })
            })
        } else {
            container["Mounts"] == json!([])
        };
        let expected_user = if matches!(
            service,
            "witness-transaction" | "mount-witness-image" | "mount-witness-volume"
        ) {
            "65533:65533"
        } else {
            "65532:65532"
        };
        self.case(
            &format!("hardened-{service}"),
            container["Config"]["User"] == expected_user
                && host["ReadonlyRootfs"] == true
                && host["Privileged"] == false
                && host["PidMode"].as_str() == Some("")
                && host["IpcMode"].as_str() == Some("private")
                && host["CapDrop"] == json!(["ALL"])
                && host["SecurityOpt"]
                    .as_array()
                    .is_some_and(|v| v.contains(&json!("no-new-privileges:true")))
                && host["PortBindings"]
                    .as_object()
                    .is_some_and(|p| p.is_empty())
                && mounts_allowed,
            json!("actual nonroot/read-only/cap-drop/no ports/no host mounts"),
        )?;
        Ok(container)
    }
    fn execute(&mut self) -> Result<()> {
        if self.mount_probe {
            return mount_probe::run(self);
        }
        if self.authority_node_ipc {
            return authority::run(self);
        }
        if self.w2_h2_product_binding {
            return h2_product_binding::run(self);
        }
        if self.w2_h2_hosted_venue {
            return h2_product_binding::run_hosted_venue(self);
        }
        if self.g4_cef_at {
            return g4_cef_at::run(self);
        }
        if self.go_product {
            return go::run(self);
        }
        if self.browser_product {
            return browser::run(self);
        }
        self.stage = "compose_config";
        let config = self.compose(&["config", "--format", "json"], true)?.stdout;
        let parsed: Value = serde_json::from_str(&config)?;
        snapshot::write(&self.evidence, "resolved-compose.json", &parsed)?;
        self.case(
            "no-published-ports",
            parsed["services"]
                .as_object()
                .ok_or("services absent")?
                .values()
                .all(|s| s.get("ports").is_none()),
            json!(snapshot::digest(config.as_bytes())),
        )?;
        eprintln!("lab: building immutable Rust image");
        self.stage = "controls_build";
        self.compose(&["build", "origin"], true)?;
        self.compose(
            &[
                "up",
                "-d",
                "--wait",
                "--wait-timeout",
                "45",
                "origin",
                "outside-canary",
            ],
            true,
        )?;
        self.compose(&["create", "attacker"], true)?;
        let origin = self.inspect("origin")?;
        let outside = self.inspect("outside-canary")?;
        let attacker = self.inspect("attacker")?;
        self.compose(&["run", "--rm", "--no-deps", "backend-control"], true)?;
        self.case(
            "origin-positive-control",
            true,
            json!("owned backend client reached same live fixture"),
        )?;
        let filter = format!("label=com.docker.compose.project={}", self.project);
        let network_ids = self
            .docker
            .call(&["network", "ls", "-q", "--filter", &filter])?;
        let mut networks = Vec::new();
        for id in network_ids.lines() {
            let states: Value =
                serde_json::from_str(&self.docker.call(&["network", "inspect", id])?)?;
            networks.push(states[0].clone());
        }
        self.case(
            "internal-networks",
            networks.len() == 3
                && networks
                    .iter()
                    .all(|n| n["Internal"] == true && n["EnableIPv6"] == false),
            json!("three internal IPv4 bridges; IPv6 unqualified"),
        )?;
        snapshot::write(&self.evidence, "networks.json", &json!(networks))?;
        let ip = |c: &Value| -> Result<String> {
            Ok(c["NetworkSettings"]["Networks"]
                .as_object()
                .ok_or("networks missing")?
                .values()
                .next()
                .ok_or("network missing")?["IPAddress"]
                .as_str()
                .ok_or("address missing")?
                .to_owned())
        };
        let ingress = networks
            .iter()
            .find(|n| n["Labels"]["com.docker.compose.network"] == "ingress")
            .ok_or("ingress missing")?;
        let gateway = ingress["IPAM"]["Config"][0]["Gateway"]
            .as_str()
            .ok_or("gateway missing")?
            .to_owned();
        self.stage = "topology_controls";
        let result = self.compose(
            &[
                "run",
                "--rm",
                "--no-deps",
                "attacker",
                "fixture",
                "probe",
                &ip(&origin)?,
                &ip(&outside)?,
                &gateway,
            ],
            false,
        )?;
        let probe: Value = serde_json::from_str(result.stdout.trim())?;
        snapshot::write(&self.evidence, "topology-report.json", &probe)?;
        self.cases.extend(
            probe["cases"]
                .as_array()
                .ok_or("probe cases missing")?
                .iter()
                .cloned(),
        );
        self.case(
            "topology-probe-exit",
            result.success,
            json!("only inspected owned addresses; no public probe"),
        )?;
        self.compose(&["stop", "-t", "1", "origin"], true)?;
        let lost = self.compose(&["run", "--rm", "--no-deps", "backend-control"], false)?;
        self.case(
            "fixture-dependency-loss",
            !lost.success,
            json!("positive control fails after origin stop"),
        )?;
        if !self.topology_only {
            self.stage = "runtime_build";
            self.compose(&["build", "runtime-pilot"], true)?;
            self.compose(&["create", "runtime-pilot"], true)?;
            let runtime = self.inspect("runtime-pilot")?;
            self.case(
                "runtime-network-none",
                runtime["HostConfig"]["NetworkMode"] == "none",
                json!("runtime/client/origin share owned loopback; not L5 isolation"),
            )?;
            eprintln!("lab: running actual sealed-runtime process pilot");
            self.stage = "runtime_process";
            let result = self.docker.invoke(
                &[
                    "start".into(),
                    "-a".into(),
                    runtime["Id"].as_str().ok_or("runtime ID absent")?.into(),
                ],
                &[],
                false,
            )?;
            let report: Value = serde_json::from_str(result.stdout.trim())?;
            let stopped: Value = serde_json::from_str(&self.docker.call(&[
                "inspect",
                runtime["Id"].as_str().ok_or("runtime ID absent")?,
            ])?)?;
            self.components["runtime"] = report.clone();
            snapshot::write(&self.evidence, "runtime-report.json", &report)?;
            self.case(
            "actual-runtime-process",
            result.success
                && stopped[0]["State"]["ExitCode"] == 0
                && report["status"] == "passed"
                && report["origin_effects"] == 2
                && report["audit_records"].as_u64().is_some_and(|n| n >= 4),
            json!(
                "actual Rust binary + verified WAL + counted owned origin; synthetic client report"
            ),
        )?;
            let image: Value = serde_json::from_str(&self.docker.call(&[
                "image",
                "inspect",
                runtime["Image"].as_str().ok_or("image missing")?,
            ])?)?;
            snapshot::write(&self.evidence, "runtime-image.json", &image[0])?;
            self.stage = "wargame_process";
            self.compose(&["create", "wargame-pilot"], true)?;
            let wargame = self.inspect("wargame-pilot")?;
            self.case(
                "wargame-network-none",
                wargame["HostConfig"]["NetworkMode"] == "none",
                json!("supervisor-owned child only; no remote target arguments"),
            )?;
            let result = self.docker.invoke(
                &[
                    "start".into(),
                    "-a".into(),
                    wargame["Id"].as_str().ok_or("wargame ID absent")?.into(),
                ],
                &[],
                false,
            )?;
            std::fs::write(
                self.evidence.join("wargame-report.raw.json"),
                result.stdout.as_bytes(),
            )?;
            let report = humanymous_platform::testing::json::decode_strict_json(
                result.stdout.as_bytes(),
                4_194_304,
            )?;
            snapshot::write(&self.evidence, "wargame-report.json", &report)?;
            let receipts: Vec<_> = result
                .stderr
                .lines()
                .filter_map(|line| line.strip_prefix("HM-WARGAME-VERIFICATION "))
                .collect();
            if receipts.len() != 1 {
                return Err("exact original-byte wargame verification receipt required".into());
            }
            let receipt = humanymous_platform::testing::json::decode_strict_json(
                receipts[0].as_bytes(),
                65536,
            )?;
            snapshot::write(&self.evidence, "wargame-byte-verification.json", &receipt)?;
            self.case("strict-original-byte-wargame-verification",receipt["schema"]=="humanymous.lab-wargame-byte-receipt/1"&&receipt["raw_sha256"]==snapshot::digest(result.stdout.as_bytes())&&receipt["verification"]["schema"]=="humanymous.wargame-verification.v1"&&receipt["verification"]["mode"]=="owned_process_network"&&receipt["verification"]["scoped_passed"]==report["scoped_passed"]&&receipt["verification"]["internally_consistent"]==true&&receipt["verification"]["full_product_qualified"]==false&&receipt["verification"]["totals"]==report["totals"],json!("strict duplicate/unknown-field decoder consumed exact emitted bytes; no unsupported capability promotion"))?;
            let manifest = std::fs::read(self.evidence.join("source-manifest.json"))?;
            let source =
                humanymous_platform::testing::json::decode_strict_json(&manifest, 4 * 1024 * 1024)?;
            self.public_receipts["wargame"] = public_receipts::wargame(
                &receipt,
                &report,
                &source,
                &snapshot::digest(&manifest),
                result.stdout.as_bytes(),
            )?;
            self.components["wargame"] = report.clone();
            self.case("actual-owned-wargame",result.success && report["scoped_passed"]==true,json!("19 actual programs plus 7 unsupported; audit verdicts and origin deltas retained"))?;
            self.stage = "witness_component";
            witness::run(self)?;
        } else {
            self.components["runtime"] =
                json!({"status":"not_run","reason":"explicit topology-only phase"});
        }
        let controls: Value = serde_json::from_str(&self.docker.call(&[
            "image",
            "inspect",
            origin["Image"].as_str().ok_or("controls image missing")?,
        ])?)?;
        snapshot::write(&self.evidence, "controls-image.json", &controls[0])?;
        let _ = attacker;
        if self.collector {
            self.stage = "collector_build";
            eprintln!(
                "lab: building/running Firefox collector component without sandbox-disabling flags"
            );
            self.compose(&["build", "collector-browser"], true)?;
            self.compose(&["create", "collector-browser"], true)?;
            let collector = self.inspect("collector-browser")?;
            self.case(
                "collector-network-none",
                collector["HostConfig"]["NetworkMode"] == "none",
                json!("no external browser traffic"),
            )?;
            self.stage = "collector_process";
            let result = self.docker.invoke(
                &[
                    "start".into(),
                    "-a".into(),
                    collector["Id"]
                        .as_str()
                        .ok_or("collector ID absent")?
                        .into(),
                ],
                &[],
                false,
            )?;
            let passed_count = successful_test_count(&result.stdout);
            let has_tests = passed_count.is_some();
            self.components["collector"] = json!({"status":if result.success && has_tests{"passed"}else{"failed"},"evidence_type":"collector-component","passed_count":passed_count,"ordinary_human":false,"sandbox_qualified":false});
            self.case(
                "collector-component",
                result.success && has_tests,
                json!("upstream generated glue; component only, no human/AT"),
            )?;
            let image: Value = serde_json::from_str(
                &self.docker.call(&[
                    "image",
                    "inspect",
                    collector["Image"]
                        .as_str()
                        .ok_or("collector image missing")?,
                ])?,
            )?;
            snapshot::write(&self.evidence, "collector-image.json", &image[0])?;
        }
        Ok(())
    }
    fn cleanup(&mut self) -> Result<()> {
        self.docker.recheck_engine()?;
        let down = self.compose(
            &["down", "--volumes", "--remove-orphans", "--timeout", "2"],
            false,
        )?;
        let filter = format!("label=com.docker.compose.project={}", self.project);
        let containers = self.docker.call(&["ps", "-aq", "--filter", &filter])?;
        let networks = self
            .docker
            .call(&["network", "ls", "-q", "--filter", &filter])?;
        let volumes = self
            .docker
            .call(&["volume", "ls", "-q", "--filter", &filter])?;
        self.case(
            "cleanup-owned-project",
            down.success
                && containers.trim().is_empty()
                && networks.trim().is_empty()
                && volumes.trim().is_empty(),
            json!(self.project),
        )
    }
}
pub fn run(args: &[String]) -> Result<i32> {
    let root = snapshot::root()?;
    let mut evidence = None;
    let mut collector = false;
    let mut topology_only = false;
    let mut browser_product = false;
    let mut go_product = false;
    let mut mount_probe = false;
    let mut authority_node_ipc = false;
    let mut w2_h2_product_binding = false;
    let mut w2_h2_hosted_venue = false;
    let mut g4_cef_at = false;
    let mut full = false;
    let mut require_go_component = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--evidence" => {
                index += 1;
                evidence = Some(PathBuf::from(
                    args.get(index).ok_or("evidence path missing")?,
                ));
            }
            "--collector" => collector = true,
            "--browser-product" => browser_product = true,
            "--go-product" => go_product = true,
            "--mount-probe" => mount_probe = true,
            "--topology-only" => topology_only = true,
            "--authority-node-ipc" => authority_node_ipc = true,
            "--w2-h2-product-binding" => w2_h2_product_binding = true,
            "--w2-h2-hosted-venue" => w2_h2_hosted_venue = true,
            "--g4-cef-at" => g4_cef_at = true,
            "--require-full" => full = true,
            "--require-go-component" => require_go_component = true,
            _ => return Err("unknown runner argument".into()),
        }
        index += 1;
    }
    if authority_node_ipc {
        // Cap CLI bound only after Unlock C TRANSFER apply (fail-closed).
        crate::unlock_gate::require_authority_cap_unlocked(&root)?;
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "authority-node-ipc Cap requires its own fresh profile; refuse foreign Cap selectors; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if w2_h2_product_binding {
        // Cap CLI bound only after Unlock B transfer-apply with code-defined pilot.
        crate::unlock_gate::require_w2_h2_product_binding_unlocked(&root)?;
        // Refuse before freeze/Docker when Root Cap topology is absent (Cap conservation).
        h2_product_binding::require_freeze_preimages(&root)?;
        h2_product_binding::require_cap_topology(&root)?;
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_hosted_venue
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "w2-h2-product-binding Cap requires its own fresh profile; refuse foreign Cap selectors and capture/feeder reburns; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if w2_h2_hosted_venue {
        // N06 Cap CLI: topology + actual_hosted_linux; Desktop Cap refuse-closed (no spoof).
        h2_product_binding::require_freeze_preimages(&root)?;
        h2_product_binding::require_hosted_venue_for_n06_join(&root)?;
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "w2-h2-hosted-venue Cap requires its own fresh profile; refuse Desktop measure reburn and foreign Cap selectors; never invent pass_product_clean_room or desktop_as_hosted"
                    .into(),
            );
        }
    }
    if g4_cef_at {
        // Cap CLI bound only after Unlock A transfer-apply with code-defined pilot.
        crate::unlock_gate::require_g4_cef_at_unlocked(&root)?;
        // Refuse before CEF when freeze/module/bundle prerequisites fail (Cap conservation).
        g4_cef_at::require_freeze_preimages(&root)?;
        g4_cef_at::require_cap_module(&root)?;
        g4_cef_at::require_cef_bundle()?;
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || full
            || require_go_component
        {
            return Err(
                "g4-cef-at Cap requires its own fresh profile; refuse foreign Cap selectors and a11y/--require-full Cap spam; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    let evidence = evidence.ok_or("--evidence required")?.canonicalize()?;
    if mount_probe
        && (collector
            || topology_only
            || browser_product
            || go_product
            || full
            || require_go_component)
    {
        return Err("mount probe requires its own diagnostic-only fresh profile".into());
    }
    if browser_product && (collector || topology_only) {
        return Err("browser product requires a separate fresh bundle/profile".into());
    }
    if go_product && (browser_product || collector || topology_only) {
        return Err("Go product requires a separate fresh bundle/profile".into());
    }
    let requirement = requirement::Requirement::select(full, require_go_component, go_product)?;
    if !evidence.starts_with(root.join(".spec.local/root/evidence"))
        || !evidence.join("index.evidence.md").is_file()
    {
        return Err("existing private Specfold bundle required".into());
    }
    if evidence.join("lab-report.json").exists()
        || evidence.join("source-manifest.json").exists()
        || evidence.join("engine-preflight.private.json").exists()
    {
        return Err("refused: evidence bundle already contains a run; create a fresh Specfold bundle to preserve failures and source identity".into());
    }
    let mut random = [0; 8];
    getrandom::fill(&mut random)?;
    let project = format!(
        "hm-rust-lab-{}",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let source = snapshot::freeze(&root, &project, &evidence)?;
    // Required actual-source gate, before any product image is built.
    let source_binding = if browser_product {
        Some(source.source_binding()?)
    } else {
        None
    };
    let expires = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 1800;
    let mut lab = Lab {
        docker: Docker::new(&root),
        project: project.clone(),
        environment: vec![
            (
                "HM_LAB_BUILD_CONTEXT".into(),
                source.0.to_string_lossy().into_owned(),
            ),
            ("HM_LAB_IMAGE_TAG".into(), project),
            ("HM_LAB_EXPIRES".into(), expires.to_string()),
            ("COMPOSE_PARALLEL_LIMIT".into(), "1".into()),
        ],
        evidence,
        collector,
        topology_only,
        browser_product,
        source_binding,
        go_product,
        mount_probe,
        authority_node_ipc,
        w2_h2_product_binding,
        w2_h2_hosted_venue,
        g4_cef_at,
        go_component_verified: false,
        stage: "initialization",
        cases: Vec::new(),
        components: json!({}),
        public_receipts: json!({"schema":"humanymous.lab-public-receipts/1"}),
    };
    let execution = lab.execute().and_then(|()| {
        if let Some(binding) = &lab.source_binding {
            binding.revalidate()?;
        }
        Ok(())
    });
    if let Err(error) = execution {
        let filter = format!("label=com.docker.compose.project={}", lab.project);
        if let Ok(ids) = lab.docker.call(&["ps", "-aq", "--filter", &filter]) {
            for id in ids.lines() {
                let _ = lab.docker.invoke(
                    &["logs".into(), "--tail".into(), "64".into(), id.into()],
                    &[],
                    false,
                );
            }
        }
        lab.cases
            .push(json!({"id":"execution","status":"failed","error_code":lab.stage,"detail":error.to_string()}));
    }
    let cleanup_verified = match lab.cleanup() {
        Ok(()) => true,
        Err(error) => {
            lab.cases.push(json!({"id":"cleanup","status":"failed","error_code":"cleanup_failed","detail":error.to_string()}));
            false
        }
    };
    snapshot::write(
        &lab.evidence,
        "docker-engine-affinity.json",
        &json!({"schema":"humanymous.lab-engine-evidence/1","observation":lab.docker.engine_identity,"actual_sdk_connection_observed":false}),
    )?;
    let failed = lab.cases.iter().any(|c| c["status"] == "failed");
    let assessment = requirement.assess(failed, lab.go_component_verified, cleanup_verified);
    let mut evidence_types = if browser_product
        || go_product
        || mount_probe
        || authority_node_ipc
        || w2_h2_product_binding
        || w2_h2_hosted_venue
        || g4_cef_at
    {
        vec![]
    } else {
        vec!["topology-control"]
    };
    for (component, kind) in [
        ("runtime", "runtime-process"),
        ("wargame", "supervisor-owned-runtime-child-http1"),
        ("independent_witness", "independent-witness-component"),
        ("collector", "collector-component"),
        ("go_product", "actual-go-gate-bounded-http"),
        ("mount_probe", "custody-layout-mount-diagnostic"),
        (
            "browser_product",
            "same-origin-browser-runtime-isolated-pilot",
        ),
        ("authority_node_ipc", "authority-node-ipc-component"),
        ("w2_h2_product_binding", "w2-h2-product-binding-component"),
        ("w2_h2_hosted_venue", "w2-h2-hosted-venue-component"),
        ("g4_cef_at", "g4-cef-at-component"),
    ] {
        if lab.components[component].is_object() && lab.components[component]["status"] != "not_run"
        {
            evidence_types.push(kind);
        }
    }
    let mut diagnostics = safe_diagnostics(&lab.cases);
    if let Some(outcome) = lab.components.get("browser_actor_diagnostic") {
        diagnostics["browser_actor"] = crate::browser_outcome::public_outcome(outcome)?;
    }
    let report = json!({"schema":"humanymous.product-lab/2","requirement":requirement.report(&assessment),"safe_diagnostics":diagnostics,"public_receipts":lab.public_receipts,"project":lab.project,"evidence_types":evidence_types,"cases":lab.cases,"components":lab.components,"required_unavailable":["full Gate TLS/H2/capture","session-bound real browser collector","Pass recovery","human/keyboard/assistive controls","runtime binary authenticated witness transport","cross-namespace origin/audit effect product binding","signed remote target attestation","native OS qualification","provider deployment"],"full_product_qualified":false,"release_ready":false,"ttl":{"expires_unix":expires,"external_janitor_scheduled":false}});
    snapshot::write(&lab.evidence, "lab-report.json", &report)?;
    snapshot::write(
        &lab.evidence,
        "docker-transcript.json",
        &json!(lab.docker.transcript),
    )?;
    println!(
        "{}",
        json!({"report":lab.evidence.join("lab-report.json"),"passed":report["cases"].as_array().ok_or("cases missing")?.iter().filter(|c|c["status"]=="passed").count(),"failed":failed,"release_ready":false})
    );
    Ok(assessment.exit_code)
}
fn successful_test_count(output: &str) -> Option<u64> {
    output
        .lines()
        .filter_map(|line| {
            line.split_once("test result: ok. ")?
                .1
                .split_once(" passed;")?
                .0
                .trim()
                .parse::<u64>()
                .ok()
        })
        .find(|count| *count > 0)
}
pub fn owned_expired(labels: &Value, now: u64) -> bool {
    let project = labels["com.docker.compose.project"].as_str().unwrap_or("");
    let suffix = project.strip_prefix("hm-rust-lab-").unwrap_or("");
    labels["org.humanymous.owner"] == OWNER
        && suffix.len() == 16
        && suffix.bytes().all(|b| b.is_ascii_hexdigit())
        && labels["org.humanymous.expires"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .is_some_and(|n| n > 0 && n <= now)
}
pub fn cleanup_expired() -> Result<()> {
    let mut docker = Docker::new(&snapshot::root()?);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut projects = std::collections::BTreeSet::new();
    for args in [
        vec![
            "ps",
            "-aq",
            "--filter",
            "label=org.humanymous.owner=rust-product-lab",
        ],
        vec![
            "network",
            "ls",
            "-q",
            "--filter",
            "label=org.humanymous.owner=rust-product-lab",
        ],
    ] {
        for id in docker.call(&args)?.lines() {
            let inspected: Value = serde_json::from_str(&docker.call(&["inspect", id])?)?;
            let state = &inspected[0];
            let labels = state
                .get("Config")
                .map_or(&state["Labels"], |c| &c["Labels"]);
            if owned_expired(labels, now) {
                projects.insert(
                    labels["com.docker.compose.project"]
                        .as_str()
                        .ok_or("project label missing")?
                        .to_owned(),
                );
            }
        }
    }
    for project in &projects {
        docker.recheck_engine()?;
        let filter = format!("label=com.docker.compose.project={project}");
        // Delete only individually re-inspected expired owned resources. No
        // Compose interpolation, global prune, or same-project foreign deletion.
        for (kind, listing) in [
            ("container", vec!["ps", "-aq", "--filter", filter.as_str()]),
            (
                "network",
                vec!["network", "ls", "-q", "--filter", filter.as_str()],
            ),
        ] {
            for id in docker.call(&listing)?.lines() {
                let states: Value = serde_json::from_str(&docker.call(&[kind, "inspect", id])?)?;
                let state = &states[0];
                let labels = state
                    .get("Config")
                    .map_or(&state["Labels"], |c| &c["Labels"]);
                if !owned_expired(labels, now) {
                    continue;
                }
                if kind == "container" {
                    docker.call(&[kind, "rm", "--force", id])?;
                } else {
                    docker.call(&[kind, "rm", id])?;
                }
            }
        }
    }
    println!("{}", json!({"expired_owned_projects":projects}));
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn public_diagnostics_never_copy_detail_or_arbitrary_error_text() {
        let input = vec![
            serde_json::json!({"id":"execution","status":"failed","error_code":"browser_actor","detail":"Bearer private secret"}),
            serde_json::json!({"id":"private@example.com","status":"failed","error_code":"Bearer private secret"}),
        ];
        let safe = super::safe_diagnostics(&input);
        assert_eq!(safe["failure_count"], 2);
        assert_eq!(safe["failures"][0]["error_code"], "browser_actor");
        assert_eq!(safe["failures"][1]["case_id"], "unclassified_case");
        assert_eq!(safe["failures"][1]["error_code"], "assertion_failed");
        for code in [
            "go_runtime_build_deadline",
            "go_runtime_build_capture",
            "go_runtime_build_exit",
            "go_canary_build_deadline",
            "go_canary_build_capture",
            "go_canary_build_exit",
        ] {
            let projected = super::safe_diagnostics(&[
                serde_json::json!({"id":"execution","status":"failed","error_code":code}),
            ]);
            assert_eq!(projected["failures"][0]["error_code"], code);
        }
        let bytes = safe.to_string();
        assert!(
            !bytes.contains("private") && !bytes.contains("detail") && !bytes.contains("Bearer")
        );
    }
    use super::*;
    #[test]
    fn component_denominator_does_not_confuse_ten_or_forty_with_zero() {
        for count in [1, 10, 40] {
            assert_eq!(
                successful_test_count(&format!("test result: ok. {count} passed; 0 failed;")),
                Some(count)
            );
        }
        assert_eq!(
            successful_test_count("test result: ok. 0 passed; 0 failed;"),
            None
        );
        assert_eq!(
            successful_test_count("test result: FAILED. 1 passed; 1 failed;"),
            None
        );
    }
    #[test]
    fn janitor_rejects_foreign_future_and_unbounded_project() {
        let labels = json!({"org.humanymous.owner":OWNER,"com.docker.compose.project":"hm-rust-lab-0123456789abcdef","org.humanymous.expires":"100"});
        assert!(owned_expired(&labels, 101));
        for (key, value) in [
            ("org.humanymous.owner", "other"),
            ("com.docker.compose.project", "production"),
            ("org.humanymous.expires", "0"),
            ("org.humanymous.expires", "102"),
        ] {
            let mut bad = labels.clone();
            bad[key] = json!(value);
            assert!(!owned_expired(&bad, 101));
        }
    }
}
