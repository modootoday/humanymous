//! Operator ops routes on the admin plane's loopback listener: the liveness,
//! score-explanation and counters handlers of legacy cmd/server/ops.go
//! (`handleHealthz`, `handleExplain`, `handleCounters` and `configureOps`) at
//! baseline 981d25c. The bearer check reuses `operations::mlcorrect::
//! operator_authorized` (Go `operatorAuthorized`); Go `canReadSession` is
//! `gateway::resource::can_read_session`.
//!
//! Go mounted `/healthz`, `/api/explain/{id}` and `/api/counters` on the public
//! Core server. Here they are served only under [`OPS_PREFIX`] on the admin
//! plane's own loopback listener, because public data-plane listeners never
//! register, proxy or alias management handlers (sot:root/adapter-ecosystem,
//! "Management and origin isolation"; plan B09 "separate operator route/auth").
//! The public application listener refuses every `/_humanymous/ops/` path as
//! a reserved route and has no `/api/explain/` or `/api/counters` handler.
//!
//! Kept from Go:
//! - `healthz` is unauthenticated and answers any method with status, version
//!   and uptime only.
//! - `explain/{id}` and `counters` exist only when an operator token is
//!   configured; without one they are the plain 404. With one, a missing or
//!   wrong bearer is the same non-discoverable 404, never 401. The comparison is
//!   constant time, and as in Go one leading `Bearer ` is stripped, so the bare
//!   token also matches.
//! - Authorization is checked before the session lookup; an unknown (or empty)
//!   session id is 404.
//! - The explanation re-scores a copy of the stored report through the shared
//!   `humanymous_core` engine, so the stored verdict is untouched and the
//!   reported weights, thresholds and rule outcomes are the frozen ones.
//! - Bodies are Go `encoding/json` bytes: sorted map keys, struct field order,
//!   Go number text, HTML-safe escaping and a trailing newline.
//!
//! Deliberate differences, each pinned by tests:
//! - Uptime is measured from the admin plane's epoch, not process start.
//! - `counters` omits the Go-runtime fields `goroutines`, `heapAllocBytes` and
//!   `numGC` (see `gateway::operations::counters`).
//! - A non-finite scoring policy, which Go could not produce, answers 500
//!   instead of a trace.
//! - Ops requests are not meta-audited, as in Go; admin routes still are.
use super::{Clock, Response};
use crate::gateway::http::Request;
use crate::gateway::observatory::Sessions;
use crate::gateway::operations::counters;
use crate::gateway::operations::mlcorrect::operator_authorized;
use humanymous_core::model::correction::Controller;
use humanymous_core::scoring::engine::Engine;
use serde_json::json;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Ops route prefix on the admin listener (Go mounted the routes at the root
/// of the public server).
pub const OPS_PREFIX: &str = "/_humanymous/ops/";
/// Environment fallback for the operator token (Go `HMN_OPS_TOKEN`).
pub const TOKEN_ENV: &str = "HMN_OPS_TOKEN";

/// Go `configureOps`: the trimmed flag value, else the trimmed environment
/// value; empty means the gated routes are not registered.
pub fn resolve_token(flag: &str, environment: Option<&str>) -> Option<Zeroizing<String>> {
    let flag = flag.trim();
    let token = if flag.is_empty() {
        environment.unwrap_or("").trim()
    } else {
        flag
    };
    (!token.is_empty()).then(|| Zeroizing::new(token.to_owned()))
}

/// [`resolve_token`] with the environment read from [`TOKEN_ENV`].
pub fn token_from_environment(flag: &str) -> Option<Zeroizing<String>> {
    let environment = std::env::var(TOKEN_ENV).ok().map(Zeroizing::new);
    resolve_token(flag, environment.as_deref().map(String::as_str))
}

/// Live values behind the counters rollup. Defaults describe a process with
/// no model, no trace sink and no fleet correlation.
pub trait CounterSource: Send + Sync {
    /// Loaded lab correction controller.
    fn controller(&self) -> Option<&Controller> {
        None
    }
    /// Active bundle version.
    fn active_bundle(&self) -> Option<String> {
        None
    }
    /// Oracle traces written by the trace sink.
    fn oracle_traces(&self) -> u64 {
        0
    }
    /// Fleet observations that degraded to per-node correlation.
    fn fleet_fallbacks(&self) -> u64 {
        0
    }
}

/// The ops routes. Attach with `AdminPlane::with_ops`.
pub struct Ops {
    version: String,
    token: Option<Zeroizing<String>>,
    engine: Engine,
    sessions: Arc<dyn Sessions>,
    counters: Arc<dyn CounterSource>,
}

impl Ops {
    /// Routes for `version`; `token` from [`resolve_token`] gates explain and
    /// counters, `engine` is the configured scoring engine and `sessions` the
    /// stored session reports.
    pub fn new(
        version: &str,
        token: Option<Zeroizing<String>>,
        engine: Engine,
        sessions: Arc<dyn Sessions>,
        counters: Arc<dyn CounterSource>,
    ) -> Self {
        Self {
            version: version.to_owned(),
            token: token.filter(|token| !token.is_empty()),
            engine,
            sessions,
            counters,
        }
    }

    /// Serve a path under [`OPS_PREFIX`]; `None` for any other path.
    pub(super) fn handle(&self, request: &Request, clock: Clock) -> Option<Response> {
        let path = request.target.split('?').next().unwrap_or("");
        let sub = path.strip_prefix(OPS_PREFIX)?;
        let uptime = clock.monotonic.as_secs();
        if sub == "healthz" {
            return Some(Response::json(&json!({
                "status": "ok",
                "version": self.version,
                "uptimeSec": uptime,
            })));
        }
        let explain = sub.strip_prefix("explain/");
        if explain.is_none() && sub != "counters" {
            return Some(Response::not_found());
        }
        // Unregistered without a token; a wrong or missing bearer looks the same.
        if !operator_authorized(
            self.token.as_deref().map_or("", String::as_str),
            request.header("authorization"),
        ) {
            return Some(Response::not_found());
        }
        Some(match explain {
            Some(session) => self.explain(session),
            None => self.counters(uptime),
        })
    }

    fn explain(&self, session: &str) -> Response {
        let Some(mut report) = self.sessions.report(session) else {
            return Response::not_found();
        };
        // Scores a copy; the stored verdict is untouched.
        match self.engine.score_with_trace(&mut report) {
            Ok((_, trace)) => Response::json(&trace),
            Err(_) => Response::text(500, "score explanation unavailable"),
        }
    }

    fn counters(&self, uptime: u64) -> Response {
        let bundle = self.counters.active_bundle();
        Response::json(&counters::rollup(&counters::Inputs {
            version: &self.version,
            uptime_sec: uptime,
            controller: self.counters.controller(),
            active_bundle: bundle.as_deref(),
            oracle_traces: self.counters.oracle_traces(),
            fleet_fallbacks: self.counters.fleet_fallbacks(),
        }))
    }
}

#[cfg(test)]
mod tests;
