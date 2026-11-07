//! Docker-only owned loopback process pilot. No real-browser qualification.
use crate::{client::*, installation::Installation, process};
use humanymous_runtime::audit::{
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use humanymous_runtime::gateway::configuration::Configuration;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};

pub fn run() {
    let installation = Installation::new(true).unwrap();
    let directory = installation.directory.clone();
    let key_directory = installation.key_directory.clone();
    let origin = &installation.origin;
    let config = installation.config.clone();
    let mut wrong = config.clone();
    wrong["audit"]["keys"]["passphrase"] = json!("wrong-fixture");
    process::Runtime::reject(&wrong);
    let missing = directory.join("missing-custody");
    fs::create_dir(&missing).unwrap();
    fs::set_permissions(&missing, fs::Permissions::from_mode(0o700)).unwrap();
    let mut absent = config.clone();
    absent["audit"]["keys"]["directory"] = json!(missing);
    process::Runtime::reject(&absent);
    assert!(!missing.join("keys.sealed").exists());
    assert!(!missing.join("keys.pending").exists());
    let typed: Configuration = serde_json::from_value(config.clone()).unwrap();
    let (runtime, address) = process::Runtime::start(&config);
    assert!(
        humanymous_runtime::audit::keys::custody::Custody::acquire(
            &key_directory,
            humanymous_runtime::audit::keys::custody::Owner::CurrentUser
        )
        .is_err(),
        "runtime lost lifetime custody"
    );
    process::Runtime::reject(&config);

    let issued = issue(address);
    let mut collect = request("/_humanymous/collect", report(&issued));
    sign(&typed, &mut collect, &issued, 1);
    expect(address, &collect, 200);
    let mut effect = request("/effect", b"effect".to_vec());
    sign(&typed, &mut effect, &issued, 2);
    let response = expect(address, &effect, 200);
    assert_eq!(
        String::from_utf8(response)
            .unwrap()
            .matches("Set-Cookie:")
            .count(),
        2
    );
    expect(address, &effect, 403); // Replay cannot dispatch again.
    let mut altered = request("/effect", b"effect".to_vec());
    sign(&typed, &mut altered, &issued, 3);
    altered.body[0] = b'X';
    expect(address, &altered, 403);
    let mut valid = request("/effect", b"effect".to_vec());
    sign(&typed, &mut valid, &issued, 3);
    expect(address, &valid, 200);
    expect(address, &request("/effect", b"effect".to_vec()), 401);
    expect(address, &request("/tls", vec![]), 422);
    expect(address, &request("/_humanymous/admin", vec![]), 403);
    let smuggle=b"POST /effect HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n";
    assert!(exchange(address, smuggle).starts_with(b"HTTP/1.1 400 "));
    let uncollected = issue(address);
    let mut missing = request("/effect", b"effect".to_vec());
    sign(&typed, &mut missing, &uncollected, 1);
    expect(address, &missing, 403);
    assert_eq!(origin.effects(), 2);
    drop(runtime);
    let log = Log::open(
        Wal::open(&directory, WalOptions::default()).unwrap(),
        Config {
            node_id: "process-pilot".into(),
            hmac_key: installation.hmac.to_vec(),
            signing_seed: installation.seed,
            checkpoint_every: 1,
            witness: None,
        },
        None,
        false,
    )
    .unwrap();
    let records = log.records().unwrap();
    assert!(records.len() >= 4);
    let serialized = serde_json::to_string(&records).unwrap();
    for secret in [
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        "Mozilla/5.0",
        "effect",
    ] {
        assert!(!serialized.contains(secret), "raw audit data");
    }
    let sequence = log.sequence();
    drop(log);
    let (restarted, new_address) = process::Runtime::start(&config);
    let mut stale = request("/effect", b"effect".to_vec());
    sign(&typed, &mut stale, &issued, 4);
    expect(new_address, &stale, 401);
    assert_eq!(origin.effects(), 2);
    drop(restarted);
    let sealed = key_directory.join("keys.sealed");
    let mut bytes = fs::read(&sealed).unwrap();
    bytes[0] = b'!';
    fs::write(&sealed, &bytes).unwrap();
    process::Runtime::reject(&config);
    assert_eq!(
        fs::read(&sealed).unwrap(),
        bytes,
        "tamper replaced identity"
    );

    let result: Value = json!({"schema":"humanymous.runtime-process-pilot/1","status":"passed","origin_effects":2,"audit_records":sequence,
        "scope":"real binary/session/RIT/durable audit/owned loopback origin","full_runtime_qualified":false,"browser_or_human_executed":false,
        "checks":["private startup","sealed custody wrong password","missing custody no replacement","lifetime custody exclusion","tampered custody no replacement","collect","valid origin","duplicate cookies","credentials stripped","replay","body mutation","counter not consumed on bad signature","missing session","unsupported TLS","reserved admin","ambiguous framing","missing report","authenticated audit recovery","minimized audit","restart invalidates session"],
        "cli_binary_executed":true,"cli_security_platform":"linux",
        "cli_binary_checks":["doctor","dry-run no writes","install","repeat","rollback","digest refusal","mutation refuses rollback"],
        "not_qualified":["browser collector","TLS/H2 capture","Pass recovery","assistive technology","independent origin isolation","independent witness"]});
    println!("{result}");
}
