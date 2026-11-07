use crate::{Rejection, service::StopBudget};
use humanymous_authority::{
    configuration::{Peer, Scope},
    credentials::Credential,
    transport::LocalRequest,
};
use serde::{Deserialize, Serialize};

/// Trusted native endpoint/principal; no request can select a listener or role.
pub struct ManagementPolicy {
    /// Existing private parent UDS or restricted local named-pipe endpoint.
    pub endpoint: String,
    /// Independently authenticated native management client principal/process.
    pub peer: Peer,
    /// Must match the actual CLI/manager stop policy.
    pub stop_budget: StopBudget,
    /// Separate observer credential from shared generate/private provisioning.
    /// Never supply an adapter or application session credential here.
    pub credential: Credential,
}

/// Exact installation binding; no prefix matching or inferred tenant authority.
#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManagementScope {
    /// Provisioned installation.
    pub installation: String,
    /// Provisioned tenant.
    pub tenant: String,
    /// Provisioned application identity.
    pub origin: String,
    /// Current policy revision.
    pub revision: String,
    /// Exact configured application authority.
    pub authority: String,
}
impl ManagementScope {
    pub(crate) fn from_scope(scope: &Scope) -> Self {
        Self {
            installation: scope.installation.clone(),
            tenant: scope.tenant.clone(),
            origin: scope.origin.clone(),
            revision: scope.revision.clone(),
            authority: scope.authority.clone(),
        }
    }
}
pub(super) fn validate_request(
    request: &LocalRequest,
    scope: &ManagementScope,
) -> Result<(), Rejection> {
    if request.method != "POST"
        || !matches!(
            request.target.as_str(),
            "/management/v0.1/status" | "/management/v0.1/settings"
        )
    {
        return Err(Rejection::new(403, "management_observer_route"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut host = None;
    let mut content_type = None;
    for (name, value) in &request.headers {
        let name = name.to_ascii_lowercase();
        if !seen.insert(name.clone()) {
            return Err(Rejection::new(400, "management_duplicate_header"));
        }
        if name == "cookie"
            || name == "origin"
            || name == "referer"
            || name.starts_with("sec-")
            || name.starts_with("proxy-")
            || name.starts_with("x-humanymous-")
        {
            return Err(Rejection::new(403, "management_native_only"));
        }
        match name.as_str() {
            "host" => host = Some(value.as_str()),
            "content-type" => content_type = Some(value.as_str()),
            _ => {}
        }
    }
    if host != Some("humanymous.management.local") || content_type != Some("application/json") {
        return Err(Rejection::new(400, "management_envelope"));
    }
    if request.body.len() > 8192 {
        return Err(Rejection::new(413, "management_body_size"));
    }
    let presented: ManagementScope = serde_json::from_slice(&request.body)
        .map_err(|_| Rejection::new(400, "management_scope"))?;
    if presented != *scope {
        return Err(Rejection::new(403, "management_scope"));
    }
    Ok(())
}
