//! Stable destructive target framing; this data never authorizes an effect.
use crate::audit::control::{Scope, target_commitment};
use crate::gateway::{Rejection, configuration::Configuration};
use serde::Serialize;
mod state;
pub use state::{KillState, KillStatus};

/// One-way node kill target. Policy metadata cannot silently rearm this scope.
/// No constructor accepts a request-selected action, target or replacement scope.
pub struct KillTarget {
    scope: Scope,
}
#[derive(Serialize)]
struct Payload<'a> {
    protocol: &'static str,
    installation: &'a str,
    node: &'a str,
    state: &'static str,
}
impl KillTarget {
    /// Derive only from the trusted installation's validated configuration.
    pub fn for_configuration(configuration: &Configuration) -> Result<Self, Rejection> {
        configuration.validate()?;
        let scope = Scope {
            installation: configuration.scope.installation.clone(),
            node: configuration.audit.node_id.clone(),
        };
        scope
            .validate()
            .map_err(|_| Rejection::new(400, "kill_control_scope"))?;
        Ok(Self { scope })
    }
    /// Shared administrator scope, independent from mutable policy revision.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Fixed-field UTF-8 JSON, passed unchanged to the shared target commitment.
    pub fn bytes(&self) -> Result<Vec<u8>, Rejection> {
        serde_json::to_vec(&Payload {
            protocol: "humanymous.runtime-kill/0.1",
            installation: &self.scope.installation,
            node: &self.scope.node,
            state: "killed",
        })
        .map_err(|_| Rejection::new(503, "kill_target_encoding"))
    }
    /// Proposal data only. Even a matching digest is never a retained grant.
    pub fn commitment(&self) -> Result<String, Rejection> {
        Ok(target_commitment(&self.bytes()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn node_kill_binding_is_exact_and_survives_policy_revision_changes() {
        let mut configuration = crate::gateway::tests::config();
        let target = KillTarget::for_configuration(&configuration).unwrap();
        assert_eq!(target.bytes().unwrap(), br#"{"protocol":"humanymous.runtime-kill/0.1","installation":"install","node":"runtime-test","state":"killed"}"#);
        let commitment = target.commitment().unwrap();
        assert_eq!(
            commitment,
            "b5e9469977ea5654078b437e756878919f4ca58e5b127e450179b226b8cf2cef"
        );
        configuration.scope.revision = "r2".into();
        configuration.scope.tenant = "second-tenant".into();
        configuration.scope.origin = "second-origin".into();
        configuration.scope.authority = "other.test".into();
        assert_eq!(
            KillTarget::for_configuration(&configuration)
                .unwrap()
                .commitment()
                .unwrap(),
            commitment
        );
        configuration.audit.node_id = "different-node".into();
        assert_ne!(
            KillTarget::for_configuration(&configuration)
                .unwrap()
                .commitment()
                .unwrap(),
            commitment
        );
        configuration.audit.node_id.clear();
        assert!(KillTarget::for_configuration(&configuration).is_err());
        configuration.audit.node_id = "runtime-test".into();
        configuration.scope.installation = "different-installation".into();
        assert_ne!(
            KillTarget::for_configuration(&configuration)
                .unwrap()
                .commitment()
                .unwrap(),
            commitment
        );
    }
}
