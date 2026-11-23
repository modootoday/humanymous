//! Route policy presets, path resolution and the verdict-to-edge-action
//! mapping (legacy internal/gate config.go and enforce.go at baseline 981d25c,
//! SoT-21 §1-§2, SoT-24 §1-§3, SoT-28 WS5).
//!
//! Owner decision B1 (#75 comment 6979694428): an exact-parity port of the
//! oracle. Preset flags, the balanced default for an unknown name, path
//! normalization, segment matching and longest-prefix selection equal Go and
//! are proven against a golden captured from the oracle.
//!
//! One cell deliberately differs. Go `Verdict.action` passed an `Unknown`
//! verdict on a safe method of a non-sensitive route (balanced, monitor, off).
//! Owner decisions 2026-11-16 (`.agents/retirement/owner-decisions-2026-11-16.md`)
//! and 2026-11-18 C1 keep the Rust fail-closed mapping, so [`edge_action`]
//! challenges every `Unknown` verdict. No other cell differs.
//!
//! This is a pure library. The hosted gateway still uses its exact route
//! inventory; each inventoried route may name a preset
//! (`configuration::Route::policy`, owner decision #4), while the prefix
//! table here is not on the served path.
use crate::gateway::verdict_store::Verdict;
use std::collections::BTreeMap;

mod routes_file;

pub use routes_file::{
    AttestedCatchAll, AttestedRouteError, RoutesError, VALID_PRESETS, load_routes, parse_routes,
    refuse_attested_catch_all, validate_attested_routes,
};

/// Resolved policy for a matched route (Go `routePolicy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePolicy {
    /// Preset name.
    pub name: &'static str,
    /// Inject the detection bundle into HTML (SoT-20).
    pub inject: bool,
    /// `false` means monitor or shadow: score and log, act on nothing.
    pub enforce: bool,
    /// Legacy: an `Unknown` verdict challenges on this route.
    pub fail_closed: bool,
    /// Synchronous pre-mutation re-score (SoT-21 §2).
    pub sync_score: bool,
    /// Ceiling-guard #1: a plain scoring ALLOW also needs possession or a
    /// step-up proof.
    pub attest_floor: bool,
}

/// `strict`: sensitive route.
pub const PRESET_STRICT: RoutePolicy = RoutePolicy {
    name: "strict",
    inject: true,
    enforce: true,
    fail_closed: true,
    sync_score: true,
    attest_floor: false,
};
/// `balanced`: the default.
pub const PRESET_BALANCED: RoutePolicy = RoutePolicy {
    name: "balanced",
    inject: true,
    enforce: true,
    fail_closed: false,
    sync_score: false,
    attest_floor: false,
};
/// `monitor`: score and log only.
pub const PRESET_MONITOR: RoutePolicy = RoutePolicy {
    name: "monitor",
    inject: true,
    enforce: false,
    fail_closed: false,
    sync_score: false,
    attest_floor: false,
};
/// `off`: no injection, no enforcement.
pub const PRESET_OFF: RoutePolicy = RoutePolicy {
    name: "off",
    inject: false,
    enforce: false,
    fail_closed: false,
    sync_score: false,
    attest_floor: false,
};
/// `attested`: strict plus the attestation floor.
pub const PRESET_ATTESTED: RoutePolicy = RoutePolicy {
    name: "attested",
    inject: true,
    enforce: true,
    fail_closed: true,
    sync_score: true,
    attest_floor: true,
};

impl RoutePolicy {
    /// Audit mode name (Go `modeName`): `enforce` or `monitor`.
    pub fn mode_name(&self) -> &'static str {
        if self.enforce { "enforce" } else { "monitor" }
    }
}

/// Preset by name (Go `presetByName`). An unknown name, including a case
/// variant, is `balanced`, as in Go.
pub fn preset_by_name(name: &str) -> RoutePolicy {
    match name {
        "strict" => PRESET_STRICT,
        "attested" => PRESET_ATTESTED,
        "monitor" => PRESET_MONITOR,
        "off" => PRESET_OFF,
        _ => PRESET_BALANCED,
    }
}

/// Prefix-to-preset route table with the global monitor switch (the route
/// fields of Go `Config`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteTable {
    /// Path prefix to preset name.
    pub routes: BTreeMap<String, String>,
    /// Downgrade every route to monitor (SoT-24 §3).
    pub global_monitor: bool,
}

impl RouteTable {
    /// Policy for a request path (Go `Config.resolve`): the longest prefix
    /// that matches the normalized path on a segment boundary, else balanced.
    /// The global monitor switch clears `enforce` and keeps the name.
    ///
    /// When two matching prefixes share the winning length (for example `//`
    /// and `/a` on `/a`), Go picks by map iteration order. Rust takes the first
    /// in byte order, which is one of Go's possible outcomes.
    pub fn resolve(&self, raw_path: &str) -> RoutePolicy {
        let path = normalize_match_path(raw_path);
        let mut best = PRESET_BALANCED;
        let mut best_len = None;
        for (prefix, preset) in &self.routes {
            if segment_match(&path, prefix) && best_len.is_none_or(|len| prefix.len() > len) {
                best = preset_by_name(preset);
                best_len = Some(prefix.len());
            }
        }
        if self.global_monitor {
            best.enforce = false;
        }
        best
    }
}

/// Canonical path for policy matching (Go `normalizeMatchPath`): empty is
/// `/`, a missing leading slash is added, then Go `path.Clean` applies.
pub fn normalize_match_path(path: &str) -> String {
    if path.is_empty() {
        return "/".into();
    }
    if path.starts_with('/') {
        clean(path)
    } else {
        clean(&format!("/{path}"))
    }
}

/// Go `path.Clean`: collapse slashes, drop `.` and resolve `..` lexically.
fn clean(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return ".".into();
    }
    let rooted = bytes[0] == b'/';
    let length = bytes.len();
    let mut out: Vec<u8> = Vec::with_capacity(length);
    let (mut read, mut dotdot) = (0, 0);
    if rooted {
        out.push(b'/');
        read = 1;
        dotdot = 1;
    }
    while read < length {
        let at = |index: usize| bytes.get(index).copied();
        // An empty segment or a `.` segment is skipped.
        if bytes[read] == b'/' || (bytes[read] == b'.' && matches!(at(read + 1), None | Some(b'/')))
        {
            read += 1;
        } else if bytes[read] == b'.'
            && at(read + 1) == Some(b'.')
            && matches!(at(read + 2), None | Some(b'/'))
        {
            read += 2;
            if out.len() > dotdot {
                // Go moves its write cursor back to the previous separator.
                let mut cursor = out.len() - 1;
                while cursor > dotdot && out[cursor] != b'/' {
                    cursor -= 1;
                }
                out.truncate(cursor);
            } else if !rooted {
                if !out.is_empty() {
                    out.push(b'/');
                }
                out.extend_from_slice(b"..");
                dotdot = out.len();
            }
        } else {
            if (rooted && out.len() != 1) || (!rooted && !out.is_empty()) {
                out.push(b'/');
            }
            while read < length && bytes[read] != b'/' {
                out.push(bytes[read]);
                read += 1;
            }
        }
    }
    if out.is_empty() {
        return ".".into();
    }
    // Only ASCII separators and dots were removed between whole segments.
    String::from_utf8(out).expect("segment boundaries keep UTF-8")
}

/// Whether `prefix` matches `path` on a segment boundary (Go `segmentMatch`).
/// An empty or `/` prefix matches everything; trailing slashes are ignored.
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

/// Whether an HTTP method can change server state (Go `isUnsafeMethod`).
/// Matching is case-sensitive, as in Go.
pub fn is_unsafe_method(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}

/// Audit event type of an allowed request (Go `audit.EventEnfAllow`).
pub const EVENT_ENF_ALLOW: &str = "enforcement.allow";
/// Audit event type of an issued challenge (Go `audit.EventEnfChallengeIssued`).
pub const EVENT_ENF_CHALLENGE_ISSUED: &str = "enforcement.challenge.issued";
/// Audit event type of a denied request (Go `audit.EventEnfDeny`).
pub const EVENT_ENF_DENY: &str = "enforcement.deny";
/// Event type of a fail-closed mutation without a verdict (Go `applyVerdict`).
pub const EVENT_FAILCLOSED_MUTATING: &str = "enf.failclosed.mutating";

/// Audit event type and edge action name for one decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeAction {
    /// Audit event type.
    pub event_type: &'static str,
    /// `pass`, `challenge_pow` or `block`.
    pub action: &'static str,
}

/// Edge action for a sticky verdict (Go `Verdict.action` plus the
/// `enf.failclosed.mutating` relabel in `applyVerdict`).
///
/// `Allow`, `Challenge` and `Deny` equal Go on every route. `Unknown` always
/// challenges: an unsafe method is labelled `enf.failclosed.mutating` as in Go,
/// and a safe method is a plain challenge where Go passed it on balanced,
/// monitor and off routes (owner-accepted fail-closed difference). Because of
/// that difference the route policy no longer changes the result.
pub fn edge_action(verdict: Verdict, unsafe_method: bool) -> EdgeAction {
    let (event_type, action) = match verdict {
        Verdict::Deny => (EVENT_ENF_DENY, "block"),
        Verdict::Challenge => (EVENT_ENF_CHALLENGE_ISSUED, "challenge_pow"),
        Verdict::Allow => (EVENT_ENF_ALLOW, "pass"),
        Verdict::Unknown if unsafe_method => (EVENT_FAILCLOSED_MUTATING, "challenge_pow"),
        Verdict::Unknown => (EVENT_ENF_CHALLENGE_ISSUED, "challenge_pow"),
    };
    EdgeAction { event_type, action }
}

#[cfg(test)]
mod tests;
