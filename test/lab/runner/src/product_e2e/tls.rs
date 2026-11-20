//! Profile `tls`: the shipped gateway binary serving its TLS listener with a
//! per-run lab CA minted inside the Gate container, driven over real TLS
//! (HTTP/1.1 and H2), raw owned ClientHello parrots and malformed H2 frames.
//!
//! The runner relays only the public lab root from the Gate's readiness line
//! to the driver, then judges the observed readiness and topology. The Gate
//! serves its public TLS edge listener (`public_listen`, owner decision
//! B01 #1); the origin shares the Gate's namespace and stays loopback, and the
//! driver runs in its own namespace on the internal `tls` network, so its
//! results are original-client cross-namespace evidence.
use super::{COMPOSE, Lab, PROJECT, Result, empty, last_json_line, topology_assertions};
use serde_json::{Value, json};

const TGATE: &str = "humanymous-lab-product-e2e-tgate-1";
const TORIGIN: &str = "humanymous-lab-product-e2e-torigin-1";
pub(super) const TDRIVER: &str = "humanymous-lab-product-e2e-tdriver";
const LAB_CA_SCHEMA: &str = "humanymous.product-e2e-lab-ca/1";

/// Pure judgement of the TLS Gate's own readiness line against the driver's
/// trusted root.
fn readiness_assertions(ready: &Value, driver: &Value) -> Vec<(&'static str, bool)> {
    let readiness = &ready["readiness"];
    let profile = &readiness["profile"];
    let root = &ready["lab_ca"]["sha256"];
    vec![
        (
            "shipped_binary_reported_readiness",
            ready["binary"] == "/usr/local/bin/humanymous-runtime",
        ),
        (
            "tls_listener_operator_certificate",
            readiness["tls_mode"] == "operator_certificate",
        ),
        (
            "tls_readiness_profile",
            profile["profile"] == "experimental-public-tls"
                && profile["public_listen"] == true
                && profile["raw_tls"] == true,
        ),
        (
            "public_edge_listener",
            readiness["listen"] == "0.0.0.0:18443" && profile["public_listen"] == true,
        ),
        (
            "per_run_lab_root_relayed_and_trusted",
            ready["lab_ca"]["schema"] == LAB_CA_SCHEMA
                && root.as_str().is_some_and(|sha| sha.len() == 64)
                && driver["trusted_lab_root_sha256"] == *root,
        ),
    ]
}

/// The driver is an original client: its own network namespace (never the
/// Gate's), attached only to the internal `tls` network, running the built
/// image in the hardened sandbox with no published ports.
fn driver_namespace_assertions(
    network: &Value,
    gate: &Value,
    driver: &Value,
    image: &str,
) -> Vec<(&'static str, bool)> {
    let name = network["Name"].as_str().unwrap_or("");
    let shared = gate["id"]
        .as_str()
        .map(|id| format!("container:{id}"))
        .unwrap_or_default();
    vec![
        (
            "driver_in_own_network_namespace",
            !name.is_empty()
                && driver["network_mode"] != shared.as_str()
                && driver["network_mode"].as_str() != Some("host")
                && driver["network_mode"].as_str() != Some("none"),
        ),
        (
            "driver_only_on_internal_tls_network",
            network["Internal"] == true
                && driver["networks"]
                    .as_array()
                    .is_some_and(|all| all.len() == 1 && all[0].as_str() == Some(name)),
        ),
        (
            "driver_runs_built_image_hardened",
            driver["image"] == image
                && driver["read_only_rootfs"] == true
                && driver["user"] == "65532:65532"
                && driver["init"] == true
                && empty(&driver["port_bindings"])
                && empty(&driver["published_ports"]),
        ),
    ]
}

/// The lab root wrapper relayed on the driver's stdin: public fields only.
fn root_line(ready: &Value) -> Option<String> {
    let ca = &ready["lab_ca"];
    (ca["schema"] == LAB_CA_SCHEMA && ca["certificate_pem"].is_string()).then(|| {
        json!({"schema":LAB_CA_SCHEMA,"certificate_pem":ca["certificate_pem"],"sha256":ca["sha256"]})
            .to_string()
    })
}

pub(super) fn run(lab: &mut Lab, image: &str, report_line: &str) -> Result<Value> {
    let compose = [
        "compose",
        "--file",
        COMPOSE,
        "--project-name",
        PROJECT,
        "--profile",
        "tls",
    ];
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
            "tgate",
            "torigin",
        ],
    ]
    .concat();
    if !lab.docker(&up, 300, false)?.success {
        return Ok(json!({"pass":false,"error":"tls topology did not become healthy"}));
    }
    let logs = lab.docker(&["logs", TGATE], 60, false)?;
    let ready =
        last_json_line(&logs.stdout, "humanymous.product-e2e-gate-ready/1").unwrap_or(Value::Null);
    let Some(root) = root_line(&ready) else {
        return Ok(json!({"pass":false,"error":"TLS Gate readiness carried no lab root"}));
    };
    // The driver reads the collector report line first; an absent report is
    // an empty line, so the root always arrives as the second line.
    let report = if report_line.is_empty() {
        "\n"
    } else {
        report_line
    };
    let stdin = format!("{report}{root}\n");
    let drive = [
        &compose[..],
        &[
            "run", "--no-TTY", "--pull", "never", "--name", TDRIVER, "tdriver",
        ],
    ]
    .concat();
    let driven = lab.docker_input(&drive, stdin.as_bytes(), 600, false)?;
    let driver = last_json_line(&driven.stdout, "humanymous.product-e2e-tls-driver/1")
        .unwrap_or(Value::Null);
    let mut containers = Vec::new();
    for name in [TGATE, TORIGIN, TDRIVER] {
        containers.push(lab.inspect(name)?);
    }
    let network = lab.json(&[
        "network",
        "inspect",
        "--format",
        "{{json .}}",
        &format!("{PROJECT}_tls"),
    ])?;
    let mut assertions = topology_assertions(&network, &containers[..2], image);
    assertions.extend(driver_namespace_assertions(
        &network,
        &containers[0],
        &containers[2],
        image,
    ));
    assertions.extend(readiness_assertions(&ready, &driver));
    assertions.push((
        "tls_gate_healthy_after_driver",
        containers[0]["state"] == "running" && containers[0]["health"] == "healthy",
    ));
    let topology_pass = assertions.iter().all(|(_, pass)| *pass);
    let driver_pass = driven.success && driver["failed"] == 0 && driver["driver_error"].is_null();
    let mut readiness = ready["readiness"].clone();
    readiness["lab_ca_sha256"] = ready["lab_ca"]["sha256"].clone();
    readiness["lab_ca_issuance"] = ready["lab_ca"]["issuance"].clone();
    Ok(
        json!({"scope":"cross-namespace original client: tdriver runs in its own network namespace on the internal tls network and reaches tgate's public TLS edge listener; torigin shares tgate's namespace",
        "gate":"humanymous-runtime (shipped binary) with a tls section: ServiceHost::bind_tls, no witness input",
        "readiness":readiness,
        "topology":{"network":{"name":network["Name"],"internal":network["Internal"],"driver":network["Driver"]},
            "containers":containers,
            "assertions":assertions.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>()},
        "driver":driver,"driver_exit_success":driven.success,
        "pass":topology_pass && driver_pass}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> Value {
        json!({"schema":"humanymous.product-e2e-gate-ready/1","binary":"/usr/local/bin/humanymous-runtime",
            "readiness":{"listen":"0.0.0.0:18443","tls_mode":"operator_certificate",
                "profile":{"profile":"experimental-public-tls","public_listen":true,"raw_tls":true}},
            "lab_ca":{"schema":LAB_CA_SCHEMA,"certificate_pem":"PEM","sha256":"a".repeat(64),"issuance":"x"}})
    }

    #[test]
    fn readiness_must_be_the_shipped_tls_listener_trusting_the_relayed_root() {
        let driver = json!({"trusted_lab_root_sha256":"a".repeat(64)});
        assert!(
            readiness_assertions(&ready(), &driver)
                .iter()
                .all(|(_, p)| *p)
        );
        let mut plain = ready();
        plain["readiness"]["tls_mode"] = json!(null);
        plain["readiness"]["profile"]["profile"] = json!("experimental-loopback-http1");
        let failed: Vec<_> = readiness_assertions(&plain, &driver)
            .into_iter()
            .filter(|(_, p)| !p)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            failed,
            ["tls_listener_operator_certificate", "tls_readiness_profile"]
        );
        let other = json!({"trusted_lab_root_sha256":"b".repeat(64)});
        assert!(!readiness_assertions(&ready(), &other)[4].1);
        let mut library = ready();
        library["binary"] = json!("library");
        assert!(!readiness_assertions(&library, &driver)[0].1);
    }

    #[test]
    fn a_loopback_or_unflagged_listener_is_not_the_public_edge() {
        let driver = json!({"trusted_lab_root_sha256":"a".repeat(64)});
        let edge = |ready: &Value| {
            readiness_assertions(ready, &driver)
                .into_iter()
                .find(|(id, _)| *id == "public_edge_listener")
                .unwrap()
                .1
        };
        assert!(edge(&ready()));
        let mut loopback = ready();
        loopback["readiness"]["listen"] = json!("127.0.0.1:18443");
        assert!(!edge(&loopback));
        let mut unflagged = ready();
        unflagged["readiness"]["profile"]["public_listen"] = json!(false);
        assert!(!edge(&unflagged));
    }

    #[test]
    fn only_the_public_root_is_relayed() {
        let line: Value = serde_json::from_str(&root_line(&ready()).unwrap()).unwrap();
        assert_eq!(
            line.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["certificate_pem", "schema", "sha256"]
        );
        assert!(root_line(&json!({"lab_ca":{"schema":"x"}})).is_none());
        assert!(root_line(&Value::Null).is_none());
    }
}
