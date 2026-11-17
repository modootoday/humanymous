//! Redis Streams hot-tail projection (Go `RedisStreamSink`): one `XADD` per
//! sealed record with the record JSON as the single `record` field, the chain
//! sequence as the stream id (`{seq}-0`) so a re-ship is idempotent, and
//! `MAXLEN ~` trimming so the hot tier stays bounded.
use super::Projection;
use crate::audit::record::Record;
use crate::gateway::coordinator::redis::{Client, ClientSettings};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// Default hot-tail length (Go `cmd/gate`).
pub const DEFAULT_MAX_LEN: u64 = 100_000;
/// Default in-process backlog (Go `cmd/gate`).
pub const DEFAULT_BUFFER: usize = 8192;

/// Connection settings for the shipper: Go dials with a 3 s timeout and backs
/// off one second after a failure, which the breaker cooldown reproduces.
pub fn shipper_settings() -> ClientSettings {
    ClientSettings {
        cooldown: Duration::from_secs(1),
        ..ClientSettings::default()
    }
}

/// Best-effort Redis Streams projection with a bounded backlog.
pub struct RedisStreamSink {
    sender: Mutex<Option<SyncSender<Record>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    dropped: Arc<AtomicU64>,
}

impl RedisStreamSink {
    /// Start a shipper to `address` (`host:port`) writing `stream`, retaining
    /// about `max_len` entries, with a backlog of `buffer` records.
    pub fn new(
        address: impl Into<String>,
        stream: impl Into<String>,
        max_len: u64,
        buffer: usize,
        settings: ClientSettings,
    ) -> io::Result<Self> {
        let stream = stream.into();
        if stream.is_empty() || max_len == 0 || buffer == 0 {
            return Err(io::Error::other("redis projection settings invalid"));
        }
        let (sender, receiver) = sync_channel(buffer);
        let dropped = Arc::new(AtomicU64::new(0));
        let client = Client::new(address, settings);
        let counter = dropped.clone();
        let worker = thread::Builder::new()
            .name("audit-redis".into())
            .spawn(move || ship(&client, &stream, max_len, &receiver, &counter))?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            worker: Mutex::new(Some(worker)),
            dropped,
        })
    }

    /// Stop accepting records, drain the backlog and join the shipper.
    pub fn close(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
        let worker = self.worker.lock().ok().and_then(|mut worker| worker.take());
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

fn ship(
    client: &Client,
    stream: &str,
    max_len: u64,
    records: &Receiver<Record>,
    dropped: &AtomicU64,
) {
    let max_len = max_len.to_string();
    for record in records {
        let Ok(body) = serde_json::to_vec(&record) else {
            dropped.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let id = format!("{}-0", record.seq);
        let shipped = client.command(&[
            b"XADD",
            stream.as_bytes(),
            b"MAXLEN",
            b"~",
            max_len.as_bytes(),
            id.as_bytes(),
            b"record",
            &body,
        ]);
        // Go silently lost a record it could not ship; it is still durable in
        // the WAL, but the projection gap is counted here.
        if shipped.is_err() {
            dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    client.close();
}

impl Projection for RedisStreamSink {
    fn append(&self, record: &Record) {
        let sent = match self.sender.lock() {
            Ok(sender) => match sender.as_ref() {
                Some(sender) => match sender.try_send(record.clone()) {
                    Ok(()) => true,
                    Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
                },
                None => false,
            },
            Err(_) => false,
        };
        if !sent {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn flush(&self) -> io::Result<()> {
        // Go: the shipper drains asynchronously; flush is only an ack point.
        Ok(())
    }
    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl Drop for RedisStreamSink {
    fn drop(&mut self) {
        self.close();
    }
}
