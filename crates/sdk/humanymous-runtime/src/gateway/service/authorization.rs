//! Pure pre-unseal composition DATA, never native preparation or Ready.
use crate::authority::startup::authorization::SignatureCheckedAuthorization;
use crate::gateway::configuration::Configuration;

/// Static refusals expose no configuration, principal or key material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationDataError {
    /// Invalid Runtime resource or route configuration.
    Configuration,
    /// Configured scope differs from the immutable signature-checked scope.
    Scope,
    /// A secret was supplied before actual native admission.
    Secret,
    /// This observer profile must explicitly require an external witness.
    RequiredWitness,
}

/// Retains the same configuration and checked public DATA without cloning.
/// This cannot construct a native guard, open custody or activate a ServiceHost.
pub struct AuthorizationData {
    configuration: Configuration,
    authorization: SignatureCheckedAuthorization,
}

impl AuthorizationData {
    /// Bind an empty-secret configuration to the shared checked scope DATA.
    /// Actual native identity, one-claim ownership and W remain prerequisites
    /// for the separate, currently unavailable native preparation producer.
    pub fn bind(
        configuration: Configuration,
        authorization: SignatureCheckedAuthorization,
    ) -> Result<Self, AuthorizationDataError> {
        configuration
            .validate()
            .map_err(|_| AuthorizationDataError::Configuration)?;
        let expected = authorization.scope();
        let configured = &configuration.scope;
        if configured.installation != expected.installation
            || configured.tenant != expected.tenant
            || configured.origin != expected.origin
            || configured.revision != expected.revision
            || configured.authority != expected.application_authority
        {
            return Err(AuthorizationDataError::Scope);
        }
        if !configuration.audit.keys.passphrase.is_empty() {
            return Err(AuthorizationDataError::Secret);
        }
        if !configuration.audit.require_external_witness {
            return Err(AuthorizationDataError::RequiredWitness);
        }
        Ok(Self {
            configuration,
            authorization,
        })
    }

    /// Immutable configuration DATA, with no unseal passphrase.
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Shared checked DATA, not a native capability or live lease.
    pub fn authorization(&self) -> &SignatureCheckedAuthorization {
        &self.authorization
    }

    /// Move the same pair to a future actual native-admission composer.
    /// Extraction confers no native preparation, receipt or readiness.
    pub fn into_parts(self) -> (Configuration, SignatureCheckedAuthorization) {
        (self.configuration, self.authorization)
    }
}

#[cfg(test)]
mod tests;
