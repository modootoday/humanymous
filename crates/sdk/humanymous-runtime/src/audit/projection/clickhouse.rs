//! ClickHouse columnar projection (Go `CHSink`, oracle 981d25c).
//!
//! Records are buffered and inserted in batches through ClickHouse's HTTP
//! interface as one `INSERT INTO <table> FORMAT JSONEachRow` body (one JSON
//! object per line), with `async_insert=1&wait_for_async_insert=1` for a
//! durable acknowledgement and `input_format_skip_unknown_fields=1` so a new
//! record field does not break an operator's existing table. A batch is sent
//! when it reaches `max_rows`, every `max_wait`, on [`RecordSink::flush`] and
//! on close. The HTTP/1.1 request is written directly over TCP with no client
//! dependency, as Go used only its standard library.
//!
//! The operator creates the table, keyed on chain order so insert order does
//! not matter, for example
//! `ENGINE = MergeTree ORDER BY (node_id, seq) PARTITION BY toYYYYMMDD(ts)`.
//!
//! Deliberate differences from Go, each pinned by a test:
//! - Only `http://` endpoints are accepted, and an endpoint with a query,
//!   fragment or embedded credentials is refused when the sink is built. The
//!   audit feature carries no TLS client; Go's `net/http` also spoke HTTPS.
//!   Credentials go through [`ClickHouseSink::set_auth`] only, never the URL.
//! - A record appended after close is counted as dropped instead of sitting
//!   in a buffer nothing will flush.
//! - Only a 2xx status is success (Go accepted any status below 300, so an
//!   informational 1xx status line counted as a delivered insert).
//!
//! Each insert has one total deadline, as Go's `http.Client.Timeout` did:
//! resolving, connecting, writing the request and reading the response all end
//! by it, and a server that trickles its response cannot stretch it. A missed
//! deadline fails the insert and counts its rows as dropped. Host-name lookup
//! is waited on only until the deadline; see [`DeadlineStream::connect`] for
//! what that does not bound.
use super::{DeadlineStream, RecordSink};
use crate::audit::record::Record;
use base64::Engine as _;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// Table used when the configured name is not a plain SQL identifier.
pub const DEFAULT_TABLE: &str = "audit_log";
/// Batch size used when the caller passes zero (Go `maxRows <= 0`).
pub const DEFAULT_MAX_ROWS: usize = 10_000;
/// Batch interval used when the caller passes zero (Go `maxWait <= 0`).
pub const DEFAULT_MAX_WAIT: Duration = Duration::from_secs(1);
/// Backlog used when the caller passes zero (Go `bufCap <= 0`).
pub const DEFAULT_BACKLOG: usize = 100_000;
/// Total deadline for one insert, from lookup to the response headers and any
/// quoted error body (Go `http.Client{Timeout: 10 * time.Second}`).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Error body bytes quoted in a failed insert (Go `io.LimitReader(.., 512)`).
const ERROR_BODY_LIMIT: u64 = 512;
/// Longest response status or header line accepted.
const MAX_HEADER_LINE: u64 = 16 << 10;
/// Most response header lines read before giving up.
const MAX_HEADER_LINES: usize = 128;

/// Settings for one projection (Go `NewCHSink` arguments).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClickHouseOptions {
    /// HTTP endpoint such as `http://ch:8123`.
    pub endpoint: String,
    /// Target table; anything but a plain identifier falls back to [`DEFAULT_TABLE`].
    pub table: String,
    /// Rows that trigger an immediate batch; zero selects [`DEFAULT_MAX_ROWS`].
    pub max_rows: usize,
    /// Longest wait before a partial batch is sent; zero selects [`DEFAULT_MAX_WAIT`].
    pub max_wait: Duration,
    /// Buffered rows before drop-and-count; zero selects [`DEFAULT_BACKLOG`].
    pub backlog: usize,
}

/// Whether `name` is a safe SQL identifier for the projection table (Go
/// `validTableName`, `^[A-Za-z_][A-Za-z0-9_]*$`).
pub fn valid_table_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

struct Target {
    /// `host[:port]` as written, sent as the Host header.
    authority: String,
    /// `authority` with the default port made explicit, for dialling.
    dial: String,
    /// Path and query of the insert request.
    request: String,
}

impl Target {
    fn parse(endpoint: &str, table: &str) -> io::Result<Self> {
        let refuse = |why: &str| io::Error::new(io::ErrorKind::InvalidInput, why.to_owned());
        let rest = endpoint
            .strip_prefix("http://")
            .ok_or_else(|| refuse("clickhouse endpoint must be an http:// URL"))?;
        if rest.contains(['?', '#']) {
            return Err(refuse("clickhouse endpoint must not carry a query"));
        }
        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
        if authority.is_empty()
            || authority.contains('@')
            || authority
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
            || path
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(refuse(
                "clickhouse endpoint host is empty, malformed or carries credentials",
            ));
        }
        let has_port = match authority.rfind(']') {
            Some(close) => authority[close..].starts_with("]:"),
            None => authority.contains(':'),
        };
        let dial = if has_port {
            authority.to_owned()
        } else {
            format!("{authority}:80")
        };
        let table = if valid_table_name(table) {
            table
        } else {
            DEFAULT_TABLE
        };
        let query = format!("INSERT INTO {table} FORMAT JSONEachRow");
        let request = format!(
            "{path}/?query={}&async_insert=1&wait_for_async_insert=1&input_format_skip_unknown_fields=1",
            query_escape(&query)
        );
        Ok(Self {
            authority: authority.to_owned(),
            dial,
            request,
        })
    }
}

/// Go `url.QueryEscape`: unreserved bytes stay, space becomes `+`, the rest
/// is percent-encoded.
fn query_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

struct State {
    buffer: Vec<Record>,
    triggered: bool,
    closed: bool,
}

struct Shared {
    target: Target,
    max_rows: usize,
    max_wait: Duration,
    backlog: usize,
    /// Total deadline for one insert.
    request_timeout: Duration,
    /// Complete `Authorization` header value, zeroized on replacement and drop.
    authorization: Mutex<Option<Zeroizing<String>>>,
    state: Mutex<State>,
    wake: Condvar,
    dropped: AtomicU64,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn drop_rows(&self, rows: usize) {
        self.dropped
            .fetch_add(u64::try_from(rows).unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Swap out the buffer and insert it as one batch. A failed insert counts
    /// every row of the batch as dropped; the durable copy is in the WAL.
    fn flush(&self) -> io::Result<()> {
        let batch = std::mem::take(&mut self.state().buffer);
        if batch.is_empty() {
            return Ok(());
        }
        let result = batch_body(&batch).and_then(|body| self.post(&body));
        if result.is_err() {
            self.drop_rows(batch.len());
        }
        result
    }

    fn post(&self, body: &[u8]) -> io::Result<()> {
        let mut head = Zeroizing::new(format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.target.request,
            self.target.authority,
            body.len()
        ));
        let authorization = self
            .authorization
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(value) = authorization.as_deref() {
            head.push_str("Authorization: ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let deadline = Instant::now() + self.request_timeout;
        let mut stream = DeadlineStream::connect(&self.target.dial, deadline)?;
        stream.write_all(head.as_bytes())?;
        stream.write_all(body)?;
        stream.flush()?;
        let mut reader = BufReader::new(stream);
        let status = read_status(&mut reader)?;
        if (200..300).contains(&status) {
            return Ok(());
        }
        let mut message = Vec::new();
        let _ = reader.take(ERROR_BODY_LIMIT).read_to_end(&mut message);
        Err(io::Error::other(format!(
            "clickhouse insert status {status}: {}",
            String::from_utf8_lossy(&message).trim_end()
        )))
    }

    fn run(&self) {
        loop {
            let mut state = self.state();
            loop {
                if state.closed {
                    return;
                }
                if std::mem::take(&mut state.triggered) {
                    break;
                }
                let (next, waited) = self
                    .wake
                    .wait_timeout(state, self.max_wait)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state = next;
                if waited.timed_out() {
                    break;
                }
            }
            drop(state);
            let _ = self.flush();
        }
    }
}

/// Read the status line and skip the headers, leaving the body unread.
fn read_status(reader: &mut impl BufRead) -> io::Result<u16> {
    let malformed = || io::Error::new(io::ErrorKind::InvalidData, "clickhouse response malformed");
    let status_line = read_header_line(reader)?;
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or_default();
    let code = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") || code.len() != 3 {
        return Err(malformed());
    }
    let status = code.parse::<u16>().map_err(|_| malformed())?;
    for _ in 0..MAX_HEADER_LINES {
        if read_header_line(reader)?.is_empty() {
            return Ok(status);
        }
    }
    Err(malformed())
}

fn read_header_line(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = Vec::new();
    reader.take(MAX_HEADER_LINE).read_until(b'\n', &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "clickhouse response line truncated",
        ));
    }
    line.truncate(line.len() - 2);
    String::from_utf8(line)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "clickhouse response not UTF-8"))
}

/// JSONEachRow body: one record object per line (Go `chBatchBody`).
pub(super) fn batch_body(batch: &[Record]) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    for record in batch {
        serde_json::to_writer(&mut body, record).map_err(io::Error::other)?;
        body.push(b'\n');
    }
    Ok(body)
}

/// Non-blocking batched ClickHouse projection with one background flusher.
pub struct ClickHouseSink {
    shared: Arc<Shared>,
}

impl ClickHouseSink {
    /// Validate the endpoint and start the background flusher. An unsafe
    /// table name falls back to [`DEFAULT_TABLE`], as in Go.
    pub fn new(options: ClickHouseOptions) -> io::Result<Self> {
        Self::with_request_timeout(options, REQUEST_TIMEOUT)
    }

    /// [`ClickHouseSink::new`] with another total insert deadline, so tests
    /// can exercise a missed deadline quickly.
    pub(super) fn with_request_timeout(
        options: ClickHouseOptions,
        request_timeout: Duration,
    ) -> io::Result<Self> {
        let target = Target::parse(&options.endpoint, &options.table)?;
        let pick = |value: usize, default: usize| if value == 0 { default } else { value };
        let shared = Arc::new(Shared {
            target,
            max_rows: pick(options.max_rows, DEFAULT_MAX_ROWS),
            max_wait: if options.max_wait.is_zero() {
                DEFAULT_MAX_WAIT
            } else {
                options.max_wait
            },
            backlog: pick(options.backlog, DEFAULT_BACKLOG),
            request_timeout,
            authorization: Mutex::new(None),
            state: Mutex::new(State {
                buffer: Vec::new(),
                triggered: false,
                closed: false,
            }),
            wake: Condvar::new(),
            dropped: AtomicU64::new(0),
        });
        let worker = Arc::clone(&shared);
        thread::Builder::new()
            .name("audit-clickhouse-projection".into())
            .spawn(move || worker.run())?;
        Ok(Self { shared })
    }

    /// Send a ClickHouse credential as HTTP Basic authentication, kept out of
    /// the endpoint so it never appears in a logged URL (Go `SetAuth`). An
    /// empty user disables it.
    pub fn set_auth(&self, user: &str, password: &str) {
        let value = (!user.is_empty()).then(|| {
            let pair = Zeroizing::new(format!("{user}:{password}"));
            let mut value = Zeroizing::new(String::from("Basic "));
            base64::engine::general_purpose::STANDARD.encode_string(pair.as_bytes(), &mut value);
            value
        });
        *self
            .shared
            .authorization
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = value;
    }

    /// Request path and query of each insert, for operator diagnostics. It
    /// never holds a credential.
    pub fn insert_target(&self) -> &str {
        &self.shared.target.request
    }

    /// Rows dropped from this projection: a full backlog, a failed insert or
    /// an append after close.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

impl RecordSink for ClickHouseSink {
    fn append_record(&self, record: &Record) -> io::Result<()> {
        let mut state = self.shared.state();
        if state.closed || state.buffer.len() >= self.shared.backlog {
            drop(state);
            self.shared.drop_rows(1);
            return Ok(());
        }
        state.buffer.push(record.clone());
        if state.buffer.len() >= self.shared.max_rows {
            state.triggered = true;
            self.shared.wake.notify_one();
        }
        Ok(())
    }

    /// Insert the current buffer now and wait for ClickHouse's acknowledgement.
    fn flush(&self) -> io::Result<()> {
        self.shared.flush()
    }

    /// Flush the buffer, then stop the flusher. Later appends are dropped.
    fn close(&self) -> io::Result<()> {
        {
            let mut state = self.shared.state();
            if state.closed {
                return Ok(());
            }
            state.closed = true;
        }
        let result = self.shared.flush();
        self.shared.wake.notify_all();
        result
    }
}

impl Drop for ClickHouseSink {
    fn drop(&mut self) {
        let _ = RecordSink::close(self);
    }
}
