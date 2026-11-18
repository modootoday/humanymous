//! Keeps the shipped compose, Kubernetes and systemd manifests honest against
//! the runtime they claim to run. These are byte checks of repository files plus
//! the runtime's own configuration parser; no cluster, systemd or container runs.
use humanymous_runtime::{
    authority::configuration::Peer,
    gateway::{configuration::Configuration, service::read_configuration},
};
use serde_json::Value;

const EXAMPLE: &str = include_str!("../../../../../deployments/humanymous-runtime.example.json");
const K8S_DEPLOYMENT: &str = include_str!("../../../../../deployments/k8s/deployment.yaml");
const K8S_SECRET: &str = include_str!("../../../../../deployments/k8s/secret.example.yaml");
const K8S_SERVICE: &str = include_str!("../../../../../deployments/k8s/service.yaml");
const K8S_PVC: &str = include_str!("../../../../../deployments/k8s/pvc.yaml");
const COMPOSE: &str = include_str!("../../../../../deployments/compose.release.yaml");
const ENV_EXAMPLE: &str = include_str!("../../../../../deployments/.env.example");
const SYSTEMD: &str = include_str!("../../../../../deployments/systemd/humanymous-runtime.service");
const RUNTIME_IMAGE: &str = include_str!("../../../../../build/rust/runtime-candidate.Dockerfile");
const RELEASE: &str = include_str!("../../../../../.github/workflows/release.yml");

const IMAGE: &str = "ghcr.io/modootoday/humanymous-runtime:";
const STATE: &str = "/var/lib/humanymous";
const UID: u32 = 65532;

/// Lines that configure something; comments may still name Go history.
fn active(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn has(text: &str, needle: &str) -> bool {
    active(text).iter().any(|line| line.contains(needle))
}

fn parsed(document: &str) -> Configuration {
    let configuration = read_configuration(document.as_bytes())
        .expect("example configuration must pass the runtime's own parser and validate()");
    assert!(configuration.validate().is_ok());
    configuration
}

/// The `runtime.json: |` block scalar of the Kubernetes Secret stub.
fn secret_document() -> String {
    let mut lines = K8S_SECRET.lines();
    lines
        .by_ref()
        .find(|line| line.trim_end() == "  runtime.json: |")
        .expect("Secret stub must embed runtime.json");
    let mut document = String::new();
    for line in lines {
        if !line.is_empty() && !line.starts_with("    ") {
            break;
        }
        document.push_str(line.strip_prefix("    ").unwrap_or(line));
        document.push('\n');
    }
    document
}

#[test]
fn example_configuration_is_valid_and_matches_the_manifests() {
    let configuration = parsed(EXAMPLE);
    assert!(configuration.listen.ip().is_loopback());
    assert_eq!(configuration.listen.port(), 8444);
    assert!(configuration.origin.ip().is_loopback());
    assert_eq!(configuration.origin.port(), 8080);
    let Peer::Unix { uid } = configuration.owner else {
        panic!("deployment owner must be a Unix UID");
    };
    assert_eq!(uid, UID);
    for directory in [
        &configuration.audit.directory,
        &configuration.audit.keys.directory,
    ] {
        assert!(directory.starts_with(STATE), "{}", directory.display());
    }
    // Shipped bytes carry a placeholder, never a usable unseal secret.
    assert!(
        configuration
            .audit
            .keys
            .passphrase
            .starts_with("REPLACE-ME")
    );
    assert!(!configuration.audit.require_external_witness);
    assert!(!configuration.routes.is_empty());
    assert!(configuration.routes.iter().all(|route| route.required.rit
        && !route.required.raw_tls
        && !route.required.original_h2
        && !route.required.collector));
}

#[test]
fn kubernetes_secret_embeds_the_same_document() {
    let embedded = secret_document();
    parsed(&embedded);
    let embedded: Value = serde_json::from_str(&embedded).unwrap();
    let example: Value = serde_json::from_str(EXAMPLE).unwrap();
    assert_eq!(embedded, example);
}

#[test]
fn manifests_run_the_rust_runtime_not_the_go_gate() {
    for (name, text) in [
        ("k8s/deployment.yaml", K8S_DEPLOYMENT),
        ("k8s/secret.example.yaml", K8S_SECRET),
        ("k8s/service.yaml", K8S_SERVICE),
        ("k8s/pvc.yaml", K8S_PVC),
        ("compose.release.yaml", COMPOSE),
        (".env.example", ENV_EXAMPLE),
        ("systemd/humanymous-runtime.service", SYSTEMD),
    ] {
        for go in [
            "humanymous-gate",
            "bin/gate",
            "-addr",
            "-upstream",
            "-acme-",
            "-routes",
            "-keystore",
            "-audit-wal",
            "-shutdown-grace",
            "HMN_UNSEAL",
            "HMN_ADMIN_TOKENS",
            "HMN_UPSTREAM",
            "__hmn",
            "8445",
        ] {
            assert!(!has(text, go), "{name} still configures Go `{go}`");
        }
    }
    assert!(has(K8S_DEPLOYMENT, &format!("image: {IMAGE}")));
    assert!(has(COMPOSE, &format!("image: {IMAGE}")));
    // The binary refuses argv; configuration arrives on stdin only.
    for text in [K8S_DEPLOYMENT, COMPOSE] {
        assert!(!has(text, "args:") && !has(text, "command:"));
    }
    for setting in ["stdin: true", "stdinOnce: true", "tty: false"] {
        assert!(has(K8S_DEPLOYMENT, setting), "{setting}");
    }
    assert!(has(COMPOSE, "stdin_open: true"));
    let systemd = active(SYSTEMD);
    assert!(systemd.contains(&"ExecStart=/usr/local/bin/humanymous-runtime"));
    assert!(systemd.contains(&"StandardInput=file:/etc/humanymous/runtime.json"));
    // The manifests name the image the release workflow actually publishes.
    assert!(has(
        RUNTIME_IMAGE,
        "ENTRYPOINT [\"/usr/local/bin/humanymous-runtime\"]"
    ));
    assert!(has(
        RELEASE,
        "dockerfile: build/rust/runtime-candidate.Dockerfile"
    ));
    assert!(has(RELEASE, "- name: runtime"));
    assert!(has(RELEASE, "humanymous-${{ matrix.name }}"));
}

#[test]
fn manifests_keep_the_go_hardening() {
    assert!(has(RUNTIME_IMAGE, &format!("USER {UID}:{UID}")));
    for setting in [
        &format!("runAsUser: {UID}") as &str,
        "runAsNonRoot: true",
        "allowPrivilegeEscalation: false",
        "readOnlyRootFilesystem: true",
        "drop: [\"ALL\"]",
        "type: RuntimeDefault",
        &format!("mountPath: {STATE}"),
        "claimName: hmn-data",
        "terminationGracePeriodSeconds: 30",
    ] {
        assert!(has(K8S_DEPLOYMENT, setting), "k8s: {setting}");
    }
    // fsGroup would make the private audit and custody directories group-accessible.
    assert!(!has(K8S_DEPLOYMENT, "fsGroup"));
    assert!(!has(K8S_SERVICE, "NodePort"));
    for setting in [
        &format!("user: \"{UID}:{UID}\"") as &str,
        "no-new-privileges:true",
        "- ALL",
        "read_only: true",
        "network_mode: \"service:edge\"",
        &format!("hmn-data:{STATE}"),
        "stop_grace_period: 30s",
    ] {
        assert!(has(COMPOSE, setting), "compose: {setting}");
    }
    let systemd = active(SYSTEMD);
    for setting in [
        "User=humanymous",
        "NoNewPrivileges=true",
        "ProtectSystem=strict",
        "PrivateTmp=true",
        "ReadWritePaths=/var/lib/humanymous",
        "MemoryDenyWriteExecute=true",
        "SystemCallFilter=@system-service",
        "CapabilityBoundingSet=",
        "TimeoutStopSec=30",
    ] {
        assert!(
            systemd.iter().any(|line| line.starts_with(setting)),
            "systemd: {setting}"
        );
    }
}
