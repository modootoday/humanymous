//! Native loopback component run of the TLS client against the runtime
//! library's own launch path (`read_launch` + `ServiceHost::bind_tls`, exactly
//! what the shipped binary's `main` does), with real sealed custody, an
//! offline-authenticated WAL and a counting origin. It qualifies the client,
//! the row expectations and the launch path only; the product result is the
//! Docker topology stage, never this loopback test.
use super::{CAPTURE, GATE_CATALOG, GATE_EDGE, SERVER_NAME, client, envelope, pki};
use crate::attack_catalog::{client::Operator, document, gate::verify};
use crate::origin_audit::gate::{private_directory, provision};
use humanymous_defender::lab::LabError;
use humanymous_runtime::{
    audit::checkpoint::Checkpoint,
    gateway::service::{Launch, ServiceHost, ShutdownRequest, read_launch},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    os::unix::fs::MetadataExt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

struct Running {
    stop: tokio::sync::watch::Sender<Option<ShutdownRequest>>,
    thread: JoinHandle<Result<(), String>>,
}

struct InProcess {
    keys: PathBuf,
    wal: PathBuf,
    passphrase: String,
    head: Option<Checkpoint>,
    pki: pki::LabPki,
    listen: SocketAddr,
    origin: SocketAddr,
    uid: u32,
    contacts: Arc<AtomicU64>,
    contacts_at_start: u64,
    previous: Vec<Value>,
    running: Option<Running>,
}

impl InProcess {
    fn start(&mut self) -> Result<(), String> {
        let mut document = document(
            self.wal.to_str().unwrap(),
            self.keys.to_str().unwrap(),
            &self.passphrase,
            self.head.is_none(),
            serde_json::to_value(&self.head).unwrap(),
            self.uid,
        );
        document["listen"] = json!(self.listen);
        document["origin"] = json!(self.origin);
        let bytes = serde_json::to_vec(&envelope(document, &self.pki.chain, &self.pki.key))
            .map_err(|e| e.to_string())?;
        let Launch::Tls { configuration, tls } =
            read_launch(bytes.as_slice()).map_err(|e| e.to_string())?
        else {
            return Err("plain launch".into());
        };
        let (stop, receiver) = tokio::sync::watch::channel(None);
        let (ready, readiness) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            runtime.block_on(async move {
                let host = ServiceHost::bind_tls(configuration, None, None, None, None, tls)
                    .await
                    .map_err(|e| e.to_string())?;
                let _ = ready.send(serde_json::to_value(host.readiness()).unwrap());
                host.run(receiver)
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
        });
        let readiness = readiness
            .recv_timeout(Duration::from_secs(60))
            .map_err(|_| "no readiness".to_string())?;
        assert_eq!(readiness["profile"]["raw_tls"], true);
        assert_eq!(readiness["listen"], json!(self.listen));
        self.running = Some(Running { stop, thread });
        Ok(())
    }

    fn stop(&mut self) -> Value {
        if let Some(running) = self.running.take() {
            let _ = running.stop.send(Some(ShutdownRequest::new(200).unwrap()));
            running.thread.join().unwrap().unwrap();
        }
        let (summary, head) = verify(&self.keys, &self.wal, &self.passphrase, self.head.clone());
        if head.is_some() {
            self.head = head;
        }
        summary
    }
}

impl Operator for InProcess {
    fn call(&mut self, message: &Value) -> Result<Value, LabError> {
        Ok(match message["op"].as_str() {
            Some("start") => {
                self.contacts_at_start = self.contacts.load(Ordering::SeqCst);
                json!({"op":"started","ok":self.start().is_ok()})
            }
            Some("restart") => {
                let stopped = self.stop();
                let ok = stopped["verified"] == true && self.start().is_ok();
                json!({"op":"restarted","ok":ok})
            }
            Some("stop") => {
                let stopped = self.stop();
                let records = stopped["records"].as_array().cloned().unwrap_or_default();
                let sound = stopped["verified"] == true
                    && records.len() >= self.previous.len()
                    && records[..self.previous.len()] == self.previous[..];
                let delta = sound.then(|| {
                    json!({"origin_effects":self.contacts.load(Ordering::SeqCst) - self.contacts_at_start,
                        "audit":records[self.previous.len()..]})
                });
                if sound {
                    self.previous = records;
                }
                json!({"op":"effects","effects":delta})
            }
            _ => return Err(LabError("unexpected harness op")),
        })
    }
}

fn origin(contacts: Arc<AtomicU64>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            contacts.fetch_add(1, Ordering::SeqCst);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut buffer = vec![0; 65_536];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\norigin-ok",
            );
        }
    });
    address
}

fn free_port() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

fn scratch() -> PathBuf {
    let mut nonce = [0; 8];
    getrandom::fill(&mut nonce).unwrap();
    let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let root = std::env::temp_dir().join(format!("gate-tls-{name}"));
    private_directory(&root).unwrap();
    root.canonicalize().unwrap()
}

fn row<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap()
}

#[test]
fn tls_rows_against_the_runtime_launch_path_on_loopback() {
    let root = scratch();
    let keys = root.join("keys");
    private_directory(&keys).unwrap();
    let wal = root.join("wal");
    let passphrase = "ab".repeat(24);
    provision(&keys, &wal, &passphrase).unwrap();
    let contacts = Arc::new(AtomicU64::new(0));
    let origin_address = origin(contacts.clone());
    let unused_origin = free_port();
    let mut harness = InProcess {
        uid: std::fs::metadata(&root).unwrap().uid(),
        keys,
        wal,
        passphrase,
        head: None,
        pki: pki::issue(&root.join("tls"), SERVER_NAME).unwrap(),
        listen: free_port(),
        origin: origin_address,
        contacts,
        contacts_at_start: 0,
        previous: Vec::new(),
        running: None,
    };
    let ca = harness.pki.ca_pem.clone();
    let leaf = harness.pki.leaf_sha256.clone();
    let listen = harness.listen;
    let report = client::report(&mut harness, listen, unused_origin, &ca);
    for row in report["rows"].as_array().unwrap() {
        eprintln!(
            "{} status={} match={} reason={} steps={} effects={}",
            row["id"],
            row["status"],
            row["expectation_matches"],
            row["reason"],
            row["steps"],
            row["effects"]
        );
    }
    let rows = GATE_CATALOG.len() + GATE_EDGE.len() + CAPTURE.len();
    assert_eq!(report["totals"]["scheduled"], rows);
    assert_eq!(report["presented_certificates"], json!([leaf]));
    assert_eq!(report["origin_direct_refused"], true);
    assert_eq!(
        row(&report, "tls-h2-required-original-h2")["steps"][1]["http_status"],
        403
    );
    assert_eq!(report["scoped_passed"], true, "{report}");
    std::fs::remove_dir_all(&root).unwrap();
}
