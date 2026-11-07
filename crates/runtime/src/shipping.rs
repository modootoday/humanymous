//! Bounded public-role declarations: DATA only, never native admission or Ready.
mod binding;
mod scope;
mod wire;
pub use binding::RoleBinding;
pub use scope::RoleScope;

/// Explicit public application role, not a listener or authority grant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PublicRole {
    /// Local detection/demo handlers and owned browser assets.
    CoreDemo,
    /// Edge composition with separately accepted origin and admin contracts.
    GateEdge,
}

/// A required provider declaration, never an availability observation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequiredProvider {
    /// Durable decision audit.
    DurableAudit,
    /// Separately verified external witness.
    ExternalWitness,
    /// Actual native preparation owner.
    NativePreparation,
    /// Original client TLS capture owner.
    OriginalTls,
    /// Additional original H2 requirement; omission waives no later rule.
    OriginalH2,
    /// Additional collector requirement; omission waives no later rule.
    Collector,
}

/// Fixed refusals contain no caller bytes or underlying parser diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicRoleDataError {
    /// Original byte input exceeds the profile's32768-byte budget.
    Size,
    /// Closed schema, bounds or mandatory declarations were not satisfied.
    Invalid,
}
impl std::fmt::Display for PublicRoleDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("public role data rejected")
    }
}
impl std::error::Error for PublicRoleDataError {}

/// Immutable caller assertions. No serialization, raw Debug or proof conversion.
pub struct PublicRoleData {
    role: PublicRole,
    scope: RoleScope,
    binding: RoleBinding,
    providers: Vec<RequiredProvider>,
}
impl PublicRoleData {
    /// Declared role, not an active service.
    pub fn role(&self) -> PublicRole {
        self.role
    }
    /// Exact decoded scope, without normalization or address validation.
    pub fn scope(&self) -> &RoleScope {
        &self.scope
    }
    /// Epoch/generation/digest DATA; no replay reservation or artifact proof.
    pub fn binding(&self) -> &RoleBinding {
        &self.binding
    }
    /// Exact bounded declaration order; no provider has been contacted.
    pub fn required_providers(&self) -> &[RequiredProvider] {
        &self.providers
    }
}

/// Parse the one closed public-role/v1 DATA envelope, preserving caller bytes.
/// This performs no I/O, binding comparison, freshness check or native admission.
pub fn parse_public_role_data(input: &[u8]) -> Result<PublicRoleData, PublicRoleDataError> {
    if input.len() > 32_768 {
        return Err(PublicRoleDataError::Size);
    }
    let mut decoder = serde_json::Deserializer::from_slice(input);
    let parsed: wire::Envelope =
        wire::object(&mut decoder).map_err(|_| PublicRoleDataError::Invalid)?;
    decoder.end().map_err(|_| PublicRoleDataError::Invalid)?;
    if parsed.schema != "humanymous.public-role/1" {
        return Err(PublicRoleDataError::Invalid);
    }
    Ok(PublicRoleData {
        role: match parsed.role.as_str() {
            "core_demo" => PublicRole::CoreDemo,
            "gate_edge" => PublicRole::GateEdge,
            _ => return Err(PublicRoleDataError::Invalid),
        },
        scope: RoleScope::from_wire(parsed.scope)?,
        binding: RoleBinding::from_wire(parsed.binding)?,
        providers: parsed.required_providers.0,
    })
}

#[cfg(test)]
mod tests;
