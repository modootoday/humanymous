//! Config-driven gate provisioning (`configuration::gates`, owner decision
//! #2): absent installs nothing, Go source formats load, and a configured
//! source that does not load refuses. Legacy cmd/gate/main.go `-agent-keys`,
//! `-pat-issuers` and `-webauthn-creds` at baseline 981d25c. Fixture controls
//! through `Application::decide`; no browser or product e2e ran.
use super::gates::{NOW_MS, app_with, records, signed_request};
use super::*;
use crate::gateway::ban::gate::Mode;
use crate::gateway::configuration::Gates;
use crate::gateway::credential::MAX_CONFIG;
use crate::gateway::credential::privacy_pass::tests as pat_fixture;
use crate::gateway::credential::web_bot_auth::tests as wba_fixture;
use crate::gateway::credential::webauthn::tests as wa_fixture;
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rcgen::PublicKeyData as _;
use std::path::{Path, PathBuf};

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-provisioning-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn gates(value: Value) -> Gates {
    serde_json::from_value(value).unwrap()
}

fn fanout() -> Value {
    json!({"max_keys":1024,"max_distinct":1024,"max_field_bytes":256,"max_payload_bytes":1048576,
        "population_keys":16,"population_window":256,"population_key_bytes":256,
        "population_payload_bytes":1048576})
}

fn pat_pem() -> String {
    let lines: Vec<&str> = pat_fixture::ISSUER_A_SPKI
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        lines.join("\n")
    )
}

fn agent_keys(dir: &Dir) -> PathBuf {
    let key = wba_fixture::signer(7);
    let public = URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes());
    dir.file(
        "agent-keys",
        format!("# trusted agents\n\n{} {public}\n", wba_fixture::KID).as_bytes(),
    )
}

fn webauthn_creds(dir: &Dir) -> PathBuf {
    let key = wa_fixture::key();
    let spki = STANDARD.encode(key.subject_public_key_info());
    dir.file(
        "webauthn-creds",
        format!("{} {spki}\n", wa_fixture::CRED).as_bytes(),
    )
}

fn path(path: &Path) -> Value {
    json!(path.to_str().unwrap())
}

/// Absent (Go: every flag empty), nothing is installed and the served path
/// skips every gate; the sweep and ban store Go built unconditionally are
/// opt-in here (named difference).
#[test]
fn absent_gates_section_installs_nothing() {
    let configuration = config();
    let provisioned = configuration.gates.load(false).unwrap();
    assert_eq!(
        provisioned.readiness(),
        json!({"bans":false,"agent_keys":false,"pat_issuers":false,"webauthn":false,"sweep":false,
            "verdict_token":false,"route_presets":null})
    );
    let provisioned = gates(json!({})).load(true).unwrap();
    assert!(provisioned.bans.is_none() && provisioned.sweep.is_none());
}

/// Go source formats load: the agent-key allowlist (comments and blank lines
/// skipped), the issuer PEM file, the credential allowlist with its origin
/// and RP ID binding, and the sweep detector with Go's 30 s / 8 sessions.
/// A directory loaded from the file verifies a signed request on the served
/// path exactly as one built in memory.
#[test]
fn configured_sources_load_go_formats_and_serve() {
    let dir = Dir::new();
    let pem = dir.file("pat-issuers", pat_pem().as_bytes());
    let configured = gates(json!({
        "agent_keys": path(&agent_keys(&dir)),
        "pat_issuers": path(&pem),
        "webauthn": {"credentials": path(&webauthn_creds(&dir)), "origin": "https://owned.test",
            "rp_id": "owned.test", "fanout": fanout()},
        "sweep": {"max_bindings": 1024},
        "bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,"max_key_bytes":128,
            "max_payload_bytes":1048576,"gc_scan_budget":64},
    }));
    let sweep = configured.sweep.as_ref().unwrap();
    assert_eq!(
        (sweep.window_sec, sweep.max_sessions, sweep.mode),
        (30, 8, Mode::Enforce)
    );
    let provisioned = configured.load(false).unwrap();
    assert_eq!(
        provisioned.readiness(),
        json!({"bans":true,"agent_keys":true,"pat_issuers":true,"webauthn":true,"sweep":true,
            "verdict_token":false,"route_presets":null})
    );
    let (_, tracker) = provisioned.webauthn.as_ref().unwrap();
    assert_eq!(
        tracker.cap(),
        crate::gateway::attest_floor::fanout::DEFAULT_CAP
    );

    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    app.install_agent_keys(provisioned.agent_keys.unwrap());
    let key = wba_fixture::signer(7);
    let target = wba_fixture::request("owned.test", "POST", "/effect");
    let (input, signature) =
        wba_fixture::sign_wba(&target, wba_fixture::KID, &key, NOW_MS / 1000, "n-file");
    let fields = vec![
        ("Signature-Input".to_owned(), input),
        ("Signature".to_owned(), signature),
    ];
    let req = signed_request(&mut app, &fields);
    let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
    assert!(matches!(decision, Decision::Origin { .. }));
    assert_eq!(records(&audit, "agent.signature.verified").len(), 1);
}

/// Configured but unreadable, empty, malformed or oversized sources and
/// refused bounds fail closed (Go `gateFatalf`).
#[test]
fn configured_but_invalid_sources_fail_closed() {
    let dir = Dir::new();
    let missing = dir.0.join("missing");
    let empty = dir.file("empty", b"# only a comment\n");
    let garbage = dir.file("garbage", b"one two three\n");
    let oversized = dir.file("oversized", &vec![b'#'; MAX_CONFIG + 1]);
    let not_utf8 = dir.file("binary", &[0xff, 0xfe, 0x00]);
    let creds = webauthn_creds(&dir);
    let mut refused = Vec::new();
    for source in [&missing, &empty, &garbage, &oversized, &not_utf8] {
        refused.push(json!({"agent_keys": path(source)}));
        refused.push(json!({"pat_issuers": path(source)}));
        refused.push(json!({"webauthn": {"credentials": path(source), "fanout": fanout()}}));
    }
    // A PEM issuer file is not an agent-key allowlist.
    let pem = dir.file("pat-issuers", pat_pem().as_bytes());
    refused.push(json!({"agent_keys": path(&pem)}));
    for zero in ["max_keys", "max_distinct", "population_window"] {
        let mut bounds = fanout();
        bounds[zero] = json!(0);
        refused.push(json!({"webauthn": {"credentials": path(&creds), "fanout": bounds}}));
    }
    refused.push(json!({"sweep": {"max_bindings": 0}}));
    refused.push(json!({"sweep": {"max_bindings": 8, "window_sec": 0}}));
    refused.push(json!({"sweep": {"max_bindings": 8, "max_sessions": 0}}));
    refused.push(
        json!({"bans": {"capacity":0,"max_keys":64,"max_hits_per_key":256,
        "max_key_bytes":128,"max_payload_bytes":1048576,"gc_scan_budget":64}}),
    );
    // Owner #4: a short or non-hex token key; the attestation floor without a
    // configured token key (Go validateAttestedRoutes); the floor on root.
    let key = json!({"key_hex": "30".repeat(32)});
    refused.push(json!({"verdict_token": {"key_hex": "30".repeat(31)}}));
    refused.push(json!({"verdict_token": {"key_hex": "zz".repeat(32)}}));
    let attested = dir.file("routes", b"/checkout attested\n/health off\n");
    refused.push(json!({"routes": path(&attested)}));
    let root = dir.file("routes-root", b"/ attested\n");
    refused.push(json!({"routes": path(&root), "verdict_token": key}));
    refused.push(json!({"routes": path(&missing), "verdict_token": key}));
    let loaded = gates(json!({"routes": path(&attested), "verdict_token": key}))
        .load(false)
        .unwrap();
    assert_eq!(
        (
            loaded.readiness()["verdict_token"].clone(),
            loaded.readiness()["route_presets"].clone()
        ),
        (json!(true), json!(2))
    );
    for value in refused {
        assert!(gates(value.clone()).load(false).is_err(), "{value}");
    }
    // The admin listener owns the one ban store its routes write.
    let bans = json!({"bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,
        "max_key_bytes":128,"max_payload_bytes":1048576,"gc_scan_budget":64}});
    assert!(gates(bans.clone()).load(false).is_ok());
    assert!(gates(bans).load(true).is_err());
    // Unknown fields and a WebAuthn section without bounds do not parse.
    assert!(serde_json::from_value::<Gates>(json!({"agent_key": "x"})).is_err());
    assert!(
        serde_json::from_value::<Gates>(json!({"webauthn": {"credentials": path(&creds)}}))
            .is_err()
    );
}
