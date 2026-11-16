//! Operational logging for the Gate and Core roles (shipping row B27; legacy
//! cmd/gate/logging.go, cmd/server/logging.go and their internal/logger
//! dependency at baseline 981d25c).
//!
//! Operational logs are diagnostics only. They are independent of scoring,
//! enforcement, settings and the tamper-evident audit stream, and a sink
//! failure never changes a decision. The kernel holds no global logger and
//! never reads the environment or the working directory: the composition root
//! resolves each setting with [`resolve_setting`] from explicit flag and
//! environment values, passes explicit console writers, and supplies the
//! record time. Settings are nonsecret; no credential is accepted here.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - An empty level is refused for both roles; Go Gate treated it as `info`.
//! - File sink paths must be absolute without `..` components; Go resolved
//!   relative paths against the process working directory.
//! - Gate trims and lowercases its three selectors as Go did; Core keeps Go's
//!   exact-token matching.
//! - Emission is synchronous on the caller's thread with write errors counted,
//!   not Go's bounded asynchronous queue; a composition that must not block on
//!   a slow sink places this logger behind its own bounded channel.
//! - The fatal classifier only accepts `&'static str` call-site formats, so
//!   dynamic error text cannot reach it by type.
//! - Observability records allowlist the HTTP method (anything else is
//!   `other`); Go logged the parsed method token verbatim.
mod record;
mod sink;

pub use record::{Level, MAX_FIELDS, MAX_RECORD_BYTES, SCHEMA_VERSION, Value};

use record::{MAX_IDENTITY_BYTES, Record, sanitize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

/// Role whose operational log is being configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Edge Gate (`service="gate"`); selectors are trimmed and lowercased.
    Gate,
    /// Core demo server (`service="core"`); selectors match exactly.
    Core,
}

impl Role {
    /// Service name written into every record.
    pub fn service(self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Core => "core",
        }
    }
}

/// One operator-facing logging selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `off|debug|info|warn|error`.
    Level,
    /// `off|plain|jsonl`.
    ConsoleFormat,
    /// `stderr|stdout`.
    ConsoleStream,
    /// Append plain records to this absolute path; empty disables.
    PlainFile,
    /// Append JSONL records to this absolute path; empty disables.
    JsonlFile,
}

impl Setting {
    /// Command-line flag name without dashes.
    pub fn flag(self) -> &'static str {
        match self {
            Self::Level => "log-level",
            Self::ConsoleFormat => "log-console-format",
            Self::ConsoleStream => "log-console-stream",
            Self::PlainFile => "log-plain-file",
            Self::JsonlFile => "log-jsonl-file",
        }
    }
    /// Environment selector name the composition root may read.
    pub fn environment(self) -> &'static str {
        match self {
            Self::Level => "HMN_LOG_LEVEL",
            Self::ConsoleFormat => "HMN_LOG_CONSOLE_FORMAT",
            Self::ConsoleStream => "HMN_LOG_CONSOLE_STREAM",
            Self::PlainFile => "HMN_LOG_PLAIN_FILE",
            Self::JsonlFile => "HMN_LOG_JSONL_FILE",
        }
    }
    /// Role default: Gate logs at `info`, Core is off unless configured.
    pub fn fallback(self, role: Role) -> &'static str {
        match (self, role) {
            (Self::Level, Role::Gate) => "info",
            (Self::Level, Role::Core) => "off",
            (Self::ConsoleFormat, _) => "plain",
            (Self::ConsoleStream, _) => "stderr",
            (Self::PlainFile | Self::JsonlFile, _) => "",
        }
    }
}

/// Precedence: an explicitly set flag, then a present environment value (even
/// empty), then the fallback. Pure; the caller performs both lookups.
pub fn resolve_setting<'a>(
    flag: Option<&'a str>,
    environment: Option<&'a str>,
    fallback: &'a str,
) -> &'a str {
    flag.or(environment).unwrap_or(fallback)
}

/// Unvalidated selector values as resolved by the composition root.
#[derive(Debug, Clone, Copy)]
pub struct RawSettings<'a> {
    /// See [`Setting::Level`].
    pub level: &'a str,
    /// See [`Setting::ConsoleFormat`].
    pub console_format: &'a str,
    /// See [`Setting::ConsoleStream`].
    pub console_stream: &'a str,
    /// See [`Setting::PlainFile`].
    pub plain_file: &'a str,
    /// See [`Setting::JsonlFile`].
    pub jsonl_file: &'a str,
}

/// Static refusal; never echoes the rejected value or an OS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Level is not one of `off|debug|info|warn|error`.
    Level,
    /// Console format is not one of `off|plain|jsonl`.
    ConsoleFormat,
    /// Console stream is not one of `stderr|stdout`.
    ConsoleStream,
    /// File sink path is relative, empty or contains `..`.
    SinkPath,
    /// Two file sinks name the same file.
    DuplicateSink,
    /// File sink target is a symlink, not a regular file or changed identity.
    SinkTarget,
    /// File sink could not be opened (for example a missing parent).
    SinkOpen,
}

impl Refusal {
    /// Stable code for startup diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Level => "log_level",
            Self::ConsoleFormat => "log_console_format",
            Self::ConsoleStream => "log_console_stream",
            Self::SinkPath => "log_sink_path",
            Self::DuplicateSink => "log_sink_duplicate",
            Self::SinkTarget => "log_sink_target",
            Self::SinkOpen => "log_sink_open",
        }
    }
}

/// Physical line encoding of one sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Human-readable `key="value"` line.
    Plain,
    /// One JSON object per line.
    Jsonl,
}

/// Console stream selected for the console sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Standard error.
    Stderr,
    /// Standard output.
    Stdout,
}

/// Validated logging configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `None` means off.
    pub threshold: Option<Level>,
    /// Console sink, if any.
    pub console: Option<(Format, Stream)>,
    /// Plain file sink path, if any.
    pub plain_file: Option<std::path::PathBuf>,
    /// JSONL file sink path, if any.
    pub jsonl_file: Option<std::path::PathBuf>,
}

impl Settings {
    /// Validate every selector before any sink is opened.
    pub fn validate(role: Role, raw: &RawSettings<'_>) -> Result<Self, Refusal> {
        let normalize = |value: &str| match role {
            Role::Gate => value.trim().to_ascii_lowercase(),
            Role::Core => value.to_owned(),
        };
        let threshold = match normalize(raw.level).as_str() {
            "off" => None,
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => return Err(Refusal::Level),
        };
        let stream = match normalize(raw.console_stream).as_str() {
            "stderr" => Stream::Stderr,
            "stdout" => Stream::Stdout,
            _ => return Err(Refusal::ConsoleStream),
        };
        let console = match normalize(raw.console_format).as_str() {
            "off" => None,
            "plain" => Some((Format::Plain, stream)),
            "jsonl" => Some((Format::Jsonl, stream)),
            _ => return Err(Refusal::ConsoleFormat),
        };
        let path = |value: &str| -> Result<_, Refusal> {
            if value.is_empty() {
                Ok(None)
            } else {
                sink::canonical(value).map(Some)
            }
        };
        Ok(Self {
            threshold,
            console,
            plain_file: path(raw.plain_file)?,
            jsonl_file: path(raw.jsonl_file)?,
        })
    }
}

/// Sanitized record identity; empty version or node become `unknown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    service: &'static str,
    version: String,
    node: String,
}

impl Identity {
    /// Bind the role's service name with caller-supplied version and node.
    pub fn new(role: Role, version: &str, node: &str) -> Self {
        let clean = |value: &str| {
            let (text, _) = sanitize(value, MAX_IDENTITY_BYTES);
            if text.is_empty() {
                "unknown".to_owned()
            } else {
                text
            }
        };
        Self {
            service: role.service(),
            version: clean(version),
            node: clean(node),
        }
    }
}

enum Target {
    Writer(Box<dyn Write + Send>),
    File(File),
}

struct Sink {
    name: &'static str,
    format: Format,
    target: Target,
    write_errors: u64,
}

/// Bounded write-error snapshot of one sink; no dynamic error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SinkStats {
    /// `console`, `plain_file` or `jsonl_file`.
    pub name: &'static str,
    /// Encoding.
    pub format: Format,
    /// Failed writes, flushes or syncs.
    pub write_errors: u64,
}

/// One opened operational logger owning its file sinks.
pub struct OperationalLogger {
    identity: Identity,
    threshold: Option<Level>,
    sinks: Vec<Sink>,
    sequence: u64,
}

impl OperationalLogger {
    /// Open every configured sink or none. The unselected console writer is
    /// dropped. A file that cannot be opened refuses startup.
    pub fn open(
        identity: Identity,
        settings: &Settings,
        stdout: Box<dyn Write + Send>,
        stderr: Box<dyn Write + Send>,
    ) -> Result<Self, Refusal> {
        let mut sinks = Vec::with_capacity(3);
        if let Some((format, stream)) = settings.console {
            let writer = match stream {
                Stream::Stdout => stdout,
                Stream::Stderr => stderr,
            };
            sinks.push(Sink {
                name: "console",
                format,
                target: Target::Writer(writer),
                write_errors: 0,
            });
        }
        let mut seen = BTreeSet::new();
        for (name, format, path) in [
            ("plain_file", Format::Plain, &settings.plain_file),
            ("jsonl_file", Format::Jsonl, &settings.jsonl_file),
        ] {
            let Some(path) = path else { continue };
            if !seen.insert(sink::duplicate_key(path)) {
                return Err(Refusal::DuplicateSink);
            }
            let file = sink::open_append_regular(path)?;
            let opened = file.metadata().map_err(|_| Refusal::SinkOpen)?;
            for prior in &sinks {
                if let Target::File(prior) = &prior.target
                    && prior
                        .metadata()
                        .is_ok_and(|meta| sink::same_file(&meta, &opened) && cfg!(unix))
                {
                    return Err(Refusal::DuplicateSink);
                }
            }
            sinks.push(Sink {
                name,
                format,
                target: Target::File(file),
                write_errors: 0,
            });
        }
        Ok(Self {
            identity,
            threshold: settings.threshold,
            sinks,
            sequence: 0,
        })
    }

    /// Whether any record can be emitted (the level is not `off`).
    pub fn is_on(&self) -> bool {
        self.threshold.is_some()
    }

    /// Whether a record at `level` passes the threshold.
    pub fn enabled(&self, level: Level) -> bool {
        self.threshold.is_some_and(|threshold| level >= threshold)
    }

    /// Emit one record stamped with the caller-supplied time since the Unix
    /// epoch. Attributes `component` and `event` override the envelope as in
    /// Go; every other key becomes a sanitized, bounded field.
    pub fn emit(&mut self, at: Duration, level: Level, message: &str, attrs: &[(&str, Value<'_>)]) {
        if !self.enabled(level) {
            return;
        }
        let identity = [
            self.identity.service,
            self.identity.version.as_str(),
            self.identity.node.as_str(),
        ];
        let mut record = Record::new(at, level, identity, message, attrs);
        self.sequence = self.sequence.saturating_add(1);
        record.sequence = self.sequence;
        record.fit();
        let (plain, json) = (record.plain_line(), record.json_line());
        for sink in &mut self.sinks {
            let line = match sink.format {
                Format::Plain => &plain,
                Format::Jsonl => &json,
            };
            let result = match &mut sink.target {
                Target::Writer(writer) => writer.write_all(line.as_bytes()),
                Target::File(file) => file.write_all(line.as_bytes()),
            };
            if result.is_err() {
                sink.write_errors = sink.write_errors.saturating_add(1);
            }
        }
    }

    /// Flush writers and sync file sinks; failures are counted per sink.
    pub fn flush(&mut self) {
        for sink in &mut self.sinks {
            let result = match &mut sink.target {
                Target::Writer(writer) => writer.flush(),
                Target::File(file) => file.flush().and_then(|()| file.sync_data()),
            };
            if result.is_err() {
                sink.write_errors = sink.write_errors.saturating_add(1);
            }
        }
    }

    /// Per-sink write-error snapshot.
    pub fn stats(&self) -> Vec<SinkStats> {
        self.sinks
            .iter()
            .map(|sink| SinkStats {
                name: sink.name,
                format: sink.format,
                write_errors: sink.write_errors,
            })
            .collect()
    }
}

/// Fixed Gate startup/listener failure class. The original error may carry an
/// operator path, credential-bearing URL or request text, so only this class
/// is ever persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatalClass {
    /// Sealed keystore.
    Keystore,
    /// Audit write-ahead log.
    AuditWal,
    /// Audit verification.
    AuditVerification,
    /// Shared-state key.
    SharedStateKey,
    /// Token key.
    TokenKey,
    /// Settings document.
    Settings,
    /// Agent keys.
    AgentKeys,
    /// Token issuers.
    TokenIssuers,
    /// WebAuthn credentials.
    WebauthnCredentials,
    /// Route policy.
    RoutePolicy,
    /// Origin proxy.
    OriginProxy,
    /// Admin listener.
    AdminListener,
    /// TLS.
    Tls,
    /// Trusted proxies.
    TrustedProxies,
    /// Edge listener.
    EdgeListener,
    /// Credential policy refusal.
    CredentialPolicy,
    /// Anything else.
    Startup,
}

impl FatalClass {
    /// Stable `error_class` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keystore => "keystore",
            Self::AuditWal => "audit_wal",
            Self::AuditVerification => "audit_verification",
            Self::SharedStateKey => "shared_state_key",
            Self::TokenKey => "token_key",
            Self::Settings => "settings",
            Self::AgentKeys => "agent_keys",
            Self::TokenIssuers => "token_issuers",
            Self::WebauthnCredentials => "webauthn_credentials",
            Self::RoutePolicy => "route_policy",
            Self::OriginProxy => "origin_proxy",
            Self::AdminListener => "admin_listener",
            Self::Tls => "tls",
            Self::TrustedProxies => "trusted_proxies",
            Self::EdgeListener => "edge_listener",
            Self::CredentialPolicy => "credential_policy",
            Self::Startup => "startup",
        }
    }
}

/// Classify a static call-site format exactly in Go's first-match order. The
/// `'static` bound keeps formatted error text out by construction.
pub fn fatal_class(format: &'static str) -> FatalClass {
    const ORDER: [(&str, FatalClass); 16] = [
        ("keystore", FatalClass::Keystore),
        ("audit-wal", FatalClass::AuditWal),
        ("audit-verify", FatalClass::AuditVerification),
        ("REDIS_KEY", FatalClass::SharedStateKey),
        ("TOKEN_KEY", FatalClass::TokenKey),
        ("settings", FatalClass::Settings),
        ("agent-keys", FatalClass::AgentKeys),
        ("pat-issuers", FatalClass::TokenIssuers),
        ("webauthn", FatalClass::WebauthnCredentials),
        ("routes", FatalClass::RoutePolicy),
        ("proxy:", FatalClass::OriginProxy),
        ("admin", FatalClass::AdminListener),
        ("tls", FatalClass::Tls),
        ("trusted-proxies", FatalClass::TrustedProxies),
        ("edge listener", FatalClass::EdgeListener),
        ("refusing to boot", FatalClass::CredentialPolicy),
    ];
    ORDER
        .iter()
        .find(|(needle, _)| format.contains(needle))
        .map_or(FatalClass::Startup, |(_, class)| *class)
}

/// Write the Gate fatal record and flush. Process exit stays with the caller.
pub fn emit_gate_fatal(logger: &mut OperationalLogger, at: Duration, class: FatalClass) {
    logger.emit(
        at,
        Level::Error,
        "Gate stopped after a startup or listener failure.",
        &[
            ("component", Value::Str("gate.runtime")),
            ("event", Value::Str("runtime.fatal")),
            ("error_class", Value::Str(class.as_str())),
        ],
    );
    logger.flush();
}

/// HTTP protocol version recorded by observability (`"2"` or `"1.1"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/1.x.
    Http1,
    /// HTTP/2.
    Http2,
}

/// Result of an observed handler.
#[derive(Debug, PartialEq, Eq)]
pub enum Completion<R> {
    /// Handler returned normally.
    Completed(R),
    /// Handler panicked; the caller answers 500. Panic payload is discarded.
    PanicRecovered,
}

const METHODS: [&str; 9] = [
    "GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "CONNECT", "TRACE",
];

/// Run `handler` with opt-in request observability. Off, the handler runs
/// unwrapped (no recovery, no record), as Go returned the handler unchanged.
/// On, only fixed event identifiers, an allowlisted method and the protocol
/// version are recorded: no path, query, header or panic material.
pub fn observe<R>(
    logger: &mut OperationalLogger,
    at: Duration,
    method: &str,
    version: HttpVersion,
    handler: impl FnOnce() -> R,
) -> Completion<R> {
    if !logger.is_on() {
        return Completion::Completed(handler());
    }
    match catch_unwind(AssertUnwindSafe(handler)) {
        Ok(value) => {
            let method = METHODS
                .iter()
                .find(|known| **known == method)
                .copied()
                .unwrap_or("other");
            let protocol = match version {
                HttpVersion::Http1 => "1.1",
                HttpVersion::Http2 => "2",
            };
            logger.emit(
                at,
                Level::Info,
                "HTTP request completed.",
                &[
                    ("component", Value::Str("core.http")),
                    ("event", Value::Str("http.request_completed")),
                    ("method", Value::Str(method)),
                    ("protocol", Value::Str(protocol)),
                ],
            );
            Completion::Completed(value)
        }
        Err(_) => {
            logger.emit(
                at,
                Level::Error,
                "HTTP handler panic recovered.",
                &[
                    ("component", Value::Str("core.http")),
                    ("event", Value::Str("http.panic_recovered")),
                ],
            );
            Completion::PanicRecovered
        }
    }
}

const EXTERNAL_INPUT_PREFIX: &str = "external-input/";

/// Accept a lab external-input label only in its canonical bounded form:
/// the `external-input/` prefix plus at least one of `[A-Za-z0-9-._/:]`, at
/// most 192 bytes in total.
pub fn canonical_external_input_label(raw: &str) -> Option<&str> {
    if raw.len() <= EXTERNAL_INPUT_PREFIX.len()
        || raw.len() > 192
        || !raw.starts_with(EXTERNAL_INPUT_PREFIX)
    {
        return None;
    }
    raw.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-._/:".contains(c))
        .then_some(raw)
}

#[cfg(test)]
mod tests;
