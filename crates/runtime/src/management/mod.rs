//! Separate native management authority; application and browser routes never enter.
mod policy;
use crate::{Rejection, configuration::Configuration, service::ShutdownRequest};
use humanymous_authority::{
    credentials::Credential,
    decision::NativePeer,
    transport::{Limits, Listener, LocalContext, LocalHandler, LocalRequest, LocalResponse},
};
pub use policy::{ManagementPolicy, ManagementScope};
use std::{io, sync::Arc};
use tokio::sync::watch;

/// A bound management listener plus its independent process-lifetime credential.
pub struct Management {
    listener: Listener,
    handler: Handler,
    drain_ms: u64,
}
struct Handler {
    credential: Arc<Credential>,
    peer: NativePeer,
    scope: ManagementScope,
    profile: serde_json::Value,
}
impl Management {
    pub(crate) fn bind(
        policy: ManagementPolicy,
        configuration: &Configuration,
        profile: serde_json::Value,
    ) -> io::Result<Self> {
        let credential = Arc::new(policy.credential);
        let peer = match &policy.peer {
            humanymous_authority::configuration::Peer::Unix { uid } => NativePeer::Unix(*uid),
            humanymous_authority::configuration::Peer::Windows { pid, .. } => {
                NativePeer::Windows(*pid)
            }
        };
        let listener = Listener::bind_ipc(&policy.endpoint, &policy.peer)?;
        Ok(Self {
            listener,
            handler: Handler {
                credential,
                peer,
                scope: ManagementScope::from_scope(&configuration.scope),
                profile,
            },
            drain_ms: policy.stop_budget.request().drain_timeout().as_millis() as u64,
        })
    }

    /// Trusted supervisor provisioning only; never put this value in public logs.
    pub fn credential_for_private_provisioning(&self) -> &str {
        self.handler.credential.expose_for_provisioning()
    }

    pub(crate) async fn run(
        self,
        mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        self.listener
            .serve_handler_until(
                self.handler,
                Limits {
                    capacity: 8,
                    rate_per_second: 20,
                    timeout_ms: 1000,
                },
                async move {
                    loop {
                        if shutdown.borrow().is_some() {
                            return;
                        }
                        if shutdown.changed().await.is_err() {
                            return;
                        }
                    }
                },
                self.drain_ms,
            )
            .await
    }
}
impl LocalHandler for Handler {
    async fn handle(
        &self,
        request: LocalRequest,
        context: LocalContext,
    ) -> Result<LocalResponse, Rejection> {
        context.check()?;
        self.authorize(&request, context.peer())?;
        context.check()?;
        if request.target == "/management/v0.1/settings" {
            return LocalResponse::json(
                200,
                &serde_json::json!({"protocol":"humanymous.management/0.1","scope":self.scope,"settings":self.profile.get("operational"),"writable":false}),
            );
        }
        LocalResponse::json(
            200,
            &serde_json::json!({
                "protocol":"humanymous.management/0.1", "scope":self.scope,
                "role":"observer", "profile":self.profile,
                "settings_writable":false, "destructive_controls":false,
            }),
        )
    }
}
impl Handler {
    fn authorize(&self, request: &LocalRequest, peer: NativePeer) -> Result<(), Rejection> {
        if peer != self.peer {
            return Err(Rejection::new(403, "management_peer"));
        }
        if !self.credential.authenticates(&request.authorization) {
            return Err(Rejection::new(401, "management_credentials"));
        }
        policy::validate_request(request, &self.scope)
    }
}

#[cfg(test)]
mod tests;
