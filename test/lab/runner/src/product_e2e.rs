//! Roadmap unit 35, first increment: build, start, drive and tear down the
//! real Rust Gate + origin + attacker topology, then emit a source-bound receipt.
//!
//! Venue: an explicitly named cloud container whose own Linux engine listens on
//! the local Unix socket. It never claims Windows Docker Desktop affinity or a
//! GitHub-hosted run, and it never labels a partial topology product-complete.
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const ENDPOINT: &str = "unix:///var/run/docker.sock";
const IMAGE: &str = "humanymous/product-e2e:local";
const PROJECT: &str = "humanymous-lab-product-e2e";
const GATE_CONTAINER: &str = "humanymous-lab-product-e2e-gate-1";
const ORIGIN_CONTAINER: &str = "humanymous-lab-product-e2e-origin-1";
const COMPOSE: &str = "deployments/lab/compose.product-e2e.yaml";
const DOCKERFILE: &str = "build/rust/product-e2e.Dockerfile";
const ATTACKER: &str = "humanymous-lab-product-e2e-attacker";

struct Options {
    emit: PathBuf,
    build_ca: Option<PathBuf>,
    keep_image: bool,
}

fn options(args: &[String]) -> Result<Options> {
    let mut venue = None;
    let mut emit = None;
    let mut build_ca = None;
    let mut keep_image = false;
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--venue" => venue = rest.next().cloned(),
            "--emit" => emit = rest.next().map(PathBuf::from),
            "--build-ca" => build_ca = rest.next().map(PathBuf::from),
            "--keep-image" => keep_image = true,
            _ => return Err(format!("unknown product-e2e argument {flag}").into()),
        }
    }
    if venue.as_deref() != Some("cloud-container") {
        return Err("--venue cloud-container is required; other venues have their own engine affinity rules".into());
    }
    let emit = emit.ok_or("--emit PATH required")?;
    if emit.exists() {
        return Err("receipt path exists; preserve prior runs".into());
    }
    if build_ca.as_ref().is_some_and(|path| !path.is_file()) {
        return Err("--build-ca must name an existing file".into());
    }
    Ok(Options {
        emit,
        build_ca,
        keep_image,
    })
}

struct Output {
    success: bool,
    stdout: String,
}

struct Lab {
    root: PathBuf,
    transcript: Vec<Value>,
}

impl Lab {
    /// Bounded direct invocation; the engine endpoint is always explicit.
    fn docker(&mut self, args: &[&str], seconds: u64, required: bool) -> Result<Output> {
        let mut command = Command::new("docker");
        if args.first() != Some(&"context") {
            command.args(["--host", ENDPOINT]);
        }
        command.args(args).current_dir(&self.root);
        self.capture(command, args, seconds, required)
    }

    fn git(&mut self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("git");
        command.args(args).current_dir(&self.root);
        Ok(self
            .capture(command, args, 30, true)?
            .stdout
            .trim()
            .to_owned())
    }

    fn capture(
        &mut self,
        mut command: Command,
        args: &[&str],
        seconds: u64,
        required: bool,
    ) -> Result<Output> {
        let started = Instant::now();
        let captured = capture_with_stderr(
            &mut command,
            b"",
            Duration::from_secs(seconds),
            16 * 1024 * 1024,
        )?;
        let stderr = String::from_utf8_lossy(&captured.stderr);
        let tail: String = stderr
            .chars()
            .rev()
            .take(1500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let success = captured.status.success();
        self.transcript.push(json!({"argv":args,"exit":captured.status.code(),
            "seconds":started.elapsed().as_secs(),"stderr_tail":if success { Value::Null } else { json!(tail) }}));
        if required && !success {
            return Err(format!("{} failed: {:?}", args.join(" "), captured.status.code()).into());
        }
        Ok(Output {
            success,
            stdout: String::from_utf8(captured.stdout)?,
        })
    }

    fn json(&mut self, args: &[&str]) -> Result<Value> {
        Ok(serde_json::from_str(&self.docker(args, 60, true)?.stdout)?)
    }

    fn engine(&mut self) -> Result<Value> {
        let version = self.json(&["version", "--format", "{{json .}}"])?;
        let context = self
            .docker(&["context", "show"], 30, true)?
            .stdout
            .trim()
            .to_owned();
        let selected = self.json(&[
            "context",
            "inspect",
            &context,
            "--format",
            "{{json .Endpoints.docker.Host}}",
        ])?;
        let info = self.json(&["info", "--format", "{{json .}}"])?;
        let engine = json!({"kind":"cloud-container","endpoint":ENDPOINT,
            "selected_context":context,"selected_context_endpoint":selected,
            "client_version":version["Client"]["Version"],"server_version":version["Server"]["Version"],
            "server_api_version":version["Server"]["ApiVersion"],
            "engine_id":info["ID"],"os_type":info["OSType"],"info_server_version":info["ServerVersion"],
            "windows_desktop_affinity_claimed":false,"github_hosted":false});
        if engine["os_type"] != "linux"
            || !engine["engine_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
            || selected != ENDPOINT
        {
            return Err(
                "cloud-container engine identity incomplete or not the local socket".into(),
            );
        }
        Ok(engine)
    }

    fn inspect(&mut self, name: &str) -> Result<Value> {
        let value = self.json(&["inspect", "--format", "{{json .}}", name])?;
        Ok(json!({"name":name,"id":value["Id"],"image":value["Image"],
            "network_mode":value["HostConfig"]["NetworkMode"],
            "port_bindings":value["HostConfig"]["PortBindings"],
            "published_ports":value["NetworkSettings"]["Ports"],
            "read_only_rootfs":value["HostConfig"]["ReadonlyRootfs"],
            "cap_drop":value["HostConfig"]["CapDrop"],"user":value["Config"]["User"],
            "init":value["HostConfig"]["Init"],"state":value["State"]["Status"],
            "exit_code":value["State"]["ExitCode"]}))
    }
}

fn empty(value: &Value) -> bool {
    value.is_null() || value.as_object().is_some_and(serde_json::Map::is_empty)
}

/// Pure topology judgement over observed engine metadata.
fn topology_assertions(
    network: &Value,
    containers: &[Value],
    image: &str,
) -> Vec<(&'static str, bool)> {
    let gate = containers
        .first()
        .and_then(|c| c["id"].as_str())
        .unwrap_or("");
    let shared = format!("container:{gate}");
    vec![
        ("network_internal_no_egress", network["Internal"] == true),
        (
            "no_published_ports",
            containers
                .iter()
                .all(|c| empty(&c["port_bindings"]) && empty(&c["published_ports"])),
        ),
        (
            "origin_and_attacker_share_gate_namespace",
            !gate.is_empty()
                && containers
                    .iter()
                    .skip(1)
                    .all(|c| c["network_mode"] == shared.as_str()),
        ),
        (
            "all_containers_run_built_image",
            containers.iter().all(|c| c["image"] == image),
        ),
        (
            "hardened_sandbox",
            containers.iter().all(|c| {
                c["read_only_rootfs"] == true && c["user"] == "65532:65532" && c["init"] == true
            }),
        ),
    ]
}

fn last_json_line(text: &str, schema: &str) -> Option<Value> {
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .find(|value| value["schema"] == schema)
}

/// Run the measured partial topology. Teardown always runs after `up`.
pub fn run(args: &[String]) -> Result<i32> {
    if cfg!(windows) || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true") {
        return Err(
            "product-e2e cloud-container venue only; other venues are not this increment".into(),
        );
    }
    for name in ["DOCKER_HOST", "DOCKER_CONTEXT"] {
        if std::env::var(name).is_ok_and(|value| !value.is_empty() && value != ENDPOINT) {
            return Err(format!("{name} override refused; the endpoint is explicit").into());
        }
    }
    let options = options(args)?;
    let mut lab = Lab {
        root: crate::snapshot::root()?,
        transcript: Vec::new(),
    };
    let commit = lab.git(&["rev-parse", "HEAD"])?;
    let branch = lab.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let dirty = lab
        .git(&["status", "--porcelain", "--untracked-files=all"])?
        .lines()
        .count();
    let source = json!({"commit":commit,"branch":branch,"tree_clean":dirty==0,"dirty_entries":dirty,
        "dockerfile":DOCKERFILE,"compose":COMPOSE});
    let engine = lab.engine()?;
    let mut receipt = json!({"schema":"humanymous.product-e2e-receipt/1","roadmap_unit":35,"increment":1,
        "status":"started","pass_product_clean_room":false,"product_complete":false,
        "label":"partial topology: gate + origin + attacker; not product e2e completion",
        "source":source,"venue":engine,
        "components":{
          "gate":{"present":true,"real":true,"binary":"humanymous-runtime (crates/sdk/humanymous-runtime, src/gateway/main.rs)",
                  "embedded_durable_audit":"runtime-owned sealed WAL with synthetic owned custody on tmpfs; no external witness"},
          "origin":{"present":true,"real":true,"binary":"humanymous-runtime-pilot --product-origin (Rust recording origin)"},
          "attacker":{"present":true,"real":true,"binary":"humanymous-runtime-pilot --product-driver (Rust HTTP/1.1 driver)"},
          "authority":{"present":false,"note":"humanymous-authority service not deployed in this increment"},
          "audit":{"present":false,"note":"no independent audit service or witness; only the gate's embedded WAL ran"},
          "browser":{"present":false,"note":"no browser or WASM collector; collect reports are client-asserted fixtures"}},
        "constraint":"Configuration::validate requires loopback listen and origin and the accept loop drops non-loopback peers, so origin and attacker share the gate container network namespace; origin network isolation is therefore absent",
        "clean_room":false,
        "clean_room_gaps":["engine venue is a cloud container, not Windows desktop-linux or genuine GitHub-hosted Linux",
          "single shared loopback namespace for gate, origin and attacker; origin is not reachable only through a Gate backend network",
          "authority, independent audit witness and browser roles are absent"]});
    let config = lab.docker(
        &[
            "compose",
            "--file",
            COMPOSE,
            "--project-name",
            PROJECT,
            "config",
        ],
        60,
        true,
    )?;
    receipt["source"]["resolved_compose_sha256"] =
        json!(crate::snapshot::digest(config.stdout.as_bytes()));
    let started = Instant::now();
    let mut build: Vec<String> = [
        "build",
        "--file",
        DOCKERFILE,
        "--tag",
        IMAGE,
        "--label",
        &format!("org.humanymous.source={commit}"),
    ]
    .map(str::to_owned)
    .to_vec();
    if let Some(ca) = &options.build_ca {
        build.extend([
            "--secret".to_owned(),
            format!("id=egress_ca,src={}", ca.display()),
        ]);
        receipt["build_egress_trust_anchor"] =
            json!("venue CA bundle supplied as a build-only secret for dependency fetch");
    }
    build.push(".".into());
    let build_args: Vec<&str> = build.iter().map(String::as_str).collect();
    if lab.docker(&build_args, 3600, false)?.success {
        receipt["build_seconds"] = json!(started.elapsed().as_secs());
    } else {
        receipt["status"] = json!("build_failed");
        return finish(&mut lab, receipt, &options, false);
    }
    let image = lab
        .docker(
            &["image", "inspect", "--format", "{{.Id}}", IMAGE],
            60,
            true,
        )?
        .stdout
        .trim()
        .to_owned();
    let size = lab
        .docker(
            &["image", "inspect", "--format", "{{.Size}}", IMAGE],
            60,
            true,
        )?
        .stdout
        .trim()
        .to_owned();
    receipt["image"] = json!({"tag":IMAGE,"id":image,"size_bytes":size.parse::<u64>().ok()});
    let compose = ["compose", "--file", COMPOSE, "--project-name", PROJECT];
    let up = [
        &compose[..],
        &[
            "up",
            "--detach",
            "--no-build",
            "--pull",
            "never",
            "gate",
            "origin",
        ],
    ]
    .concat();
    if !lab.docker(&up, 300, false)?.success {
        receipt["status"] = json!("topology_start_failed");
        return finish(&mut lab, receipt, &options, true);
    }
    let drive = [
        &compose[..],
        &[
            "run",
            "--no-deps",
            "--no-TTY",
            "--pull",
            "never",
            "--name",
            ATTACKER,
            "attacker",
        ],
    ]
    .concat();
    let driven = lab.docker(&drive, 600, false)?;
    let driver = last_json_line(&driven.stdout, "humanymous.product-e2e-driver/1");
    let gate_log = lab.docker(&["logs", GATE_CONTAINER], 60, false)?;
    receipt["gate_readiness"] =
        last_json_line(&gate_log.stdout, "humanymous.product-e2e-gate-ready/1")
            .map_or(Value::Null, |value| value["readiness"].clone());
    let mut containers = Vec::new();
    for name in [GATE_CONTAINER, ORIGIN_CONTAINER, ATTACKER] {
        containers.push(lab.inspect(name)?);
    }
    let network = lab.json(&[
        "network",
        "inspect",
        "--format",
        "{{json .}}",
        &format!("{PROJECT}_product"),
    ])?;
    let assertions = topology_assertions(&network, &containers, &image);
    let topology_pass = assertions.iter().all(|(_, pass)| *pass);
    receipt["topology"] = json!({"project":PROJECT,"network":{"name":network["Name"],"internal":network["Internal"],"driver":network["Driver"]},
        "containers":containers,
        "assertions":assertions.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>()});
    let driver_pass = driven.success
        && driver
            .as_ref()
            .is_some_and(|d| d["failed"] == 0 && d["driver_error"].is_null());
    receipt["driver"] = driver.unwrap_or(Value::Null);
    receipt["driver_exit_success"] = json!(driven.success);
    receipt["status"] = json!(if driver_pass && topology_pass {
        "partial_topology_measured"
    } else {
        "partial_topology_failed"
    });
    finish(&mut lab, receipt, &options, true)
}

fn finish(lab: &mut Lab, mut receipt: Value, options: &Options, started: bool) -> Result<i32> {
    let mut cleanup = json!({});
    if started {
        let down = [
            "compose",
            "--file",
            COMPOSE,
            "--project-name",
            PROJECT,
            "down",
            "--volumes",
            "--remove-orphans",
            "--timeout",
            "5",
        ];
        cleanup["compose_down"] = json!(lab.docker(&down, 120, false)?.success);
        let removed = lab.docker(&["rm", "--force", ATTACKER], 60, false)?.success;
        cleanup["attacker_container_removed"] = json!(removed);
        let left = lab.docker(
            &[
                "ps",
                "--all",
                "--quiet",
                "--filter",
                &format!("label=com.docker.compose.project={PROJECT}"),
            ],
            60,
            false,
        )?;
        cleanup["containers_remaining"] = json!(left.stdout.lines().count());
    }
    if !options.keep_image {
        cleanup["image_removed"] = json!(lab.docker(&["image", "rm", IMAGE], 120, false)?.success);
    }
    let after = lab.engine()?;
    let stable = after["engine_id"] == receipt["venue"]["engine_id"];
    receipt["venue"]["engine_id_stable"] = json!(stable);
    if !stable {
        receipt["status"] = json!("engine_identity_changed");
    }
    receipt["cleanup"] = cleanup;
    receipt["transcript"] = json!(lab.transcript);
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    std::fs::write(&options.emit, &bytes)?;
    println!("{}", String::from_utf8(bytes)?);
    Ok(if receipt["status"] == "partial_topology_measured" {
        0
    } else {
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn container(id: &str, mode: &str) -> Value {
        json!({"id":id,"image":"sha256:i","network_mode":mode,"port_bindings":{},"published_ports":{},
            "read_only_rootfs":true,"user":"65532:65532","init":true})
    }
    #[test]
    fn topology_requires_internal_network_shared_namespace_and_no_ports() {
        let good = vec![
            container("g", "humanymous-lab-product-e2e_product"),
            container("o", "container:g"),
            container("a", "container:g"),
        ];
        let internal = json!({"Internal":true});
        assert!(
            topology_assertions(&internal, &good, "sha256:i")
                .iter()
                .all(|(_, p)| *p)
        );
        let external = json!({"Internal":false});
        assert!(!topology_assertions(&external, &good, "sha256:i")[0].1);
        let mut published = good.clone();
        published[1]["published_ports"] = json!({"18080/tcp":[{"HostPort":"18080"}]});
        assert!(!topology_assertions(&internal, &published, "sha256:i")[1].1);
        let mut detached = good.clone();
        detached[2]["network_mode"] = json!("bridge");
        assert!(!topology_assertions(&internal, &detached, "sha256:i")[2].1);
        assert!(!topology_assertions(&internal, &good, "sha256:other")[3].1);
    }
    #[test]
    fn venue_is_explicit_and_receipts_are_never_overwritten() {
        assert!(options(&[]).is_err());
        let args = ["--venue", "windows-desktop", "--emit", "x"].map(str::to_owned);
        assert!(options(&args).is_err());
        let existing = std::env::current_exe().unwrap();
        let args = [
            "--venue".into(),
            "cloud-container".into(),
            "--emit".into(),
            existing.display().to_string(),
        ];
        assert!(options(&args).is_err());
        let fresh = std::env::temp_dir().join(format!("product-e2e-absent-{}", std::process::id()));
        let args = [
            "--venue".into(),
            "cloud-container".into(),
            "--emit".into(),
            fresh.display().to_string(),
        ];
        assert!(options(&args).is_ok());
    }
    #[test]
    fn driver_receipt_is_found_by_schema_not_position() {
        let text =
            "noise\n{\"schema\":\"humanymous.product-e2e-driver/1\",\"failed\":0}\n{\"other\":1}\n";
        let found = last_json_line(text, "humanymous.product-e2e-driver/1").unwrap();
        assert_eq!(found["failed"], 0);
        assert!(last_json_line("{}", "x").is_none());
    }
}
