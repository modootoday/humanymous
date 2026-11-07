//! Bounded public authorization DATA, never native admission or Ready authority.
mod binding;
mod codec;
mod hello;
mod verification;
pub use codec::parse_bundle;
pub use hello::{ParsedHello, parse_hello, verify_hello};
pub use verification::verify_bundle;

/// Static refusal classes contain no packet, principal or key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationError {
    /// Declared or actual size exceeds the closed profile.
    Size,
    /// Unknown, truncated, noncanonical or trailing data.
    Encoding,
    /// Invalid expected-data bounds or role separation.
    Context,
    /// Signed claims do not match the explicit expected DATA.
    Binding,
    /// Invalid key, weak point, or invalid/noncanonical Ed25519 R/S signature.
    /// No independent canonical-A normalization claim is made.
    Signature,
    /// Invalid time ordering, reference expiry or configured cap.
    Lifetime,
}
type Result<T> = std::result::Result<T, AuthorizationError>;

/// Fixed role tags, not native identity evidence.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AudienceRole {
    /// Service sender, wire commitment tag1.
    Service,
    /// Observer receiver, wire commitment tag2.
    Observer,
}
/// Caller-provided original installer projection, never a native trust factory.
#[derive(Clone)]
pub struct InstallerData {
    /// Expected original Ed25519 key; never selected from the packet.
    pub public_key: [u8; 32],
    /// Original installer canonical binary SID.
    pub sid: Vec<u8>,
    /// Original installation epoch.
    pub installation_epoch: [u8; 16],
    /// Exact independently selected complete delegation artifact commitment.
    pub delegation_commitment: [u8; 32],
    /// Selected delegation identifier.
    pub delegation_id: [u8; 32],
    /// Exact selected certificate validity start.
    pub not_before_ms: u64,
    /// Exact selected certificate validity end.
    pub not_after_ms: u64,
}
/// Expected publisher claims; not a retained process or service-store owner.
#[derive(Clone)]
pub struct PublisherData {
    /// Exact owned service name.
    pub name: String,
    /// Canonical primary TokenUser SID projection.
    pub sid: Vec<u8>,
    /// I-authorized P verification key, distinct from I.
    pub public_key: [u8; 32],
    /// Installed artifact digest, not loaded-page attestation.
    pub artifact: [u8; 32],
    /// Actual process identifier projection.
    pub pid: u32,
    /// Native creation FILETIME projection, without millisecond conversion.
    pub creation: u64,
    /// Publisher process epoch.
    pub epoch: [u8; 16],
}
/// Expected pair member DATA; actual retained native checks remain separate.
#[derive(Clone, PartialEq, Eq)]
pub struct RoleData {
    /// Exact Service/Observer position.
    pub role: AudienceRole,
    /// Exact owned service name.
    pub name: String,
    /// Canonical primary TokenUser SID projection.
    pub sid: Vec<u8>,
    /// Native process identifier projection.
    pub pid: u32,
    /// Native creation FILETIME projection.
    pub creation: u64,
    /// Role process epoch.
    pub epoch: [u8; 16],
    /// Exact immutable BASE plan; no launch-ticket/cursor fallback.
    pub base_plan: [u8; 32],
    /// Installed artifact digest, not loaded-page attestation.
    pub artifact: [u8; 32],
}
/// Bounded original configured scope, not a request-selected identity.
#[derive(Clone)]
pub struct ScopeData {
    /// Installation name.
    pub installation: String,
    /// Tenant name.
    pub tenant: String,
    /// Origin name.
    pub origin: String,
    /// Policy revision; never replaces stable kill/control identity.
    pub revision: String,
    /// Exact application authority, including case/port.
    pub application_authority: String,
}
/// Explicit one-attempt policy DATA; no implicit lease or native consumption.
#[derive(Clone)]
pub struct AttemptData {
    /// Expected nonzero grant identifier.
    pub grant_id: [u8; 32],
    /// Expected nonzero credential epoch.
    pub credential_epoch: [u8; 16],
    /// THE exact configured positive cap, shared by Prepared and Commit.
    pub startup_cap_ms: u32,
    /// Earliest admitted issuance time.
    pub earliest_issued_ms: u64,
    /// Latest admitted expiry time.
    pub latest_not_after_ms: u64,
}
/// Closed caller DATA. Supplying this does not establish native trust/provenance.
#[derive(Clone)]
pub struct ExpectedAuthorizationData {
    /// Original installer selection.
    pub installer: InstallerData,
    /// Delegated finite publisher.
    pub publisher: PublisherData,
    /// Exact service audience.
    pub service: RoleData,
    /// Exact observer audience.
    pub observer: RoleData,
    /// Exact configured scope.
    pub scope: ScopeData,
    /// Exact attempt binding and limits.
    pub attempt: AttemptData,
}

struct Signed<T> {
    claims: T,
    body: Vec<u8>,
    signature: [u8; 64],
    artifact: Vec<u8>,
}
struct Delegation {
    installation_epoch: [u8; 16],
    scope: [u8; 32],
    installer: [u8; 32],
    installer_key: [u8; 32],
    publisher_sid: Vec<u8>,
    publisher_name: String,
    publisher_key: [u8; 32],
    publisher_artifact: [u8; 32],
    pair: [u8; 32],
    id: [u8; 32],
    not_before: u64,
    not_after: u64,
}
struct Prepared {
    delegation: [u8; 32],
    scope: [u8; 32],
    installation: [u8; 32],
    publisher_key: [u8; 32],
    grant: [u8; 32],
    publisher_instance: [u8; 32],
    publisher_pid: u32,
    publisher_creation: u64,
    publisher_epoch: [u8; 16],
    credential_epoch: [u8; 16],
    service: RoleData,
    observer: RoleData,
    issued: u64,
    not_after: u64,
    cap: u32,
}
struct Commit {
    delegation: [u8; 32],
    grant: [u8; 32],
    prepared: [u8; 32],
    publisher_key: [u8; 32],
    service_epoch: [u8; 16],
    observer_epoch: [u8; 16],
    credential_epoch: [u8; 16],
    issued: u64,
    not_after: u64,
    cap: u32,
}
/// Structurally parsed PUBLIC records, still untrusted and not native evidence.
pub struct ParsedAuthorization {
    delegation: Signed<Delegation>,
    prepared: Signed<Prepared>,
    commit: Signed<Commit>,
}
/// Strict signatures/bindings checked against caller DATA only, NOT NativeProof.
/// There is deliberately no conversion into observer preparation or Ready.
pub struct SignatureCheckedAuthorization {
    records: ParsedAuthorization,
    scope: ScopeData,
}
impl SignatureCheckedAuthorization {
    /// Immutable validated scope DATA, detached from later expected-data mutation.
    /// This is not an installer, service or native origin capability.
    pub fn scope(&self) -> &ScopeData {
        &self.scope
    }
    /// Signed service audience DATA, not a process handle or native peer proof.
    pub fn service(&self) -> &RoleData {
        &self.records.prepared.claims.service
    }
    /// Signed observer audience DATA, not a process handle or native peer proof.
    pub fn observer(&self) -> &RoleData {
        &self.records.prepared.claims.observer
    }
    /// Complete selected delegation link, not a protected-store capability.
    pub fn delegation_commitment(&self) -> &[u8; 32] {
        &self.records.prepared.claims.delegation
    }
    /// Signed credential epoch DATA, not an actual credential or Ready receipt.
    pub fn credential_epoch(&self) -> &[u8; 16] {
        &self.records.prepared.claims.credential_epoch
    }
    /// Public grant identifier; not an actual one-use reservation.
    pub fn grant_id(&self) -> &[u8; 32] {
        &self.records.prepared.claims.grant
    }
    /// Public expiry, not a trusted clock/liveness observation.
    pub fn not_after_ms(&self) -> u64 {
        self.records.prepared.claims.not_after
    }
    /// Exact configured startup cap, not a lease renewal.
    pub fn startup_cap_ms(&self) -> u32 {
        self.records.prepared.claims.cap
    }
}

#[cfg(test)]
mod tests;
