//! Bounded client session state; server consumption is never inferred from status.
#[cfg(test)]
mod tests;
mod wire;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_integrity::request::{
    browser::{self, Binding, MAX_SAFE_INTEGER},
    compute_token, seed,
};
use serde::Deserialize;
use zeroize::{Zeroize, Zeroizing};

/// Provisioned non-secret application scope; bootstrap must match it exactly.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Installation identifier.
    pub installation: String,
    /// Tenant identifier.
    pub tenant: String,
    /// Origin policy identity, not an arbitrary URL.
    pub origin: String,
    /// Immutable revision.
    pub revision: String,
    /// Expected actual browser/server authority.
    pub authority: String,
}
/// Final browser-controlled request representation.
pub struct Request<'a> {
    /// Uppercase method.
    pub method: &'a str,
    /// Exact origin-form path/query.
    pub target: &'a str,
    /// Final Content-Type value, or empty.
    pub content_type: &'a str,
    /// Final application Authorization, or empty.
    pub authorization: &'a str,
    /// Bounded exact body bytes.
    pub body: &'a [u8],
}
/// Static errors never include secret input, application body or wire replies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// Invalid strict bootstrap or key material.
    Bootstrap,
    /// Bootstrap did not match the provisioned application.
    Scope,
    /// Clock moved backwards or exceeded exact numeric bounds.
    Clock,
    /// The session/request deadline expired.
    Expired,
    /// Another request is already in flight.
    Busy,
    /// Session was invalidated or closed.
    Closed,
    /// Final request cannot be represented by this profile.
    Binding,
    /// Server consumption acknowledgement is absent or invalid.
    Acknowledgement,
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bootstrap => "invalid session bootstrap",
            Self::Scope => "session scope mismatch",
            Self::Clock => "invalid session clock",
            Self::Expired => "session deadline expired",
            Self::Busy => "session request in flight",
            Self::Closed => "session closed",
            Self::Binding => "invalid request binding",
            Self::Acknowledgement => "invalid consumption acknowledgement",
        })
    }
}
impl std::error::Error for SessionError {}

/// Transient wire fields. No Debug/Clone or serialized secret diagnostic.
pub struct Signed {
    id: Zeroizing<String>,
    token: Zeroizing<String>,
    /// Exact next counter.
    pub counter: u64,
    /// Ten-second request bucket.
    pub bucket: u64,
    /// Signed UTC millisecond deadline.
    pub deadline: u64,
}
impl Signed {
    /// Transient session capability; do not log.
    pub fn session(&self) -> &str {
        &self.id
    }
    /// Transient request MAC; do not log.
    pub fn token(&self) -> &str {
        &self.token
    }
}
struct Pending {
    token: Zeroizing<String>,
    counter: u64,
    bucket: u64,
    deadline: u64,
}
/// One volatile client key epoch. No transport, human or producer attestation.
pub struct Session {
    scope: Scope,
    id: Zeroizing<String>,
    key: Zeroizing<[u8; 32]>,
    last: u64,
    expires: u64,
    last_now: u64,
    timeout: u64,
    pending: Option<Pending>,
    closed: bool,
}
impl Session {
    /// Parse bounded strict bootstrap against caller-provisioned scope.
    pub fn from_json(
        scope: Scope,
        body: &[u8],
        now: u64,
        timeout_ms: u64,
    ) -> Result<Self, SessionError> {
        if body.len() > 8192 || !(1..=30_000).contains(&timeout_ms) || now > MAX_SAFE_INTEGER {
            return Err(SessionError::Bootstrap);
        }
        let reply: wire::Bootstrap =
            serde_json::from_slice(body).map_err(|_| SessionError::Bootstrap)?;
        if reply.protocol != browser::PROTOCOL
            || reply.counter != 0
            || reply.expires > MAX_SAFE_INTEGER
            || reply.expires <= now
        {
            return Err(SessionError::Bootstrap);
        }
        if reply.installation != scope.installation
            || reply.tenant != scope.tenant
            || reply.origin != scope.origin
            || reply.revision != scope.revision
            || reply.authority != scope.authority
        {
            return Err(SessionError::Scope);
        }
        let key = wire::key(&reply.key)?;
        let session = Self {
            scope,
            id: reply.session,
            key,
            last: 0,
            expires: reply.expires,
            last_now: now,
            timeout: timeout_ms,
            pending: None,
            closed: false,
        };
        // Reuse the exact shared scope/session validation without a second parser.
        let dummy = Request {
            method: "POST",
            target: "/",
            content_type: "",
            authorization: "",
            body: &[],
        };
        let _validated =
            Zeroizing::new(session.canonical(&dummy, 1, now / 10_000, reply.expires)?);
        Ok(session)
    }
    /// Provisioned non-secret scope.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// Whether this epoch has been invalidated. Does not imply server validity.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    /// Whether a caller must await the current request before signing another.
    pub fn in_flight(&self) -> bool {
        self.pending.is_some()
    }
    /// Sign a validated final request and reserve exactly the next counter.
    pub fn begin(&mut self, request: &Request<'_>, now: u64) -> Result<Signed, SessionError> {
        self.observe(now)?;
        if self.pending.is_some() {
            return Err(SessionError::Busy);
        }
        let Some(counter) = self.last.checked_add(1).filter(|n| *n <= MAX_SAFE_INTEGER) else {
            self.invalidate();
            return Err(SessionError::Closed);
        };
        let deadline = now
            .checked_add(self.timeout)
            .ok_or(SessionError::Clock)?
            .min(self.expires);
        let bucket = now / 10_000;
        let canonical = Zeroizing::new(self.canonical(request, counter, bucket, deadline)?);
        let derived = Zeroizing::new(seed(self.key.as_ref(), self.last));
        let token = Zeroizing::new(compute_token(derived.as_ref(), &canonical, counter, bucket));
        self.pending = Some(Pending {
            token: token.clone(),
            counter,
            bucket,
            deadline,
        });
        Ok(Signed {
            id: self.id.clone(),
            token,
            counter,
            bucket,
            deadline,
        })
    }
    /// Advance only after the complete bounded body and exact server ACK arrive.
    /// Any failure closes the epoch, even if the server may have consumed it.
    pub fn complete(
        &mut self,
        protocol: &str,
        counter: u64,
        status: u16,
        acknowledgement: &str,
        now: u64,
    ) -> Result<(), SessionError> {
        self.observe(now)?;
        let result = (|| {
            let pending = self.pending.as_ref().ok_or(SessionError::Acknowledgement)?;
            if now >= pending.deadline {
                return Err(SessionError::Expired);
            }
            if protocol != browser::PROTOCOL
                || counter != pending.counter
                || !browser::verify_acknowledgement(
                    self.key.as_ref(),
                    &pending.token,
                    pending.counter,
                    pending.bucket,
                    status,
                    acknowledgement,
                )
            {
                return Err(SessionError::Acknowledgement);
            }
            Ok(pending.counter)
        })();
        match result {
            Ok(counter) => {
                self.last = counter;
                self.pending = None;
                Ok(())
            }
            Err(error) => {
                self.invalidate();
                Err(error)
            }
        }
    }
    fn canonical(
        &self,
        request: &Request<'_>,
        counter: u64,
        bucket: u64,
        deadline: u64,
    ) -> Result<String, SessionError> {
        browser::canonical(&Binding {
            installation: &self.scope.installation,
            tenant: &self.scope.tenant,
            origin_identity: &self.scope.origin,
            revision: &self.scope.revision,
            authority: &self.scope.authority,
            session: &self.id,
            method: request.method,
            target: request.target,
            counter,
            bucket,
            deadline,
            content_type: request.content_type,
            application_authorization: request.authorization,
            body: request.body,
        })
        .map_err(|_| SessionError::Binding)
    }
    fn observe(&mut self, now: u64) -> Result<(), SessionError> {
        if self.closed {
            return Err(SessionError::Closed);
        }
        if now < self.last_now || now > MAX_SAFE_INTEGER {
            self.invalidate();
            return Err(SessionError::Clock);
        }
        if now >= self.expires {
            self.invalidate();
            return Err(SessionError::Expired);
        }
        self.last_now = now;
        Ok(())
    }
    /// Close on ambiguous network/timeout/abort/drop; never guess consumed state.
    pub fn invalidate(&mut self) {
        self.closed = true;
        self.pending = None;
        self.key.zeroize();
        self.id.zeroize();
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.invalidate();
    }
}
