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
mod attack_catalog;
mod authority;
mod browser;
mod browser_actor;
mod browser_epochs;
mod browser_failure;
pub(crate) mod cap0_product_honesty;
pub(crate) mod clean_room_product;
pub(crate) mod clean_room_product_matrix;
pub(crate) mod coherent_ceiling;
mod collector_parity;
mod deployment;
pub(crate) mod g2_collector_product_binding;
pub(crate) mod g4_cef_at;
pub(crate) mod g5_witness_transport;
pub(crate) mod g6_origin_audit_binding;
mod gate_tls;
mod go;
mod go_fixture;
mod go_public;
pub(crate) mod h2_product_binding;
mod mount_probe;
mod origin_audit;
mod pass_recovery;
mod public_receipts;
mod qualification;
pub(crate) mod red_blue_product;
mod requirement;
mod startup_failure;
mod target_attestation;
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
    n06_owner_join_readiness: bool,
    g2_collector_product_binding: bool,
    g2_collector_product_binding_readiness: bool,
    g5_witness_transport_honesty: bool,
    g5_witness_transport: bool,
    clean_room_product: bool,
    g6_origin_audit_binding: bool,
    red_blue_product: bool,
    clean_room_product_matrix: bool,
    require_full: bool,
    browser_ready_interactive_at_honesty: bool,
    n06_owner_auth_handoff_honesty: bool,
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
        if self.n06_owner_join_readiness {
            // Owner-join readiness is receipt/auth gate only; no Docker Cap burn.
            return Err(
                "n06-owner-join-readiness Cap has no Docker compose topology; Cap=0 readiness/auth handoff only; never invent pass_product_clean_room or Root ownership tokens"
                    .into(),
            );
        }
        if self.g2_collector_product_binding {
            // Cap topology is Cap-ready until TRANSFER; Cap CLI refuse-closed when absent.
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.g2-collector-product-binding.yaml")?,
                "--profile".into(),
                "g2-collector-product-binding".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.g2_collector_product_binding_readiness {
            return Err(
                "g2-collector-product-binding-readiness Cap has no Docker compose topology; Cap=0 honesty only; never invent pass_product_clean_room"
                    .into(),
            );
        }
        if self.g5_witness_transport {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.g5-witness-transport.yaml")?,
                "--profile".into(),
                "g5-witness-transport".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.g5_witness_transport_honesty {
            return Err(
                "g5-witness-transport-honesty Cap has no Docker compose topology; Cap=0 honesty only; never invent authenticated witness transport product"
                    .into(),
            );
        }
        if self.clean_room_product {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.clean-room-product.yaml")?,
                "--profile".into(),
                "clean-room-product".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.g6_origin_audit_binding {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.g6-origin-audit-binding.yaml")?,
                "--profile".into(),
                "g6-origin-audit-binding".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.red_blue_product {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.red-blue-product.yaml")?,
                "--profile".into(),
                "red-blue-product".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.clean_room_product_matrix {
            let mut command = vec![
                "compose".into(),
                "-p".into(),
                self.project.clone(),
                "-f".into(),
                self.config_file("deployments/lab/compose.clean-room-product-matrix.yaml")?,
                "--profile".into(),
                "clean-room-product-matrix".into(),
            ];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            return Ok(command);
        }
        if self.browser_ready_interactive_at_honesty {
            return Err(
                "browser-ready-interactive-at-honesty Cap has no Docker compose topology; Cap=0 honesty only; never invent BrowserReady or Root AT tokens"
                    .into(),
            );
        }
        if self.n06_owner_auth_handoff_honesty {
            return Err(
                "n06-owner-auth-handoff-honesty Cap has no Docker compose topology; Cap=0 honesty only; never invent n06_owner_join or Root ownership tokens"
                    .into(),
            );
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
        let mounts_allowed = if let Some(destinations) = match service {
            "oa-gate" => Some(&["/writer/keys", "/writer/wal"][..]),
            "oa-tamper" => Some(&["/writer/wal"][..]),
            _ => None,
        } {
            // Origin/audit volumes: the tamper actor never sees custody.
            container["Mounts"].as_array().is_some_and(|mounts| {
                mounts.len() == destinations.len()
                    && mounts.iter().all(|m| {
                        m["Type"] == "volume"
                            && destinations.contains(&m["Destination"].as_str().unwrap_or(""))
                    })
            })
        } else if service.starts_with("witness-")
            || matches!(
                service,
                "browser-gate" | "go-gate" | "mount-writer-volume" | "mount-witness-volume"
            )
        {
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
        if self.n06_owner_join_readiness {
            return h2_product_binding::run_n06_owner_join_readiness(self);
        }
        if self.g2_collector_product_binding {
            return g2_collector_product_binding::run(self);
        }
        if self.g2_collector_product_binding_readiness {
            return cap0_product_honesty::run_g2_collector_product_binding_readiness(self);
        }
        if self.g5_witness_transport {
            return g5_witness_transport::run(self);
        }
        if self.g5_witness_transport_honesty {
            return cap0_product_honesty::run_g5_witness_transport_honesty(self);
        }
        if self.clean_room_product {
            return clean_room_product::run(self);
        }
        if self.g6_origin_audit_binding {
            return g6_origin_audit_binding::run(self);
        }
        if self.red_blue_product {
            return red_blue_product::run(self, self.require_full);
        }
        if self.clean_room_product_matrix {
            return clean_room_product_matrix::run(self, self.require_full);
        }
        if self.browser_ready_interactive_at_honesty {
            return cap0_product_honesty::run_browser_ready_interactive_at_honesty(self);
        }
        if self.n06_owner_auth_handoff_honesty {
            return cap0_product_honesty::run_n06_owner_auth_handoff_honesty(self);
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
            self.stage = "attestation_process";
            target_attestation::run(self)?;
            self.stage = "pass_recovery";
            pass_recovery::run(self)?;
            self.stage = "origin_audit";
            origin_audit::run(self)?;
            self.stage = "attack_catalog";
            attack_catalog::run(self)?;
            self.stage = "gate_tls";
            gate_tls::run(self)?;
            self.stage = "witness_component";
            witness::run(self)?;
            self.stage = "deployment";
            deployment::run(self)?;
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
            self.case(
                "collector-tmpfs",
                collector_writable_tmp(&collector["HostConfig"]),
                json!("read-only root with a writable /tmp tmpfs for the test runner"),
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
            if !(result.success && has_tests) {
                // Name the failure mode publicly without exposing raw output.
                let exit = self
                    .docker
                    .transcript
                    .last()
                    .and_then(|entry| entry["exit"].as_i64());
                self.stage =
                    collector_outcome(result.success, exit, &result.stdout, &result.stderr);
            }
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
        // Host-only Caps never open compose topology; still recheck Desktop
        // engine affinity (HUMANYMOUS_DOCKER_DESKTOP_BASELINE) then prove the
        // owned project left no Docker resources. Never invent compose down.
        // G2/G5/CR Cap=1 Docker burns use compose topology → compose down path below.
        // Cap=0 red-blue-product honesty never opens compose when Cap topology
        // is absent; host-only cleanup. Cap=1 with live topology uses compose down.
        let red_blue_host_only = self.red_blue_product
            && red_blue_product::missing_cap_topology(std::path::Path::new(&self.docker.root))
                .len()
                == red_blue_product::REQUIRED_CAP_TOPOLOGY.len();
        let product_matrix_host_only = self.clean_room_product_matrix
            && clean_room_product_matrix::missing_cap_topology(std::path::Path::new(
                &self.docker.root,
            ))
            .len()
                == clean_room_product_matrix::REQUIRED_CAP_TOPOLOGY.len();
        if self.g4_cef_at
            || self.n06_owner_join_readiness
            || self.g2_collector_product_binding_readiness
            || self.g5_witness_transport_honesty
            || self.browser_ready_interactive_at_honesty
            || self.n06_owner_auth_handoff_honesty
            || red_blue_host_only
            || product_matrix_host_only
        {
            self.docker.recheck_engine()?;
            let filter = format!("label=com.docker.compose.project={}", self.project);
            let containers = self.docker.call(&["ps", "-aq", "--filter", &filter])?;
            let networks = self
                .docker
                .call(&["network", "ls", "-q", "--filter", &filter])?;
            let volumes = self
                .docker
                .call(&["volume", "ls", "-q", "--filter", &filter])?;
            return self.case(
                "cleanup-owned-project",
                containers.trim().is_empty()
                    && networks.trim().is_empty()
                    && volumes.trim().is_empty(),
                json!(self.project),
            );
        }
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
    let mut n06_owner_join_readiness = false;
    let mut g2_collector_product_binding = false;
    let mut g2_collector_product_binding_readiness = false;
    let mut g5_witness_transport_honesty = false;
    let mut g5_witness_transport = false;
    let mut clean_room_product = false;
    let mut g6_origin_audit_binding = false;
    let mut red_blue_product = false;
    let mut clean_room_product_matrix = false;
    let mut browser_ready_interactive_at_honesty = false;
    let mut n06_owner_auth_handoff_honesty = false;
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
            "--n06-owner-join-readiness" => n06_owner_join_readiness = true,
            "--g2-collector-product-binding" => g2_collector_product_binding = true,
            "--g2-collector-product-binding-readiness" => {
                g2_collector_product_binding_readiness = true
            }
            "--g5-witness-transport-honesty" => g5_witness_transport_honesty = true,
            "--g5-witness-transport" => g5_witness_transport = true,
            "--clean-room-product" => clean_room_product = true,
            "--g6-origin-audit-binding" => g6_origin_audit_binding = true,
            "--red-blue-product" => red_blue_product = true,
            "--clean-room-product-matrix" => clean_room_product_matrix = true,
            "--browser-ready-interactive-at-honesty" => browser_ready_interactive_at_honesty = true,
            "--n06-owner-auth-handoff-honesty" => n06_owner_auth_handoff_honesty = true,
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
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
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
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
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
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
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
    if n06_owner_join_readiness {
        // Cursor-owned readiness/auth gate; Cap=0 without Root auth handoff (no Docker burn).
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "n06-owner-join-readiness Cap requires its own fresh profile; refuse hosted Cap reburn / Unlock A invent / foreign Cap selectors; Cap=0 without Root auth; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if g2_collector_product_binding {
        // Cap CLI Cap=0 scaffold: Cap free / topology refuse-closed inside Cap module
        // before Docker. Cap=1 only after Cap realloc + live Cap topology TRANSFER.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "g2-collector-product-binding Cap requires its own fresh profile; refuse readiness/--require-full Cap spam; Cap=0 scaffold refuse-closed without Cap realloc+topology; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if g5_witness_transport {
        // Cap CLI Cap=0 scaffold: Cap free / topology refuse-closed inside Cap module
        // before Docker. Cap=1 only after Cap realloc + live Cap topology TRANSFER.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "g5-witness-transport Cap requires its own fresh profile; refuse honesty/--require-full Cap spam; Cap=0 scaffold refuse-closed without Cap realloc+topology; never invent authenticated witness transport product"
                    .into(),
            );
        }
    }
    if g2_collector_product_binding_readiness {
        // Cap=0 honesty only; no Cap burn / no require-full Cap spam.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "g2-collector-product-binding-readiness Cap requires its own fresh Cap=0 profile; refuse collector/--require-full Cap spam; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if g5_witness_transport_honesty {
        // Cap=0 honesty only; no Cap burn / no require-full Cap spam.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "g5-witness-transport-honesty Cap requires its own fresh Cap=0 profile; refuse witness/--require-full Cap spam; never invent authenticated witness transport product"
                    .into(),
            );
        }
    }
    if browser_ready_interactive_at_honesty {
        // Cap=0 honesty only; no Cap burn / no Unlock A Cap reburn / no invent AT.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "browser-ready-interactive-at-honesty Cap requires its own fresh Cap=0 profile; refuse g4/--require-full Cap spam; never invent BrowserReady or Root AT tokens"
                    .into(),
            );
        }
    }
    if n06_owner_auth_handoff_honesty {
        // Cap=0 honesty only; no Cap burn / no Unlock A / hosted Cap reburn / invent join.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "n06-owner-auth-handoff-honesty Cap requires its own fresh Cap=0 profile; refuse hosted/Unlock A/--require-full Cap spam; never invent n06_owner_join or Root ownership tokens"
                    .into(),
            );
        }
    }
    if clean_room_product {
        // Cap CLI Cap=0 scaffold: Cap free / topology refuse-closed inside Cap module
        // before Docker. Cap=1 only after Cap realloc + live Cap topology TRANSFER.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "clean-room-product Cap requires its own fresh profile; refuse G2/G5 twin/--require-full Cap spam; Cap=0 scaffold refuse-closed without Cap realloc+topology; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }
    if g6_origin_audit_binding {
        // Cap CLI Cap=0 scaffold: Cap free / topology refuse-closed inside Cap module
        // before Docker. Cap=1 only after Cap realloc + live Cap topology TRANSFER.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || full
            || require_go_component
        {
            return Err(
                "g6-origin-audit-binding Cap requires its own fresh profile; refuse G2/G5/CR twin/--require-full Cap spam; Cap=0 scaffold refuse-closed without Cap realloc+topology; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }

    if red_blue_product {
        // Cap=0 require-full honesty / Cap=1 product-red-blue feeder after Cap realloc.
        // --require-full is intentional (product feeder); refuse Cap twin spam.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || require_go_component
        {
            return Err(
                "red-blue-product Cap requires its own fresh profile; refuse G2/G5/CR/G6 Cap twin spam; Cap=0 require-full honesty lists missing product cells; Cap=1 after Cap realloc is product/red-blue feeder; never invent pass_product_clean_room"
                    .into(),
            );
        }
    }

    if clean_room_product_matrix {
        // Cap=0 require-full honesty / Cap=1 product-matrix feeder after Cap realloc.
        // Distinct from spent --clean-room-product Cap twin; --require-full intentional.
        if collector
            || topology_only
            || browser_product
            || go_product
            || mount_probe
            || authority_node_ipc
            || w2_h2_product_binding
            || w2_h2_hosted_venue
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
            || g4_cef_at
            || require_go_component
        {
            return Err(
                "clean-room-product-matrix Cap requires its own fresh profile; refuse spent CR Cap twin / G2/G5/G6/RB Cap twin spam; Cap=0 require-full honesty lists missing product cells; Cap=1 after Cap realloc is product-matrix feeder; never invent pass_product_clean_room"
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
            || n06_owner_join_readiness
            || g2_collector_product_binding
            || g2_collector_product_binding_readiness
            || g5_witness_transport_honesty
            || g5_witness_transport
            || clean_room_product
            || g6_origin_audit_binding
            || red_blue_product
            || clean_room_product_matrix
            || browser_ready_interactive_at_honesty
            || n06_owner_auth_handoff_honesty
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
        n06_owner_join_readiness,
        g2_collector_product_binding,
        g2_collector_product_binding_readiness,
        g5_witness_transport_honesty,
        g5_witness_transport,
        clean_room_product,
        g6_origin_audit_binding,
        red_blue_product,
        clean_room_product_matrix,
        require_full: full,
        browser_ready_interactive_at_honesty,
        n06_owner_auth_handoff_honesty,
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
        // Specfold Cap public log must show fail-closed stage/detail (private transcript stays private).
        if w2_h2_hosted_venue || w2_h2_product_binding || authority_node_ipc {
            eprintln!("lab Cap fail-closed: stage={} detail={}", lab.stage, error);
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
    // Derived, never hard-coded: absent or failed required cases stay unavailable.
    let qualification = qualification::assess(&lab.cases, cleanup_verified);
    let assessment = requirement.assess(
        failed,
        lab.go_component_verified,
        cleanup_verified,
        qualification.full_product_qualified,
    );
    let mut evidence_types = if browser_product
        || go_product
        || mount_probe
        || authority_node_ipc
        || w2_h2_product_binding
        || w2_h2_hosted_venue
        || n06_owner_join_readiness
        || g2_collector_product_binding
        || g2_collector_product_binding_readiness
        || g5_witness_transport_honesty
        || g5_witness_transport
        || clean_room_product
        || g6_origin_audit_binding
        || red_blue_product
        || clean_room_product_matrix
        || browser_ready_interactive_at_honesty
        || n06_owner_auth_handoff_honesty
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
        ("n06_owner_join_readiness", "n06-owner-join-readiness"),
        (
            "g2_collector_product_binding",
            "g2-collector-product-binding-cap",
        ),
        (
            "g2_collector_product_binding_readiness",
            "g2-collector-product-binding-readiness",
        ),
        (
            "g5_witness_transport_honesty",
            "g5-witness-transport-honesty",
        ),
        ("g5_witness_transport", "g5-witness-transport-cap"),
        ("clean_room_product", "clean-room-product-cap"),
        ("red_blue_product", "red-blue-product-cap"),
        ("clean_room_product_matrix", "clean-room-product-matrix-cap"),
        (
            "browser_ready_interactive_at_honesty",
            "browser-ready-interactive-at-honesty",
        ),
        (
            "n06_owner_auth_handoff_honesty",
            "n06-owner-auth-handoff-honesty",
        ),
        ("g4_cef_at", "g4-cef-at-component"),
        (
            "target_attestation",
            "pinned-key-fresh-nonce-target-attestation",
        ),
        ("pass_recovery", "pass-step-up-recovery"),
        ("origin_audit", "cross-namespace-origin-audit-binding"),
        ("attack_catalog", "docker-network-attack-catalog"),
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
    let source_sha = std::fs::read(lab.evidence.join("source-manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|manifest| manifest["head"].as_str().map(str::to_owned));
    let documentation_review_sha256 = std::fs::read(root.join("docs/reviews.json"))
        .ok()
        .map(|bytes| snapshot::digest(&bytes));
    let mut report = json!({"schema":"humanymous.product-lab/2","requirement":requirement.report(&assessment),"safe_diagnostics":diagnostics,"public_receipts":lab.public_receipts,"project":lab.project,"evidence_types":evidence_types,"cases":lab.cases,"components":lab.components,"source_sha":source_sha,"documentation_review_sha256":documentation_review_sha256,"release_ready":false,"ttl":{"expires_unix":expires,"external_janitor_scheduled":false}});
    for (key, value) in qualification
        .report_fields()
        .as_object()
        .ok_or("qualification fields")?
    {
        report[key] = value.clone();
    }
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
/// Enumerated, content-free classification of a failed collector run. Raw
/// stdout and stderr stay in the private transcript.
fn collector_outcome(success: bool, exit: Option<i64>, stdout: &str, stderr: &str) -> &'static str {
    let both = [stdout, stderr];
    let any = |needle: &str| both.iter().any(|text| text.contains(needle));
    if exit == Some(137) {
        "collector_killed"
    } else if matches!(exit, Some(125..=127)) {
        "collector_exec_error"
    } else if any("test result: FAILED") {
        "collector_tests_failed"
    } else if any("timed out") || any("Timed out") {
        "collector_timeout"
    } else if any("geckodriver") || any("failed to bind") || any("WebDriver") {
        "collector_driver"
    } else if any("panicked at") {
        "collector_runner_panic"
    } else if any("No such file or directory") {
        "collector_missing_file"
    } else if denied(stdout) || denied(stderr) {
        collector_denial(stdout, stderr)
    } else if stdout.trim().is_empty() && stderr.trim().is_empty() {
        "collector_silent_exit"
    } else if success {
        "collector_no_summary_exit_zero"
    } else {
        "collector_no_summary_exit_nonzero"
    }
}
/// The collector needs a read-only root and a tmpfs on /tmp; otherwise the
/// runner cannot create its temp dir.
fn collector_writable_tmp(host: &Value) -> bool {
    host["ReadonlyRootfs"] == true
        && host["Tmpfs"]
            .as_object()
            .is_some_and(|mounts| mounts.contains_key("/tmp"))
}
fn denied(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("read-only file system") || text.contains("permission denied")
}
/// Names a denied write by origin, runner stage and kind, from public markers only.
fn collector_denial(stdout: &str, stderr: &str) -> &'static str {
    let both = [stdout, stderr];
    let any = |needle: &str| both.iter().any(|text| text.contains(needle));
    let read_only = both
        .iter()
        .any(|text| text.to_ascii_lowercase().contains("read-only file system"));
    if any("Error response from daemon") || any("OCI runtime") {
        return if read_only {
            "collector_daemon_read_only"
        } else {
            "collector_daemon_permission"
        };
    }
    let started_tests = both.iter().any(|text| {
        text.lines().any(|line| {
            line.strip_prefix("running ")
                .and_then(|rest| rest.split_once(" test"))
                .is_some_and(|(count, _)| count.parse::<u64>().is_ok())
        })
    });
    let stage = if started_tests {
        2
    } else if any("Running headless tests") {
        1
    } else {
        0
    };
    // The runner prints its timeout line only after creating its temp dir.
    let tempdir_made = any("Set WASM_BINDGEN_TEST_TIMEOUT");
    match (stage, read_only) {
        (0, true) if !tempdir_made => "collector_tempdir_read_only",
        (0, false) if !tempdir_made => "collector_tempdir_permission",
        (0, true) => "collector_driver_start_read_only",
        (0, false) => "collector_driver_start_permission",
        (1, true) => "collector_browser_start_read_only",
        (1, false) => "collector_browser_start_permission",
        (_, true) => "collector_tests_read_only",
        (_, false) => "collector_tests_permission",
    }
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
    fn collector_failures_are_named_by_enumerated_public_codes() {
        use super::collector_outcome as outcome;
        let cases = [
            (
                false,
                "test result: FAILED. 3 passed; 1 failed;",
                "",
                "collector_tests_failed",
            ),
            (
                false,
                "",
                "Timed out waiting for the test suite",
                "collector_timeout",
            ),
            (false, "", "failed to bind geckodriver", "collector_driver"),
            (
                false,
                "",
                "thread 'main' panicked at src/main.rs",
                "collector_runner_panic",
            ),
            (true, "", "", "collector_silent_exit"),
            (false, "", "", "collector_silent_exit"),
            (
                false,
                "",
                "error: cannot open wasm: No such file or directory",
                "collector_missing_file",
            ),
            (
                false,
                "",
                "mkdir: Read-only file system",
                "collector_tempdir_read_only",
            ),
            (
                true,
                "running 3 tests",
                "",
                "collector_no_summary_exit_zero",
            ),
            (
                false,
                "running 3 tests",
                "",
                "collector_no_summary_exit_nonzero",
            ),
        ];
        let denial = [
            (
                "",
                "Error response from daemon: read-only file system",
                "collector_daemon_read_only",
            ),
            (
                "",
                "Error response from daemon: permission denied",
                "collector_daemon_permission",
            ),
            (
                "",
                "Permission denied (os error 13)",
                "collector_tempdir_permission",
            ),
            (
                "",
                "Error: Read-only file system (os error 30)",
                "collector_tempdir_read_only",
            ),
            (
                "Set WASM_BINDGEN_TEST_TIMEOUT to 90 seconds...",
                "Read-only file system",
                "collector_driver_start_read_only",
            ),
            (
                "Set WASM_BINDGEN_TEST_TIMEOUT to 90 seconds...",
                "Permission denied",
                "collector_driver_start_permission",
            ),
            (
                "Running headless tests in Firefox",
                "Read-only file system",
                "collector_browser_start_read_only",
            ),
            (
                "Running headless tests in Firefox",
                "Permission denied",
                "collector_browser_start_permission",
            ),
            (
                "Running headless tests\n\nrunning 66 tests\n",
                "Read-only file system",
                "collector_tests_read_only",
            ),
            (
                "Running headless tests\n\nrunning 1 test\n",
                "Permission denied",
                "collector_tests_permission",
            ),
        ];
        for (stdout, stderr, code) in denial {
            assert_eq!(
                outcome(false, Some(1), stdout, stderr),
                code,
                "{stdout:?} {stderr:?}"
            );
        }
        assert_eq!(outcome(false, Some(137), "running", ""), "collector_killed");
        let tmp = serde_json::json!({"ReadonlyRootfs":true,"Tmpfs":{"/tmp":"rw,size=512m"}});
        assert!(super::collector_writable_tmp(&tmp));
        let shm_only = serde_json::json!({"ReadonlyRootfs":true,"Tmpfs":{"/dev/shm":"rw"}});
        assert!(!super::collector_writable_tmp(&shm_only));
        assert!(!super::collector_writable_tmp(
            &serde_json::json!({"ReadonlyRootfs":true})
        ));
        let writable_root = serde_json::json!({"ReadonlyRootfs":false,"Tmpfs":{"/tmp":"rw"}});
        assert!(!super::collector_writable_tmp(&writable_root));
        assert_eq!(outcome(false, Some(126), "", ""), "collector_exec_error");
        assert_eq!(
            outcome(
                false,
                Some(1),
                "test result: FAILED. 1 passed; 1 failed;",
                ""
            ),
            "collector_tests_failed"
        );
        for (success, stdout, stderr, code) in cases {
            assert_eq!(outcome(success, Some(1), stdout, stderr), code);
            let safe = super::safe_diagnostics(&[serde_json::json!(
                {"id":"execution","status":"failed","error_code":code,"detail":stderr}
            )]);
            assert_eq!(
                safe["failures"][0]["error_code"], code,
                "{code} not in contract"
            );
        }
    }
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
