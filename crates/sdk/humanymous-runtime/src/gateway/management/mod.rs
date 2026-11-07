//! Separate native management authority; application and browser routes never enter.
mod operator;
mod policy;
use crate::authority::{
    credentials::Credential,
    decision::NativePeer,
    transport::{Limits, Listener, LocalContext, LocalHandler, LocalRequest, LocalResponse},
};
use crate::gateway::{Rejection, configuration::Configuration, service::ShutdownRequest};
pub use operator::OperatorPolicy;
pub(in crate::gateway) use operator::SettingsCommand;
pub use policy::{ManagementPolicy, ManagementScope};
use std::{
    io,
    sync::{Arc, RwLock},
};
use tokio::sync::{mpsc, watch};
pub(in crate::gateway) type SharedProfile = Arc<RwLock<serde_json::Value>>;

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
    profile: SharedProfile,
    operator: Option<operator::Operator>,
}
impl Management {
    pub(in crate::gateway) fn bind(
        policy: ManagementPolicy,
        configuration: &Configuration,
        profile: SharedProfile,
        operator: Option<(OperatorPolicy, mpsc::Sender<SettingsCommand>)>,
    ) -> io::Result<Self> {
        let credential = Arc::new(policy.credential);
        let operator = operator
            .map(|(policy, commands)| -> io::Result<operator::Operator> {
                policy.validate_observer(&credential)?;
                Ok(operator::Operator {
                    credential: Arc::new(policy.credential),
                    commands,
                })
            })
            .transpose()?;
        let peer = match &policy.peer {
            crate::authority::configuration::Peer::Unix { uid } => NativePeer::Unix(*uid),
            crate::authority::configuration::Peer::Windows { pid, .. } => NativePeer::Windows(*pid),
        };
        let listener = Listener::bind_ipc(&policy.endpoint, &policy.peer)?;
        Ok(Self {
            listener,
            handler: Handler {
                credential,
                peer,
                scope: ManagementScope::from_scope(&configuration.scope),
                profile,
                operator,
            },
            drain_ms: policy.stop_budget.request().drain_timeout().as_millis() as u64,
        })
    }

    /// Trusted supervisor provisioning only; never put this value in public logs.
    pub fn credential_for_private_provisioning(&self) -> &str {
        self.handler.credential.expose_for_provisioning()
    }
    pub(in crate::gateway) fn operator_credential_for_private_provisioning(&self) -> Option<&str> {
        self.handler
            .operator
            .as_ref()
            .map(|operator| operator.credential.expose_for_provisioning())
    }

    pub(in crate::gateway) async fn run(
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
        if request.target == "/management/v0.1/settings/commit" {
            return self.change_settings(request, context).await;
        }
        self.authorize(&request, context.peer())?;
        context.check()?;
        let profile = self
            .profile
            .read()
            .map_err(|_| Rejection::new(503, "management_profile_unavailable"))?
            .clone();
        if request.target == "/management/v0.1/settings" {
            return LocalResponse::json(
                200,
                &serde_json::json!({"protocol":"humanymous.management/0.1","scope":self.scope,"settings":profile.get("operational"),"revision":profile.get("operational_revision"),"writable":false,"operator_available":self.operator.is_some()}),
            );
        }
        LocalResponse::json(
            200,
            &serde_json::json!({
                "protocol":"humanymous.management/0.1", "scope":self.scope,
                "role":"observer", "profile":profile,
                "settings_writable":false, "operator_available":self.operator.is_some(), "destructive_controls":false,
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
