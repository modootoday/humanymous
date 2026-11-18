//! Go `encoding/json` wire behavior for the overlay types.
//!
//! Encoding follows Go struct field order, `omitempty`, sorted map keys, Go
//! float text and HTML escaping. Decoding validates the whole document first
//! (as Go's `checkValid` does, with its 10000 nesting limit), then maps fields
//! with Go's rules: exact then case-folded key match, later duplicates win,
//! duplicate objects and maps merge, `null` leaves strings and timestamps
//! unchanged, invalid UTF-8 bytes and lone surrogate escapes become U+FFFD.
//! Error message text is this crate's own; only success versus refusal and the
//! decoded value are oracle-equivalent.
use super::{Mode, Overlay, RateLimitPatch, ScoringPatch, Timestamp};
use humanymous_core::signals::wire::{format_float_go, json_float_go, quote_json_go};
use std::collections::BTreeMap;

/// The document is not an overlay the oracle would decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayDecodeError(String);

impl std::fmt::Display for OverlayDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "overlay decode: {}", self.0)
    }
}

impl std::error::Error for OverlayDecodeError {}

/// Go `json.Marshal` refused a value; the message equals the oracle's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEncodeError(String);

impl std::fmt::Display for OverlayEncodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OverlayEncodeError {}

/// Value tree written with Go encoder semantics. Objects keep insertion order;
/// callers insert struct fields in declaration order and map keys sorted.
pub(super) enum Json {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Time(Timestamp),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub(super) fn strings<'a>(values: impl IntoIterator<Item = &'a str>) -> Self {
        Self::Array(
            values
                .into_iter()
                .map(|value| Self::Str(value.into()))
                .collect(),
        )
    }
    pub(super) fn modes(map: &BTreeMap<String, Mode>) -> Self {
        Self::Object(
            map.iter()
                .map(|(key, mode)| (key.clone(), Self::Str(mode.as_str().into())))
                .collect(),
        )
    }
    pub(super) fn texts(map: &BTreeMap<String, String>) -> Self {
        Self::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), Self::Str(value.clone())))
                .collect(),
        )
    }
    pub(super) fn floats(map: &BTreeMap<String, f64>) -> Self {
        Self::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), Self::Float(*value)))
                .collect(),
        )
    }
    /// Compact (`json.Marshal`) or two-space indented (`json.MarshalIndent`).
    pub(super) fn encode(&self, indent: bool) -> Result<String, OverlayEncodeError> {
        let mut output = String::new();
        self.write(&mut output, indent, 0)?;
        Ok(output)
    }
    fn write(
        &self,
        output: &mut String,
        indent: bool,
        depth: usize,
    ) -> Result<(), OverlayEncodeError> {
        let newline = |output: &mut String, depth: usize| {
            if indent {
                output.push('\n');
                output.push_str(&"  ".repeat(depth));
            }
        };
        match self {
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Int(value) => output.push_str(&value.to_string()),
            Self::Float(value) => match json_float_go(*value) {
                Some(text) => output.push_str(&text),
                None => {
                    return Err(OverlayEncodeError(format!(
                        "json: unsupported value: {}",
                        format_float_go(*value)
                    )));
                }
            },
            Self::Str(value) => output.push_str(&quote_json_go(value)),
            Self::Time(value) => {
                let text = value.format().map_err(|error| {
                    OverlayEncodeError(format!(
                        "json: error calling MarshalJSON for type time.Time: {error}"
                    ))
                })?;
                output.push_str(&quote_json_go(&text));
            }
            Self::Array(items) if items.is_empty() => output.push_str("[]"),
            Self::Object(fields) if fields.is_empty() => output.push_str("{}"),
            Self::Array(items) => {
                output.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    newline(output, depth + 1);
                    item.write(output, indent, depth + 1)?;
                }
                newline(output, depth);
                output.push(']');
            }
            Self::Object(fields) => {
                output.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    newline(output, depth + 1);
                    output.push_str(&quote_json_go(key));
                    output.push_str(if indent { ": " } else { ":" });
                    value.write(output, indent, depth + 1)?;
                }
                newline(output, depth);
                output.push('}');
            }
        }
        Ok(())
    }
}

fn scoring_json(patch: &ScoringPatch) -> Json {
    let mut fields = Vec::new();
    for (name, value) in [
        ("challengeAt", patch.challenge_at),
        ("denyAt", patch.deny_at),
        ("layerCap", patch.layer_cap),
    ] {
        if let Some(value) = value {
            fields.push((name.into(), Json::Float(value)));
        }
    }
    Json::Object(fields)
}

fn rate_json(patch: &RateLimitPatch) -> Json {
    let mut fields = Vec::new();
    for (name, value) in [
        ("windowSec", patch.window_sec),
        ("soft", patch.soft),
        ("hard", patch.hard),
    ] {
        if let Some(value) = value {
            fields.push((name.into(), Json::Int(value)));
        }
    }
    Json::Object(fields)
}

/// Shared tail of the full and canonical encodings (identical field order).
fn push_policy_fields(fields: &mut Vec<(String, Json)>, overlay: &Overlay) {
    if !overlay.gates.is_empty() {
        fields.push(("gates".into(), Json::modes(&overlay.gates)));
    }
    if !overlay.hard_rules.is_empty() {
        fields.push(("hardRules".into(), Json::modes(&overlay.hard_rules)));
    }
    if let Some(scoring) = &overlay.scoring {
        fields.push(("scoring".into(), scoring_json(scoring)));
    }
    if !overlay.weight_multipliers.is_empty() {
        fields.push((
            "weightMultipliers".into(),
            Json::floats(&overlay.weight_multipliers),
        ));
    }
    if !overlay.net_policy.is_empty() {
        fields.push(("netPolicy".into(), Json::modes(&overlay.net_policy)));
    }
    if !overlay.routes.is_empty() {
        fields.push(("routes".into(), Json::texts(&overlay.routes)));
    }
    if let Some(rate) = &overlay.rate_limit {
        fields.push(("rateLimit".into(), rate_json(rate)));
    }
    if let Some(monitor) = overlay.global_monitor {
        fields.push(("globalMonitor".into(), Json::Bool(monitor)));
    }
}

fn push_nonempty(fields: &mut Vec<(String, Json)>, name: &str, value: &str) {
    if !value.is_empty() {
        fields.push((name.into(), Json::Str(value.into())));
    }
}

/// Oracle `CanonicalJSON` wire struct.
pub(super) fn canonical(overlay: &Overlay) -> Result<String, OverlayEncodeError> {
    let mut fields = vec![
        (
            "schemaVersion".into(),
            Json::Str(overlay.schema_version.clone()),
        ),
        ("overlayId".into(), Json::Str(overlay.overlay_id.clone())),
        ("status".into(), Json::Str(overlay.status.clone())),
    ];
    push_nonempty(
        &mut fields,
        "parentConfigVersion",
        &overlay.parent_config_version,
    );
    push_nonempty(&mut fields, "mutationClass", &overlay.mutation_class);
    push_policy_fields(&mut fields, overlay);
    Json::Object(fields).encode(false)
}

/// Oracle `Overlay` struct under `json.MarshalIndent(o, "", "  ")`.
pub(super) fn indented(overlay: &Overlay) -> Result<String, OverlayEncodeError> {
    let mut fields = vec![
        (
            "schemaVersion".into(),
            Json::Str(overlay.schema_version.clone()),
        ),
        ("overlayId".into(), Json::Str(overlay.overlay_id.clone())),
        ("createdAt".into(), Json::Time(overlay.created_at)),
    ];
    push_nonempty(&mut fields, "createdBy", &overlay.created_by);
    push_nonempty(&mut fields, "approvedBy", &overlay.approved_by);
    fields.push(("status".into(), Json::Str(overlay.status.clone())));
    if let Some(expires) = overlay.expires_at {
        fields.push(("expiresAt".into(), Json::Time(expires)));
    }
    push_nonempty(
        &mut fields,
        "parentConfigVersion",
        &overlay.parent_config_version,
    );
    push_nonempty(&mut fields, "mutationClass", &overlay.mutation_class);
    push_policy_fields(&mut fields, overlay);
    Json::Object(fields).encode(true)
}

// ---------------------------------------------------------------------------
// Decoding

const MAX_DEPTH: usize = 10_000;

fn error(message: impl Into<String>) -> OverlayDecodeError {
    OverlayDecodeError(message.into())
}

/// Replace every byte of an invalid UTF-8 sequence with U+FFFD individually,
/// as Go's `utf8.DecodeRune` loop in `unquote` does.
fn replace_invalid_utf8(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len());
    let mut rest = bytes;
    loop {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                output.push_str(valid);
                return output;
            }
            Err(failure) => {
                let (valid, invalid) = rest.split_at(failure.valid_up_to());
                output.push_str(std::str::from_utf8(valid).expect("validated prefix"));
                output.push('\u{fffd}');
                rest = &invalid[1..];
            }
        }
    }
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

/// Syntax check of the complete document (Go `checkValid`), iterative so the
/// 10000-level nesting limit cannot exhaust the stack.
fn check_valid(bytes: &[u8]) -> Result<(), OverlayDecodeError> {
    let mut parser = Parser { bytes, position: 0 };
    let mut stack: Vec<u8> = Vec::new();
    parser.space();
    'value: loop {
        // Parse one value start; containers push and continue.
        match parser.peek() {
            Some(b'{') => {
                parser.position += 1;
                stack.push(b'{');
                if stack.len() > MAX_DEPTH {
                    return Err(error("exceeded max depth"));
                }
                parser.space();
                if parser.peek() == Some(b'}') {
                    parser.position += 1;
                    stack.pop();
                } else {
                    parser.string_raw()?;
                    parser.space();
                    parser.expect(b':')?;
                    parser.space();
                    continue 'value;
                }
            }
            Some(b'[') => {
                parser.position += 1;
                stack.push(b'[');
                if stack.len() > MAX_DEPTH {
                    return Err(error("exceeded max depth"));
                }
                parser.space();
                if parser.peek() == Some(b']') {
                    parser.position += 1;
                    stack.pop();
                } else {
                    continue 'value;
                }
            }
            Some(b'"') => {
                parser.string_raw()?;
            }
            Some(b't') => parser.word("true")?,
            Some(b'f') => parser.word("false")?,
            Some(b'n') => parser.word("null")?,
            _ => {
                parser.number()?;
            }
        }
        // After a complete value: close containers or continue with siblings.
        loop {
            parser.space();
            match stack.last() {
                None => {
                    return if parser.position == bytes.len() {
                        Ok(())
                    } else {
                        Err(error("invalid character after top-level value"))
                    };
                }
                Some(&container) => match parser.peek() {
                    Some(b',') => {
                        parser.position += 1;
                        parser.space();
                        if container == b'{' {
                            parser.string_raw()?;
                            parser.space();
                            parser.expect(b':')?;
                            parser.space();
                        }
                        continue 'value;
                    }
                    Some(b'}') if container == b'{' => {
                        parser.position += 1;
                        stack.pop();
                    }
                    Some(b']') if container == b'[' => {
                        parser.position += 1;
                        stack.pop();
                    }
                    _ => return Err(error("invalid character in container")),
                },
            }
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }
    fn space(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.position += 1;
        }
    }
    fn expect(&mut self, byte: u8) -> Result<(), OverlayDecodeError> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(error(format!("expected {:?}", char::from(byte))))
        }
    }
    fn word(&mut self, word: &str) -> Result<(), OverlayDecodeError> {
        if self.bytes[self.position..].starts_with(word.as_bytes()) {
            self.position += word.len();
            Ok(())
        } else {
            Err(error("invalid literal"))
        }
    }
    fn digits(&mut self) -> usize {
        let start = self.position;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        self.position - start
    }
    /// RFC 8259 number; returns its literal text.
    fn number(&mut self) -> Result<&'a str, OverlayDecodeError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        match self.peek() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(error("invalid character looking for value")),
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            if self.digits() == 0 {
                return Err(error("invalid number fraction"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if self.digits() == 0 {
                return Err(error("invalid number exponent"));
            }
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.position]).expect("ASCII number"))
    }
    /// String token; returns the raw content between the quotes.
    fn string_raw(&mut self) -> Result<&'a [u8], OverlayDecodeError> {
        self.expect(b'"')?;
        let start = self.position;
        loop {
            match self.peek() {
                None => return Err(error("unterminated string")),
                Some(b'"') => {
                    let raw = &self.bytes[start..self.position];
                    self.position += 1;
                    return Ok(raw);
                }
                Some(b'\\') => {
                    self.position += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.position += 1;
                        }
                        Some(b'u') => {
                            let hex = self.bytes.get(self.position + 1..self.position + 5);
                            if !hex.is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit)) {
                                return Err(error("invalid unicode escape"));
                            }
                            self.position += 5;
                        }
                        _ => return Err(error("invalid escape")),
                    }
                }
                Some(byte) if byte < 0x20 => return Err(error("control character in string")),
                Some(_) => self.position += 1,
            }
        }
    }
    /// Skip one already-validated value.
    fn skip(&mut self) -> Result<(), OverlayDecodeError> {
        let mut depth = 0_usize;
        loop {
            self.space();
            match self.peek() {
                Some(b'"') => {
                    self.string_raw()?;
                }
                Some(b'{' | b'[') => {
                    self.position += 1;
                    depth += 1;
                    continue;
                }
                Some(b'}' | b']') => {
                    self.position += 1;
                    depth -= 1;
                }
                Some(b',' | b':') => {
                    self.position += 1;
                    continue;
                }
                Some(b't') => self.word("true")?,
                Some(b'f') => self.word("false")?,
                Some(b'n') => self.word("null")?,
                _ => {
                    self.number()?;
                }
            }
            if depth == 0 {
                return Ok(());
            }
        }
    }
    fn null(&mut self) -> bool {
        self.space();
        if self.bytes[self.position..].starts_with(b"null") {
            self.position += 4;
            true
        } else {
            false
        }
    }
    /// Iterate the members of a validated object; `field` consumes each value.
    fn object(
        &mut self,
        mut field: impl FnMut(&mut Self, String) -> Result<(), OverlayDecodeError>,
    ) -> Result<(), OverlayDecodeError> {
        self.space();
        self.expect(b'{')
            .map_err(|_| error("cannot decode non-object into object"))?;
        loop {
            self.space();
            match self.peek() {
                Some(b'}') => {
                    self.position += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.position += 1;
                    continue;
                }
                _ => {}
            }
            let key = unquote(self.string_raw()?);
            self.space();
            self.expect(b':')?;
            self.space();
            field(self, key)?;
        }
    }
    fn string_value(&mut self) -> Result<Option<String>, OverlayDecodeError> {
        if self.null() {
            return Ok(None);
        }
        if self.peek() != Some(b'"') {
            return Err(error("cannot decode non-string into string"));
        }
        Ok(Some(unquote(self.string_raw()?)))
    }
    fn float_value(&mut self) -> Result<Option<f64>, OverlayDecodeError> {
        if self.null() {
            return Ok(None);
        }
        if !matches!(self.peek(), Some(b'-' | b'0'..=b'9')) {
            return Err(error("cannot decode non-number into float64"));
        }
        let literal = self.number()?;
        let value: f64 = literal.parse().map_err(|_| error("invalid float"))?;
        if value.is_infinite() {
            return Err(error(format!("number {literal} out of float64 range")));
        }
        Ok(Some(value))
    }
    fn int_value(&mut self) -> Result<Option<i64>, OverlayDecodeError> {
        if self.null() {
            return Ok(None);
        }
        if !matches!(self.peek(), Some(b'-' | b'0'..=b'9')) {
            return Err(error("cannot decode non-number into int"));
        }
        let literal = self.number()?;
        literal
            .parse()
            .map(Some)
            .map_err(|_| error(format!("number {literal} is not an int64")))
    }
    fn bool_value(&mut self) -> Result<Option<bool>, OverlayDecodeError> {
        if self.null() {
            return Ok(None);
        }
        if self.bytes[self.position..].starts_with(b"true") {
            self.position += 4;
            Ok(Some(true))
        } else if self.bytes[self.position..].starts_with(b"false") {
            self.position += 5;
            Ok(Some(false))
        } else {
            Err(error("cannot decode non-boolean into bool"))
        }
    }
    /// Go `Time.UnmarshalJSON`: raw quoted bytes without unescaping; `null`
    /// leaves the value unchanged.
    fn time_value(&mut self) -> Result<Option<Timestamp>, OverlayDecodeError> {
        if self.null() {
            return Ok(None);
        }
        if self.peek() != Some(b'"') {
            return Err(error("Time.UnmarshalJSON: input is not a JSON string"));
        }
        let raw = self.string_raw()?;
        Timestamp::parse(raw)
            .map(Some)
            .map_err(|failure| error(failure.to_string()))
    }
    /// Merge object members into an existing map (Go reuses a non-nil map);
    /// `null` clears it and a `null` member stores the zero value.
    fn map_value<T: Default>(
        &mut self,
        map: &mut BTreeMap<String, T>,
        mut element: impl FnMut(&mut Self) -> Result<Option<T>, OverlayDecodeError>,
    ) -> Result<(), OverlayDecodeError> {
        if self.null() {
            map.clear();
            return Ok(());
        }
        self.object(|parser, key| {
            let value = element(parser)?.unwrap_or_default();
            map.insert(key, value);
            Ok(())
        })
    }
}

/// Go `unquote` of validated string content: lone or mismatched surrogate
/// escapes become U+FFFD; the input is already valid UTF-8.
fn unquote(raw: &[u8]) -> String {
    let text = std::str::from_utf8(raw).expect("replaced invalid UTF-8");
    if !text.contains('\\') {
        return text.into();
    }
    let hex = |at: usize| -> Option<u32> {
        let digits = text.get(at..at + 4)?;
        u32::from_str_radix(digits, 16).ok()
    };
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while let Some(offset) = text[index..].find('\\') {
        output.push_str(&text[index..index + offset]);
        index += offset + 1;
        let escape = text.as_bytes()[index];
        index += 1;
        let simple = match escape {
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let code = hex(index).expect("validated escape");
                index += 4;
                if (0xd800..0xdc00).contains(&code)
                    && text[index..].starts_with("\\u")
                    && let Some(low) = hex(index + 2).filter(|low| (0xdc00..0xe000).contains(low))
                {
                    index += 6;
                    char::from_u32(0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00))
                        .expect("valid surrogate pair")
                } else {
                    char::from_u32(code).unwrap_or('\u{fffd}')
                }
            }
            other => char::from(other),
        };
        output.push(simple);
    }
    output.push_str(&text[index..]);
    output
}

/// Go `encoding/json` field lookup: exact name first, then the case-folded
/// name. Only `ſ` and the Kelvin sign fold onto ASCII letters.
fn field(key: &str, names: &[&'static str]) -> Option<&'static str> {
    if let Some(name) = names.iter().find(|name| **name == key) {
        return Some(name);
    }
    let folded: String = key
        .chars()
        .map(|character| match character {
            '\u{17f}' => 'S',
            '\u{212a}' => 'K',
            other => other.to_ascii_uppercase(),
        })
        .collect();
    names
        .iter()
        .find(|name| name.to_ascii_uppercase() == folded)
        .copied()
}

const OVERLAY_FIELDS: [&str; 17] = [
    "schemaVersion",
    "overlayId",
    "createdAt",
    "createdBy",
    "approvedBy",
    "status",
    "expiresAt",
    "parentConfigVersion",
    "mutationClass",
    "gates",
    "hardRules",
    "scoring",
    "weightMultipliers",
    "netPolicy",
    "routes",
    "rateLimit",
    "globalMonitor",
];

fn assign(slot: &mut String, value: Option<String>) {
    if let Some(value) = value {
        *slot = value;
    }
}

fn mode(parser: &mut Parser<'_>) -> Result<Option<Mode>, OverlayDecodeError> {
    Ok(parser.string_value()?.map(|value| Mode::parse(&value)))
}

fn scoring(
    parser: &mut Parser<'_>,
    slot: &mut Option<ScoringPatch>,
) -> Result<(), OverlayDecodeError> {
    if parser.null() {
        *slot = None;
        return Ok(());
    }
    let patch = slot.get_or_insert_default();
    parser.object(|parser, key| {
        let target = match field(&key, &["challengeAt", "denyAt", "layerCap"]) {
            Some("challengeAt") => &mut patch.challenge_at,
            Some("denyAt") => &mut patch.deny_at,
            Some(_) => &mut patch.layer_cap,
            None => return parser.skip(),
        };
        *target = parser.float_value()?;
        Ok(())
    })
}

fn rate_limit(
    parser: &mut Parser<'_>,
    slot: &mut Option<RateLimitPatch>,
) -> Result<(), OverlayDecodeError> {
    if parser.null() {
        *slot = None;
        return Ok(());
    }
    let patch = slot.get_or_insert_default();
    parser.object(|parser, key| {
        let target = match field(&key, &["windowSec", "soft", "hard"]) {
            Some("windowSec") => &mut patch.window_sec,
            Some("soft") => &mut patch.soft,
            Some(_) => &mut patch.hard,
            None => return parser.skip(),
        };
        *target = parser.int_value()?;
        Ok(())
    })
}

/// Decode an overlay document with the oracle's `json.Unmarshal` semantics.
pub(super) fn decode(bytes: &[u8]) -> Result<Overlay, OverlayDecodeError> {
    let text = replace_invalid_utf8(bytes);
    check_valid(text.as_bytes())?;
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    let mut overlay = Overlay::default();
    if parser.null() {
        return Ok(overlay);
    }
    parser
        .object(|parser, key| {
            let Some(name) = field(&key, &OVERLAY_FIELDS) else {
                return parser.skip();
            };
            let overlay = &mut overlay;
            match name {
                "schemaVersion" => assign(&mut overlay.schema_version, parser.string_value()?),
                "overlayId" => assign(&mut overlay.overlay_id, parser.string_value()?),
                "createdBy" => assign(&mut overlay.created_by, parser.string_value()?),
                "approvedBy" => assign(&mut overlay.approved_by, parser.string_value()?),
                "status" => assign(&mut overlay.status, parser.string_value()?),
                "parentConfigVersion" => {
                    assign(&mut overlay.parent_config_version, parser.string_value()?)
                }
                "mutationClass" => assign(&mut overlay.mutation_class, parser.string_value()?),
                "createdAt" => {
                    if let Some(time) = parser.time_value()? {
                        overlay.created_at = time;
                    }
                }
                "expiresAt" => overlay.expires_at = parser.time_value()?,
                "gates" => parser.map_value(&mut overlay.gates, mode)?,
                "hardRules" => parser.map_value(&mut overlay.hard_rules, mode)?,
                "netPolicy" => parser.map_value(&mut overlay.net_policy, mode)?,
                "routes" => parser.map_value(&mut overlay.routes, Parser::string_value)?,
                "weightMultipliers" => {
                    parser.map_value(&mut overlay.weight_multipliers, Parser::float_value)?
                }
                "scoring" => scoring(parser, &mut overlay.scoring)?,
                "rateLimit" => rate_limit(parser, &mut overlay.rate_limit)?,
                _ => overlay.global_monitor = parser.bool_value()?,
            }
            Ok(())
        })
        .map_err(|failure| {
            if failure.0 == "cannot decode non-object into object" {
                error("cannot decode non-object into Overlay")
            } else {
                failure
            }
        })?;
    Ok(overlay)
}
