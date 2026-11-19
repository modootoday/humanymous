//! Roadmap unit 35: build, start, drive and tear down the real Rust Gate +
//! origin + attacker topology, then emit a source-bound receipt. Increment 2
//! adds shell-free health checks with `service_healthy` dependencies, an
//! origin-loss-after-readiness case, and a witness-required Gate whose audit
//! checkpoints are counter-signed by an independent witness container, with a
//! witness-loss-after-readiness case. Increment 3 adds a real browser that
//! runs the Rust WASM collector against a witness-required Gate from its own
//! network namespace, and turns the witnessed Gate probe into a readiness
//! probe over the runtime's serving-epoch latch. Increment 5 runs the owned
//! attack catalog in a network-none wargame container and emits the
//! required-case ledger through the shared `qualification::assess` derivation,
//! mapping only the required cases this run actually evaluates.
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

mod browser;
mod ledger;
mod witnessed;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const ENDPOINT: &str = "unix:///var/run/docker.sock";
const IMAGE: &str = "humanymous/product-e2e:local";
const PROJECT: &str = "humanymous-lab-product-e2e";
const GATE_CONTAINER: &str = "humanymous-lab-product-e2e-gate-1";
const ORIGIN_CONTAINER: &str = "humanymous-lab-product-e2e-origin-1";
const COMPOSE: &str = "deployments/lab/compose.product-e2e.yaml";
const DOCKERFILE: &str = "build/rust/product-e2e.Dockerfile";
const ATTACKER: &str = "humanymous-lab-product-e2e-attacker";
const ORIGIN_LOST_ATTACKER: &str = "humanymous-lab-product-e2e-attacker-origin-lost";

struct Options {
    emit: PathBuf,
    build_ca: Option<PathBuf>,
    build_proxy: Option<String>,
    keep_image: bool,
    /// Engine ID observed by the operator before the run; when given, the
    /// observed engine must match it.
    engine_id: Option<String>,
}

fn options(args: &[String]) -> Result<Options> {
    let mut venue = None;
    let mut emit = None;
    let mut build_ca = None;
    let mut build_proxy = None;
    let mut keep_image = false;
    let mut engine_id = None;
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--venue" => venue = rest.next().cloned(),
            "--emit" => emit = rest.next().map(PathBuf::from),
            "--build-ca" => build_ca = rest.next().map(PathBuf::from),
            "--build-proxy" => build_proxy = rest.next().cloned(),
            "--keep-image" => keep_image = true,
            "--engine-id" => {
                engine_id = Some(rest.next().cloned().ok_or("--engine-id value missing")?)
            }
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
    if let Some(id) = &engine_id {
        crate::docker::Venue::parse(Some("cloud-container"), Some(id))?;
    }
    if build_ca.as_ref().is_some_and(|path| !path.is_file()) {
        return Err("--build-ca must name an existing file".into());
    }
    // Build-only egress for venues whose containers reach the network only
    // through a host-loopback proxy; never applied to a lab container.
    if build_proxy.as_ref().is_some_and(|proxy| {
        !(proxy.starts_with("http://127.0.0.1:") || proxy.starts_with("http://[::1]:"))
    }) {
        return Err("--build-proxy must be a host-loopback http:// proxy".into());
    }
    Ok(Options {
        emit,
        build_ca,
        build_proxy,
        keep_image,
        engine_id,
    })
}

struct Output {
    success: bool,
    stdout: String,
    stderr: String,
}

struct Lab {
    root: PathBuf,
    transcript: Vec<Value>,
    /// Operator-supplied engine ID the observed engine must equal, if any.
    expected_engine: Option<String>,
}

impl Lab {
    /// Bounded direct invocation; the engine endpoint is always explicit.
    fn docker(&mut self, args: &[&str], seconds: u64, required: bool) -> Result<Output> {
        self.docker_input(args, b"", seconds, required)
    }

    /// As `docker`, with private stdin bytes that the transcript never records.
    fn docker_input(
        &mut self,
        args: &[&str],
        input: &[u8],
        seconds: u64,
        required: bool,
    ) -> Result<Output> {
        let mut command = Command::new("docker");
        if args.first() != Some(&"context") {
            command.args(["--host", ENDPOINT]);
        }
        command.args(args).current_dir(&self.root);
        self.capture(command, args, input, seconds, required)
    }

    fn git(&mut self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("git");
        command.args(args).current_dir(&self.root);
        Ok(self
            .capture(command, args, b"", 30, true)?
            .stdout
            .trim()
            .to_owned())
    }

    fn capture(
        &mut self,
        mut command: Command,
        args: &[&str],
        input: &[u8],
        seconds: u64,
        required: bool,
    ) -> Result<Output> {
        let started = Instant::now();
        let captured = capture_with_stderr(
            &mut command,
            input,
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
            "stdin_bytes":input.len(),"input_recorded":false,
            "seconds":started.elapsed().as_secs(),"stderr_tail":if success { Value::Null } else { json!(tail) }}));
        if required && !success {
            return Err(format!("{} failed: {:?}", args.join(" "), captured.status.code()).into());
        }
        Ok(Output {
            success,
            stdout: String::from_utf8(captured.stdout)?,
            stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
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
            "windows_desktop_affinity_claimed":false,"github_hosted":false,
            "explicit":true,"bound_engine_id":self.expected_engine,
            "authenticated_provenance":false,"qualifying_venue":false});
        if engine["os_type"] != "linux"
            || !engine["engine_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
            || selected != ENDPOINT
            || self
                .expected_engine
                .as_ref()
                .is_some_and(|expected| engine["engine_id"] != expected.as_str())
        {
            return Err(
                "cloud-container engine identity incomplete, not the local socket or not the supplied engine ID".into(),
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
            "exit_code":value["State"]["ExitCode"],"health":value["State"]["Health"]["Status"],
            "healthcheck_test":value["Config"]["Healthcheck"]["Test"],
            "depends_on":value["Config"]["Labels"]["com.docker.compose.depends_on"],
            "mounts":value["Mounts"]}))
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

/// Exec-form probes (never `CMD-SHELL`), observed healthy, and the compose
/// `service_healthy` dependency edges as recorded by the engine.
fn health_assertions(containers: &[Value]) -> Vec<(&'static str, bool)> {
    let exec_form = |c: &Value| {
        c["healthcheck_test"][0] == "CMD"
            && c["healthcheck_test"][1] == "/usr/local/bin/humanymous-runtime-pilot"
    };
    let depends = |c: &Value, edge: &str| {
        c["depends_on"]
            .as_str()
            .is_some_and(|text| text.split(',').any(|item| item.starts_with(edge)))
    };
    let [gate, origin, attacker] = containers else {
        return vec![("three_main_containers_observed", false)];
    };
    vec![
        (
            "gate_and_origin_exec_form_health_probe",
            exec_form(gate) && exec_form(origin),
        ),
        (
            "gate_and_origin_healthy_before_driver",
            gate["health"] == "healthy" && origin["health"] == "healthy",
        ),
        (
            "origin_depends_on_gate_service_healthy",
            depends(origin, "gate:service_healthy"),
        ),
        (
            "attacker_depends_on_gate_and_origin_service_healthy",
            depends(attacker, "gate:service_healthy")
                && depends(attacker, "origin:service_healthy"),
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
        expected_engine: options.engine_id.clone(),
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
    let mut receipt = json!({"schema":"humanymous.product-e2e-receipt/1","roadmap_unit":35,"increment":5,
        "status":"started","pass_product_clean_room":false,"product_complete":false,
        "label":"partial topology: gate + origin + attacker, a witness-required gate with an independent witness container, a real browser running the Rust WASM collector against a witnessed gate with an owned DNS canary, and the owned attack catalog in a network-none wargame; the other topologies replay the real collector report; not product e2e completion",
        "source":source,"venue":engine,
        "components":{
          "gate":{"present":true,"real":true,"binary":"humanymous-runtime (crates/sdk/humanymous-runtime, src/gateway/main.rs)",
                  "listener":"plaintext loopback HTTP/1.1 (ServiceHost::bind); the configuration has no tls section",
                  "embedded_durable_audit":"runtime-owned sealed WAL with synthetic owned custody on tmpfs; this topology configures no audit.witness_input, so the shipped binary starts writer-only"},
          "witnessed_gate":{"present":true,"real":"runtime library, lab composition root","composition":"humanymous-runtime-pilot --product-gate-witnessed: the humanymous-runtime library ServiceHost::bind_with_witness with require_external_witness=true",
                  "why_not_the_shipped_binary":"the shipped binary accepts a witness: crates/sdk/humanymous-runtime/src/gateway/main.rs pins inherited witness pipes (startup::inherited_witness, configured by audit.witness_input) before custody and selects ServiceHost::bind_tls, bind_with_witness or bind; this topology does not yet hand it inherited witness descriptors, so the witnessed gate is still the library composed by the lab supervisor"},
          "origin":{"present":true,"real":true,"binary":"humanymous-runtime-pilot --product-origin (Rust recording origin)"},
          "attacker":{"present":true,"real":true,"binary":"humanymous-runtime-pilot --product-driver (Rust HTTP/1.1 driver)"},
          "authority":{"present":false,"gate_can_use_remote_authority":false,
                  "reason":"the gateway decides in-process: crates/sdk/humanymous-runtime/src/gateway/service.rs builds Application::new with its own Engine and crates/sdk/humanymous-runtime/src/gateway/application.rs calls self.decide_request;the gateway has no authority client. humanymous-authority (src/authority/main.rs) serves adapter decisions over a restricted native IPC listener only (src/authority/transport/mod.rs:60, no TCP fallback)"},
          "audit":{"present":true,"scope":"witnessed topology only","binary":"humanymous-runtime-pilot --product-witness: audit::checkpoint::service::serve with its own sealed identity and head state",
                  "independence":"separate container, UID 65533, network none, own tmpfs custody and passphrase; the runner brokers typed frames over private stdio",
                  "not_qualified":"identity is first-contact pinned through the broker, the head pin is not persisted outside the witness, tmpfs gives no power-loss durability"},
          "browser":{"present":"see browser.pass and browser.actor","scope":"browser profile only",
                  "binary":"humanymous-runtime-pilot --browser-driver: Firefox ESR through geckodriver, driven by the Rust wasm-bindgen-test-runner 0.2.130",
                  "collector":"crates/collector built for wasm32-unknown-unknown with feature runtime-pilot; one fixture-selected test (crates/collector/src/browser/transport/runtime_pilot.rs) uses the real BrowserSession and BrowserCollector",
                  "gate":"humanymous-runtime-pilot --browser-gate: the runtime library ServiceHost::bind_with_witness with an owned lab frontend proxy on :8080; the gateway itself stays loopback-only",
                  "main_and_witnessed_drivers":"collect reports in the gate, origin-loss and witnessed topologies are the browser topology's captured real collector report, sessionId rebound per session; the drivers' HTTP request headers remain synthetic",
                  "dns_canary":"humanymous-runtime-pilot --product-dns-canary: a real UDP :53 listener on its own internal network, with positive controls from that network before and after the browser"}},
        "constraint":"Configuration::validate requires loopback listen and origin (crates/sdk/humanymous-runtime/src/gateway/configuration.rs) and the accept loop drops non-loopback peers (crates/sdk/humanymous-runtime/src/gateway/application.rs), so origin and attacker share the gate container network namespace; origin network isolation is therefore absent",
        "clean_room":false,
        "clean_room_gaps":["engine venue is a cloud container, not Windows desktop-linux or genuine GitHub-hosted Linux",
          "single shared loopback namespace for gate, origin and attacker; origin is not reachable only through a Gate backend network",
          "the authority role is absent; the Gate cannot use a remote authority",
          "the browser reaches the browser Gate through an owned lab proxy pair (browser-local and Gate frontend), not original client TLS/H2; the browser Gate is the runtime library, not the shipped binary",
          "the witnessed gate is the runtime library composed by a lab supervisor, not the shipped binary, although the shipped binary now accepts inherited witness pipes",
          "the shipped-binary gate runs its plaintext loopback listener; its TLS listener (ServiceHost::bind_tls, ALPN h2 and http/1.1) is not driven by this topology",
          "the shipped-binary gate probe is TCP liveness only: crates/sdk/humanymous-runtime/src/gateway/main.rs writes readiness once and exposes no live serving-epoch latch to its supervisor; only the witnessed gate probe reads ServiceHost::epoch_health",
          "the attack catalog runs against the shipped runtime as a supervisor-owned child in a network-none wargame container, not through the product topology gate",
          "readiness withdrawal is lazy: the serving-epoch latch fails on the first failed audit append after a witness loss, not when the witness stops"]});
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
    if options.build_ca.is_some() {
        receipt["build_egress_trust_anchor"] =
            json!("venue CA bundle supplied as a build-only secret for dependency fetch");
    }
    if options.build_proxy.is_some() {
        receipt["build_egress_proxy"] = json!(
            "venue host-loopback HTTPS proxy supplied as a build-only proxy argument; builds used the host network; lab containers never do"
        );
    }
    if !build_image(&mut lab, &options, &commit, None, IMAGE)? {
        receipt["status"] = json!("build_failed");
        return finish(&mut lab, receipt, &options, false);
    }
    receipt["build_seconds"] = json!(started.elapsed().as_secs());
    let started = Instant::now();
    let browser_built = build_image(
        &mut lab,
        &options,
        &commit,
        Some("browser"),
        browser::BROWSER_IMAGE,
    )?;
    receipt["browser_build_seconds"] = json!(started.elapsed().as_secs());
    receipt["browser_build_success"] = json!(browser_built);
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
    let browser_image = if browser_built {
        let id = lab
            .docker(
                &[
                    "image",
                    "inspect",
                    "--format",
                    "{{.Id}} {{.Size}}",
                    browser::BROWSER_IMAGE,
                ],
                60,
                true,
            )?
            .stdout;
        let mut parts = id.split_whitespace();
        let (id, size) = (parts.next().unwrap_or("").to_owned(), parts.next());
        receipt["browser_image"] = json!({"tag":browser::BROWSER_IMAGE,"id":id,
            "size_bytes":size.and_then(|s| s.parse::<u64>().ok())});
        Some(id)
    } else {
        None
    };
    // Increment 4: the browser topology runs first, because its real collector
    // report is the one every collected session in the other topologies submits.
    let (browser, report_line) = match &browser_image {
        Some(browser_image) => match browser::run(&mut lab, &image, browser_image) {
            Ok(result) => result,
            Err(error) => (json!({"pass":false,"error":error.to_string()}), None),
        },
        None => (
            json!({"pass":false,"error":"browser image build failed"}),
            None,
        ),
    };
    let browser_pass = browser["pass"] == true;
    receipt["components"]["browser"]["present"] =
        json!(browser["actor"]["test_exit_code"].is_number());
    receipt["browser"] = browser;
    receipt["collect_report_source"] = json!({"real_collector_report_captured":report_line.is_some(),
        "fixture_fallback":false,
        "note":"every collected session in the gate, origin-loss and witnessed drivers submits the browser topology's captured collector report with only sessionId rebound; without one those sessions fail"});
    let stdin_report = report_line.map(|line| line + "\n").unwrap_or_default();
    let compose = ["compose", "--file", COMPOSE, "--project-name", PROJECT];
    let up = [
        &compose[..],
        &[
            "up",
            "--detach",
            "--no-build",
            "--pull",
            "never",
            "--wait",
            "--wait-timeout",
            "180",
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
            "run", "--no-TTY", "--pull", "never", "--name", ATTACKER, "attacker",
        ],
    ]
    .concat();
    // Without --no-deps: compose itself enforces the service_healthy edges.
    let driven = lab.docker_input(&drive, stdin_report.as_bytes(), 600, false)?;
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
    let mut assertions = topology_assertions(&network, &containers, &image);
    assertions.extend(health_assertions(&containers));
    assertions.push((
        "driver_spoof_expectation_matches_gate_readiness",
        spoof_mode_agrees(&receipt["gate_readiness"], driver.as_ref()),
    ));
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
    let origin_lost = origin_loss(&mut lab, &compose, &stdin_report)?;
    let origin_lost_pass = origin_lost["pass"] == true;
    receipt["origin_loss_after_readiness"] = origin_lost;
    let witnessed = match witnessed::run(&mut lab, &image, &stdin_report) {
        Ok(report) => report,
        Err(error) => json!({"pass":false,"error":error.to_string()}),
    };
    let witnessed_pass = witnessed["pass"] == true;
    receipt["witnessed"] = witnessed;
    let catalog = match ledger::wargame(&mut lab, &compose) {
        Ok(report) => report,
        Err(error) => json!({"pass":false,"error":error.to_string()}),
    };
    let catalog_pass = catalog["pass"] == true;
    receipt["attack_catalog"] = catalog;
    receipt["status"] = json!(if driver_pass
        && topology_pass
        && origin_lost_pass
        && witnessed_pass
        && browser_pass
        && catalog_pass
    {
        "partial_topology_measured"
    } else {
        "partial_topology_failed"
    });
    finish(&mut lab, receipt, &options, true)
}

/// The driver's forged-header expectation must follow the Gate's own
/// reported spoof-header mode, not only the shared configuration document.
fn spoof_mode_agrees(readiness: &Value, driver: Option<&Value>) -> bool {
    let reported = &readiness["profile"]["edge"]["spoof_header"];
    let recorded = driver
        .and_then(|d| d["cases"].as_array())
        .and_then(|cases| {
            cases
                .iter()
                .find(|c| c["id"] == "forged_trust_headers_without_session")
        })
        .map(|case| &case["spoof_header_mode"]);
    reported.is_string() && recorded == Some(reported)
}

/// Stop the origin after readiness, then require the Gate to fail closed.
fn origin_loss(lab: &mut Lab, compose: &[&str], report: &str) -> Result<Value> {
    let stopped = lab
        .docker(&["stop", "--time", "2", ORIGIN_CONTAINER], 60, false)?
        .success;
    let origin = lab.inspect(ORIGIN_CONTAINER)?;
    let drive = [
        compose,
        &[
            "run",
            "--no-deps",
            "--no-TTY",
            "--pull",
            "never",
            "--name",
            ORIGIN_LOST_ATTACKER,
            "attacker",
            "--product-driver-origin-lost",
        ],
    ]
    .concat();
    let driven = lab.docker_input(&drive, report.as_bytes(), 300, false)?;
    let driver = last_json_line(
        &driven.stdout,
        "humanymous.product-e2e-origin-lost-driver/1",
    )
    .unwrap_or(Value::Null);
    let gate = lab.inspect(GATE_CONTAINER)?;
    let pass = stopped
        && origin["state"] == "exited"
        && driven.success
        && driver["failed"] == 0
        && driver["driver_error"].is_null();
    Ok(
        json!({"origin_stopped":stopped,"origin_state":origin["state"],
        "gate_state_after":gate["state"],"gate_health_after":gate["health"],
        "driver":driver,"driver_exit_success":driven.success,"pass":pass}),
    )
}

/// Build one target of the product e2e Dockerfile from the source tree.
fn build_image(
    lab: &mut Lab,
    options: &Options,
    commit: &str,
    target: Option<&str>,
    tag: &str,
) -> Result<bool> {
    let mut build: Vec<String> = [
        "build",
        "--file",
        DOCKERFILE,
        "--tag",
        tag,
        "--label",
        &format!("org.humanymous.source={commit}"),
    ]
    .map(str::to_owned)
    .to_vec();
    if let Some(target) = target {
        build.extend(["--target".to_owned(), target.to_owned()]);
    }
    if let Some(ca) = &options.build_ca {
        build.extend([
            "--secret".to_owned(),
            format!("id=egress_ca,src={}", ca.display()),
        ]);
    }
    if let Some(proxy) = &options.build_proxy {
        build.extend([
            "--network".to_owned(),
            "host".to_owned(),
            "--build-arg".to_owned(),
            format!("https_proxy={proxy}"),
        ]);
    }
    build.push(".".into());
    let build_args: Vec<&str> = build.iter().map(String::as_str).collect();
    Ok(lab.docker(&build_args, 3600, false)?.success)
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
            "--profile",
            "driver",
            "--profile",
            "witnessed",
            "--profile",
            "browser",
            "--profile",
            "wargame",
            "down",
            "--volumes",
            "--remove-orphans",
            "--timeout",
            "5",
        ];
        cleanup["compose_down"] = json!(lab.docker(&down, 120, false)?.success);
        let removed = lab
            .docker(
                &[
                    "rm",
                    "--force",
                    ATTACKER,
                    ORIGIN_LOST_ATTACKER,
                    browser::BROWSER,
                    browser::DNSPROBE_BEFORE,
                    browser::DNSPROBE_AFTER,
                    ledger::WARGAME,
                ],
                60,
                false,
            )?
            .success;
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
        cleanup["browser_image_removed"] = json!(
            lab.docker(&["image", "rm", browser::BROWSER_IMAGE], 120, false)?
                .success
        );
    }
    let after = lab.engine()?;
    let stable = after["engine_id"] == receipt["venue"]["engine_id"];
    receipt["venue"]["engine_id_stable"] = json!(stable);
    if !stable {
        receipt["status"] = json!("engine_identity_changed");
    }
    let cleanup_verified = stable && cleanup_verified(&cleanup, started, options.keep_image);
    receipt["qualification"] = ledger::ledger(&receipt, cleanup_verified);
    receipt["full_product_qualified"] = receipt["qualification"]["full_product_qualified"].clone();
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

/// Owned teardown observed complete: the project is down with no container
/// left, and both images are gone unless the operator kept them.
fn cleanup_verified(cleanup: &Value, started: bool, keep_image: bool) -> bool {
    let topology =
        !started || (cleanup["compose_down"] == true && cleanup["containers_remaining"] == 0);
    let images = keep_image
        || (cleanup["image_removed"] == true && cleanup["browser_image_removed"] == true);
    topology && images
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spoof_expectation_must_match_the_gate_readiness_mode() {
        let readiness = json!({"profile":{"edge":{"spoof_header":"enforce"}}});
        let driver = |mode: &str| json!({"cases":[{"id":"forged_trust_headers_without_session","spoof_header_mode":mode}]});
        assert!(spoof_mode_agrees(&readiness, Some(&driver("enforce"))));
        assert!(!spoof_mode_agrees(&readiness, Some(&driver("monitor"))));
        assert!(!spoof_mode_agrees(&readiness, None));
        assert!(!spoof_mode_agrees(&json!(null), Some(&driver("enforce"))));
        assert!(!spoof_mode_agrees(&readiness, Some(&json!({"cases":[]}))));
    }
    #[test]
    fn cleanup_is_verified_only_from_observed_teardown() {
        let done = json!({"compose_down":true,"containers_remaining":0,"image_removed":true,"browser_image_removed":true});
        assert!(cleanup_verified(&done, true, false));
        let mut left = done.clone();
        left["containers_remaining"] = json!(1);
        assert!(!cleanup_verified(&left, true, false));
        let mut kept = done.clone();
        kept["browser_image_removed"] = json!(false);
        assert!(!cleanup_verified(&kept, true, false));
        assert!(cleanup_verified(&kept, true, true));
        assert!(!cleanup_verified(&json!({}), true, true));
        assert!(cleanup_verified(&json!({}), false, true));
    }
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
    fn health_requires_exec_form_healthy_state_and_service_healthy_edges() {
        let probe = json!([
            "CMD",
            "/usr/local/bin/humanymous-runtime-pilot",
            "--product-health"
        ]);
        let mut good = vec![
            json!({"healthcheck_test":probe,"health":"healthy","depends_on":null}),
            json!({"healthcheck_test":probe,"health":"healthy","depends_on":"gate:service_healthy:false"}),
            json!({"depends_on":"gate:service_healthy:false,origin:service_healthy:false"}),
        ];
        assert!(health_assertions(&good).iter().all(|(_, p)| *p));
        good[0]["healthcheck_test"] = json!(["CMD-SHELL", "true"]);
        assert!(!health_assertions(&good)[0].1);
        good[0]["healthcheck_test"] = probe.clone();
        good[1]["health"] = json!("starting");
        assert!(!health_assertions(&good)[1].1);
        good[1]["depends_on"] = json!("gate:service_started:false");
        assert!(!health_assertions(&good)[2].1);
        good[2]["depends_on"] = json!("gate:service_healthy:false");
        assert!(!health_assertions(&good)[3].1);
        assert!(!health_assertions(&good[..2])[0].1);
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
        let with_id = |id: &str| {
            let mut bound = args.to_vec();
            bound.extend(["--engine-id".into(), id.into()]);
            options(&bound)
        };
        assert_eq!(
            with_id("c7f5a196-202c").unwrap().engine_id.as_deref(),
            Some("c7f5a196-202c")
        );
        assert!(with_id("").is_err());
        assert!(with_id("id with space").is_err());
    }
    #[test]
    fn build_proxy_is_host_loopback_only() {
        let fresh = std::env::temp_dir().join(format!("product-e2e-proxy-{}", std::process::id()));
        let with = |proxy: &str| {
            options(&[
                "--venue".into(),
                "cloud-container".into(),
                "--emit".into(),
                fresh.display().to_string(),
                "--build-proxy".into(),
                proxy.into(),
            ])
        };
        assert!(with("http://127.0.0.1:3128").is_ok());
        assert!(with("http://proxy.example:3128").is_err());
        assert!(with("https://127.0.0.1:3128").is_err());
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
