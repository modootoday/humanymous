//! Pass mount: serves `gateway::pass_flow` on the Core plane through the
//! [`PassRoutes`] mount point, as Go cmd/server `routes` mounted
//! pass_handler.go at baseline 981d25c (`/pass`, `/api/pass/new`, `pow`,
//! `attest`, `solve`, `kpi`).
//!
//! [`PassMount`] owns one [`PassFlow`] behind its own lock. The Core server
//! hands it the session resolved by Go `cookieValue` (the `hsid` cookie), the
//! legacy `passFP` fingerprint (stable JA4 prefix of the connection's own
//! ClientHello and the trust-gated client subnet), the request time read
//! under the Core state lock and a [`Backend`] over the Core collector store,
//! the frozen engine and the live feed, as Go's handlers shared `a.store`,
//! `a.hub` and `a.masterKey` with the rest of the app.
//!
//! Only [`MotorEvidence::Off`] is mounted: [`PassMount::new`] refuses
//! [`MotorEvidence::GoParity`], which gates or penalises Pass on motor
//! richness and timing (accessibility hard rule). A poisoned flow lock or an
//! unservable body answers 503 instead of an unmetered Pass.
use super::{PassRequest, PassRoutes};
use crate::gateway::observatory::{Body, Response as FlowResponse};
use crate::gateway::pass_flow::{self, Backend, MotorEvidence, PassFlow};
use crate::gateway::resource;
use crate::network::http::first_value;
use std::sync::Mutex;

/// Red-team label header (Go `r.Header.Get("X-HM-Redteam")`).
pub const RED_TEAM_HEADER: &str = "X-HM-Redteam";

/// Why a [`PassMount`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountRefusal {
    /// Motor evidence other than [`MotorEvidence::Off`] was requested on the
    /// served path; enabling it needs an owner decision.
    MotorEvidence,
    /// The flow's limiter configuration was refused.
    Limiter,
}

/// [`PassRoutes`] adapter over one mounted [`PassFlow`].
pub struct PassMount {
    flow: Mutex<PassFlow>,
}

impl PassMount {
    /// Mount the flow with the legacy limiter policy and operating-system
    /// nonces; refuses any motor-evidence policy but [`MotorEvidence::Off`].
    pub fn new(config: pass_flow::Config) -> Result<Self, MountRefusal> {
        if config.motor_evidence != MotorEvidence::Off {
            return Err(MountRefusal::MotorEvidence);
        }
        let flow = pass_flow::mount(config).map_err(|_| MountRefusal::Limiter)?;
        Ok(Self {
            flow: Mutex::new(flow),
        })
    }

    /// Replay the recorded challenge nonces (deterministic tests only).
    #[cfg(test)]
    pub(crate) fn with_nonce_source(
        self,
        source: impl FnMut() -> Result<
            crate::gateway::pass_store::PassNonce,
            crate::gateway::pass_store::Refusal,
        > + Send
        + 'static,
    ) -> Self {
        let flow = self
            .flow
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Self {
            flow: Mutex::new(flow.with_nonce_source(source)),
        }
    }
}

fn unavailable() -> resource::Response {
    resource::Response::text(503, Vec::new(), "pass unavailable")
}

/// Write a flow answer as a Core handler reply. The page body is the web
/// root's `pass.html`, served as the Core page handlers serve theirs (Go
/// `http.ServeFile`); the flow's `Set-Cookie` survives a file error, as Go's
/// `ensureSession` header did.
fn reply(response: FlowResponse) -> resource::Response {
    match response.body {
        Body::Bytes(body) => resource::Response {
            status: response.status,
            headers: response.headers,
            body,
        },
        Body::File(path) => {
            let served = super::routes::file_response(&path);
            let headers = response
                .headers
                .into_iter()
                .filter(|(name, _)| !name.eq_ignore_ascii_case("Content-Type"))
                .chain(served.headers)
                .collect();
            resource::Response {
                status: served.status,
                headers,
                body: served.body,
            }
        }
        Body::Events(_) => unavailable(),
    }
}

impl PassRoutes for PassMount {
    fn handle(&self, request: &PassRequest<'_>, backend: &mut dyn Backend) -> resource::Response {
        let Ok(mut flow) = self.flow.lock() else {
            return unavailable();
        };
        // The Core already resolved the session exactly as Go `cookieValue`
        // did (every Cookie line, quotes stripped, cookie octets only); the
        // value is quoted so the flow's cookie reader keeps it byte for byte.
        let cookie = (!request.session.is_empty())
            .then(|| format!("{}=\"{}\"", pass_flow::SESSION_COOKIE, request.session));
        let red_team = first_value(&request.request.headers, RED_TEAM_HEADER);
        let flow_request = pass_flow::Request {
            method: request.method,
            path: request.path,
            cookie: cookie.as_deref(),
            red_team: (!red_team.is_empty()).then_some(red_team),
            fingerprint: request.fingerprint,
            body: &request.request.body,
        };
        match flow.handle(backend, &flow_request, request.now) {
            Some(response) => reply(response),
            None => resource::not_found(),
        }
    }
}

#[cfg(test)]
mod tests;
