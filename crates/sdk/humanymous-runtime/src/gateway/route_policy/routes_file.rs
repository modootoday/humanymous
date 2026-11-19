//! Route table file loading and attested-route preconditions (legacy
//! cmd/gate routes.go at baseline 981d25c, SoT-31 R2).
//!
//! Format: one `<path-prefix> <preset>` per line; blank lines and lines whose
//! trimmed text starts with `#` are ignored. Lines are split like Go
//! `bufio.Scanner` with `ScanLines` (one trailing `\r` dropped, 64 KiB line
//! limit), trimmed and split on Unicode white space like Go `strings`.
//! Error text equals Go, with the path written where Go wrote it.
//!
//! One deliberate difference: Go accepts invalid UTF-8 bytes in a prefix; this
//! port refuses the file ([`RoutesError::NotUtf8`]) because prefixes are Rust
//! strings.
use humanymous_core::signals::wire::quote_go;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Preset names a route file may use (Go `validPresets`).
pub const VALID_PRESETS: [&str; 5] = ["strict", "balanced", "attested", "monitor", "off"];

/// Go `bufio.MaxScanTokenSize`: a line of this many bytes or more is refused.
const MAX_LINE: usize = 64 * 1024;

/// Route file refusal.
#[derive(Debug)]
pub enum RoutesError {
    /// The file could not be read. The text is the OS error, not Go's.
    Io(std::io::Error),
    /// A line does not have exactly two fields.
    Malformed {
        /// Path as given.
        path: String,
        /// One-based line number.
        line_number: usize,
        /// Trimmed line.
        line: String,
    },
    /// The preset is not in [`VALID_PRESETS`].
    UnknownPreset {
        /// Path as given.
        path: String,
        /// One-based line number.
        line_number: usize,
        /// The preset field.
        preset: String,
    },
    /// `attested` on the catch-all prefix (`/` or empty).
    AttestedCatchAll {
        /// Path as given.
        path: String,
        /// One-based line number.
        line_number: usize,
        /// The prefix field.
        prefix: String,
    },
    /// A line reached the 64 KiB scanner limit.
    LineTooLong,
    /// A line is not UTF-8 (Go accepted it; see the module notes).
    NotUtf8 {
        /// One-based line number.
        line_number: usize,
    },
}

impl fmt::Display for RoutesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Malformed {
                path,
                line_number,
                line,
            } => write!(
                formatter,
                "routes {path}:{line_number}: expected `<prefix> <preset>`, got {}",
                quote_go(line)
            ),
            Self::UnknownPreset {
                path,
                line_number,
                preset,
            } => write!(
                formatter,
                "routes {path}:{line_number}: unknown preset {} (want strict|balanced|attested|monitor|off)",
                quote_go(preset)
            ),
            Self::AttestedCatchAll {
                path,
                line_number,
                prefix,
            } => write!(
                formatter,
                "routes {path}:{line_number}: preset \"attested\" is refused on the catch-all prefix {} \u{2014} mark specific high-value routes (e.g. /checkout), not the whole site",
                quote_go(prefix)
            ),
            Self::LineTooLong => formatter.write_str("bufio.Scanner: token too long"),
            Self::NotUtf8 { line_number } => {
                write!(formatter, "routes line {line_number}: not UTF-8")
            }
        }
    }
}

impl std::error::Error for RoutesError {}

/// Read and parse a route file (Go `loadRoutes`).
pub fn load_routes(path: &Path) -> Result<BTreeMap<String, String>, RoutesError> {
    let bytes = std::fs::read(path).map_err(RoutesError::Io)?;
    parse_routes(&path.display().to_string(), &bytes)
}

/// Parse route file bytes; `path` only labels error text.
pub fn parse_routes(path: &str, bytes: &[u8]) -> Result<BTreeMap<String, String>, RoutesError> {
    let mut routes = BTreeMap::new();
    let mut rest = bytes;
    let mut line_number = 0;
    while !rest.is_empty() {
        let (raw, next) = match rest.iter().position(|&byte| byte == b'\n') {
            Some(end) => (&rest[..end], &rest[end + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        if raw.len() >= MAX_LINE {
            return Err(RoutesError::LineTooLong);
        }
        rest = next;
        line_number += 1;
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let text = std::str::from_utf8(raw).map_err(|_| RoutesError::NotUtf8 { line_number })?;
        let line = text.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [prefix, preset] = fields[..] else {
            return Err(RoutesError::Malformed {
                path: path.to_owned(),
                line_number,
                line: line.to_owned(),
            });
        };
        if !VALID_PRESETS.contains(&preset) {
            return Err(RoutesError::UnknownPreset {
                path: path.to_owned(),
                line_number,
                preset: preset.to_owned(),
            });
        }
        if preset == "attested" && (prefix == "/" || prefix.is_empty()) {
            return Err(RoutesError::AttestedCatchAll {
                path: path.to_owned(),
                line_number,
                prefix: prefix.to_owned(),
            });
        }
        routes.insert(prefix.to_owned(), preset.to_owned());
    }
    Ok(routes)
}

/// An `attested` route without a shared token key (Go
/// `validateAttestedRoutes` fatal error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedRouteError {
    /// The attested prefix.
    pub prefix: String,
}

impl fmt::Display for AttestedRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "route {} uses preset \"attested\" but no shared HMN_TOKEN_KEY is set \u{2014} the step-up receipt from the Core Pass could never verify at the Gate, making the route an unredeemable Pass loop for humans; set a shared HMN_TOKEN_KEY (hex, \u{2265}16 bytes) across Core and Gate",
            quote_go(&self.prefix)
        )
    }
}

impl std::error::Error for AttestedRouteError {}

/// Ceiling-guard #1 deployment preconditions (Go `validateAttestedRoutes`):
/// an attested route without a shared token key is fatal; without a
/// possession verifier it warns once per attested route.
///
/// Go visits routes in map order, so with several attested routes its error
/// names any one of them and its warnings come in any order. Rust visits in
/// byte order, which is one of Go's possible outcomes.
pub fn validate_attested_routes(
    routes: &BTreeMap<String, String>,
    shared_token_key: bool,
    has_verifier: bool,
) -> Result<Vec<String>, AttestedRouteError> {
    let mut warnings = Vec::new();
    for (prefix, preset) in routes {
        if preset != "attested" {
            continue;
        }
        if !shared_token_key {
            return Err(AttestedRouteError {
                prefix: prefix.clone(),
            });
        }
        if !has_verifier {
            warnings.push(format!(
                "route {} uses preset \"attested\" but NO possession verifier (-agent-keys/-pat-issuers/-webauthn-creds) is configured \u{2014} every user (incl. returning humans) must solve a Pass to clear the floor (still no-lockout, but no attestation fast-path). Configure a credential verifier to give possession-holders a shortcut.",
                quote_go(prefix)
            ));
        }
    }
    Ok(warnings)
}

/// The attestation floor mapped onto the catch-all prefix of a programmatic
/// route table (Go `errAttestedCatchAll`, checked by `NewServer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedCatchAll {
    /// The offending prefix as configured.
    pub prefix: String,
}

impl fmt::Display for AttestedCatchAll {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("gate: the \"attested\" preset (attestation floor) is refused on the catch-all/root prefix \u{2014} mark specific high-value routes (e.g. /checkout), not the whole site")
    }
}

impl std::error::Error for AttestedCatchAll {}

/// Ceiling-guard #1 defense in depth (Go `NewServer`): refuse a route table
/// that maps a preset carrying the attestation floor onto a prefix that
/// normalizes to the root, or onto the empty prefix, whoever built the table.
/// The route file loader refuses `/` itself; this covers every other source.
pub fn refuse_attested_catch_all(
    routes: &BTreeMap<String, String>,
) -> Result<(), AttestedCatchAll> {
    match routes.iter().find(|(prefix, preset)| {
        super::preset_by_name(preset).attest_floor
            && (prefix.is_empty() || super::normalize_match_path(prefix) == "/")
    }) {
        Some((prefix, _)) => Err(AttestedCatchAll {
            prefix: prefix.clone(),
        }),
        None => Ok(()),
    }
}
