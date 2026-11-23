//! Opt-in HTML loader injection on the served path (legacy internal/gate
//! gate.go `modifyResponse`, `auditInjectSkip` and the `Accept-Encoding:
//! identity` rewrite at baseline 981d25c; owner decision #7). Composes
//! `configuration::inject` and the ported injector (`gateway::inject`) over the
//! complete buffered origin response.
//!
//! Kept from Go: only a 200 `text/html` response on an inject route is
//! eligible; `gate.inject` off does nothing; monitor and shadow record
//! `html.injection.skipped` (HR-27a, `gate.inject <mode> (would inject)`) and
//! leave the response untouched; enforce skips a response with a
//! Content-Encoding (`decomp_bomb_guard`, recorded) and otherwise inserts the
//! snippet once, reframes the length and sets `Cache-Control: no-store`. The
//! inject route's origin request asks for `Accept-Encoding: identity`.
//!
//! Deliberate differences, each pinned by `gateway::tests::inject`:
//! - Routes are listed in configuration, not taken from route presets.
//! - No loader is served and no CSP report-only companion is added: the
//!   gateway has no `loader.js` or `csp-report` route.
//! - Go's 8 MiB `too_large` skip cannot occur: complete origin entities are
//!   bounded by `http::MAX_BODY`.
//! - The skip record is appended before the response is written; an append
//!   failure poisons the epoch and answers 503 `durable_audit` (Go emitted it
//!   best-effort).
use super::Application;
use crate::audit::record::{Actor, Record};
use crate::gateway::Rejection;
use crate::gateway::ban::gate::Mode;
use crate::gateway::http::{self, Request};

/// Go `audit.EventInjectSkipped`.
const EVENT: &str = "html.injection.skipped";

impl Application {
    /// Whether `request` is on a configured inject route.
    fn injects(&self, request: &Request) -> bool {
        self.config
            .inject
            .as_ref()
            .is_some_and(|inject| inject.covers(&request.method, &request.target))
    }

    /// The origin request bytes; an inject route asks for an identity entity
    /// (Go `Rewrite` set `Accept-Encoding: identity`).
    pub(super) fn origin_bytes(&self, request: &Request, origin_auth: Option<&str>) -> Vec<u8> {
        if !self.injects(request) {
            return http::origin_request(request, &self.config.scope.authority, origin_auth);
        }
        let mut headers: Vec<_> = request
            .headers
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("accept-encoding"))
            .cloned()
            .collect();
        headers.push(("Accept-Encoding".into(), "identity".into()));
        let identity = Request {
            method: request.method.clone(),
            target: request.target.clone(),
            headers,
            body: request.body.clone(),
        };
        http::origin_request(&identity, &self.config.scope.authority, origin_auth)
    }

    /// Legacy `modifyResponse` injection step for one complete response.
    pub(super) fn inject_html(
        &mut self,
        request: &Request,
        response: &mut http::Response,
    ) -> Result<(), Rejection> {
        let Some(inject) = self.config.inject.as_ref() else {
            return Ok(());
        };
        if !inject.covers(&request.method, &request.target) || !response.html() {
            return Ok(());
        }
        let snippet = inject.snippet();
        let reason = match self.gate_mode("gate.inject", inject.mode) {
            Mode::Off => return Ok(()),
            Mode::Monitor => ("gate.inject monitor (would inject)", "monitor"),
            Mode::Shadow => ("gate.inject shadow (would inject)", "shadow"),
            Mode::Enforce if response.encoded() => ("decomp_bomb_guard", "enforce"),
            Mode::Enforce => {
                response.inject(&snippet);
                return Ok(());
            }
        };
        let record = Record {
            event_type: EVENT.into(),
            actor: Actor {
                kind: "system".into(),
                ..Default::default()
            },
            tenant_id: self.config.scope.tenant.clone(),
            route_class: "html".into(),
            triggered_rules: Some(vec!["HR-27a".into()]),
            fail_reason: reason.0.into(),
            enforcement_mode: reason.1.into(),
            key_id: "k1".into(),
            ..Default::default()
        };
        self.append_gate_record(record)
    }
}
