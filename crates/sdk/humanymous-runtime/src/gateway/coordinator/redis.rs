//! Dependency-free synchronous RESP2 client, ported from Go `internal/redis`
//! (oracle 981d25c): one mutex-guarded connection with lazy dial, reconnect
//! after any I/O error, optional AUTH on every connect, bounded reply parsing
//! and a circuit breaker that fast-fails after a failure or a slow reply so a
//! degraded coordinator never stalls the caller.
use std::{
    fmt,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Mutex,
    time::{Duration, Instant},
};

/// Largest bulk string accepted (Go `maxBulkLen`, 32 MiB).
pub const MAX_BULK_LEN: usize = 32 << 20;
/// Largest array accepted (Go `maxArrayLen`).
pub const MAX_ARRAY_LEN: usize = 1 << 20;
/// Deepest array nesting accepted (Go `maxDepth`; SCAN needs depth 2).
pub const MAX_DEPTH: usize = 8;
/// Longest reply header line accepted, terminator included. Go read the line
/// unbounded; a header never legitimately exceeds a decimal length.
pub const MAX_LINE_LEN: usize = 64 << 10;
const EWMA_ALPHA: f64 = 0.4;

/// One parsed RESP2 reply. An error reply is surfaced as [`RedisError::Server`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// `+` simple string.
    Simple(String),
    /// `:` integer.
    Integer(i64),
    /// `$` bulk string, binary safe.
    Bulk(Vec<u8>),
    /// `*` array.
    Array(Vec<Reply>),
    /// `$-1` or `*-1` null.
    Nil,
}

impl Reply {
    /// Text of a simple or bulk string reply, if it is valid UTF-8.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Simple(text) => Some(text),
            Self::Bulk(bytes) => std::str::from_utf8(bytes).ok(),
            _ => None,
        }
    }
}

/// Why a command produced no reply.
#[derive(Debug)]
pub enum RedisError {
    /// The breaker is open after a recent failure or slow reply; serve locally.
    BreakerOpen,
    /// Dial, write or read failed; the connection was dropped.
    Io(std::io::Error),
    /// The coordinator answered with a RESP error reply.
    Server(String),
    /// The reply was malformed or exceeded a size or depth cap.
    Protocol(&'static str),
}

impl fmt::Display for RedisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BreakerOpen => formatter.write_str("redis: circuit breaker open"),
            Self::Io(error) => write!(formatter, "redis: {error}"),
            Self::Server(message) => write!(formatter, "redis: {message}"),
            Self::Protocol(detail) => write!(formatter, "redis: {detail}"),
        }
    }
}

impl std::error::Error for RedisError {}

impl From<std::io::Error> for RedisError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Timing and credential settings; defaults match Go `redis.New`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientSettings {
    /// Dial, read and write timeout.
    pub timeout: Duration,
    /// How long the breaker stays open after a trip.
    pub cooldown: Duration,
    /// One reply slower than this trips the breaker.
    pub slow: Duration,
    /// A latency EWMA above this trips the breaker.
    pub slow_ewma: Duration,
    /// Optional ACL user name for AUTH.
    pub user: Option<String>,
    /// Optional password; AUTH is sent on every connect when present.
    pub password: Option<String>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            cooldown: Duration::from_secs(2),
            slow: Duration::from_millis(750),
            slow_ewma: Duration::from_millis(400),
            user: None,
            password: None,
        }
    }
}

struct State {
    connection: Option<BufReader<TcpStream>>,
    open_until: Option<Instant>,
    ewma_nanos: f64,
}

/// Synchronous RESP2 client over one guarded TCP connection.
pub struct Client {
    address: String,
    settings: ClientSettings,
    state: Mutex<State>,
}

impl fmt::Debug for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The password is never printed.
        formatter
            .debug_struct("Client")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Client for `host:port`; the connection is dialled on first use.
    pub fn new(address: impl Into<String>, settings: ClientSettings) -> Self {
        Self {
            address: address.into(),
            settings,
            state: Mutex::new(State {
                connection: None,
                open_until: None,
                ewma_nanos: 0.0,
            }),
        }
    }

    /// Send one command and return its reply. Any I/O or protocol error drops
    /// the connection and opens the breaker for the cooldown.
    pub fn command(&self, args: &[&[u8]]) -> Result<Reply, RedisError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RedisError::Protocol("client state poisoned"))?;
        let started = Instant::now();
        if state.open_until.is_some_and(|until| started < until) {
            return Err(RedisError::BreakerOpen);
        }
        let result = self.exchange(&mut state, args);
        let reply = match result {
            Ok(reply) => reply,
            Err(error) => {
                // A server error reply leaves the stream in sync; anything else
                // may not, so the connection is dropped before the next attempt.
                if !matches!(error, RedisError::Server(_)) {
                    state.connection = None;
                }
                state.open_until = Some(Instant::now() + self.settings.cooldown);
                return Err(error);
            }
        };
        // A slow success trips the breaker too (one sample over `slow`, or the
        // rolling EWMA over `slow_ewma`); the reply is still returned and the
        // connection kept so a transient blip heals as the EWMA decays.
        let elapsed = started.elapsed();
        state.ewma_nanos =
            EWMA_ALPHA * elapsed.as_nanos() as f64 + (1.0 - EWMA_ALPHA) * state.ewma_nanos;
        let slow = !self.settings.slow.is_zero() && elapsed > self.settings.slow;
        let sustained = !self.settings.slow_ewma.is_zero()
            && state.ewma_nanos > self.settings.slow_ewma.as_nanos() as f64;
        state.open_until = (slow || sustained).then(|| Instant::now() + self.settings.cooldown);
        Ok(reply)
    }

    /// Release the connection; the next command reconnects.
    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.connection = None;
        }
    }

    fn exchange(&self, state: &mut State, args: &[&[u8]]) -> Result<Reply, RedisError> {
        if state.connection.is_none() {
            state.connection = Some(self.connect()?);
        }
        let connection = state
            .connection
            .as_mut()
            .ok_or(RedisError::Protocol("connection absent"))?;
        roundtrip(connection, args)
    }

    fn connect(&self) -> Result<BufReader<TcpStream>, RedisError> {
        let mut last = None;
        let mut stream = None;
        for address in self.address.to_socket_addrs()? {
            match TcpStream::connect_timeout(&address, self.settings.timeout) {
                Ok(connected) => {
                    stream = Some(connected);
                    break;
                }
                Err(error) => last = Some(error),
            }
        }
        let stream = match (stream, last) {
            (Some(stream), _) => stream,
            (None, Some(error)) => return Err(error.into()),
            (None, None) => return Err(RedisError::Protocol("address resolved to nothing")),
        };
        stream.set_read_timeout(Some(self.settings.timeout))?;
        stream.set_write_timeout(Some(self.settings.timeout))?;
        stream.set_nodelay(true)?;
        let mut connection = BufReader::new(stream);
        if let Some(password) = self.settings.password.as_deref().filter(|p| !p.is_empty()) {
            let mut args: Vec<&[u8]> = vec![b"AUTH"];
            if let Some(user) = self.settings.user.as_deref().filter(|u| !u.is_empty()) {
                args.push(user.as_bytes());
            }
            args.push(password.as_bytes());
            roundtrip(&mut connection, &args)?;
        }
        Ok(connection)
    }
}

fn roundtrip(connection: &mut BufReader<TcpStream>, args: &[&[u8]]) -> Result<Reply, RedisError> {
    let stream = connection.get_mut();
    stream.write_all(&encode(args))?;
    stream.flush()?;
    read_reply(connection, 0)
}

/// Encode a command as a RESP array of bulk strings.
pub fn encode(args: &[&[u8]]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(32);
    buffer.extend_from_slice(format!("*{}\r\n", args.len()).as_bytes());
    for arg in args {
        buffer.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        buffer.extend_from_slice(arg);
        buffer.extend_from_slice(b"\r\n");
    }
    buffer
}

/// Parse one RESP2 reply, refusing nesting deeper than [`MAX_DEPTH`].
pub fn read_reply(reader: &mut impl BufRead, depth: usize) -> Result<Reply, RedisError> {
    if depth > MAX_DEPTH {
        return Err(RedisError::Protocol("reply nesting exceeds cap"));
    }
    let line = read_line(reader)?;
    let (&kind, body) = line
        .split_first()
        .ok_or(RedisError::Protocol("short reply"))?;
    let text =
        std::str::from_utf8(body).map_err(|_| RedisError::Protocol("reply header not UTF-8"))?;
    match kind {
        b'+' => Ok(Reply::Simple(text.to_owned())),
        b'-' => Err(RedisError::Server(text.to_owned())),
        b':' => text
            .parse()
            .map(Reply::Integer)
            .map_err(|_| RedisError::Protocol("integer reply malformed")),
        b'$' => {
            let Some(length) = length(text, MAX_BULK_LEN, "bulk length exceeds cap")? else {
                return Ok(Reply::Nil);
            };
            let mut payload = vec![0; length + 2];
            reader.read_exact(&mut payload)?;
            if !payload.ends_with(b"\r\n") {
                return Err(RedisError::Protocol("bulk string not CRLF terminated"));
            }
            payload.truncate(length);
            Ok(Reply::Bulk(payload))
        }
        b'*' => {
            let Some(count) = length(text, MAX_ARRAY_LEN, "array length exceeds cap")? else {
                return Ok(Reply::Nil);
            };
            let mut items = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                items.push(read_reply(reader, depth + 1)?);
            }
            Ok(Reply::Array(items))
        }
        _ => Err(RedisError::Protocol("unknown reply type")),
    }
}

fn length(text: &str, cap: usize, over: &'static str) -> Result<Option<usize>, RedisError> {
    let declared: i64 = text
        .parse()
        .map_err(|_| RedisError::Protocol("length malformed"))?;
    if declared < 0 {
        return Ok(None);
    }
    let declared = usize::try_from(declared).map_err(|_| RedisError::Protocol(over))?;
    if declared > cap {
        return Err(RedisError::Protocol(over));
    }
    Ok(Some(declared))
}

fn read_line(reader: &mut impl BufRead) -> Result<Vec<u8>, RedisError> {
    let mut line = Vec::new();
    reader
        .take(MAX_LINE_LEN as u64)
        .read_until(b'\n', &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(if line.len() >= MAX_LINE_LEN {
            RedisError::Protocol("reply line exceeds cap")
        } else if line.is_empty() {
            RedisError::Io(std::io::ErrorKind::UnexpectedEof.into())
        } else {
            RedisError::Protocol("reply line not CRLF terminated")
        });
    }
    line.truncate(line.len() - 2);
    Ok(line)
}

#[cfg(test)]
#[path = "redis_tests.rs"]
mod tests;
