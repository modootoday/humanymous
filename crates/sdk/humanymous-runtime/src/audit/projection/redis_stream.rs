//! Redis Streams hot-tail projection (Go `RedisStreamSink`, oracle 981d25c).
//!
//! Each sealed record becomes one `XADD <stream> MAXLEN ~ <max_len> <seq>-0
//! record <json>`: the chain sequence is the stream id, so a re-ship after a
//! reconnect cannot create a second entry, and `MAXLEN ~` bounds the hot
//! window. RESP is spoken directly over TCP with no client dependency, as in
//! Go. The audit feature does not build the gateway, so the B23 coordinator
//! client is not available here; this sink keeps its own minimal command
//! encoder and reply reader with the same framing rules.
//!
//! Deliberate differences from Go, each pinned by a test:
//! - A record lost to a dial failure, an I/O error, a server error reply or a
//!   close is counted in [`RedisStreamSink::dropped`]; Go counted only a full
//!   backlog and lost the others silently.
//! - The whole reply is consumed. Go read only the first line, so the bulk id
//!   that `XADD` returns stayed in the buffer and later replies were read one
//!   line late. A server error reply keeps the connection (the stream is still
//!   in sync); any other failure drops it and the next record redials.
use super::RecordSink;
use crate::audit::record::Record;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, Thread},
    time::Duration,
};

/// Backlog used when the caller passes zero (Go `bufSize <= 0`).
pub const DEFAULT_BACKLOG: usize = 4096;
/// Dial, write and read timeout (Go 3 s deadlines).
const IO_TIMEOUT: Duration = Duration::from_secs(3);
/// Pause after a failed dial (Go `time.Sleep(time.Second)`).
const RECONNECT_BACKOFF: Duration = Duration::from_secs(1);
/// Longest reply line or bulk payload accepted; an XADD reply is a short id.
const MAX_REPLY_LEN: usize = 64 << 10;

/// Settings for one projection (Go `NewRedisStreamSink` arguments).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedisStreamOptions {
    /// Redis `host:port`.
    pub address: String,
    /// Stream key, for example `audit:node-1`.
    pub stream: String,
    /// Approximate retained hot window (`MAXLEN ~`).
    pub max_len: u64,
    /// In-process backlog before drop-and-count; zero selects [`DEFAULT_BACKLOG`].
    pub backlog: usize,
}

struct Shared {
    dropped: AtomicU64,
    closed: AtomicBool,
}

impl Shared {
    fn drop_one(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Non-blocking Redis Streams projection with a bounded backlog and one
/// background shipper thread.
pub struct RedisStreamSink {
    shared: Arc<Shared>,
    sender: Mutex<Option<SyncSender<Box<Record>>>>,
    worker: Option<Thread>,
}

impl RedisStreamSink {
    /// Start the background shipper. The connection is dialled lazily for the
    /// first record; nothing here waits for Redis.
    pub fn new(options: RedisStreamOptions) -> io::Result<Self> {
        let backlog = if options.backlog == 0 {
            DEFAULT_BACKLOG
        } else {
            options.backlog
        };
        let (sender, receiver) = mpsc::sync_channel(backlog);
        let shared = Arc::new(Shared {
            dropped: AtomicU64::new(0),
            closed: AtomicBool::new(false),
        });
        let shipper = Shipper {
            address: options.address,
            stream: options.stream,
            max_len: options.max_len.to_string(),
        };
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("audit-redis-projection".into())
            .spawn(move || shipper.run(&receiver, &worker_shared))?;
        Ok(Self {
            shared,
            sender: Mutex::new(Some(sender)),
            worker: Some(worker.thread().clone()),
        })
    }

    /// A sink whose shipper never runs, so the backlog only fills (Go test
    /// `TestRedisStreamSinkDropsWhenFull` builds the struct without `run`).
    #[cfg(test)]
    pub(super) fn unstarted(backlog: usize) -> (Self, Receiver<Box<Record>>) {
        let (sender, receiver) = mpsc::sync_channel(backlog);
        let sink = Self {
            shared: Arc::new(Shared {
                dropped: AtomicU64::new(0),
                closed: AtomicBool::new(false),
            }),
            sender: Mutex::new(Some(sender)),
            worker: None,
        };
        (sink, receiver)
    }

    /// Records accepted but not delivered: a full backlog, a failed dial or
    /// write, a server error reply, or a close with records still queued.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

impl RecordSink for RedisStreamSink {
    fn append_record(&self, record: &Record) -> io::Result<()> {
        let Ok(sender) = self.sender.lock() else {
            self.shared.drop_one();
            return Ok(());
        };
        // A full backlog or a closed shipper drops the record and counts it.
        let queued = sender
            .as_ref()
            .is_some_and(|s| s.try_send(Box::new(record.clone())).is_ok());
        if !queued {
            self.shared.drop_one();
        }
        // Best-effort projection: never fail the seal.
        Ok(())
    }

    /// The shipper drains asynchronously; there is no delivery acknowledgement
    /// to wait for (Go `Flush` is a no-op too).
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    /// Stop the shipper without waiting for in-flight I/O (Go `Close`).
    /// Records still queued are counted as dropped by the shipper.
    fn close(&self) -> io::Result<()> {
        self.shared.closed.store(true, Ordering::Release);
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
        if let Some(worker) = &self.worker {
            worker.unpark();
        }
        Ok(())
    }
}

impl Drop for RedisStreamSink {
    fn drop(&mut self) {
        let _ = RecordSink::close(self);
    }
}

enum Failure {
    /// A RESP error reply; the stream is still in sync.
    Server,
    /// I/O or framing failure; the connection must be dropped.
    Transport,
}

struct Shipper {
    address: String,
    stream: String,
    max_len: String,
}

impl Shipper {
    fn run(&self, receiver: &Receiver<Box<Record>>, shared: &Shared) {
        let mut connection: Option<BufReader<TcpStream>> = None;
        while let Ok(record) = receiver.recv() {
            if shared.closed.load(Ordering::Acquire) {
                shared.drop_one();
                continue;
            }
            if connection.is_none() {
                match self.dial() {
                    Ok(dialled) => connection = Some(dialled),
                    Err(_) => {
                        // The durable copy is in the WAL; this projection loses it.
                        shared.drop_one();
                        thread::park_timeout(RECONNECT_BACKOFF);
                        continue;
                    }
                }
            }
            self.ship(&mut connection, &record, shared);
        }
    }

    fn ship(
        &self,
        connection: &mut Option<BufReader<TcpStream>>,
        record: &Record,
        shared: &Shared,
    ) {
        let Some(open) = connection.as_mut() else {
            shared.drop_one();
            return;
        };
        match self.xadd(open, record) {
            Ok(()) => {}
            Err(Failure::Server) => shared.drop_one(),
            Err(Failure::Transport) => {
                shared.drop_one();
                *connection = None;
            }
        }
    }

    fn dial(&self) -> io::Result<BufReader<TcpStream>> {
        let mut last = None;
        for address in self.address.to_socket_addrs()? {
            match TcpStream::connect_timeout(&address, IO_TIMEOUT) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(IO_TIMEOUT))?;
                    stream.set_write_timeout(Some(IO_TIMEOUT))?;
                    stream.set_nodelay(true)?;
                    return Ok(BufReader::new(stream));
                }
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| io::Error::other("address resolved to nothing")))
    }

    fn xadd(&self, connection: &mut BufReader<TcpStream>, record: &Record) -> Result<(), Failure> {
        // An unserializable record is skipped so it cannot wedge the shipper.
        let body = serde_json::to_vec(record).map_err(|_| Failure::Server)?;
        let id = format!("{}-0", record.seq);
        let command = resp_command(&[
            b"XADD",
            self.stream.as_bytes(),
            b"MAXLEN",
            b"~",
            self.max_len.as_bytes(),
            id.as_bytes(),
            b"record",
            &body,
        ]);
        let stream = connection.get_mut();
        stream
            .write_all(&command)
            .and_then(|()| stream.flush())
            .map_err(|_| Failure::Transport)?;
        read_reply(connection)
    }
}

/// Encode a command as a RESP array of bulk strings (Go `respCommand`).
pub(super) fn resp_command(args: &[&[u8]]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(32);
    buffer.extend_from_slice(format!("*{}\r\n", args.len()).as_bytes());
    for arg in args {
        buffer.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        buffer.extend_from_slice(arg);
        buffer.extend_from_slice(b"\r\n");
    }
    buffer
}

/// Consume one non-array reply. `-` is a server error; `+`, `:` and a bulk
/// string (the id `XADD` returns, or nil) are success.
fn read_reply(reader: &mut impl BufRead) -> Result<(), Failure> {
    let line = read_line(reader)?;
    match line.split_first() {
        Some((b'-', _)) => Err(Failure::Server),
        Some((b'+' | b':', _)) => Ok(()),
        Some((b'$', length)) => {
            let declared: i64 = std::str::from_utf8(length)
                .ok()
                .and_then(|text| text.parse().ok())
                .ok_or(Failure::Transport)?;
            if declared < 0 {
                return Ok(());
            }
            let declared = usize::try_from(declared)
                .ok()
                .filter(|&n| n <= MAX_REPLY_LEN)
                .ok_or(Failure::Transport)?;
            let mut payload = vec![0; declared + 2];
            reader
                .read_exact(&mut payload)
                .map_err(|_| Failure::Transport)?;
            if payload.ends_with(b"\r\n") {
                Ok(())
            } else {
                Err(Failure::Transport)
            }
        }
        _ => Err(Failure::Transport),
    }
}

fn read_line(reader: &mut impl BufRead) -> Result<Vec<u8>, Failure> {
    let mut line = Vec::new();
    reader
        .take(MAX_REPLY_LEN as u64)
        .read_until(b'\n', &mut line)
        .map_err(|_| Failure::Transport)?;
    if !line.ends_with(b"\r\n") {
        return Err(Failure::Transport);
    }
    line.truncate(line.len() - 2);
    Ok(line)
}
