//! Explicit cookie-free browser transport policy and per-request ACK ownership.
use crate::{
    Rejection,
    configuration::{Configuration, RitProfile},
    http::Request,
};
use humanymous_core::integrity::request::browser::{self, PROTOCOL};
use serde::Serialize;
use zeroize::Zeroizing;

/// Classify only an explicit protocol marker; no inference from User-Agent.
pub fn profile(request: &Request) -> Result<RitProfile, Rejection> {
    match request.header("x-humanymous-protocol") {
        Some(PROTOCOL) => Ok(RitProfile::BrowserAuthOnly),
        None | Some("humanymous.runtime/0.1")
            if request.header("x-humanymous-session").is_none() =>
        {
            Ok(RitProfile::NativeFull)
        }
        None | Some("humanymous.runtime/0.1") => {
            Err(Rejection::new(409, "mixed_integrity_profile"))
        }
        _ => Err(Rejection::new(400, "integrity_protocol")),
    }
}

/// Reject ambient-cookie authority and cross-origin browser invocations.
/// Fetch metadata is a browser-origin restriction, not native peer authentication.
pub fn validate(config: &Configuration, request: &Request) -> Result<(), Rejection> {
    let policy = config
        .browser
        .as_ref()
        .ok_or(Rejection::new(422, "browser_profile_unavailable"))?;
    if request.header("cookie").is_some() {
        return Err(Rejection::new(
            422,
            "browser_cookie_credentials_unsupported",
        ));
    }
    match request.header("origin") {
        Some(origin) if origin == policy.origin => {}
        None if matches!(request.method.as_str(), "GET" | "HEAD") => {}
        _ => return Err(Rejection::new(403, "browser_origin")),
    }
    if request.header("sec-fetch-site") != Some("same-origin")
        || request.header("sec-fetch-mode") != Some("same-origin")
        || request.header("sec-fetch-dest") != Some("empty")
    {
        return Err(Rejection::new(403, "browser_fetch_metadata"));
    }
    Ok(())
}

/// Strict ten-field bootstrap. Secret fields are intentionally not Debug/Clone.
#[derive(Serialize)]
pub struct Bootstrap {
    /// Explicit version/domain.
    pub protocol: &'static str,
    /// Volatile random capability.
    pub session: Zeroizing<String>,
    /// Independent random signing key, canonical base64url.
    pub key: Zeroizing<String>,
    /// Last consumed counter, initially zero.
    pub counter: u64,
    /// Fixed session expiry in Unix milliseconds.
    pub expires: u64,
    /// Provisioned installation.
    pub installation: String,
    /// Provisioned tenant.
    pub tenant: String,
    /// Provisioned origin identity, not an arbitrary URL.
    pub origin: String,
    /// Immutable configuration revision.
    pub revision: String,
    /// Exact HTTP authority.
    pub authority: String,
}

/// One request's consumed-counter signer. Never retained as a reusable verdict.
/// It owns a zeroizing key copy so response-time expiry/GC cannot forge or lose
/// the fact that this already authenticated request consumed its counter.
pub(crate) struct Ack {
    key: Zeroizing<[u8; 32]>,
    request_token: Zeroizing<String>,
    counter: u64,
    bucket: u64,
}
impl Ack {
    pub(crate) fn new(key: &[u8; 32], request: &Request, counter: u64, bucket: u64) -> Self {
        Self {
            key: Zeroizing::new(*key),
            request_token: Zeroizing::new(
                request
                    .header("x-humanymous-token")
                    .unwrap_or("")
                    .to_owned(),
            ),
            counter,
            bucket,
        }
    }
    /// Only gateway-owned fixed-name fields are generated; origin fields must
    /// first be removed by the response-head owner.
    pub(crate) fn headers(&self, status: u16) -> Result<Vec<(String, String)>, Rejection> {
        let mac = browser::acknowledgement(
            self.key.as_ref(),
            &self.request_token,
            self.counter,
            self.bucket,
            status,
        )
        .map_err(|_| Rejection::new(503, "counter_ack"))?;
        Ok(vec![
            ("X-Humanymous-Protocol".into(), PROTOCOL.into()),
            ("X-Humanymous-Counter".into(), self.counter.to_string()),
            ("X-Humanymous-Ack".into(), mac),
        ])
    }
}
