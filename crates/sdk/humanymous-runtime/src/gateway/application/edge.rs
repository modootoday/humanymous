//! Front-of-path edge gates on the served path (legacy internal/gate gate.go
//! `ServeHTTP` and gates.go `banGate`/`spoofHeaderGate`, the `Rewrite` origin
//! cloak and the control-plane `/healthz` and `/readyz` probes, at baseline
//! 981d25c). The decisions are the existing kernels `ban::gate::evaluate` and
//! `guard::spoof_gate`; this module only composes them, appends their records
//! durably before acting and writes their fixed responses.
//!
//! Kept from Go, pinned against `tests/fixtures/gateway/edge-gates-go-golden.json`:
//! - Gate order after the control-plane split: ban, then spoof header. The
//!   reserved control routes (`/_humanymous/...`) are not gated, as Go's
//!   control plane was not.
//! - A banned source answers 403 with the incident pointer, `Retry-After` for
//!   a temporary ban and the static body; the scorer and origin are never
//!   touched. Both keys are metered: `ip:` and, with a User-Agent, `fp:`.
//! - A forged trust header answers 403 with the incident pointer and the
//!   static deny body when `gate.spoof_header` enforces; otherwise the HR-27b
//!   record carries mode `monitor` and the request continues with every
//!   internal trust header stripped, so neither scoring nor the origin sees
//!   the forged claim.
//! - Probes: `{"plane":"gate","status":"ok"}` and `"ready"`/`"draining"`
//!   (503) with a trailing newline.
//!
//! Deliberate differences, each pinned by tests:
//! - The ban gate needs the trusted client address (PROXY v2,
//!   `ip_reputation.client_address`); HTTP forwarding headers are never a
//!   source. Installing a store without one is refused; an unattributed
//!   connection (LOCAL/UNSPEC) is refused 503 `ban_client_address` when the
//!   gate enforces and passes ungated otherwise. A ledger refusal or a
//!   poisoned store lock answers 503 `ban_ledger` (fail closed).
//! - The ban gate keeps its own monotonic epoch (`install_bans`); a store
//!   shared with `admin_plane::AdminPlane` should be installed when that
//!   plane starts serving so temporary-ban expiries line up.
//! - In monitor, shadow and off mode a forged trust header is stripped from
//!   the view that is scored and forwarded, as Go's `stripInbound(r)` did.
//!   Request-integrity verification and the request commitment still cover
//!   the bytes the client sent and signed (Go had no request integrity).
//! - Records carry no subject pseudonym: no session is authenticated yet.
//! - A record that cannot be appended poisons the epoch and answers 503
//!   `durable_audit`, so no gate effect is unaudited.
//! - Probes answer GET and HEAD (HEAD: same head and Content-Length, no body,
//!   as Go's server wrote it) for the exact target and the configured
//!   authority only; Go's mux also answered every other method, which stays
//!   refused here as a reserved route. Readiness is also withdrawn when the
//!   serving epoch has failed closed. The draining body is sent as
//!   `application/json` (Go's server sniffed `text/plain; charset=utf-8`).
//! - Gate refusals to a HEAD request keep the head and omit the body.
use super::{Application, Decision};
use crate::gateway::Rejection;
use crate::gateway::ban::{
    Store,
    gate::{self, Mode, Subject},
};
use crate::gateway::guard;
use crate::gateway::http::Request;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Liveness probe path.
pub const HEALTHZ: &str = "/_humanymous/healthz";
/// Readiness probe path; 503 `draining` once withdrawn.
pub const READYZ: &str = "/_humanymous/readyz";
/// Liveness body (Go `writeJSON` of the control-plane probe).
const PROBE_OK: &str = "{\"plane\":\"gate\",\"status\":\"ok\"}\n";
/// Readiness body while serving.
const PROBE_READY: &str = "{\"plane\":\"gate\",\"status\":\"ready\"}\n";
/// Readiness body once withdrawn (503).
const PROBE_DRAINING: &str = "{\"plane\":\"gate\",\"status\":\"draining\"}\n";

/// Outcome of the front gates for a proxied request.
pub(super) enum Front {
    /// Answer locally; the origin is never contacted.
    Refuse(Decision),
    /// Continue. `Some` carries the request's fields after the spoof gate
    /// stripped forged trust headers (Go strips them from the request before
    /// scoring and forwarding); `None` when nothing was stripped.
    Continue(Option<Vec<(String, String)>>),
}

/// A fixed edge-gate refusal written in Go `http.Error` form.
pub struct GateResponse {
    /// HTTP status.
    pub status: u16,
    /// Fixed fields: the incident pointer and, for a temporary ban, Retry-After.
    pub fields: Vec<(&'static str, String)>,
    /// Static body; a newline is appended on the wire.
    pub body: &'static str,
}

/// Readiness gate shared with a supervisor (Go `ControlPlane.SetReady`).
/// Liveness is unaffected.
#[derive(Clone)]
pub struct ReadySwitch(Arc<AtomicBool>);

impl ReadySwitch {
    /// Withdraw (`false`) or restore (`true`) readiness.
    pub fn set_ready(&self, ready: bool) {
        self.0.store(ready, Ordering::Release);
    }
}

impl Default for ReadySwitch {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
}

/// Installed ban store and the monotonic epoch of its `now`.
pub(super) struct Bans {
    store: Arc<Mutex<Store>>,
    epoch: Instant,
}

impl Application {
    /// Run the ban gate against `store`, a store an admin plane may share.
    /// `epoch` is the monotonic origin of the store's clock. Refused unless
    /// a trusted client-address source is configured (or the gate is off).
    pub fn install_bans(
        &mut self,
        store: Arc<Mutex<Store>>,
        epoch: Instant,
    ) -> Result<(), Rejection> {
        if self.config.edge.ban != Mode::Off && self.edge_policy().is_none() {
            return Err(Rejection::new(503, "configuration"));
        }
        self.bans = Some(Bans { store, epoch });
        Ok(())
    }

    /// Handle to withdraw readiness ahead of a drain.
    pub fn readiness_switch(&self) -> ReadySwitch {
        self.ready.clone()
    }

    /// Liveness and readiness probes for GET and HEAD; `None` for any other
    /// request. HEAD gets the GET head and Content-Length without the body.
    pub(super) fn probe(&self, req: &Request) -> Option<Result<Decision, Rejection>> {
        if !matches!(req.method.as_str(), "GET" | "HEAD")
            || req.target != HEALTHZ && req.target != READYZ
        {
            return None;
        }
        if req.header("host") != Some(self.config.scope.authority.as_str()) {
            return Some(Err(Rejection::new(403, "authority")));
        }
        let (status, body) = if req.target == HEALTHZ {
            (200, PROBE_OK)
        } else if self.ready.0.load(Ordering::Acquire) && self.sessions.health().serving() {
            (200, PROBE_READY)
        } else {
            (503, PROBE_DRAINING)
        };
        Some(Ok(Decision::Probe(status, body)))
    }

    fn append_gate_record(
        &mut self,
        record: crate::audit::record::Record,
    ) -> Result<(), Rejection> {
        if self.audit.append(record).is_err() {
            self.sessions.poison();
            return Err(Rejection::new(503, "durable_audit"));
        }
        Ok(())
    }

    /// Ban then spoof-header gate for one proxied request.
    pub(super) fn front_gates(&mut self, req: &Request) -> Result<Front, Rejection> {
        let route_class = crate::gateway::request::route_class(&req.headers);
        if let Some(block) = self.ban_gate(req, route_class.as_str())? {
            return Ok(Front::Refuse(block));
        }
        let mut headers = req.headers.clone();
        let tenant = self.config.scope.tenant.clone();
        let enforce = self.config.edge.spoof_header == Mode::Enforce;
        let Some(spoof) = guard::spoof_gate(&mut headers, enforce, &tenant, "") else {
            // Go strips only when a forged value was found: empty-valued
            // internal fields stay in the scored view (the forwarder still
            // drops them toward the origin).
            return Ok(Front::Continue(None));
        };
        let (deny, fields) = (spoof.deny, spoof.headers());
        self.append_gate_record(spoof.record)?;
        Ok(if deny {
            Front::Refuse(Decision::Gate(GateResponse {
                status: 403,
                fields,
                body: guard::SPOOF_DENY_BODY,
            }))
        } else {
            Front::Continue(Some(headers))
        })
    }

    fn ban_gate(
        &mut self,
        req: &Request,
        route_class: &str,
    ) -> Result<Option<Decision>, Rejection> {
        let mode = self.config.edge.ban;
        let Some(bans) = self.bans.as_ref().filter(|_| mode != Mode::Off) else {
            return Ok(None);
        };
        let Some(ip) = self.client else {
            return match mode {
                Mode::Enforce => Err(Rejection::new(503, "ban_client_address")),
                _ => Ok(None),
            };
        };
        let ledger = Rejection::new(503, "ban_ledger");
        let fingerprint = crate::gateway::token::bind_key(
            req.header("user-agent").unwrap_or(""),
            req.header("accept-language").unwrap_or(""),
            req.header("sec-ch-ua").unwrap_or(""),
        );
        let keys = gate::keys(ip, fingerprint.as_deref()).map_err(|_| ledger)?;
        let subject = Subject {
            tenant: &self.config.scope.tenant,
            pseudonym: "",
            route_class,
        };
        let now = bans.epoch.elapsed();
        let result = {
            let mut store = bans.store.lock().map_err(|_| ledger)?;
            gate::evaluate(&mut *store, &keys, mode, &subject, now)
        };
        let outcome = result.map_err(|_| ledger)?;
        for record in outcome.records {
            self.append_gate_record(record)?;
        }
        Ok(outcome.block.map(|block| {
            Decision::Gate(GateResponse {
                status: gate::BLOCK_STATUS,
                fields: block.headers(),
                body: gate::BLOCK_BODY,
            })
        }))
    }

    /// Origin-cloak token for the current wall-clock epoch, when enabled.
    pub(super) fn origin_auth(&self) -> Result<Option<String>, Rejection> {
        let Some(key) = &self.origin_key else {
            return Ok(None);
        };
        let now = crate::authority::now_ms()? / 1000;
        let now = i64::try_from(now).map_err(|_| Rejection::new(503, "clock"))?;
        Ok(Some(key.token(&guard::origin_epoch(now))))
    }
}
