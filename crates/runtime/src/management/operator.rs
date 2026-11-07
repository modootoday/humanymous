//! Independent non-destructive resource operator; no control grants or scoring.
use super::*;
use crate::configuration::OperationalSettings;
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

/// Trusted installation input, independent from the observer credential.
pub struct OperatorPolicy {
    /// Provision only over the installation owner's private channel.
    pub credential: Credential,
}
impl OperatorPolicy {
    pub(crate) fn validate_observer(&self, observer: &Credential) -> io::Result<()> {
        let presented =
            zeroize::Zeroizing::new(format!("Bearer {}", observer.expose_for_provisioning()));
        if self.credential.authenticates(&presented) {
            return Err(io::Error::other(
                "observer and operator credentials must differ",
            ));
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettingsChange {
    pub scope: ManagementScope,
    pub expected_revision: u64,
    pub settings: OperationalSettings,
}
pub(crate) struct SettingsCommand {
    pub change: SettingsChange,
    pub context: LocalContext,
    pub response: oneshot::Sender<Result<serde_json::Value, Rejection>>,
}
pub(super) struct Operator {
    pub credential: Arc<Credential>,
    pub commands: mpsc::Sender<SettingsCommand>,
}
impl Handler {
    pub(super) async fn change_settings(
        &self,
        request: LocalRequest,
        context: LocalContext,
    ) -> Result<LocalResponse, Rejection> {
        let operator = self
            .operator
            .as_ref()
            .ok_or(Rejection::new(403, "management_readonly"))?;
        context.check()?;
        let change = self.authorize_change(&request, context.peer(), &operator.credential)?;
        let (sender, receiver) = oneshot::channel();
        operator
            .commands
            .try_send(SettingsCommand {
                change,
                context: context.clone(),
                response: sender,
            })
            .map_err(|_| Rejection::new(503, "management_queue_unavailable"))?;
        let result = tokio::time::timeout(context.remaining(), receiver)
            .await
            .map_err(|_| Rejection::new(503, "management_outcome_unknown"))?
            .map_err(|_| Rejection::new(503, "management_outcome_unknown"))??;
        context.check()?;
        LocalResponse::json(200, &result)
    }

    pub(super) fn authorize_change(
        &self,
        request: &LocalRequest,
        peer: NativePeer,
        credential: &Credential,
    ) -> Result<SettingsChange, Rejection> {
        if peer != self.peer {
            return Err(Rejection::new(403, "management_peer"));
        }
        if !credential.authenticates(&request.authorization) {
            return Err(Rejection::new(401, "management_credentials"));
        }
        if request.method != "POST" || request.target != "/management/v0.1/settings/commit" {
            return Err(Rejection::new(403, "management_operator_route"));
        }
        policy::validate_headers(request)?;
        let change: SettingsChange = serde_json::from_slice(&request.body)
            .map_err(|_| Rejection::new(400, "management_settings"))?;
        if change.scope != self.scope {
            return Err(Rejection::new(403, "management_scope"));
        }
        change.settings.validate()?;
        if change.expected_revision > humanymous_core::integrity::request::browser::MAX_SAFE_INTEGER
        {
            return Err(Rejection::new(409, "settings_parent_revision"));
        }
        Ok(change)
    }
}
