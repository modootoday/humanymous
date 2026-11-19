//! Admin bearer authentication, the four-role admin model and the pending
//! two-person approval store (legacy `internal/gate/adminauth.go` and
//! `internal/gate/approval.go`).
//!
//! This is a gateway-side kernel. It does not replace the signed two-approval
//! quorum and durable consumption in `audit::control`; it ports the Go pending
//! store, role rules and HMAC approval ticket so they are available to an admin
//! surface. Nothing here scores a request.
//!
//! Kept from Go:
//! - `Authorization: Bearer <token>` is the only credential form. A missing or
//!   unknown token answers [`DENY_STATUS`] (404, deny by default and not
//!   discoverable), never 403.
//! - Authentication compares the presented token with every registered token
//!   in constant time and never exits early on a match.
//! - Capabilities: any role reads; operator and DPO operate; approver and DPO
//!   approve; only DPO approves erasure. DPO satisfies an approver requirement,
//!   never the reverse.
//! - Approval refusals are checked in Go order: unknown id, expired (the entry
//!   is removed), self-approval, wrong role. A successful approval removes the
//!   entry, so it commits exactly once.
//! - The ticket is hex HMAC-SHA256 over `id|kind|requester|approver` followed
//!   by `|k=v` for the Go binding keys, so it cannot be moved to another target.
//! - Parameters are copied on create; the caller cannot retarget a pending
//!   action by mutating its own map.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - Empty tokens and empty operator ids are refused at registration, and a
//!   token already registered is refused instead of being silently reassigned.
//! - Ids come from the OS entropy source; if it fails, create is refused
//!   instead of storing an action under a predictable id.
//! - The pending set is bounded by [`MAX_PENDING`]; when it is full after
//!   expiry collection, create is refused.
//! - The caller supplies a monotonic time. A time earlier than an entry's
//!   creation never expires it.
//! - [`ApprovalStore::pending`] returns entries in id order, not map order.
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Mutex, PoisonError, RwLock};
use std::time::Duration;
use subtle::ConstantTimeEq;

/// HTTP status for a missing or invalid admin credential (Go: 404, never 403).
pub const DENY_STATUS: u16 = 404;
/// Bound on retained pending actions.
pub const MAX_PENDING: usize = 4096;
/// Random bytes in a pending-action id (hex encoded: 24 characters).
pub const ID_BYTES: usize = 12;
/// Parameter keys bound into the approval ticket, in Go order.
pub const TICKET_KEYS: [&str; 9] = [
    "key",
    "durationSec",
    "subject",
    "legalBasis",
    "overlayId",
    "parentConfigVersion",
    "mutationClass",
    "body",
    "action",
];

/// An admin role (Go `Role`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Role {
    /// Read only.
    Auditor,
    /// Read, request bans or erasure, lift bans; cannot self-approve.
    Operator,
    /// Read and approve pending dual-control actions.
    Approver,
    /// Read and approve erasure (data-protection officer).
    Dpo,
}

impl Role {
    /// The Go wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auditor => "auditor",
            Self::Operator => "operator",
            Self::Approver => "approver",
            Self::Dpo => "dpo",
        }
    }

    /// Parses an exact Go wire name.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "auditor" => Some(Self::Auditor),
            "operator" => Some(Self::Operator),
            "approver" => Some(Self::Approver),
            "dpo" => Some(Self::Dpo),
            _ => None,
        }
    }
}

/// Whether a held role satisfies a required role. DPO satisfies approver.
pub fn has_role(have: Role, need: Role) -> bool {
    have == need || (need == Role::Approver && have == Role::Dpo)
}

/// An authenticated admin principal. The id is server derived and is the
/// audit actor; client-supplied identity fields are never trusted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operator {
    /// Operator id.
    pub id: String,
    /// Operator role.
    pub role: Role,
}

impl Operator {
    /// Any authenticated role may read.
    pub fn can_read(&self) -> bool {
        true
    }

    /// Operator and DPO may request mutations.
    pub fn can_operate(&self) -> bool {
        matches!(self.role, Role::Operator | Role::Dpo)
    }

    /// Approver and DPO may approve pending actions.
    pub fn can_approve(&self) -> bool {
        matches!(self.role, Role::Approver | Role::Dpo)
    }

    /// Only DPO may approve erasure.
    pub fn can_approve_erasure(&self) -> bool {
        self.role == Role::Dpo
    }
}

/// A refused admin registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthRefusal {
    /// Empty bearer token.
    EmptyToken,
    /// Empty operator id.
    EmptyId,
    /// The token is already registered.
    Duplicate,
}

impl fmt::Display for AuthRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyToken => "empty admin token",
            Self::EmptyId => "empty operator id",
            Self::Duplicate => "admin token already registered",
        })
    }
}

impl std::error::Error for AuthRefusal {}

/// Bearer token table (Go `AdminAuth`).
#[derive(Default)]
pub struct AdminAuth {
    tokens: RwLock<Vec<(Vec<u8>, Operator)>>,
}

impl AdminAuth {
    /// An empty table: every request is denied.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an operator under a bearer token.
    pub fn add(&self, token: &str, id: &str, role: Role) -> Result<(), AuthRefusal> {
        if token.is_empty() {
            return Err(AuthRefusal::EmptyToken);
        }
        if id.is_empty() {
            return Err(AuthRefusal::EmptyId);
        }
        let mut tokens = self.tokens.write().unwrap_or_else(PoisonError::into_inner);
        if tokens.iter().any(|(known, _)| known == token.as_bytes()) {
            return Err(AuthRefusal::Duplicate);
        }
        tokens.push((
            token.as_bytes().to_vec(),
            Operator {
                id: id.to_owned(),
                role,
            },
        ));
        Ok(())
    }

    /// Resolves an `Authorization` header value to an operator. `None` means
    /// deny with [`DENY_STATUS`].
    pub fn authenticate(&self, authorization: Option<&str>) -> Option<Operator> {
        let presented = bearer(authorization?)?.as_bytes();
        let tokens = self.tokens.read().unwrap_or_else(PoisonError::into_inner);
        let mut found = None;
        for (known, operator) in tokens.iter() {
            if bool::from(known.as_slice().ct_eq(presented)) {
                found = Some(operator.clone());
            }
        }
        found
    }
}

/// Extracts the token from `Bearer <token>`; the prefix is case sensitive and
/// the token must be non-empty, as in Go.
pub fn bearer(header: &str) -> Option<&str> {
    header
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

/// A destructive action awaiting a distinct approver (Go `PendingAction`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAction {
    /// Random hex id.
    pub id: String,
    /// Action kind, such as `ban` or `erasure`.
    pub kind: String,
    /// Copied action parameters.
    pub params: BTreeMap<String, String>,
    /// Authenticated requesting operator id.
    pub requester: String,
    /// Role required to approve.
    pub needs_role: Role,
    /// Monotonic creation time.
    pub created: Duration,
}

/// A refused create or approval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalError {
    /// No pending action with that id (Go `errNoPending`).
    NoPending,
    /// The action outlived its TTL and was removed (Go `errExpired`).
    Expired,
    /// Approver equals requester (Go `errSelfApproval`).
    SelfApproval,
    /// Approver lacks the required role (Go `errWrongRole`).
    WrongRole,
    /// The pending set is full.
    Saturated,
    /// The OS entropy source failed.
    Entropy,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPending => "no such pending action",
            Self::Expired => "approval expired",
            Self::SelfApproval => "approver must differ from requester (dual-control)",
            Self::WrongRole => "approver lacks the required role",
            Self::Saturated => "too many pending actions",
            Self::Entropy => "approval id entropy unavailable",
        })
    }
}

impl std::error::Error for ApprovalError {}

/// Pending two-person approvals (Go `ApprovalStore`).
pub struct ApprovalStore {
    key: Vec<u8>,
    ttl: Duration,
    pending: Mutex<HashMap<String, PendingAction>>,
}

impl ApprovalStore {
    /// A store with a ticket key and a pending TTL.
    pub fn new(key: &[u8], ttl: Duration) -> Self {
        Self {
            key: key.to_vec(),
            ttl,
            pending: Mutex::new(HashMap::new()),
        }
    }

    fn expired(&self, action: &PendingAction, now: Duration) -> bool {
        now.saturating_sub(action.created) > self.ttl
    }

    /// Records a pending action with copied parameters.
    pub fn create(
        &self,
        kind: &str,
        params: &BTreeMap<String, String>,
        requester: &str,
        needs_role: Role,
        now: Duration,
    ) -> Result<PendingAction, ApprovalError> {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.retain(|_, action| !self.expired(action, now));
        if pending.len() >= MAX_PENDING {
            return Err(ApprovalError::Saturated);
        }
        let id = loop {
            let mut bytes = [0_u8; ID_BYTES];
            getrandom::fill(&mut bytes).map_err(|_| ApprovalError::Entropy)?;
            let id = hex::encode(bytes);
            if !pending.contains_key(&id) {
                break id;
            }
        };
        let action = PendingAction {
            id: id.clone(),
            kind: kind.to_owned(),
            params: params.clone(),
            requester: requester.to_owned(),
            needs_role,
            created: now,
        };
        pending.insert(id, action.clone());
        Ok(action)
    }

    /// Commits a pending action by a distinct approver with the required role,
    /// returning the action and its ticket.
    pub fn approve(
        &self,
        id: &str,
        approver: &Operator,
        now: Duration,
    ) -> Result<(PendingAction, String), ApprovalError> {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        let action = pending.get(id).ok_or(ApprovalError::NoPending)?;
        if self.expired(action, now) {
            pending.remove(id);
            return Err(ApprovalError::Expired);
        }
        if approver.id == action.requester {
            return Err(ApprovalError::SelfApproval);
        }
        if !has_role(approver.role, action.needs_role) {
            return Err(ApprovalError::WrongRole);
        }
        let ticket = self.ticket(action, &approver.id);
        let action = pending.remove(id).ok_or(ApprovalError::NoPending)?;
        Ok((action, ticket))
    }

    /// Remove a pending action without committing it (a create whose audit
    /// record could not be written). Returns whether it was pending.
    pub fn withdraw(&self, id: &str) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id)
            .is_some()
    }

    /// Unexpired pending actions in id order; expired ones are removed.
    pub fn pending(&self, now: Duration) -> Vec<PendingAction> {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.retain(|_, action| !self.expired(action, now));
        let mut out: Vec<_> = pending.values().cloned().collect();
        out.sort_by(|left, right| left.id.cmp(&right.id));
        out
    }

    /// Hex HMAC-SHA256 binding the approval to the exact action and approver.
    pub fn ticket(&self, action: &PendingAction, approver: &str) -> String {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.key) else {
            unreachable!("HMAC accepts any key length")
        };
        for part in [
            action.id.as_str(),
            "|",
            action.kind.as_str(),
            "|",
            action.requester.as_str(),
            "|",
            approver,
        ] {
            mac.update(part.as_bytes());
        }
        for key in TICKET_KEYS {
            let value = action.params.get(key).map_or("", String::as_str);
            mac.update(format!("|{key}={value}").as_bytes());
        }
        hex::encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests;
