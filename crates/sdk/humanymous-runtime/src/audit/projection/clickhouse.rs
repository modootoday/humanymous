//! ClickHouse columnar projection (Go `CHSink`): records are buffered and
//! inserted in `JSONEachRow` batches over the HTTP interface with
//! `async_insert=1&wait_for_async_insert=1` for a durable acknowledgement and
//! `input_format_skip_unknown_fields=1` so a newer record field never breaks
//! an operator's existing table. Credentials travel as HTTP Basic auth, never
//! in the URL.
//!
//! Expected table (created by the operator):
//! `ENGINE = MergeTree ORDER BY (node_id, seq) PARTITION BY toYYYYMMDD(ts)`.
use super::Projection;
use crate::audit::record::Record;
use base64::Engine as _;
use std::{
    fmt, io,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use zeroize::Zeroizing;

/// Per-request timeout (Go `http.Client{Timeout: 10s}`).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const ERROR_BODY_LIMIT: u64 = 512;

/// Batch and backlog settings; defaults match Go `NewCHSink` and `cmd/gate`.
#[derive(Clone)]
pub struct ClickHouseSettings {
    /// HTTP(S) endpoint, e.g. `http://clickhouse:8123`.
    pub base: String,
    /// Target table; must be a plain SQL identifier.
    pub table: String,
    /// Rows that trigger an immediate batch.
    pub max_rows: usize,
    /// Longest a buffered row waits for a batch.
    pub max_wait: Duration,
    /// Backlog bound before drop-and-count.
    pub capacity: usize,
    /// Optional Basic-auth user (Go `HMN_CLICKHOUSE_USER`).
    pub user: Option<String>,
    /// Optional Basic-auth password (Go `HMN_CLICKHOUSE_PASSWORD`).
    pub password: Option<Zeroizing<String>>,
}

impl ClickHouseSettings {
    /// Go defaults for `base`: table `audit_log`, 10 000 rows, 1 s, 100 000.
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            table: "audit_log".into(),
            max_rows: 10_000,
            max_wait: Duration::from_secs(1),
            capacity: 100_000,
            user: None,
            password: None,
        }
    }
}

impl fmt::Debug for ClickHouseSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClickHouseSettings")
            .field("base", &self.base)
            .field("table", &self.table)
            .field("max_rows", &self.max_rows)
            .field("max_wait", &self.max_wait)
            .field("capacity", &self.capacity)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

/// Plain SQL identifier (Go `validTableName`): `[A-Za-z_][A-Za-z0-9_]*`.
pub fn valid_table_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn query_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                escaped.push(byte as char);
            }
            b' ' => escaped.push('+'),
            _ => escaped.push_str(&format!("%{byte:02X}")),
        }
    }
    escaped
}

/// Encode records as `JSONEachRow`: one JSON object per line.
pub fn batch_body(batch: &[Record]) -> Vec<u8> {
    let mut body = Vec::new();
    for record in batch {
        if serde_json::to_writer(&mut body, record).is_ok() {
            body.push(b'\n');
        }
    }
    body
}

struct State {
    buffer: Vec<Record>,
    closed: bool,
}

struct Shared {
    url: String,
    authorization: Option<Zeroizing<String>>,
    agent: ureq::Agent,
    max_rows: usize,
    max_wait: Duration,
    capacity: usize,
    state: Mutex<State>,
    wake: Condvar,
    dropped: AtomicU64,
}

impl Shared {
    fn flush(&self) -> io::Result<()> {
        let batch = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("clickhouse projection poisoned"))?;
            std::mem::take(&mut state.buffer)
        };
        if batch.is_empty() {
            return Ok(());
        }
        let lost = |error: io::Error| {
            // The batch is lost from this projection; the WAL keeps it.
            self.dropped.fetch_add(
                u64::try_from(batch.len()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            error
        };
        let mut request = self
            .agent
            .post(&self.url)
            .header("Content-Type", "application/x-ndjson");
        if let Some(authorization) = &self.authorization {
            request = request.header("Authorization", authorization.as_str());
        }
        let mut response = request
            .send(&batch_body(&batch)[..])
            .map_err(|error| lost(io::Error::other(format!("clickhouse insert: {error}"))))?;
        let status = response.status().as_u16();
        if status >= 300 {
            let message = response
                .body_mut()
                .with_config()
                .limit(ERROR_BODY_LIMIT)
                .read_to_string()
                .unwrap_or_default();
            return Err(lost(io::Error::other(format!(
                "clickhouse insert status {status}: {message}"
            ))));
        }
        Ok(())
    }

    fn run(&self) {
        loop {
            let Ok(state) = self.state.lock() else {
                return;
            };
            let Ok((state, _)) = self.wake.wait_timeout_while(state, self.max_wait, |state| {
                !state.closed && state.buffer.len() < self.max_rows
            }) else {
                return;
            };
            let closed = state.closed;
            drop(state);
            let _ = self.flush();
            if closed {
                return;
            }
        }
    }
}

/// Best-effort batched ClickHouse projection with a bounded backlog.
pub struct ClickHouseSink {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl ClickHouseSink {
    /// Start a batch flusher. Refuses an unsafe table name or a non-HTTP base
    /// (Go silently fell back to `audit_log`; writing elsewhere than configured
    /// is refused here instead).
    pub fn new(settings: ClickHouseSettings) -> io::Result<Self> {
        let base = settings.base.trim_end_matches('/');
        if !(base.starts_with("http://") || base.starts_with("https://"))
            || !valid_table_name(&settings.table)
            || settings.max_rows == 0
            || settings.max_wait.is_zero()
            || settings.capacity == 0
        {
            return Err(io::Error::other("clickhouse projection settings invalid"));
        }
        let query = format!("INSERT INTO {} FORMAT JSONEachRow", settings.table);
        let url = format!(
            "{base}/?query={}&async_insert=1&wait_for_async_insert=1&input_format_skip_unknown_fields=1",
            query_escape(&query)
        );
        let authorization = settings.user.filter(|user| !user.is_empty()).map(|user| {
            let password = settings.password.as_deref().map_or("", String::as_str);
            let pair = Zeroizing::new(format!("{user}:{password}"));
            Zeroizing::new(format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(pair.as_bytes())
            ))
        });
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        let shared = Arc::new(Shared {
            url,
            authorization,
            agent,
            max_rows: settings.max_rows,
            max_wait: settings.max_wait,
            capacity: settings.capacity,
            state: Mutex::new(State {
                buffer: Vec::new(),
                closed: false,
            }),
            wake: Condvar::new(),
            dropped: AtomicU64::new(0),
        });
        let worker = shared.clone();
        let worker = thread::Builder::new()
            .name("audit-clickhouse".into())
            .spawn(move || worker.run())?;
        Ok(Self {
            shared,
            worker: Mutex::new(Some(worker)),
        })
    }

    /// Flush what is buffered and stop the flusher.
    pub fn close(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.closed = true;
        }
        self.shared.wake.notify_all();
        let worker = self.worker.lock().ok().and_then(|mut worker| worker.take());
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

impl Projection for ClickHouseSink {
    fn append(&self, record: &Record) {
        let Ok(mut state) = self.shared.state.lock() else {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if state.closed || state.buffer.len() >= self.shared.capacity {
            drop(state);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        state.buffer.push(record.clone());
        let full = state.buffer.len() >= self.shared.max_rows;
        drop(state);
        if full {
            self.shared.wake.notify_all();
        }
    }
    fn flush(&self) -> io::Result<()> {
        self.shared.flush()
    }
    fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

impl Drop for ClickHouseSink {
    fn drop(&mut self) {
        self.close();
    }
}
