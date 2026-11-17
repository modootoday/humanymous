//! Gate supervisor: provisions sealed custody once, then runs one session of
//! the shipped `humanymous-runtime` binary per runner command.
//!
//! Stdin line 1 (private, from the runner): `{"passphrase":HEX}`.
//! Then one command per line: `start`, `restart`, `stop` or `exit`.
//! Stdout: `ac_gate_provisioned`; `ac_gate_ready` or `ac_gate_refused` after
//! `start`/`restart`; `ac_gate_stopped` with the offline-authenticated history
//! after `stop`. The passphrase never reaches stdout or the WAL.
//!
//! `--gate-tls` runs the same supervisor behind TLS terminated by the runtime
//! itself (roadmap 2a): an ephemeral lab CA is generated in this container,
//! the runtime receives the closed TLS launch envelope with the provisioned
//! server pair, and the ingress relay carries TCP bytes unchanged (L4
//! passthrough) so the runtime observes the original client's TLS and H2.
use super::{FRONTEND_PORT, GATE_LISTEN, NODE, ORIGIN_LOOPBACK, ORIGIN_UPSTREAM, document};
use crate::gate_tls::{self, pki::LabPki};
use crate::origin_audit::{
    emit,
    gate::{Runtime, private_directory, provision, start},
    read_line, relay,
};
use humanymous_runtime::audit::{
    checkpoint::Checkpoint,
    keys::custody::Owner,
    log::{SealedLog, SealedLogOptions, UnlockCredential},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{self, BufRead, BufReader},
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const RUNTIME: &str = "/usr/local/bin/humanymous-runtime";
const ROOT: &str = "/tmp/attack-gate";
const OWNER_UID: u32 = 65532;

fn digest(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

fn passphrase(control: &Value) -> Result<Zeroizing<String>> {
    let value = control["passphrase"].as_str().ok_or("passphrase")?;
    if value.len() < 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("passphrase must be at least 32 hex digits".into());
    }
    Ok(Zeroizing::new(value.to_owned()))
}

/// Offline authentication of the persisted history with the same custody,
/// after the runtime released it. Records are projected exactly as the
/// owned-child lab projects them (`humanymous_defender::lab::AuditEvent`).
pub(crate) fn verify(
    keys: &Path,
    wal: &Path,
    passphrase: &str,
    trusted_head: Option<Checkpoint>,
) -> (Value, Option<Checkpoint>) {
    let options = SealedLogOptions {
        key_directory: keys.into(),
        wal_directory: wal.into(),
        owner: Owner::CurrentUser,
        node_id: NODE.into(),
        checkpoint_every: 1,
        allow_new_history: trusted_head.is_none(),
        trusted_head,
        require_external_witness: false,
    };
    let mut unlock = Zeroizing::new(passphrase.to_owned());
    match SealedLog::open(&options, UnlockCredential::Utf8(&mut unlock), None) {
        Ok(log) => {
            let records = log.records().unwrap_or_default();
            let head = log.checkpoints().ok().and_then(|c| c.last().cloned());
            let contiguous = records
                .iter()
                .enumerate()
                .all(|(index, record)| record.seq == index as u64 + 1);
            let projected: Vec<_> = records
                .into_iter()
                .map(|r| {
                    json!({"seq":r.seq,"event_type":r.event_type,"verdict":r.verdict,
                    "risk_score":r.risk_score,"rules":r.triggered_rules.unwrap_or_default(),
                    "record_hash":r.record_hash})
                })
                .collect();
            let anchored = head.as_ref().map(|h| h.tree_size) == Some(log.sequence());
            (
                json!({"verified":true,"sequence":log.sequence(),"contiguous":contiguous,
                    "anchored":anchored,"records":projected}),
                head,
            )
        }
        Err(error) => (
            json!({"verified":false,"stage":format!("{:?}",error.stage())}),
            None,
        ),
    }
}

struct Supervisor {
    keys: PathBuf,
    wal: PathBuf,
    passphrase: Zeroizing<String>,
    head: Option<Checkpoint>,
    runtime: Option<Runtime>,
    sessions: u64,
    runtime_sha256: String,
    tls: Option<LabPki>,
}

impl Supervisor {
    /// Start one runtime session over the existing custody and history.
    fn start(&mut self, restarted: bool) -> Result<Value> {
        let document = document(
            self.wal.to_str().ok_or("history path")?,
            self.keys.to_str().ok_or("custody path")?,
            &self.passphrase,
            self.head.is_none(),
            serde_json::to_value(&self.head)?,
            OWNER_UID,
        );
        let launch = match &self.tls {
            Some(pki) => gate_tls::envelope(document, &pki.chain, &pki.key),
            None => document,
        };
        let started = start(&launch)?;
        drop(launch);
        let (runtime, readiness) = match started {
            Ok(ready) => ready,
            Err(code) => {
                return Ok(json!({"role":"ac_gate_refused","runtime_exit":code}));
            }
        };
        if readiness["listen"] != GATE_LISTEN {
            return Err("runtime listener differs from the configured loopback address".into());
        }
        // The shipped binary reports which ingress it actually bound.
        if (readiness["profile"]["raw_tls"] == true) != self.tls.is_some() {
            return Err("runtime ingress profile differs from the launch document".into());
        }
        let process_sha256 = digest(Path::new(&format!("/proc/{}/exe", runtime.0.id())))?;
        self.runtime = Some(runtime);
        self.sessions += 1;
        Ok(
            json!({"role":"ac_gate_ready","session":self.sessions,"restarted":restarted,
            "readiness":readiness,"runtime_sha256":self.runtime_sha256,
            "process_sha256":process_sha256,"binary":RUNTIME,"tls":self.tls.is_some()}),
        )
    }

    /// Stop the running session (if any) and authenticate the history.
    fn stop(&mut self) -> Value {
        let running = self.runtime.take().is_some();
        let (mut summary, head) =
            verify(&self.keys, &self.wal, &self.passphrase, self.head.clone());
        if head.is_some() {
            self.head = head;
        }
        summary["role"] = json!("ac_gate_stopped");
        summary["was_running"] = json!(running);
        summary
    }

    fn command(&mut self, command: &str) -> Result<Option<Value>> {
        Ok(Some(match command {
            "start" if self.runtime.is_none() => self.start(false)?,
            "restart" if self.runtime.is_some() => {
                let stopped = self.stop();
                if stopped["verified"] != true {
                    json!({"role":"ac_gate_refused","stopped":stopped})
                } else {
                    self.start(true)?
                }
            }
            "stop" => self.stop(),
            "exit" => return Ok(None),
            _ => return Err("unexpected Gate command".into()),
        }))
    }
}

/// Plain HTTP/1.1 ingress (roadmap 2 attack catalog).
pub fn run() -> Result<()> {
    supervise(false)
}

/// Runtime-terminated TLS ingress (roadmap 2a full Gate TLS/H2/capture).
pub fn run_tls() -> Result<()> {
    supervise(true)
}

fn supervise(tls: bool) -> Result<()> {
    let mut input = BufReader::new(io::stdin());
    let passphrase = passphrase(&read_line(&mut input, 4096)?)?;
    let root = PathBuf::from(ROOT);
    private_directory(&root)?;
    let keys = root.join("keys");
    private_directory(&keys)?;
    let wal = root.join("wal");
    provision(&keys, &wal, &passphrase)?;
    let pki = if tls {
        Some(gate_tls::pki::issue(
            &root.join("tls"),
            gate_tls::SERVER_NAME,
        )?)
    } else {
        None
    };
    let (frontend, backend) = if tls {
        (gate_tls::FRONTEND_PORT, gate_tls::ORIGIN_UPSTREAM)
    } else {
        (FRONTEND_PORT, ORIGIN_UPSTREAM)
    };
    // The backend relay exists before any runtime so no forward races it; the
    // ingress relay closes a client while no runtime session listens.
    relay::spawn(TcpListener::bind(ORIGIN_LOOPBACK)?, backend.to_owned());
    relay::spawn(
        TcpListener::bind((Ipv4Addr::UNSPECIFIED, frontend))?,
        GATE_LISTEN.to_owned(),
    );
    let mut supervisor = Supervisor {
        keys,
        wal,
        passphrase,
        head: None,
        runtime: None,
        sessions: 0,
        runtime_sha256: digest(Path::new(RUNTIME))?,
        tls: pki,
    };
    // Public material only: the CA certificate and the server certificate digest.
    let (ca_pem, leaf_sha256) = supervisor
        .tls
        .as_ref()
        .map_or((Value::Null, Value::Null), |pki| {
            (json!(pki.ca_pem), json!(pki.leaf_sha256))
        });
    emit(
        &json!({"role":"ac_gate_provisioned","runtime_sha256":supervisor.runtime_sha256,
        "frontend":format!("0.0.0.0:{frontend}"),"backend":backend,
        "tls":tls,"ca_pem":ca_pem,"leaf_sha256":leaf_sha256}),
    )?;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Err("control channel closed without exit".into());
        }
        match supervisor.command(line.trim())? {
            Some(reply) => emit(&reply)?,
            None => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrase_must_be_long_hex() {
        assert!(passphrase(&json!({"passphrase":"ab".repeat(16)})).is_ok());
        assert!(passphrase(&json!({"passphrase":"short"})).is_err());
        assert!(passphrase(&json!({"passphrase":"zz".repeat(16)})).is_err());
        assert!(passphrase(&json!({})).is_err());
    }

    #[test]
    fn offline_verification_of_a_fresh_history_and_wrong_passphrase() {
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let root = std::env::temp_dir().join(format!("attack-gate-{name}"));
        private_directory(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let keys = root.join("keys");
        private_directory(&keys).unwrap();
        let wal = root.join("wal");
        let secret = "cd".repeat(16);
        provision(&keys, &wal, &secret).unwrap();
        let (summary, _) = verify(&keys, &wal, &secret, None);
        assert_eq!(summary["verified"], true, "{summary}");
        assert_eq!(summary["contiguous"], true);
        let (refused, head) = verify(&keys, &wal, &"ef".repeat(16), None);
        assert_eq!(refused["verified"], false);
        assert!(head.is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn commands_are_state_checked() {
        let mut supervisor = Supervisor {
            keys: "/nonexistent/keys".into(),
            wal: "/nonexistent/wal".into(),
            passphrase: Zeroizing::new("ab".repeat(16)),
            head: None,
            runtime: None,
            sessions: 0,
            runtime_sha256: String::new(),
            tls: None,
        };
        assert!(supervisor.command("restart").is_err());
        assert!(supervisor.command("bogus").is_err());
        assert!(supervisor.command("exit").unwrap().is_none());
        let stopped = supervisor.command("stop").unwrap().unwrap();
        assert_eq!(stopped["role"], "ac_gate_stopped");
        assert_eq!(stopped["verified"], false);
        assert_eq!(stopped["was_running"], false);
    }
}
