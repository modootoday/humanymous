//! Canonical operational record: sanitization, bounding and the shared plain
//! and JSONL encodings (legacy internal/logger record.go at baseline 981d25c).
//!
//! Both encodings render the same record so a reader of either sink sees the
//! same envelope. Plain and JSONL output for ASCII text is byte-identical to
//! Go (pinned by a golden test). Two deliberate approximations remain and are
//! documented rather than hidden: plain quoting escapes only `"`, `\` and a
//! fixed set of invisible format code points (Go `strconv.Quote` escapes every
//! non-printable rune), and JSON float text follows serde rather than Go.
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Duration;

/// Version of the record envelope shared by both encodings.
pub const SCHEMA_VERSION: &str = "1.0.0";
/// Upper bound on one physical line in either encoding, newline included.
pub const MAX_RECORD_BYTES: usize = 4096;
/// Upper bound on structured fields retained per record.
pub const MAX_FIELDS: usize = 64;
pub(super) const MAX_IDENTITY_BYTES: usize = 256;
const MAX_MESSAGE_BYTES: usize = 1024;
const MAX_FIELD_BYTES: usize = 1024;
const MAX_FIELD_KEY_BYTES: usize = 128;

/// Operational severity; `Off` is a threshold, never a record level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Diagnostic detail.
    Debug,
    /// Normal lifecycle events.
    Info,
    /// Degraded but serving.
    Warn,
    /// Failed operation.
    Error,
}

impl Level {
    /// Lowercase wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One structured field value. There is deliberately no variant for errors,
/// byte slices or arbitrary `Display` values: Go mapped those to the fixed
/// text `unsupported`; here they cannot be expressed at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Unsigned integer.
    U64(u64),
    /// Float; non-finite values become the text `non_finite`.
    F64(f64),
    /// Text; sanitized and bounded before encoding.
    Str(&'a str),
}

/// Owned attribute for the queued logger, where a record is built on the
/// producer thread and attributes bound with `with_attrs` outlive the call.
/// A group nests its children under its key as Go `slog.Group` did.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    /// Raw key; sanitized when the record is built.
    pub key: String,
    /// Scalar value or nested group.
    pub value: AttrValue,
}

/// Value of an owned [`Attr`].
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Unsigned integer.
    U64(u64),
    /// Float; non-finite values become the text `non_finite`.
    F64(f64),
    /// Text; sanitized and bounded before encoding.
    Str(String),
    /// Children recorded as `key.child`; a group never sets the envelope.
    Group(Vec<Attr>),
}

impl Attr {
    /// Scalar attribute.
    pub fn new(key: &str, value: Value<'_>) -> Self {
        let value = match value {
            Value::Bool(v) => AttrValue::Bool(v),
            Value::I64(v) => AttrValue::I64(v),
            Value::U64(v) => AttrValue::U64(v),
            Value::F64(v) => AttrValue::F64(v),
            Value::Str(v) => AttrValue::Str(v.to_owned()),
        };
        Self {
            key: key.to_owned(),
            value,
        }
    }

    /// Group attribute; an empty key drops the group and marks truncation.
    pub fn group(key: &str, children: Vec<Attr>) -> Self {
        Self {
            key: key.to_owned(),
            value: AttrValue::Group(children),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
enum Stored {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Record {
    schema_version: &'static str,
    ts: String,
    level: &'static str,
    service: String,
    version: String,
    node: String,
    component: String,
    event: String,
    message: String,
    pub(super) sequence: u64,
    fields: BTreeMap<String, Stored>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    truncated: bool,
}

impl Record {
    pub(super) fn new(
        at: Duration,
        level: Level,
        identity: [&str; 3],
        message: &str,
        attrs: &[(&str, Value<'_>)],
    ) -> Self {
        let mut record = Self::envelope(at, level, identity, message);
        for (key, value) in attrs {
            record.add_scalar("", key, *value);
        }
        record
    }

    /// Envelope without fields; attributes follow with [`Self::add_attr`].
    pub(super) fn envelope(at: Duration, level: Level, identity: [&str; 3], message: &str) -> Self {
        let [service, version, node] = identity;
        let (message, message_changed) = sanitize(message, MAX_MESSAGE_BYTES);
        let (component, component_changed) =
            sanitize(&format!("{service}.runtime"), MAX_IDENTITY_BYTES);
        Self {
            schema_version: SCHEMA_VERSION,
            ts: rfc3339_nano(at),
            level: level.as_str(),
            service: service.to_owned(),
            version: version.to_owned(),
            node: node.to_owned(),
            component,
            event: "runtime.message".to_owned(),
            message,
            sequence: 0,
            fields: BTreeMap::new(),
            truncated: message_changed || component_changed,
        }
    }

    /// Go `record.addAttr`: the sanitized key joins a non-empty group
    /// `prefix` with a dot; only top-level `component` and `event` set the
    /// envelope; a group recurses with its joined key as the new prefix.
    pub(super) fn add_attr(&mut self, prefix: &str, attr: &Attr) {
        let value = match &attr.value {
            AttrValue::Group(children) => {
                if let Some(key) = self.field_key(prefix, &attr.key) {
                    for child in children {
                        self.add_attr(&key, child);
                    }
                }
                return;
            }
            AttrValue::Bool(v) => Value::Bool(*v),
            AttrValue::I64(v) => Value::I64(*v),
            AttrValue::U64(v) => Value::U64(*v),
            AttrValue::F64(v) => Value::F64(*v),
            AttrValue::Str(v) => Value::Str(v),
        };
        self.add_scalar(prefix, &attr.key, value);
    }

    fn field_key(&mut self, prefix: &str, raw_key: &str) -> Option<String> {
        let (key, changed) = sanitize(raw_key, MAX_FIELD_KEY_BYTES);
        let mut key = sanitize_key(&key);
        if !prefix.is_empty() {
            key = format!("{prefix}.{key}");
        }
        if changed || key.is_empty() {
            self.truncated = true;
        }
        (!key.is_empty()).then_some(key)
    }

    fn add_scalar(&mut self, prefix: &str, raw_key: &str, value: Value<'_>) {
        let Some(key) = self.field_key(prefix, raw_key) else {
            return;
        };
        if prefix.is_empty() && (key == "component" || key == "event") {
            let (text, mut changed) = sanitize(&value_text(value), MAX_IDENTITY_BYTES);
            if key == "component" {
                if !text.is_empty() {
                    self.component = text;
                }
            } else if valid_event(&text) {
                self.event = text;
            } else {
                self.event = "runtime.invalid_event".to_owned();
                changed = true;
            }
            self.truncated |= changed;
            return;
        }
        if self.fields.len() >= MAX_FIELDS && !self.fields.contains_key(&key) {
            self.truncated = true;
            return;
        }
        let (stored, changed) = match value {
            Value::Bool(v) => (Stored::Bool(v), false),
            Value::I64(v) => (Stored::I64(v), false),
            Value::U64(v) => (Stored::U64(v), false),
            Value::F64(v) if v.is_finite() => (Stored::F64(v), false),
            Value::F64(_) => (Stored::Str("non_finite".to_owned()), true),
            Value::Str(v) => {
                let (text, changed) = sanitize(v, MAX_FIELD_BYTES);
                (Stored::Str(text), changed)
            }
        };
        self.fields.insert(key, stored);
        self.truncated |= changed;
    }

    /// Bound both encodings to `MAX_RECORD_BYTES`: drop fields from the
    /// lexically last key, then shorten the message in 64-byte steps.
    pub(super) fn fit(&mut self) {
        if self.fits() {
            return;
        }
        self.truncated = true;
        while self.fields.pop_last().is_some() {
            if self.fits() {
                return;
            }
        }
        while !self.message.is_empty() {
            let target = self.message.len().saturating_sub(64);
            self.message = truncate_utf8(&self.message, target);
            if self.fits() {
                return;
            }
        }
    }

    fn fits(&self) -> bool {
        self.json_line().len() <= MAX_RECORD_BYTES && self.plain_line().len() <= MAX_RECORD_BYTES
    }

    pub(super) fn json_line(&self) -> String {
        // Serializing owned strings, integers and finite floats cannot fail.
        let encoded = serde_json::to_string(self).unwrap_or_default();
        // Go's encoder escapes HTML-significant bytes; these only occur
        // inside JSON strings here, so a global replacement is exact.
        let mut line = encoded
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026");
        line.push('\n');
        line
    }

    pub(super) fn plain_line(&self) -> String {
        let mut line = String::with_capacity(256);
        line.push_str(&self.ts);
        line.push(' ');
        line.push_str(&self.level.to_ascii_uppercase());
        for (key, value) in [
            ("service", &self.service),
            ("version", &self.version),
            ("node", &self.node),
            ("component", &self.component),
            ("event", &self.event),
        ] {
            let _ = write!(line, " {key}={}", quote(value));
        }
        let _ = write!(line, " sequence={}", self.sequence);
        let _ = write!(line, " message={}", quote(&self.message));
        for (key, value) in &self.fields {
            let text = match value {
                Stored::Bool(v) => v.to_string(),
                Stored::I64(v) => v.to_string(),
                Stored::U64(v) => v.to_string(),
                Stored::F64(v) => v.to_string(),
                Stored::Str(v) => quote(v),
            };
            let _ = write!(line, " field.{key}={text}");
        }
        if self.truncated {
            line.push_str(" truncated=true");
        }
        line.push('\n');
        line
    }
}

fn value_text(value: Value<'_>) -> String {
    match value {
        Value::Bool(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::Str(v) => v.to_owned(),
    }
}

/// Strip ANSI escapes, control characters and bidirectional/separator code
/// points, then bound to `max` bytes on a UTF-8 boundary. Returns whether the
/// text changed.
pub(super) fn sanitize(value: &str, max: usize) -> (String, bool) {
    let clean: String = strip_ansi(value)
        .chars()
        .filter(|c| !c.is_control() && !is_direction_or_separator(*c))
        .collect();
    let clean = truncate_utf8(&clean, max);
    let changed = clean != value;
    (clean, changed)
}

fn truncate_utf8(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_owned();
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn strip_ansi(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b'[' => {
                i += 1;
                while i < bytes.len() {
                    let last = (0x40..=0x7e).contains(&bytes[i]);
                    i += 1;
                    if last {
                        break;
                    }
                }
            }
            b']' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    // Escape sequences may end inside a multi-byte character; repair exactly
    // as Go's later ToValidUTF8 pass would.
    String::from_utf8_lossy(&out).into_owned()
}

fn is_direction_or_separator(c: char) -> bool {
    matches!(c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}'
        | '\u{2066}'..='\u{2069}' | '\u{2028}' | '\u{2029}' | '\u{feff}')
}

pub(super) fn sanitize_key(value: &str) -> String {
    let mapped: String = value
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '_' | '.' => c,
            _ => '_',
        })
        .collect();
    mapped.trim_matches(['_', '.']).to_owned()
}

fn valid_event(value: &str) -> bool {
    if !value.starts_with(|c: char| c.is_ascii_lowercase()) || value.ends_with('.') {
        return false;
    }
    !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
}

fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{00ad}' | '\u{200b}'..='\u{200d}' | '\u{2060}'..='\u{2064}' | '\u{180e}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c if c.is_whitespace() && c != ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// RFC 3339 UTC with trailing-zero-trimmed nanoseconds, as Go `RFC3339Nano`.
pub(super) fn rfc3339_nano(since_epoch: Duration) -> String {
    let secs = since_epoch.as_secs();
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let mut text = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    );
    let nanos = since_epoch.subsec_nanos();
    if nanos != 0 {
        let fraction = format!("{nanos:09}");
        text.push('.');
        text.push_str(fraction.trim_end_matches('0'));
    }
    text.push('Z');
    text
}

/// Days since 1970-01-01 to proleptic Gregorian date (Hinnant's algorithm).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}
