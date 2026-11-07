use super::{PublicRoleDataError, wire};

/// Bounded installation scope assertions, never request-selected authority.
pub struct RoleScope {
    installation: String,
    tenant: String,
    origin: String,
    revision: String,
    authority: String,
}
impl RoleScope {
    pub(super) fn from_wire(value: wire::Scope) -> Result<Self, PublicRoleDataError> {
        for token in [
            &value.installation,
            &value.tenant,
            &value.origin,
            &value.revision,
        ] {
            if token.is_empty()
                || token.len() > 128
                || !token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            {
                return Err(PublicRoleDataError::Invalid);
            }
        }
        let authority = value.application_authority;
        if authority.is_empty()
            || authority.len() > 1024
            || !authority
                .bytes()
                .all(|b| (33..127).contains(&b) && !b"@/#?\\,".contains(&b))
        {
            return Err(PublicRoleDataError::Invalid);
        }
        Ok(Self {
            installation: value.installation,
            tenant: value.tenant,
            origin: value.origin,
            revision: value.revision,
            authority,
        })
    }
    /// Exact installation token.
    pub fn installation(&self) -> &str {
        &self.installation
    }
    /// Exact tenant token.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }
    /// Exact origin token, not an origin address.
    pub fn origin(&self) -> &str {
        &self.origin
    }
    /// Exact policy revision token.
    pub fn revision(&self) -> &str {
        &self.revision
    }
    /// Exact authority DATA including case/port; no address resolution occurs.
    pub fn application_authority(&self) -> &str {
        &self.authority
    }
}
