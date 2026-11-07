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
mod witness;
struct Lab {
    docker: Docker,
    project: String,
    environment: Vec<(String, String)>,
    evidence: PathBuf,
    collector: bool,
    topology_only: bool,
    cases: Vec<Value>,
    components: Value,
}
impl Lab {
    fn compose(&mut self, args: &[&str], required: bool) -> Result<crate::docker::Output> {
        let mut command = vec![
            "compose".into(),
            "-p".into(),
            self.project.clone(),
            "-f".into(),
            "deployments/lab/compose.rust.yaml".into(),
            "--profile".into(),
            "controls".into(),
        ];
        if !self.topology_only {
            command.extend([
                "-f".into(),
                "deployments/lab/compose.rust.witness.yaml".into(),
                "--profile".into(),
                "witness".into(),
            ]);
        }
        if self.collector {
            command.extend([
                "-f".into(),
                "deployments/lab/compose.rust.collector.yaml".into(),
                "--profile".into(),
                "collector".into(),
            ]);
        }
        command.extend(args.iter().map(|s| (*s).to_owned()));
        self.docker.invoke(&command, &self.environment, required)
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
        let mounts_allowed = if service.starts_with("witness-") {
            let destinations: &[&str] = if service == "witness-writer" {
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
        let expected_user = if service == "witness-transaction" {
            "65533:65533"
        } else {
            "65532:65532"
        };
        self.case(
            &format!("hardened-{service}"),
            container["Config"]["User"] == expected_user
                && host["ReadonlyRootfs"] == true
                && host["Privileged"] == false
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
            self.compose(&["build", "runtime-pilot"], true)?;
            self.compose(&["create", "runtime-pilot"], true)?;
            let runtime = self.inspect("runtime-pilot")?;
            self.case(
                "runtime-network-none",
                runtime["HostConfig"]["NetworkMode"] == "none",
                json!("runtime/client/origin share owned loopback; not L5 isolation"),
            )?;
            eprintln!("lab: running actual sealed-runtime process pilot");
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
            let report: Value = serde_json::from_str(result.stdout.trim())?;
            snapshot::write(&self.evidence, "wargame-report.json", &report)?;
            self.components["wargame"] = report.clone();
            self.case("actual-owned-wargame",result.success && report["scoped_passed"]==true,json!("19 actual programs plus 7 unsupported; audit verdicts and origin deltas retained"))?;
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
    let mut full = false;
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
            "--topology-only" => topology_only = true,
            "--require-full" => full = true,
            _ => return Err("unknown runner argument".into()),
        }
        index += 1;
    }
    let evidence = evidence.ok_or("--evidence required")?.canonicalize()?;
    if !evidence.starts_with(root.join(".spec.local/root/evidence"))
        || !evidence.join("index.evidence.md").is_file()
    {
        return Err("existing private Specfold bundle required".into());
    }
    if evidence.join("lab-report.json").exists() || evidence.join("source-manifest.json").exists() {
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
        ],
        evidence,
        collector,
        topology_only,
        cases: Vec::new(),
        components: json!({}),
    };
    if let Err(error) = lab.execute() {
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
            .push(json!({"id":"execution","status":"failed","detail":error.to_string()}));
    }
    if let Err(error) = lab.cleanup() {
        lab.cases
            .push(json!({"id":"cleanup","status":"failed","detail":error.to_string()}));
    }
    let failed = lab.cases.iter().any(|c| c["status"] == "failed");
    let mut evidence_types = vec!["topology-control"];
    for (component, kind) in [
        ("runtime", "runtime-process"),
        ("wargame", "supervisor-owned-runtime-child-http1"),
        ("independent_witness", "independent-witness-component"),
        ("collector", "collector-component"),
    ] {
        if lab.components[component].is_object() && lab.components[component]["status"] != "not_run"
        {
            evidence_types.push(kind);
        }
    }
    let report = json!({"schema":"humanymous.product-lab/2","project":lab.project,"evidence_types":evidence_types,"cases":lab.cases,"components":lab.components,"required_unavailable":["full Gate TLS/H2/capture","session-bound real browser collector","Pass recovery","human/keyboard/assistive controls","runtime binary authenticated witness transport","cross-namespace origin/audit effect product binding","signed remote target attestation","native OS qualification","provider deployment"],"full_product_qualified":false,"release_ready":false,"ttl":{"expires_unix":expires,"external_janitor_scheduled":false}});
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
    Ok(if failed {
        1
    } else if full {
        2
    } else {
        0
    })
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
