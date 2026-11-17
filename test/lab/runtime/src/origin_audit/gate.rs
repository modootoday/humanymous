//! Gate supervisor: one session of the shipped `humanymous-runtime` binary
//! over the persistent custody and WAL volumes.
//!
//! Stdin line 1 (private, from the runner):
//! `{"provision":bool,"passphrase":HEX,"history":NAME,"allow_new":bool,"trusted_head":CHECKPOINT|null}`.
//! Stdout lines: `oa_gate_ready` or `oa_gate_refused`; after a `stop` line on
//! stdin, `oa_gate_stopped` with the offline-authenticated history summary.
//! The passphrase never reaches stdout, the WAL or the runner transcript.
use super::{
    FRONTEND_PORT, GATE_LISTEN, KEYS, LIVE, NODE, ORIGIN_LOOPBACK, ORIGIN_UPSTREAM, WAL_ROOT,
    document, emit, history_name, relay,
};
use humanymous_runtime::{
    audit::{
        checkpoint::Checkpoint,
        keys::{
            KeyMaterial,
            custody::{Custody, Owner},
        },
        log::{SealedLog, SealedLogOptions, UnlockCredential},
    },
    gateway::{configuration::Configuration, startup::open_audit},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    net::{Ipv4Addr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const RUNTIME: &str = "/usr/local/bin/humanymous-runtime";
const OWNER_UID: u32 = 65532;

struct Control {
    provision: bool,
    passphrase: Zeroizing<String>,
    history: String,
    allow_new: bool,
    trusted_head: Option<Checkpoint>,
}

fn control(value: &Value) -> Result<Control> {
    let history = value["history"].as_str().ok_or("history")?.to_owned();
    if !history_name(&history) {
        return Err("history name".into());
    }
    let passphrase = value["passphrase"].as_str().ok_or("passphrase")?;
    if passphrase.len() < 32 || !passphrase.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("passphrase must be at least 32 hex digits".into());
    }
    let provision = value["provision"].as_bool().ok_or("provision")?;
    if provision && history != LIVE {
        return Err("provisioning only creates the live history".into());
    }
    Ok(Control {
        provision,
        passphrase: Zeroizing::new(passphrase.to_owned()),
        history,
        allow_new: value["allow_new"].as_bool().ok_or("allow_new")?,
        trusted_head: serde_json::from_value(value["trusted_head"].clone())?,
    })
}

pub(crate) fn private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// First boot only: fresh sealed custody and an empty live history. A missing
/// custody on a later boot is never recreated.
pub(crate) fn provision(keys: &Path, live: &Path, passphrase: &str) -> Result<()> {
    if keys.join("keys.sealed").exists() || live.exists() {
        return Err("provisioning refused: custody or history already present".into());
    }
    private_directory(live)?;
    let mut seed = Zeroizing::new([0; 32]);
    let mut hmac = Zeroizing::new([0; 32]);
    getrandom::fill(&mut seed[..])?;
    getrandom::fill(&mut hmac[..])?;
    let mut custody = Custody::acquire(keys, Owner::CurrentUser)?;
    custody.load_or_create(passphrase.as_bytes(), || {
        KeyMaterial::new(*seed, hmac.to_vec(), None, None)
    })?;
    Ok(())
}

pub(crate) struct Runtime(pub(crate) Child);
impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Start the shipped binary with the private document; `Ok(None)` when it
/// exits without readiness (a startup refusal), with its exit code.
pub(crate) fn start(
    document: &Value,
) -> Result<std::result::Result<(Runtime, Value), Option<i32>>> {
    let mut child = Command::new(RUNTIME)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    {
        let mut input = child.stdin.take().ok_or("configuration pipe")?;
        serde_json::to_writer(&mut input, document)?;
        input.flush()?;
    }
    let stdout = child.stdout.take().ok_or("readiness pipe")?;
    let mut runtime = Runtime(child);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line);
        let _ = sender.send((result.ok(), line));
    });
    match receiver.recv_timeout(Duration::from_secs(60)) {
        Ok((Some(n), line)) if n > 0 => Ok(Ok((runtime, serde_json::from_str(&line)?))),
        Ok(_) => {
            let status = runtime.0.wait()?;
            Ok(Err(status.code()))
        }
        Err(_) => Err("runtime neither ready nor exited".into()),
    }
}

/// Offline authentication of the persisted history with the same custody,
/// after the runtime has released it.
fn verify(keys: &Path, wal: &Path, control: &Control) -> Value {
    let options = SealedLogOptions {
        key_directory: keys.into(),
        wal_directory: wal.into(),
        owner: Owner::CurrentUser,
        node_id: NODE.into(),
        checkpoint_every: 1,
        allow_new_history: control.allow_new,
        trusted_head: control.trusted_head.clone(),
        require_external_witness: false,
    };
    let mut unlock = Zeroizing::new(control.passphrase.to_string());
    match SealedLog::open(&options, UnlockCredential::Utf8(&mut unlock), None) {
        Ok(log) => {
            let records = log.records().unwrap_or_default();
            let checkpoints = log.checkpoints().unwrap_or_default();
            let contiguous = records
                .iter()
                .enumerate()
                .all(|(index, record)| record.seq == index as u64 + 1);
            let summary: Vec<_> = records
                .iter()
                .map(|r| json!({"seq":r.seq,"event_type":r.event_type,"verdict":r.verdict,"enforcement_action":r.enforcement_action}))
                .collect();
            json!({"verified":true,"sequence":log.sequence(),"contiguous":contiguous,
                "checkpoints":checkpoints.len(),"head":checkpoints.last(),"records":summary})
        }
        Err(error) => json!({"verified":false,"stage":format!("{:?}",error.stage())}),
    }
}

fn session(control: &Control, input: &mut impl BufRead) -> Result<()> {
    let keys = PathBuf::from(KEYS);
    let wal = Path::new(WAL_ROOT).join(&control.history);
    if control.provision {
        provision(&keys, &wal, &control.passphrase)?;
    } else if !keys.join("keys.sealed").is_file() || !wal.is_dir() {
        emit(&json!({"role":"oa_gate_refused","runtime_started":false,
            "reason":"custody or history absent; never reprovisioned"}))?;
        return Ok(());
    }
    let document = document(
        wal.to_str().ok_or("history path")?,
        KEYS,
        &control.passphrase,
        control.allow_new,
        serde_json::to_value(&control.trusted_head)?,
        OWNER_UID,
    );
    // The backend relay exists before the runtime so no forward races it.
    relay::spawn(
        TcpListener::bind(ORIGIN_LOOPBACK)?,
        ORIGIN_UPSTREAM.to_owned(),
    );
    let (runtime, readiness) = match start(&document)? {
        Ok(ready) => ready,
        Err(code) => {
            // Classify with the same startup function the binary runs.
            let mut typed: Configuration = serde_json::from_value(document)?;
            let refusal = open_audit(&mut typed).err();
            emit(
                &json!({"role":"oa_gate_refused","runtime_started":true,"runtime_exit":code,
                "audit_refusal":refusal,"history":control.history}),
            )?;
            return Ok(());
        }
    };
    drop(document);
    if readiness["listen"] != GATE_LISTEN {
        return Err("runtime listener differs from the configured loopback address".into());
    }
    relay::spawn(
        TcpListener::bind((Ipv4Addr::UNSPECIFIED, FRONTEND_PORT))?,
        GATE_LISTEN.to_owned(),
    );
    emit(
        &json!({"role":"oa_gate_ready","readiness":readiness,"history":control.history,
        "frontend":format!("0.0.0.0:{FRONTEND_PORT}"),"backend":ORIGIN_UPSTREAM,
        "binary":RUNTIME}),
    )?;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Err("control channel closed without stop".into());
        }
        if line.trim() == "stop" {
            break;
        }
    }
    drop(runtime);
    let mut stopped = verify(&keys, &wal, control);
    stopped["role"] = json!("oa_gate_stopped");
    stopped["history"] = json!(control.history);
    emit(&stopped)?;
    Ok(())
}

pub fn run() -> Result<()> {
    let mut input = BufReader::new(io::stdin());
    let control = control(&super::read_line(&mut input, 65536)?)?;
    session(&control, &mut input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(provision: bool, history: &str) -> Value {
        json!({"provision":provision,"passphrase":"ab".repeat(16),"history":history,
            "allow_new":provision,"trusted_head":null})
    }

    #[test]
    fn control_refuses_reprovisioning_cases_weak_secrets_and_bad_names() {
        assert!(control(&value(true, LIVE)).is_ok());
        assert!(control(&value(false, "case-forge")).is_ok());
        assert!(control(&value(true, "case-forge")).is_err());
        assert!(control(&value(false, "../keys")).is_err());
        let mut weak = value(false, LIVE);
        weak["passphrase"] = json!("short");
        assert!(control(&weak).is_err());
        let mut head = value(false, LIVE);
        head["trusted_head"] = json!({"tree_size":"x"});
        assert!(control(&head).is_err());
    }

    #[test]
    fn provisioning_never_replaces_existing_custody() {
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let root = std::env::temp_dir().join(format!("origin-audit-gate-{name}"));
        private_directory(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let keys = root.join("keys");
        private_directory(&keys).unwrap();
        let passphrase = "cd".repeat(16);
        provision(&keys, &root.join(LIVE), &passphrase).unwrap();
        assert!(keys.join("keys.sealed").is_file());
        let before = fs::read(keys.join("keys.sealed")).unwrap();
        assert!(provision(&keys, &root.join("other"), &passphrase).is_err());
        assert_eq!(fs::read(keys.join("keys.sealed")).unwrap(), before);
        fs::remove_dir_all(&root).unwrap();
    }
}
