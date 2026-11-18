//! Ports of legacy internal/correlation/correlation_shared_test.go (oracle
//! 981d25c) against an in-process RESP coordinator with scripted replies.
use super::*;
use crate::gateway::coordinator::redis::{ClientSettings, read_reply};
use crate::network::correlation::{Config, Limits, Registry, Scope, key_digest};
use crate::network::history::anomaly;
use std::{
    io::{BufReader, Write},
    net::TcpListener,
    sync::Mutex,
    thread,
};

const T0: Duration = Duration::from_secs(1_700_000_000);
const WINDOW: Duration = Duration::from_secs(120);
const TTL: Duration = Duration::from_secs(3600);

/// Scripted reply shape (Go `fakeDoer`).
#[derive(Clone, Copy)]
enum Mode {
    /// `[1, velocity]`, velocity from the script or 1 once it runs out.
    Healthy,
    /// A one-element array (the broken-script case).
    Malformed,
    /// A two-element array whose velocity is a bulk string.
    NonInteger,
}

struct Script {
    mode: Mode,
    velocities: Vec<i64>,
    commands: Vec<Vec<Vec<u8>>>,
}

type Shared = Arc<Mutex<Script>>;

fn reply(script: &mut Script, args: Vec<Vec<u8>>) -> Vec<u8> {
    let call = script.commands.len();
    script.commands.push(args);
    match script.mode {
        Mode::Healthy => {
            let velocity = script.velocities.get(call).copied().unwrap_or(1);
            format!("*2\r\n:1\r\n:{velocity}\r\n").into_bytes()
        }
        Mode::Malformed => b"*1\r\n:1\r\n".to_vec(),
        Mode::NonInteger => b"*2\r\n:1\r\n$1\r\n9\r\n".to_vec(),
    }
}

fn coordinator(mode: Mode, velocities: Vec<i64>) -> (Arc<Client>, Shared) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let shared = Arc::new(Mutex::new(Script {
        mode,
        velocities,
        commands: Vec::new(),
    }));
    let state = shared.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let state = state.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut writer = stream;
                while let Ok(Reply::Array(items)) = read_reply(&mut reader, 0) {
                    let args = items
                        .into_iter()
                        .map(|item| match item {
                            Reply::Bulk(bytes) => bytes,
                            _ => Vec::new(),
                        })
                        .collect();
                    let bytes = reply(&mut state.lock().unwrap(), args);
                    if writer.write_all(&bytes).is_err() {
                        return;
                    }
                }
            });
        }
    });
    (
        Arc::new(Client::new(address, ClientSettings::default())),
        shared,
    )
}

/// Client whose coordinator is down for the whole test (Go `ErrBreakerOpen`).
fn unreachable() -> Arc<Client> {
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    Arc::new(Client::new(
        address.to_string(),
        ClientSettings {
            cooldown: Duration::from_secs(60),
            ..ClientSettings::default()
        },
    ))
}

fn registry() -> Registry {
    Registry::new(
        Config::new(
            TTL,
            Limits::new(1000, 1000, 1000, 256, 1_000_000).unwrap(),
            anomaly::Limits::new(16, 256, 256, 1_000_000).unwrap(),
        )
        .unwrap(),
    )
}

fn request(key: &str, subnet: &str, now: Duration) -> CoordinatorRequest {
    CoordinatorRequest {
        digest: key_digest(&[], key),
        subnet: subnet.into(),
        now,
        window: WINDOW,
        ttl: TTL,
    }
}

/// Go `Registry.Observe` with a configured coordinator: the fleet step first,
/// then the local registry with its projected reply.
fn observe(
    registry: &mut Registry,
    fleet: &mut SharedCorrelation,
    key: &str,
    subnet: &str,
    session: &str,
    now: Duration,
) -> crate::network::correlation::Outcome {
    let input = fleet.fleet_input(&request(key, subnet, now));
    registry.observe(key, subnet, session, now, input).unwrap()
}

fn velocity_fired(outcome: &crate::network::correlation::Outcome) -> bool {
    outcome
        .signals
        .iter()
        .any(|signal| signal.id == "l5.correlation.ip_velocity")
}

#[test]
fn shared_eval_command_contract() {
    let (client, script) = coordinator(Mode::Healthy, Vec::new());
    let mut fleet = SharedCorrelation::new(client);
    let mut registry = registry();
    observe(&mut registry, &mut fleet, "fp|ja4", "10.0.0", "s1", T0);

    let script = script.lock().unwrap();
    assert_eq!(script.commands.len(), 1, "exactly one EVAL");
    let digest = key_digest(&[], "fp|ja4");
    let call = &script.commands[0];
    assert_eq!(call[0], b"EVAL");
    assert_eq!(call[1], FLEET_CORRELATION_SCRIPT.as_bytes());
    assert_eq!(call[2], b"2");
    assert_eq!(call[3], format!("hmn:corr:sub:{digest}").as_bytes());
    assert_eq!(call[4], format!("hmn:corr:vel:{digest}").as_bytes());
    assert_eq!(call[5], b"1700000000000000000");
    assert_eq!(call[6], b"120000000000");
    assert_eq!(call[7], b"3600000");
    assert_eq!(call.last().unwrap(), b"10.0.0", "last ARGV is the subnet");
    assert!(!call.iter().any(|arg| arg.as_slice() == b"fp|ja4"));
}

#[test]
fn shared_fleet_count_drives_velocity() {
    let mut velocities = vec![1; 40];
    velocities.push(50);
    let (client, _) = coordinator(Mode::Healthy, velocities);
    let mut fleet = SharedCorrelation::new(client);
    let mut registry = registry();
    let mut fired = false;
    for index in 0..=40 {
        let outcome = observe(
            &mut registry,
            &mut fleet,
            &format!("fp-{index}|ja4"),
            &format!("10.{index}.0"),
            &format!("s-{index}"),
            T0,
        );
        assert_eq!(outcome.scope, Scope::CoordinatorReportedUnqualified);
        fired |= velocity_fired(&outcome);
    }
    assert!(fired, "a high fleet velocity must fire ip_velocity");
    assert_eq!(registry.fleet_fallbacks(), 0);
}

#[test]
fn shared_coordinator_outage_falls_back_to_local() {
    let mut fleet = SharedCorrelation::new(unreachable());
    let mut registry = registry();
    for index in 0..40 {
        observe(
            &mut registry,
            &mut fleet,
            &format!("org-{index}|ja4"),
            &format!("10.{index}.0"),
            &format!("s-{index}"),
            T0,
        );
    }
    let mut fired = false;
    for index in 0..30u64 {
        let outcome = observe(
            &mut registry,
            &mut fleet,
            "bot|ja4",
            &format!("172.16.{index}"),
            &format!("b-{index}"),
            T0 + Duration::from_secs(index),
        );
        assert_eq!(outcome.scope, Scope::LocalDegraded);
        fired |= velocity_fired(&outcome);
    }
    assert!(fired, "an outage must fall back to per-process velocity");
    assert_eq!(registry.fleet_fallbacks(), 70);
}

#[test]
fn shared_fleet_fallback_counter() {
    let observe_n = |fleet: Option<&mut SharedCorrelation>| {
        let mut registry = registry();
        let mut fleet = fleet;
        for index in 0..20 {
            let key = format!("fp-{index}|ja4");
            let subnet = format!("10.{index}.0");
            let input = match fleet.as_deref_mut() {
                Some(fleet) => fleet.fleet_input(&request(&key, &subnet, T0)),
                None => FleetInput::LocalOnly,
            };
            registry
                .observe(&key, &subnet, &format!("s-{index}"), T0, input)
                .unwrap();
        }
        registry.fleet_fallbacks()
    };
    let (healthy, _) = coordinator(Mode::Healthy, Vec::new());
    assert_eq!(observe_n(Some(&mut SharedCorrelation::new(healthy))), 0);
    assert_eq!(
        observe_n(Some(&mut SharedCorrelation::new(unreachable()))),
        20
    );
    let (malformed, script) = coordinator(Mode::Malformed, Vec::new());
    assert_eq!(observe_n(Some(&mut SharedCorrelation::new(malformed))), 20);
    assert_eq!(script.lock().unwrap().commands.len(), 20);
    assert_eq!(observe_n(None), 0, "per-process registry never falls back");
}

#[test]
fn non_integer_replies_and_raw_keys_are_unavailable() {
    let (client, script) = coordinator(Mode::NonInteger, Vec::new());
    let mut fleet = SharedCorrelation::new(client);
    assert_eq!(
        fleet.observe(&request("fp|ja4", "10.0.0", T0)),
        Err(CoordinatorUnavailable)
    );
    assert!(matches!(
        fleet.fleet_input(&request("fp|ja4", "10.0.0", T0)),
        FleetInput::Unavailable
    ));
    assert_eq!(script.lock().unwrap().commands.len(), 2);

    let (client, script) = coordinator(Mode::Healthy, Vec::new());
    let mut fleet = SharedCorrelation::new(client);
    let digest = key_digest(&[], "fp|ja4");
    for raw in [
        "fp|ja4".to_owned(),
        String::new(),
        digest.to_uppercase(),
        format!("{digest}0"),
        digest[..63].to_owned(),
    ] {
        let refused = CoordinatorRequest {
            digest: raw,
            ..request("fp|ja4", "10.0.0", T0)
        };
        assert_eq!(fleet.observe(&refused), Err(CoordinatorUnavailable));
    }
    assert!(
        script.lock().unwrap().commands.is_empty(),
        "no I/O on refusal"
    );
    assert_eq!(
        fleet.observe(&request("fp|ja4", "10.0.0", T0)),
        Ok(vec![1, 1])
    );
}
