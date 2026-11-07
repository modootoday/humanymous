//! Per-browser-namespace deny-only DNS with paired outside canary controls.
use crate::dns_wire;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io,
    net::{Ipv4Addr, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Default)]
struct Counts {
    received: u64,
    invalid: u64,
    questions: BTreeMap<String, u64>,
}
pub struct DenyResolver {
    stop: Arc<AtomicBool>,
    counts: Arc<Mutex<Counts>>,
    worker: Option<JoinHandle<io::Result<()>>>,
}
impl DenyResolver {
    pub fn start() -> io::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:53")?;
        socket.set_read_timeout(Some(Duration::from_millis(150)))?;
        let stop = Arc::new(AtomicBool::new(false));
        let counts = Arc::new(Mutex::new(Counts::default()));
        let done = stop.clone();
        let observed = counts.clone();
        let worker = std::thread::Builder::new()
            .name("owned-deny-dns".into())
            .spawn(move || {
                while !done.load(Ordering::Acquire) {
                    let mut packet = [0; 513];
                    match socket.recv_from(&mut packet) {
                        Ok((length, peer)) => {
                            let mut counts = observed
                                .lock()
                                .map_err(|_| io::Error::other("DNS counter poisoned"))?;
                            counts.received = counts
                                .received
                                .checked_add(1)
                                .ok_or_else(|| io::Error::other("DNS count exhausted"))?;
                            if let Ok(name) = dns_wire::question(&packet[..length]) {
                                let hash = format!("{:x}", Sha256::digest(name.as_bytes()));
                                if counts.questions.len() >= 64
                                    && !counts.questions.contains_key(&hash)
                                {
                                    return Err(io::Error::other("DNS inventory bound"));
                                }
                                *counts.questions.entry(hash).or_default() += 1;
                            } else {
                                counts.invalid =
                                    counts.invalid.checked_add(1).ok_or_else(|| {
                                        io::Error::other("DNS invalid count exhausted")
                                    })?;
                            }
                            drop(counts);
                            if let Ok(reply) = dns_wire::reply(&packet[..length], 5) {
                                socket.send_to(&reply, peer)?;
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                            ) => {}
                        Err(error) => return Err(error),
                    }
                }
                Ok(())
            })?;
        Ok(Self {
            stop,
            counts,
            worker: Some(worker),
        })
    }
    pub fn snapshot(&self) -> io::Result<Value> {
        let counts = self
            .counts
            .lock()
            .map_err(|_| io::Error::other("DNS counter poisoned"))?;
        Ok(
            json!({"received":counts.received,"invalid":counts.invalid,"question_hash_counts":counts.questions,"forwards":0}),
        )
    }
    pub fn finish(mut self) -> io::Result<Value> {
        self.stop.store(true, Ordering::Release);
        self.worker
            .take()
            .ok_or_else(|| io::Error::other("DNS owner absent"))?
            .join()
            .map_err(|_| io::Error::other("DNS worker panic"))??;
        self.snapshot()
    }
}
impl Drop for DenyResolver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub async fn probe(resolver: &DenyResolver, name: &str, outside: Ipv4Addr) -> io::Result<Value> {
    let packet = dns_wire::query(name).map_err(io::Error::other)?;
    if !outside.is_private() {
        return Err(io::Error::other("owned external DNS canary IP required"));
    }
    let hash = format!("{:x}", Sha256::digest(name.as_bytes()));
    let hostname = format!("{name}.");
    let resolved = tokio::time::timeout(
        Duration::from_secs(4),
        tokio::task::spawn_blocking(move || {
            (hostname.as_str(), 80)
                .to_socket_addrs()
                .map(|values| values.count())
        }),
    )
    .await
    .map_err(|_| io::Error::other("DNS lookup deadline unverified"))?
    .map_err(io::Error::other)?;
    let error = resolved.expect_err("owned deny resolver must not answer an address");
    let local = resolver.snapshot()?;
    if !local["question_hash_counts"][&hash]
        .as_u64()
        .is_some_and(|n| n > 0)
    {
        return Err(io::Error::other(
            "actual DNS query did not reach owned deny resolver",
        ));
    }
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    let denied = match socket.connect((outside, 53)).await {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NetworkUnreachable
                    | io::ErrorKind::HostUnreachable
                    | io::ErrorKind::PermissionDenied
            ) =>
        {
            json!({"kind":format!("{:?}",error.kind()),"os_error":error.raw_os_error()})
        }
        Err(_) => return Err(io::Error::other("external UDP route outcome unverified")),
        Ok(()) => {
            let _ = socket.send(&packet).await;
            return Err(io::Error::other(
                "external DNS canary route unexpectedly available",
            ));
        }
    };
    Ok(
        json!({"query_hash":hash,"local_deny_observed":true,"lookup_error_kind":format!("{:?}",error.kind()),"outside_udp_denied":denied,"timeout_counted_as_denial":false}),
    )
}
