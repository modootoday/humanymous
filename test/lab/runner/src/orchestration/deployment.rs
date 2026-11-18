//! Roadmap 4 items 2h and 2i: the shipped adopter manifest brings up the real
//! runtime, and the documented deployment examples work as documented.
//!
//! The runner executes `deployments/compose.release.yaml` (Go-parity compose
//! manifest) with only `deployments/lab/compose.deployment.yaml` on top, using
//! the manifest's documented commands: `docker network create` for its
//! external origin network, `up -d edge`, then `run --rm -T runtime <
//! runtime.json`. The layer may change only the runtime's image (built from
//! this source in the published image's shape), build, pull policy and labels;
//! that is checked against `compose config` with and without the layer. The
//! edge is the manifest's adopter placeholder (lab L4 relay, no TLS); custody
//! provisioning is the documented adopter step with no shipped command.
//!
//! Two required cases come out of one ledger:
//! - `deployment`: manifest binding, the runtime container as actually run
//!   (manifest user/hardening/namespace/volume, no argv, source-built image),
//!   the documented routes through the edge from an ingress-only client with
//!   independently counted origin contacts, the authenticated audit history on
//!   the manifest volume across a `run --rm` restart, and lab-scoped target
//!   attestation: the deployed image generates its installation key inside
//!   the volume and a network-none challenger pins it and checks fresh nonces.
//! - `documentation-examples`: the example configuration exactly as shipped
//!   (placeholder refused), with only the documented passphrase edit (ready on
//!   the documented listener, every documented route enforced), the
//!   Kubernetes Secret stub's embedded configuration identical to it, and the
//!   reviewed documentation digest bound into the evidence.
//!
//! Go parity (981d25c): Go shipped compose, Kubernetes and systemd manifests
//! and `internal/attestation` as a local HMAC demo; no live provider. The
//! Kubernetes and systemd manifests are not executed here (no cluster and no
//! systemd PID 1 in the hardened lab); the runtime's gateway test checks them.
use super::{
    Lab,
    origin_audit::{Lines, contacts, next, passphrase, reader},
};
use crate::{docker::Result, snapshot};
use humanymous_platform::testing::{json::decode_strict_json, process::ManagedChild};
use serde_json::{Value, json};
use std::{
    io::Write,
    process::ChildStdin,
    time::{Duration, Instant},
};

const RELEASE: &str = "deployments/compose.release.yaml";
const LAYER: &str = "deployments/lab/compose.deployment.yaml";
const EXAMPLE: &str = "deployments/humanymous-runtime.example.json";
const SECRET: &str = "deployments/k8s/secret.example.yaml";
const REVIEWS: &str = "docs/reviews.json";
/// The only edit the manifest documents for the example (step 2).
const PLACEHOLDER: &str = "REPLACE-ME-with-the-custody-unseal-passphrase";
const MANIFEST_IMAGE: &str = "ghcr.io/modootoday/humanymous-runtime:";
/// Runtime keys the lab layer may change; anything else fails the binding.
const RUNTIME_LAYER_KEYS: &[&str] = &["build", "image", "labels", "pull_policy"];
const REQUEST_SCHEMA: &str = "humanymous.target-attestation-request/1";
const ENROLLMENT_SCHEMA: &str = "humanymous.target-attestation-enrollment/1";
const INSTALLATION: &str = "/var/lib/humanymous/installation";
const FOREIGN: &str = "/var/lib/humanymous/foreign-installation";
const STATE: &str = "/var/lib/humanymous";
const CLIENT_SCHEMA: &str = "humanymous.deployment-client/1";
const CHALLENGER_SCHEMA: &str = "humanymous.deployment-attestation/1";
const PHASES: &[&str] = &["first", "restarted"];
/// Challenger rows the runner relays, in the challenger's order.
const CHALLENGE_ROWS: &[&str] = &[
    "deployed-answer-verified",
    "stale-answer-refused",
    "fresh-answer-after-stale-verified",
    "foreign-installation-key-refused",
    "changed-configuration-refused",
];
const CHALLENGER_CASES: &[&str] = &[
    "enrollment-binds-deployed-configuration",
    "deployed-answer-verified",
    "stale-answer-refused",
    "fresh-answer-after-stale-verified",
    "tampered-signature-refused",
    "foreign-installation-key-refused",
    "changed-configuration-refused",
];
/// Services the deployment run may leave as one-off containers on failure.
const ONE_OFF: &[&str] = &["runtime", "dep-client", "dep-custody", "dep-challenger"];

struct Deployment {
    project: String,
    release: String,
    layer: String,
    environment: Vec<(String, String)>,
    network: String,
    expires: String,
}
impl Deployment {
    fn new(lab: &Lab) -> Result<Self> {
        let value = |name: &str| -> Result<String> {
            Ok(lab
                .environment
                .iter()
                .find(|(key, _)| key == name)
                .ok_or("owned lab environment missing")?
                .1
                .clone())
        };
        let tag = value("HM_LAB_IMAGE_TAG")?;
        let network = format!("{}-origin", lab.project);
        let mut environment = lab.environment.clone();
        // The manifest's own adopter variables (deployments/.env.example).
        environment.push((
            "HMN_EDGE_IMAGE".into(),
            format!("humanymous/product-lab:{tag}"),
        ));
        environment.push(("HMN_ORIGIN_NET".into(), network.clone()));
        Ok(Self {
            project: lab.project.clone(),
            release: lab.config_file(RELEASE)?,
            layer: lab.config_file(LAYER)?,
            environment,
            network,
            expires: value("HM_LAB_EXPIRES")?,
        })
    }
    fn args(&self, layered: bool, rest: &[&str]) -> Vec<String> {
        let mut args = vec![
            "compose".to_owned(),
            "-p".into(),
            self.project.clone(),
            "-f".into(),
            self.release.clone(),
        ];
        if layered {
            args.extend(["-f".to_owned(), self.layer.clone()]);
        }
        args.extend(rest.iter().map(|s| (*s).to_owned()));
        args
    }
    fn compose(&self, lab: &mut Lab, rest: &[&str]) -> Result<crate::docker::Output> {
        lab.docker
            .invoke(&self.args(true, rest), &self.environment, true)
    }
    /// One-shot `compose run --rm -T SERVICE [ARG]` fed private stdin bytes.
    fn run(&self, lab: &mut Lab, service: &[&str], input: &[u8]) -> Result<crate::docker::Output> {
        let mut rest = vec!["run", "--rm", "-T"];
        rest.extend(service);
        lab.docker
            .invoke_input(&self.args(true, &rest), &self.environment, false, input)
    }
    fn config(&self, lab: &mut Lab, layered: bool) -> Result<Value> {
        let output = lab.docker.invoke(
            &self.args(layered, &["config", "--format", "json"]),
            &self.environment,
            true,
        )?;
        Ok(decode_strict_json(output.stdout.as_bytes(), 1 << 20)?)
    }
    fn container(&self, lab: &mut Lab, service: &str) -> Result<String> {
        let id = self
            .compose(lab, &["ps", "-q", service])?
            .stdout
            .trim()
            .to_owned();
        if id.is_empty() || id.contains('\n') {
            return Err("exactly one deployment container expected".into());
        }
        Ok(id)
    }
    fn one_off(&self, lab: &mut Lab, service: &str) -> Result<Vec<String>> {
        let project = format!("label=com.docker.compose.project={}", self.project);
        let service = format!("label=com.docker.compose.service={service}");
        Ok(lab
            .docker
            .call(&["ps", "-aq", "--filter", &project, "--filter", &service])?
            .lines()
            .map(str::to_owned)
            .collect())
    }
    /// Remove every resource this stage created; failures stay visible.
    fn teardown(&self, lab: &mut Lab) -> Result<bool> {
        let mut clean = true;
        for service in ONE_OFF {
            for id in self.one_off(lab, service)? {
                clean &= lab
                    .docker
                    .invoke(&["rm".into(), "-f".into(), id], &[], false)?
                    .success;
            }
        }
        // No --remove-orphans: the rest of the lab project is not ours.
        clean &= lab
            .docker
            .invoke(
                &self.args(true, &["down", "--volumes", "--timeout", "2"]),
                &self.environment,
                false,
            )?
            .success;
        let present = lab.docker.call(&[
            "network",
            "ls",
            "-q",
            "--filter",
            &format!("name=^{}$", self.network),
        ])?;
        if !present.trim().is_empty() {
            clean &= lab
                .docker
                .invoke(
                    &["network".into(), "rm".into(), self.network.clone()],
                    &[],
                    false,
                )?
                .success;
        }
        Ok(clean)
    }
}

/// One serving runtime started with the documented `run --rm -T runtime`.
struct Serving {
    child: ManagedChild,
    lines: Lines,
}

fn last_line(output: &crate::docker::Output) -> Result<Value> {
    let line = output
        .stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or("actor produced no report line")?;
    Ok(decode_strict_json(line.as_bytes(), 1 << 20)?)
}

fn sha256(bytes: &[u8]) -> String {
    snapshot::digest(bytes)
}

/// The `runtime.json: |` block scalar of the Kubernetes Secret stub.
fn secret_document(text: &str) -> Option<String> {
    let mut lines = text.lines();
    lines
        .by_ref()
        .find(|line| line.trim_end() == "  runtime.json: |")?;
    let mut document = String::new();
    for line in lines {
        if !line.is_empty() && !line.starts_with("    ") {
            break;
        }
        document.push_str(line.strip_prefix("    ").unwrap_or(line));
        document.push('\n');
    }
    Some(document)
}

/// Sorted service keys whose values differ between two `compose config` runs.
fn changed_keys(pure: &Value, layered: &Value, service: &str) -> Vec<String> {
    let empty = serde_json::Map::new();
    let a = pure["services"][service].as_object().unwrap_or(&empty);
    let b = layered["services"][service].as_object().unwrap_or(&empty);
    let mut keys: Vec<_> = a
        .keys()
        .chain(b.keys())
        .filter(|key| a.get(*key) != b.get(*key))
        .cloned()
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn address(container: &Value, network: &str) -> Option<String> {
    container["NetworkSettings"]["Networks"][network]["IPAddress"]
        .as_str()
        .filter(|ip| !ip.is_empty())
        .map(str::to_owned)
}

fn inspect(lab: &mut Lab, id: &str) -> Result<Value> {
    let state: Value = serde_json::from_str(&lab.docker.call(&["inspect", id])?)?;
    Ok(state[0].clone())
}

/// Facts of the runtime container as Docker actually ran it.
fn runtime_facts(container: &Value, edge: &str, volume: &str) -> Value {
    let host = &container["HostConfig"];
    json!({
        "user":container["Config"]["User"],
        "entrypoint":container["Config"]["Entrypoint"],
        "cmd":container["Config"]["Cmd"],
        "image":container["Image"],
        "read_only":host["ReadonlyRootfs"],
        "privileged":host["Privileged"],
        "cap_drop":host["CapDrop"],
        "security_opt":host["SecurityOpt"],
        "pids_limit":host["PidsLimit"],
        "tmpfs":host["Tmpfs"],
        "network_mode_is_edge":host["NetworkMode"] == format!("container:{edge}"),
        "port_bindings":host["PortBindings"],
        "mounts":container["Mounts"].as_array().map(|mounts| mounts.iter().map(|m| json!({
            "type":m["Type"],"name":m["Name"],"destination":m["Destination"],"rw":m["RW"]})).collect::<Vec<_>>()),
        "volume":volume,
    })
}

fn serve(lab: &mut Lab, deployment: &Deployment, document: &[u8]) -> Result<(Serving, Value)> {
    let (child, mut input, output) = lab.docker.duplex_with(
        &deployment.args(true, &["run", "--rm", "-T", "runtime"]),
        &deployment.environment,
    )?;
    input.write_all(document)?;
    drop(input);
    let serving = Serving {
        child,
        lines: reader(output),
    };
    let readiness = next(&serving.lines, 180)?;
    Ok((serving, readiness))
}

fn stop(lab: &mut Lab, deployment: &Deployment, mut serving: Serving) -> Result<bool> {
    let mut killed = true;
    for id in deployment.one_off(lab, "runtime")? {
        killed &= lab.docker.invoke(&["kill".into(), id], &[], false)?.success;
    }
    serving.child.wait(Duration::from_secs(60))?;
    Ok(killed)
}

fn attest(lab: &mut Lab, deployment: &Deployment, mode: &str, request: &Value) -> Result<Value> {
    let output = deployment.run(lab, &["runtime", mode], &serde_json::to_vec(request)?)?;
    let refusal = output
        .stderr
        .lines()
        .filter_map(|line| line.trim().strip_prefix("humanymous-runtime: "))
        .next_back()
        .unwrap_or("")
        .to_owned();
    let document = if output.success {
        last_line(&output).ok()
    } else {
        None
    };
    Ok(
        json!({"success":output.success,"stdout_empty":output.stdout.trim().is_empty(),
        "refusal":refusal,"document":document}),
    )
}

fn enrollment(lab: &mut Lab, deployment: &Deployment, directory: &str) -> Result<Value> {
    attest(
        lab,
        deployment,
        "attestation-provision",
        &json!({"schema":REQUEST_SCHEMA,"installation_directory":directory}),
    )
}

fn answer(
    lab: &mut Lab,
    deployment: &Deployment,
    directory: &str,
    installation: &str,
    nonce: &str,
) -> Result<Value> {
    attest(
        lab,
        deployment,
        "attestation-answer",
        &json!({"schema":REQUEST_SCHEMA,"installation_directory":directory,
            "installation":installation,"nonce":nonce}),
    )
}

fn send(input: &mut ChildStdin, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *input, value)?;
    input.write_all(b"\n")?;
    Ok(input.flush()?)
}

/// Relay the challenger's nonces to the deployed installation and its answers
/// back. The runner never verifies; it only carries public documents.
fn challenge(
    lab: &mut Lab,
    deployment: &Deployment,
    enrolled: &Value,
    installation: &str,
    configuration_sha256: &str,
) -> Result<Value> {
    let (mut child, mut input, output) = lab.docker.duplex_with(
        &deployment.args(true, &["run", "--rm", "-T", "dep-challenger"]),
        &deployment.environment,
    )?;
    let lines = reader(output);
    send(
        &mut input,
        &json!({"enrollment":enrolled,"installation":installation,
            "configuration_sha256":configuration_sha256}),
    )?;
    let mut relayed = Vec::new();
    let mut previous = Value::Null;
    for expected in CHALLENGE_ROWS {
        let challenge = next(&lines, 120)?;
        if challenge["role"] != "challenge" || challenge["row"] != *expected {
            return Err("challenger row order differs from the relay contract".into());
        }
        let nonce = challenge["nonce"]
            .as_str()
            .ok_or("challenge nonce")?
            .to_owned();
        let reply = match *expected {
            "stale-answer-refused" => {
                json!({"success":true,"document":previous.clone(),"relayed":"previous answer"})
            }
            "foreign-installation-key-refused" => {
                answer(lab, deployment, FOREIGN, installation, &nonce)?
            }
            "changed-configuration-refused" => {
                let changed = deployment.run(
                    lab,
                    &["dep-custody"],
                    format!("{}\n", json!({"action":"change"})).as_bytes(),
                )?;
                let mut reply = answer(lab, deployment, INSTALLATION, installation, &nonce)?;
                reply["change"] = last_line(&changed).unwrap_or(Value::Null);
                reply
            }
            _ => answer(lab, deployment, INSTALLATION, installation, &nonce)?,
        };
        if *expected == "deployed-answer-verified" {
            previous = reply["document"].clone();
        }
        send(&mut input, &json!({"answer":reply["document"]}))?;
        relayed.push(json!({"row":expected,"target":reply}));
    }
    let report = next(&lines, 120)?;
    drop(input);
    let exited = child.wait(Duration::from_secs(60))?.success();
    Ok(json!({"report":report,"relayed":relayed,"process_success":exited}))
}

struct Inputs {
    example: String,
    edited: String,
    secret_matches_example: bool,
    placeholder_occurrences: usize,
    digests: Value,
}

fn inputs(lab: &Lab) -> Result<Inputs> {
    let read =
        |relative: &str| -> Result<Vec<u8>> { Ok(std::fs::read(lab.config_file(relative)?)?) };
    let example = String::from_utf8(read(EXAMPLE)?)?;
    let secret = String::from_utf8(read(SECRET)?)?;
    let parsed: Value = serde_json::from_str(&example)?;
    let embedded: Option<Value> =
        secret_document(&secret).and_then(|doc| serde_json::from_str(&doc).ok());
    let digests = json!({
        "release_sha256":sha256(&read(RELEASE)?),
        "layer_sha256":sha256(&read(LAYER)?),
        "example_sha256":sha256(example.as_bytes()),
        "secret_sha256":sha256(secret.as_bytes()),
        "documentation_review_sha256":sha256(&read(REVIEWS)?),
    });
    let edited = example.replace(PLACEHOLDER, &passphrase()?);
    Ok(Inputs {
        placeholder_occurrences: example.matches(PLACEHOLDER).count(),
        secret_matches_example: embedded.as_ref() == Some(&parsed),
        example,
        edited,
        digests,
    })
}

/// The documented example with custody fields replaced: request signing
/// binds only the scope and routes, so the client never sees the secret.
fn client_configuration(example: &str) -> Result<Value> {
    let mut document: Value = serde_json::from_str(example)?;
    document["audit"]["keys"]["passphrase"] = json!("not-sent-to-the-client");
    Ok(document)
}

fn evaluate(
    lab: &mut Lab,
    deployment: &Deployment,
    input: &Inputs,
    ledger: &mut Value,
) -> Result<()> {
    let configuration: Value = serde_json::from_str(&input.example)?;
    let installation = configuration["scope"]["installation"]
        .as_str()
        .ok_or("example installation")?
        .to_owned();
    let routes = configuration["routes"].as_array().map_or(0, Vec::len);
    ledger["routes"] = json!(routes);

    lab.stage = "deployment_manifest";
    let pure = deployment.config(lab, false)?;
    let layered = deployment.config(lab, true)?;
    ledger["manifest"] = json!({
        "image":pure["services"]["runtime"]["image"],
        "runtime_layer_keys":changed_keys(&pure, &layered, "runtime"),
        "edge_layer_keys":changed_keys(&pure, &layered, "edge"),
        "runtime_command":pure["services"]["runtime"]["command"],
        "runtime_entrypoint":pure["services"]["runtime"]["entrypoint"],
        "origin_network_external":pure["networks"]["origin-net"]["external"],
    });

    lab.stage = "deployment_build";
    let build = deployment.args(true, &["build", "runtime"]);
    lab.docker
        .invoke_image_build(&build, &deployment.environment, true)?;
    let tag = format!(
        "humanymous/deployment-runtime:{}",
        deployment
            .environment
            .iter()
            .find(|(key, _)| key == "HM_LAB_IMAGE_TAG")
            .ok_or("image tag")?
            .1
    );
    let image = lab
        .docker
        .call(&["image", "inspect", "--format", "{{.Id}}", &tag])?;
    ledger["image"] = json!(image.trim());

    lab.stage = "deployment_topology";
    // Documented: `docker network create ${HMN_ORIGIN_NET}`; owned and internal.
    let owner = "org.humanymous.owner=rust-product-lab".to_owned();
    let expires = format!("org.humanymous.expires={}", deployment.expires);
    let project = format!("com.docker.compose.project={}", deployment.project);
    lab.docker.call(&[
        "network",
        "create",
        "--internal",
        "--label",
        &owner,
        "--label",
        &expires,
        "--label",
        &project,
        &deployment.network,
    ])?;
    deployment.compose(lab, &["up", "-d", "dep-origin"])?;
    let origin_id = deployment.container(lab, "dep-origin")?;
    let deadline = Instant::now() + Duration::from_secs(60);
    while !contacts(lab, &origin_id)?.0 {
        if Instant::now() > deadline {
            return Err("owned origin not ready".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    lab.stage = "deployment_custody";
    // The deployed image mounts hmn-data first, so Docker copies the image's
    // own /var/lib/humanymous owner and mode into the empty volume; custody
    // then reports what that layout actually is. The created container is
    // removed again so only `run --rm` one-offs carry the runtime service.
    deployment.compose(lab, &["create", "runtime"])?;
    deployment.compose(lab, &["rm", "-f", "runtime"])?;
    let provision = json!({"action":"provision","configuration":input.edited});
    let provisioned = deployment.run(lab, &["dep-custody"], format!("{provision}\n").as_bytes())?;
    ledger["custody"] =
        json!({"success":provisioned.success,"report":last_line(&provisioned).ok()});

    lab.stage = "deployment_edge";
    let documented_up = deployment.args(true, &["up", "-d", "edge"]);
    deployment.compose(lab, &["up", "-d", "edge"])?;
    let edge_id = deployment.container(lab, "edge")?;
    let deadline = Instant::now() + Duration::from_secs(60);
    let edge_ready = loop {
        let logs = lab.docker.call(&["logs", &edge_id])?;
        if logs.contains("\"dep_edge_ready\"") {
            break true;
        }
        if Instant::now() > deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    let edge = inspect(lab, &edge_id)?;
    let origin = inspect(lab, &origin_id)?;
    let ingress = format!("{}_dep-ingress", deployment.project);
    let edge_origin_ip = address(&edge, &deployment.network).ok_or("edge origin-net address")?;
    let edge_ingress_ip = address(&edge, &ingress).ok_or("edge ingress address")?;
    let origin_ip = address(&origin, &deployment.network).ok_or("origin address")?;
    let mut networks: Vec<String> = origin["NetworkSettings"]["Networks"]
        .as_object()
        .map(|n| n.keys().cloned().collect())
        .unwrap_or_default();
    networks.sort();
    ledger["topology"] = json!({"edge_ready":edge_ready,"edge_origin_ip":edge_origin_ip,
        "edge_ingress_ip":edge_ingress_ip,"origin_ip":origin_ip,"origin_networks":networks,
        "origin_network":deployment.network});

    lab.stage = "deployment_placeholder";
    // The example exactly as shipped: its placeholder cannot unseal custody.
    let shipped = deployment.run(lab, &["runtime"], input.example.as_bytes())?;
    ledger["placeholder"] = json!({"exit_success":shipped.success,
        "readiness_absent":shipped.stdout.trim().is_empty()});

    let volume = format!("{}_hmn-data", deployment.project);
    let documented_run = deployment.args(true, &["run", "--rm", "-T", "runtime"]);
    let client = client_configuration(&input.example)?;
    let mut phases = Vec::new();
    let mut attestation = Value::Null;
    for phase in PHASES {
        lab.stage = "deployment_serve";
        let (serving, readiness) = serve(lab, deployment, input.edited.as_bytes())?;
        let ids = deployment.one_off(lab, "runtime")?;
        let facts = match ids.as_slice() {
            [id] => runtime_facts(&inspect(lab, id)?, &edge_id, &volume),
            _ => json!({"one_off_containers":ids.len()}),
        };
        lab.stage = "deployment_client";
        let control =
            json!({"edge":edge_ingress_ip,"origin":origin_ip,"phase":phase,"configuration":client});
        let report = deployment.run(lab, &["dep-client"], format!("{control}\n").as_bytes())?;
        let client_report = last_line(&report).unwrap_or(Value::Null);
        if *phase == "restarted" {
            lab.stage = "deployment_attestation";
            let enrolled = enrollment(lab, deployment, INSTALLATION)?;
            let foreign = enrollment(lab, deployment, FOREIGN)?;
            let challenged = challenge(
                lab,
                deployment,
                &enrolled["document"],
                &installation,
                &sha256(input.edited.as_bytes()),
            )?;
            let again = enrollment(lab, deployment, INSTALLATION)?;
            attestation = json!({"enrollment":enrolled,"foreign_enrollment":foreign,
                "challenger":challenged,"re_enrollment":again});
        }
        let stopped = stop(lab, deployment, serving)?;
        lab.stage = "deployment_verify";
        let verify = json!({"action":"verify","configuration":input.edited});
        let verified = deployment.run(lab, &["dep-custody"], format!("{verify}\n").as_bytes())?;
        phases.push(
            json!({"phase":phase,"readiness":readiness,"container":facts,
            "client":client_report,"client_process_success":report.success,"killed":stopped,
            "custody":last_line(&verified).unwrap_or(Value::Null)}),
        );
    }
    ledger["phases"] = json!(phases);
    ledger["attestation"] = attestation;
    let (_, peers) = contacts(lab, &origin_id)?;
    ledger["contact_peers"] = json!(peers);
    ledger["documented_commands"] = json!([
        format!("docker network create {}", deployment.network),
        documented_up.join(" "),
        format!("{} < runtime.json", documented_run.join(" ")),
    ]);
    Ok(())
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let deployment = Deployment::new(lab)?;
    let input = inputs(lab)?;
    let mut ledger = json!({"inputs":input.digests,
        "secret_matches_example":input.secret_matches_example,
        "placeholder_occurrences":input.placeholder_occurrences,
        "edited_example_sha256":sha256(input.edited.as_bytes()),
        "full_product_qualified":false});
    let outcome = evaluate(lab, &deployment, &input, &mut ledger);
    lab.stage = "deployment_teardown";
    let clean = deployment.teardown(lab);
    ledger["evaluation_error"] = json!(outcome.as_ref().err().map(ToString::to_string));
    ledger["teardown_clean"] = json!(clean.as_ref().is_ok_and(|clean| *clean));
    snapshot::write(&lab.evidence, "deployment-ledger.json", &ledger)?;
    let deployed = deployment_case(&ledger);
    let documented = examples_case(&ledger);
    lab.components["deployment"] = json!({
        "status":if deployed && documented {"passed"} else {"failed"},
        "evidence":"deployment-ledger.json","full_product_qualified":false,
        "executed":["deployments/compose.release.yaml via its documented commands",
            "deployments/humanymous-runtime.example.json as shipped and with the documented edit"],
        "not_qualified":[
            "live cloud provider (container SoT keeps it out of the Docker lab)",
            "Kubernetes and systemd manifests: parsed by the runtime gateway test, not executed (no cluster or systemd PID 1 in the hardened lab)",
            "published ghcr image: the lab builds the same shape from this source",
            "adopter TLS edge: the lab edge is a plain L4 relay",
            "custody provisioning: no shipped command; a lab actor performs the documented step",
            "target attestation is lab-scoped: key generated in the topology, not hardware/OS-loader/listener binding"]});
    outcome?;
    clean?;
    let examples = lab.case(
        "documentation-examples",
        documented,
        json!("deployments/humanymous-runtime.example.json run by the real runtime through the documented `run --rm -T runtime < runtime.json`: as shipped its placeholder is refused; with only the documented passphrase edit it is ready on the documented listener and every documented route refuses unsigned and forwards signed requests in both phases; the Kubernetes Secret stub embeds the identical document; docs/reviews.json digest bound"),
    );
    let deployed_case = lab.case(
        "deployment",
        deployed,
        json!("deployments/compose.release.yaml executed with only image/build/pull_policy/labels layered on the runtime: manifest user, read-only root, cap drop, no-new-privileges, pids limit, tmpfs, edge network namespace and hmn-data volume as run, no argv; documented routes through the edge from an ingress-only client with counted origin contacts; authenticated audit history on the manifest volume across a run --rm restart; deployed image generates its attestation key inside the volume and a network-none challenger verifies fresh nonces and refuses stale, tampered, foreign-key and changed-configuration answers"),
    );
    examples.and(deployed_case)
}

/// Verified contiguous history whose signed head covers every record.
fn sound(history: &Value) -> bool {
    let sequence = history["sequence"].as_u64().unwrap_or(0);
    history["verified"] == true
        && history["contiguous"] == true
        && sequence > 0
        && history["records"]
            .as_array()
            .is_some_and(|records| records.len() as u64 == sequence)
        && history["head"]["tree_size"].as_u64() == Some(sequence)
}

fn allow_decisions(history: &Value) -> usize {
    history["records"].as_array().map_or(0, |records| {
        records
            .iter()
            .filter(|r| r["event_type"] == "runtime.decision" && r["enforcement_action"] == "ALLOW")
            .count()
    })
}

fn client_passed(report: &Value, phase: &str, routes: usize) -> bool {
    let rows = report["cases"].as_array().map_or(&[][..], Vec::as_slice);
    report["schema"] == CLIENT_SCHEMA
        && report["phase"] == phase
        && report["status"] == "passed"
        && report["expected_origin_effects"] == routes
        && rows.len() == 2 + 2 * routes + 1
        && rows.iter().all(|row| row["status"] == "passed")
}

fn hardened(facts: &Value, image: &Value, volume: &str) -> bool {
    facts["user"] == "65532:65532"
        && facts["entrypoint"] == json!(["/usr/local/bin/humanymous-runtime"])
        && facts["cmd"].as_array().is_none_or(Vec::is_empty)
        && facts["image"] == *image
        && facts["read_only"] == true
        && facts["privileged"] == false
        && facts["cap_drop"] == json!(["ALL"])
        && facts["security_opt"]
            .as_array()
            .is_some_and(|v| v.contains(&json!("no-new-privileges:true")))
        && facts["pids_limit"] == 512
        && facts["tmpfs"].get("/tmp").is_some()
        && facts["network_mode_is_edge"] == true
        && facts["port_bindings"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        && facts["mounts"] == json!([{"type":"volume","name":volume,"destination":STATE,"rw":true}])
}

fn attestation_passed(ledger: &Value) -> bool {
    let attestation = &ledger["attestation"];
    let enrolled = &attestation["enrollment"];
    let document = &enrolled["document"];
    let public_only = document.as_object().is_some_and(|fields| {
        let mut keys: Vec<_> = fields.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys == [
            "configuration_sha256",
            "installation",
            "public_key",
            "schema",
        ]
    });
    let challenger = &attestation["challenger"];
    let report = &challenger["report"];
    let cases = report["cases"].as_array().map_or(&[][..], Vec::as_slice);
    let exact = cases.len() == CHALLENGER_CASES.len()
        && CHALLENGER_CASES
            .iter()
            .all(|id| cases.iter().filter(|row| row["id"] == *id).count() == 1);
    let again = &attestation["re_enrollment"];
    let foreign = &attestation["foreign_enrollment"]["document"];
    let custody = &ledger["phases"][PHASES.len() - 1]["custody"];
    let key = |name: &str| {
        let facts = &custody[name];
        facts["present"] == true
            && facts["mode"] == "600"
            && facts["length"] == 64
            && facts["uid"] == 65532
    };
    enrolled["success"] == true
        && public_only
        && document["schema"] == ENROLLMENT_SCHEMA
        && document["configuration_sha256"] == ledger["edited_example_sha256"]
        && foreign["public_key"].is_string()
        && foreign["public_key"] != document["public_key"]
        && challenger["process_success"] == true
        && report["schema"] == CHALLENGER_SCHEMA
        && report["status"] == "passed"
        && report["private_key_exported"] == false
        && report["distinct_nonces"] == true
        && report["pinned_public_key"] == document["public_key"]
        && exact
        && cases.iter().all(|row| row["status"] == "passed")
        && again["success"] == false
        && again["stdout_empty"] == true
        && again["refusal"] == "already_provisioned"
        && key("installation_key")
        && key("foreign_key")
}

/// Shared facts both cases rest on: inputs bound, manifest executed, every
/// phase ready on the documented listener with a passing client.
fn executed(ledger: &Value) -> bool {
    let routes = ledger["routes"].as_u64().unwrap_or(0) as usize;
    let phases = ledger["phases"].as_array().map_or(&[][..], Vec::as_slice);
    ledger["evaluation_error"].is_null()
        && ledger["teardown_clean"] == true
        && routes > 0
        && ledger["custody"]["success"] == true
        && ledger["topology"]["edge_ready"] == true
        && phases.len() == PHASES.len()
        && phases.iter().zip(PHASES).all(|(phase, name)| {
            phase["phase"] == *name
                && phase["readiness"]["listen"] == "127.0.0.1:8444"
                && phase["client_process_success"] == true
                && client_passed(&phase["client"], name, routes)
        })
}

fn deployment_case(ledger: &Value) -> bool {
    let routes = ledger["routes"].as_u64().unwrap_or(0) as usize;
    let manifest = &ledger["manifest"];
    let custody = &ledger["custody"]["report"];
    let topology = &ledger["topology"];
    let phases = ledger["phases"].as_array().map_or(&[][..], Vec::as_slice);
    let peers = ledger["contact_peers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let volume = phases
        .first()
        .and_then(|phase| phase["container"]["volume"].as_str())
        .unwrap_or("");
    let histories: Vec<_> = phases.iter().map(|p| &p["custody"]["history"]).collect();
    let continues = match histories.as_slice() {
        [first, second] => match (first["records"].as_array(), second["records"].as_array()) {
            (Some(a), Some(b)) => b.len() > a.len() && b[..a.len()] == a[..],
            _ => false,
        },
        _ => false,
    };
    executed(ledger)
        && manifest["image"]
            .as_str()
            .is_some_and(|image| image.starts_with(MANIFEST_IMAGE))
        && manifest["runtime_layer_keys"] == json!(RUNTIME_LAYER_KEYS)
        && manifest["runtime_command"].is_null()
        && manifest["runtime_entrypoint"].is_null()
        && manifest["origin_network_external"] == true
        && ledger["image"]
            .as_str()
            .is_some_and(|id| id.starts_with("sha256:"))
        && custody["sealed"] == true
        && custody["audit_directory_empty"] == true
        && custody["volume"] == json!({"uid":65532,"mode":"700"})
        && topology["origin_networks"] == json!([topology["origin_network"]])
        && topology["edge_origin_ip"] != topology["origin_ip"]
        && !volume.is_empty()
        && phases.iter().all(|phase| {
            phase["killed"] == true && hardened(&phase["container"], &ledger["image"], volume)
        })
        && histories.iter().all(|history| sound(history))
        && histories
            .iter()
            .enumerate()
            .all(|(index, history)| allow_decisions(history) == routes * (index + 1))
        && continues
        && peers.len() == routes * PHASES.len()
        && peers.iter().all(|peer| *peer == topology["edge_origin_ip"])
        && attestation_passed(ledger)
}

fn examples_case(ledger: &Value) -> bool {
    let hex = |value: &Value| {
        value
            .as_str()
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    let inputs = &ledger["inputs"];
    executed(ledger)
        && [
            "release_sha256",
            "layer_sha256",
            "example_sha256",
            "secret_sha256",
            "documentation_review_sha256",
        ]
        .iter()
        .all(|field| hex(&inputs[*field]))
        && ledger["secret_matches_example"] == true
        && ledger["placeholder_occurrences"] == 1
        && ledger["edited_example_sha256"] != inputs["example_sha256"]
        && ledger["placeholder"]["exit_success"] == false
        && ledger["placeholder"]["readiness_absent"] == true
        && ledger["documented_commands"]
            .as_array()
            .is_some_and(|c| c.len() == 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_BYTES: &str =
        include_str!("../../../../../deployments/humanymous-runtime.example.json");
    const SECRET_BYTES: &str = include_str!("../../../../../deployments/k8s/secret.example.yaml");
    const RELEASE_BYTES: &str = include_str!("../../../../../deployments/compose.release.yaml");
    const LAYER_BYTES: &str =
        include_str!("../../../../../deployments/lab/compose.deployment.yaml");

    #[test]
    fn shipped_examples_match_the_lab_contract() {
        let example: Value = serde_json::from_str(EXAMPLE_BYTES).unwrap();
        assert_eq!(EXAMPLE_BYTES.matches(PLACEHOLDER).count(), 1);
        assert_eq!(example["listen"], "127.0.0.1:8444");
        assert_eq!(example["origin"], "127.0.0.1:8080");
        assert_eq!(example["owner"]["uid"], 65532);
        for directory in [
            &example["audit"]["directory"],
            &example["audit"]["keys"]["directory"],
        ] {
            assert!(
                directory
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("{STATE}/"))
            );
        }
        let embedded: Value =
            serde_json::from_str(&secret_document(SECRET_BYTES).unwrap()).unwrap();
        assert_eq!(embedded, example);
        assert!(RELEASE_BYTES.contains(MANIFEST_IMAGE));
        assert!(RELEASE_BYTES.contains("run --rm -T runtime < runtime.json"));
        assert!(RELEASE_BYTES.contains("up -d edge"));
        assert!(RELEASE_BYTES.contains("network_mode: \"service:edge\""));
        // The lab layer never touches the runtime's hardening keys.
        let runtime = LAYER_BYTES
            .split("\n  runtime:\n")
            .nth(1)
            .and_then(|rest| rest.split("\n  edge:\n").next())
            .unwrap();
        for forbidden in [
            "user:",
            "read_only",
            "cap_drop",
            "security_opt",
            "network_mode",
            "volumes",
            "entrypoint",
            "command",
            "tmpfs",
            "pids_limit",
        ] {
            assert!(!runtime.contains(forbidden), "{forbidden}");
        }
        let client = client_configuration(EXAMPLE_BYTES).unwrap();
        assert_ne!(
            client["audit"]["keys"]["passphrase"],
            example["audit"]["keys"]["passphrase"]
        );
        assert_eq!(client["routes"], example["routes"]);
    }

    #[test]
    fn layered_keys_are_reported_exactly() {
        let pure = json!({"services":{"runtime":{"image":"a","user":"65532:65532"}}});
        let layered =
            json!({"services":{"runtime":{"image":"b","user":"65532:65532","labels":{}}}});
        assert_eq!(
            changed_keys(&pure, &layered, "runtime"),
            vec!["image", "labels"]
        );
        let mut widened = layered.clone();
        widened["services"]["runtime"]["user"] = json!("0:0");
        assert_eq!(
            changed_keys(&pure, &widened, "runtime"),
            vec!["image", "labels", "user"]
        );
    }

    fn history(allows: usize, extra: usize) -> Value {
        let mut records: Vec<_> = (0..allows)
            .map(|_| json!({"event_type":"runtime.decision","enforcement_action":"ALLOW"}))
            .collect();
        records.extend(
            (0..extra)
                .map(|_| json!({"event_type":"runtime.decision","enforcement_action":"DENY"})),
        );
        for (index, record) in records.iter_mut().enumerate() {
            record["seq"] = json!(index + 1);
        }
        let n = records.len();
        json!({"verified":true,"contiguous":true,"sequence":n,"records":records,"head":{"tree_size":n}})
    }

    fn client(phase: &str, routes: usize) -> Value {
        let rows: Vec<_> = (0..2 + 2 * routes + 1)
            .map(|i| json!({"id":i,"status":"passed"}))
            .collect();
        json!({"schema":CLIENT_SCHEMA,"phase":phase,"status":"passed","cases":rows,"expected_origin_effects":routes})
    }

    fn ledger() -> Value {
        let routes = 2;
        let image = "sha256:".to_owned() + &"a".repeat(64);
        let volume = "p_hmn-data";
        let facts = json!({"user":"65532:65532","entrypoint":["/usr/local/bin/humanymous-runtime"],"cmd":null,
            "image":image,"read_only":true,"privileged":false,"cap_drop":["ALL"],
            "security_opt":["no-new-privileges:true"],"pids_limit":512,"tmpfs":{"/tmp":""},
            "network_mode_is_edge":true,"port_bindings":{},
            "mounts":[{"type":"volume","name":volume,"destination":STATE,"rw":true}],"volume":volume});
        let first = history(routes, 2 * routes);
        let mut second_records = first["records"].as_array().unwrap().clone();
        second_records.extend(
            history(routes, 2 * routes)["records"]
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
        for (index, record) in second_records.iter_mut().enumerate() {
            record["seq"] = json!(index + 1);
        }
        let n = second_records.len();
        let second = json!({"verified":true,"contiguous":true,"sequence":n,"records":second_records,"head":{"tree_size":n}});
        let key = json!({"present":true,"mode":"600","length":64,"uid":65532});
        let digest = "b".repeat(64);
        let cases: Vec<_> = CHALLENGER_CASES
            .iter()
            .map(|id| json!({"id":id,"status":"passed"}))
            .collect();
        let enrollment = json!({"schema":ENROLLMENT_SCHEMA,"installation":"example-installation","configuration_sha256":digest,"public_key":"c".repeat(64)});
        json!({
            "inputs":{"release_sha256":"1".repeat(64),"layer_sha256":"2".repeat(64),"example_sha256":"3".repeat(64),
                "secret_sha256":"4".repeat(64),"documentation_review_sha256":"5".repeat(64)},
            "secret_matches_example":true,"placeholder_occurrences":1,"edited_example_sha256":digest,
            "routes":routes,
            "manifest":{"image":format!("{MANIFEST_IMAGE}latest"),"runtime_layer_keys":RUNTIME_LAYER_KEYS,
                "edge_layer_keys":["entrypoint"],"runtime_command":null,"runtime_entrypoint":null,"origin_network_external":true},
            "image":image,
            "custody":{"success":true,"report":{"sealed":true,"audit_directory_empty":true,"volume":{"uid":65532,"mode":"700"}}},
            "topology":{"edge_ready":true,"edge_origin_ip":"10.0.0.2","edge_ingress_ip":"10.1.0.2","origin_ip":"10.0.0.3",
                "origin_networks":["p-origin"],"origin_network":"p-origin"},
            "placeholder":{"exit_success":false,"readiness_absent":true},
            "phases":[
                {"phase":"first","readiness":{"listen":"127.0.0.1:8444"},"container":facts,"client":client("first",routes),
                 "client_process_success":true,"killed":true,"custody":{"history":first}},
                {"phase":"restarted","readiness":{"listen":"127.0.0.1:8444"},"container":facts,"client":client("restarted",routes),
                 "client_process_success":true,"killed":true,"custody":{"history":second,"installation_key":key,"foreign_key":key}}],
            "attestation":{"enrollment":{"success":true,"document":enrollment},
                "foreign_enrollment":{"success":true,"document":{"public_key":"d".repeat(64)}},
                "challenger":{"process_success":true,"report":{"schema":CHALLENGER_SCHEMA,"status":"passed",
                    "private_key_exported":false,"distinct_nonces":true,"pinned_public_key":"c".repeat(64),"cases":cases}},
                "re_enrollment":{"success":false,"stdout_empty":true,"refusal":"already_provisioned"}},
            "contact_peers":["10.0.0.2","10.0.0.2","10.0.0.2","10.0.0.2"],
            "documented_commands":["a","b","c"],
            "evaluation_error":null,"teardown_clean":true})
    }

    #[test]
    fn complete_ledger_passes_both_cases() {
        let good = ledger();
        assert!(deployment_case(&good));
        assert!(examples_case(&good));
    }

    #[test]
    fn each_observation_is_required() {
        let good = ledger();
        let breaks: Vec<(&str, Value)> = vec![
            ("/teardown_clean", json!(false)),
            ("/evaluation_error", json!("x")),
            (
                "/manifest/runtime_layer_keys",
                json!(["build", "image", "labels", "pull_policy", "user"]),
            ),
            ("/manifest/image", json!("humanymous/gate:latest")),
            ("/custody/report/volume/uid", json!(0)),
            ("/phases/0/container/user", json!("0:0")),
            ("/phases/1/container/network_mode_is_edge", json!(false)),
            ("/phases/0/container/cmd", json!(["--flag"])),
            ("/phases/1/container/pids_limit", json!(64)),
            ("/phases/0/readiness/listen", json!("0.0.0.0:8444")),
            ("/phases/1/client/status", json!("failed")),
            ("/phases/0/custody/history/verified", json!(false)),
            ("/contact_peers/3", json!("10.1.0.2")),
            ("/attestation/re_enrollment/success", json!(true)),
            (
                "/attestation/challenger/report/cases/4/status",
                json!("failed"),
            ),
            (
                "/attestation/challenger/report/pinned_public_key",
                json!("e".repeat(64)),
            ),
            (
                "/attestation/foreign_enrollment/document/public_key",
                json!("c".repeat(64)),
            ),
            ("/attestation/enrollment/document/private_key", json!("x")),
            ("/phases/1/custody/installation_key/mode", json!("644")),
        ];
        for (pointer, value) in breaks {
            let mut bad = good.clone();
            match bad.pointer_mut(pointer) {
                Some(slot) => *slot = value,
                None => {
                    let (parent, field) = pointer.rsplit_once('/').unwrap();
                    bad.pointer_mut(parent).unwrap()[field] = value;
                }
            }
            assert!(!deployment_case(&bad), "{pointer}");
        }
        let mut short = good.clone();
        short["contact_peers"].as_array_mut().unwrap().pop();
        assert!(!deployment_case(&short));
        let mut restart = good.clone();
        restart["phases"][1]["custody"]["history"] =
            restart["phases"][0]["custody"]["history"].clone();
        assert!(!deployment_case(&restart));
    }

    #[test]
    fn documentation_examples_need_their_own_observations() {
        let good = ledger();
        for (pointer, value) in [
            ("/placeholder/exit_success", json!(true)),
            ("/placeholder/readiness_absent", json!(false)),
            ("/secret_matches_example", json!(false)),
            ("/placeholder_occurrences", json!(2)),
            ("/inputs/documentation_review_sha256", json!("")),
            ("/phases/1/client/status", json!("failed")),
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(!examples_case(&bad), "{pointer}");
        }
        let mut unedited = good;
        unedited["edited_example_sha256"] = unedited["inputs"]["example_sha256"].clone();
        assert!(!examples_case(&unedited));
    }
}
