//! Enforcement route presets and the route table (legacy internal/gate
//! config.go route resolution, configsign.go and cmd/gate routes.go at
//! baseline 981d25c; roadmap B12).
//!
//! A preset only selects enforcement behaviour for a path (inject, enforce,
//! fail closed, synchronous re-score, attestation floor). It carries no score
//! weight, threshold or signal, so detection scoring is untouched.
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{collections::BTreeMap, fmt, time::Duration};

/// Enforcement behaviour for one route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePolicy {
    /// Preset name.
    pub name: &'static str,
    /// Inject the detection bundle into HTML (SoT-20).
    pub inject: bool,
    /// `false` is monitor/shadow: score and log, act on nothing (SoT-24 §3).
    pub enforce: bool,
    /// An Unknown verdict challenges (sensitive routes, SoT-21).
    pub fail_closed: bool,
    /// Synchronous pre-mutation re-score (high assurance, SoT-21 §2).
    pub sync_score: bool,
    /// Attested-route floor (ceiling-guard #1).
    pub attest_floor: bool,
}

const fn preset(
    name: &'static str,
    inject: bool,
    enforce: bool,
    fail_closed: bool,
    sync_score: bool,
    attest_floor: bool,
) -> RoutePolicy {
    RoutePolicy {
        name,
        inject,
        enforce,
        fail_closed,
        sync_score,
        attest_floor,
    }
}

/// `strict`: enforce, fail closed, synchronous re-score.
pub const STRICT: RoutePolicy = preset("strict", true, true, true, true, false);
/// `balanced`: the default for unmatched paths.
pub const BALANCED: RoutePolicy = preset("balanced", true, true, false, false, false);
/// `monitor`: score and log only.
pub const MONITOR: RoutePolicy = preset("monitor", true, false, false, false, false);
/// `off`: no injection and no enforcement.
pub const OFF: RoutePolicy = preset("off", false, false, false, false, false);
/// `attested`: strict plus the attestation floor.
pub const ATTESTED: RoutePolicy = preset("attested", true, true, true, true, true);

/// Preset vocabulary accepted in a routes file.
pub const PRESET_NAMES: [&str; 5] = ["strict", "balanced", "attested", "monitor", "off"];

/// Preset by name; anything unknown is `balanced` (Go `presetByName`).
pub fn preset_by_name(name: &str) -> RoutePolicy {
    match name {
        "strict" => STRICT,
        "attested" => ATTESTED,
        "monitor" => MONITOR,
        "off" => OFF,
        _ => BALANCED,
    }
}

/// Default rate window, soft and hard limits (Go `DefaultRate*`).
pub const DEFAULT_RATE_WINDOW: Duration = Duration::from_secs(10);
/// Default soft rate limit.
pub const DEFAULT_RATE_SOFT: u32 = 60;
/// Default hard rate limit.
pub const DEFAULT_RATE_HARD: u32 = 120;

/// Prefix to preset name table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteTable {
    /// Prefix to preset name.
    pub routes: BTreeMap<String, String>,
    /// Global monitor mode: nothing enforces.
    pub global_monitor: bool,
}

impl RouteTable {
    /// Policy for `raw_path`: the longest segment-anchored prefix match on the
    /// cleaned path, else `balanced`. Path confusion (`/./admin`, `//admin`,
    /// `/x/../admin`) resolves to the protected route (deep-review bypass).
    pub fn resolve(&self, raw_path: &str) -> RoutePolicy {
        let path = normalize_match_path(raw_path);
        let mut best = BALANCED;
        let mut best_len = None;
        for (prefix, name) in &self.routes {
            if segment_match(&path, prefix) && best_len.is_none_or(|len| prefix.len() > len) {
                best = preset_by_name(name);
                best_len = Some(prefix.len());
            }
        }
        if self.global_monitor {
            best.enforce = false;
        }
        best
    }
}

/// Lexically cleaned, rooted path (Go `path.Clean` on a rooted path).
pub fn normalize_match_path(raw: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    format!("/{}", segments.join("/"))
}

/// `path` equals `prefix` or continues it at a segment boundary; `/` and the
/// empty prefix match everything.
pub fn segment_match(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() || prefix == "/" {
        return true;
    }
    let prefix = prefix.trim_end_matches('/');
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Why a routes file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutesError {
    /// A line is not `<prefix> <preset>`.
    Malformed {
        /// One-based line number.
        line: usize,
        /// The trimmed line.
        text: String,
    },
    /// The preset is outside [`PRESET_NAMES`].
    UnknownPreset {
        /// One-based line number.
        line: usize,
        /// The refused preset.
        preset: String,
    },
    /// `attested` on the catch-all prefix (ceiling-guard #1).
    AttestedCatchAll {
        /// One-based line number.
        line: usize,
        /// The catch-all prefix.
        prefix: String,
    },
    /// An `attested` route without a shared token key: the step-up receipt
    /// could never verify, an unredeemable Pass loop for humans.
    AttestedWithoutSharedKey {
        /// The attested prefix.
        prefix: String,
    },
}

impl fmt::Display for RoutesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { line, text } => write!(
                formatter,
                "routes line {line}: expected `<prefix> <preset>`, got {text:?}"
            ),
            Self::UnknownPreset { line, preset } => write!(
                formatter,
                "routes line {line}: unknown preset {preset:?} (want strict|balanced|attested|monitor|off)"
            ),
            Self::AttestedCatchAll { line, prefix } => write!(
                formatter,
                "routes line {line}: preset \"attested\" is refused on the catch-all prefix {prefix:?}; mark specific high-value routes, not the whole site"
            ),
            Self::AttestedWithoutSharedKey { prefix } => write!(
                formatter,
                "route {prefix:?} uses preset \"attested\" but no shared HMN_TOKEN_KEY is set; the Core step-up receipt could never verify at the Gate"
            ),
        }
    }
}

impl std::error::Error for RoutesError {}

/// Parse a routes file: one `<path-prefix> <preset>` per line, `#` comments
/// and blank lines ignored (SoT-31 R2).
pub fn parse_routes(text: &str) -> Result<BTreeMap<String, String>, RoutesError> {
    let mut routes = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [prefix, preset] = fields[..] else {
            return Err(RoutesError::Malformed {
                line: line_number,
                text: line.into(),
            });
        };
        if !PRESET_NAMES.contains(&preset) {
            return Err(RoutesError::UnknownPreset {
                line: line_number,
                preset: preset.into(),
            });
        }
        if preset == "attested" && (prefix == "/" || prefix.is_empty()) {
            return Err(RoutesError::AttestedCatchAll {
                line: line_number,
                prefix: prefix.into(),
            });
        }
        routes.insert(prefix.to_owned(), preset.to_owned());
    }
    Ok(routes)
}

/// Read and parse a routes file.
pub fn load_routes(path: &std::path::Path) -> Result<BTreeMap<String, String>, LoadError> {
    let text = std::fs::read_to_string(path).map_err(LoadError::Read)?;
    parse_routes(&text).map_err(LoadError::Invalid)
}

/// A routes file that could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read.
    Read(std::io::Error),
    /// The file content was refused.
    Invalid(RoutesError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "routes file: {error}"),
            Self::Invalid(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LoadError {}

/// Ceiling-guard #1 startup preconditions: an `attested` route without a
/// shared token key is fatal; one without a possession verifier is a warning
/// (the floor degrades to a Pass for everyone).
pub fn validate_attested_routes(
    routes: &BTreeMap<String, String>,
    shared_token_key: bool,
    has_verifier: bool,
) -> Result<Vec<String>, RoutesError> {
    let mut warnings = Vec::new();
    for (prefix, preset) in routes {
        if preset != "attested" {
            continue;
        }
        if !shared_token_key {
            return Err(RoutesError::AttestedWithoutSharedKey {
                prefix: prefix.clone(),
            });
        }
        if !has_verifier {
            warnings.push(format!(
                "route {prefix:?} uses preset \"attested\" but no possession verifier is configured; every user must solve a Pass to clear the floor"
            ));
        }
    }
    Ok(warnings)
}

/// Inputs bound into the signed configuration version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePolicy<'a> {
    /// Route table and global monitor mode.
    pub routes: &'a RouteTable,
    /// Kill switch engaged.
    pub kill_switch: bool,
    /// Rate window.
    pub rate_window: Duration,
    /// Soft rate limit.
    pub rate_soft: u32,
    /// Hard rate limit.
    pub rate_hard: u32,
}

/// Deterministic keyed hash of the effective policy, `cfg-` plus 12 hex
/// digits (SoT-28 WS8): records name the exact policy in force, so an
/// unapproved change is detectable.
pub fn config_version(key: &[u8], policy: &EffectivePolicy<'_>) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key");
    let mut text = format!(
        "monitor={}|kill={}",
        policy.routes.global_monitor, policy.kill_switch
    );
    for (prefix, preset) in &policy.routes.routes {
        text.push_str(&format!("|{prefix}={preset}"));
    }
    text.push_str(&format!(
        "|rl={}/{}/{}",
        policy.rate_window.as_secs(),
        policy.rate_soft,
        policy.rate_hard
    ));
    mac.update(text.as_bytes());
    let digest = mac.finalize().into_bytes();
    format!("cfg-{}", hex::encode(&digest[..6]))
}

#[cfg(test)]
mod tests;
