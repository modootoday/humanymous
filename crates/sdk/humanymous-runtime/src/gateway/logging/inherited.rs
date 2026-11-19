//! Gate operational logging selected by the runtime binary's private inherited
//! configuration (legacy cmd/gate main.go `-log-*` flags, `HMN_LOG_*`
//! environment, `openGateLogger`, the "Operational logging initialized."
//! record and `gateFatalf` at baseline 981d25c).
//!
//! The runtime binary takes no flags and reads no environment: its operator
//! settings arrive in the inherited configuration, so the five Go selectors
//! are the fields of one optional `logging` object, with the Go Gate
//! defaults for any field left out (`info`, `plain`, `stderr`, no files).
//!
//! Deliberate differences, each pinned by tests:
//! - Without a `logging` object the binary writes no operational records
//!   (Go always opened the Gate logger at its defaults); the object opts in.
//! - `console_stream: "stdout"` is refused (`log_console_stream_reserved`):
//!   the binary's standard output is the one-line readiness pipe.
//! - Settings are validated and file sinks opened before custody; a refusal
//!   exits non-zero with its static code and no readiness line (Go exited
//!   with a fixed message).
use super::{
    Identity, Level, OperationalLogger, RawSettings, Refusal, Role, Setting, Settings, Value,
};
use serde::Deserialize;
use std::io::Write;
use std::time::Duration;

fn level() -> String {
    Setting::Level.fallback(Role::Gate).into()
}
fn console_format() -> String {
    Setting::ConsoleFormat.fallback(Role::Gate).into()
}
fn console_stream() -> String {
    Setting::ConsoleStream.fallback(Role::Gate).into()
}

/// The `logging` object of the inherited configuration.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InheritedLogging {
    /// Go `-log-level`: `off|debug|info|warn|error`.
    #[serde(default = "level")]
    pub level: String,
    /// Go `-log-console-format`: `off|plain|jsonl`.
    #[serde(default = "console_format")]
    pub console_format: String,
    /// Go `-log-console-stream`; only `stderr` is accepted here.
    #[serde(default = "console_stream")]
    pub console_stream: String,
    /// Go `-log-plain-file`: absolute path, empty disables.
    #[serde(default)]
    pub plain_file: String,
    /// Go `-log-jsonl-file`: absolute path, empty disables.
    #[serde(default)]
    pub jsonl_file: String,
}

impl Default for InheritedLogging {
    fn default() -> Self {
        Self {
            level: level(),
            console_format: console_format(),
            console_stream: console_stream(),
            plain_file: String::new(),
            jsonl_file: String::new(),
        }
    }
}

impl InheritedLogging {
    /// Validate every selector, then open every sink or none. `stderr` is the
    /// console writer; standard output is never a sink.
    pub fn open(
        &self,
        version: &str,
        node: &str,
        stderr: Box<dyn Write + Send>,
    ) -> Result<OperationalLogger, Refusal> {
        let settings = Settings::validate(
            Role::Gate,
            &RawSettings {
                level: &self.level,
                console_format: &self.console_format,
                console_stream: &self.console_stream,
                plain_file: &self.plain_file,
                jsonl_file: &self.jsonl_file,
            },
        )?;
        if matches!(settings.console, Some((_, super::Stream::Stdout))) {
            return Err(Refusal::ConsoleStreamReserved);
        }
        OperationalLogger::open(
            Identity::new(Role::Gate, version, node),
            &settings,
            Box::new(std::io::sink()),
            stderr,
        )
    }

    /// Go's "Operational logging initialized." record, written right after the
    /// logger opens; file paths are reported as enabled flags only.
    pub fn emit_configured(&self, logger: &mut OperationalLogger, at: Duration) {
        logger.emit(
            at,
            Level::Info,
            "Operational logging initialized.",
            &[
                ("component", Value::Str("gate.runtime")),
                ("event", Value::Str("runtime.logging_configured")),
                ("console_format", Value::Str(&self.console_format)),
                ("console_stream", Value::Str(&self.console_stream)),
                (
                    "plain_file_enabled",
                    Value::Bool(!self.plain_file.is_empty()),
                ),
                (
                    "jsonl_file_enabled",
                    Value::Bool(!self.jsonl_file.is_empty()),
                ),
            ],
        );
        logger.flush();
    }
}
