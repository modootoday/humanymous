use std::io;
use winsafe::{HACCESSTOKEN, HTHREAD, TokenInfo, co};

/// Facts queried from the actual current process token.
pub struct WindowsIdentity {
    /// Native current user SID, not an account-name translation.
    sid: String,
    /// Actual TokenElevation value.
    elevated: bool,
    /// Administrators SID enabled for access checks; deny-only is not enabled.
    enabled_administrator: bool,
}
impl WindowsIdentity {
    /// Native token user SID; this identity cannot be caller-constructed.
    pub fn sid(&self) -> &str {
        &self.sid
    }

    /// Whether the current token satisfies the explicit installer policy.
    pub fn installer_allowed(&self) -> bool {
        super::installer_token_allowed(self.elevated, self.enabled_administrator)
    }
}
fn error(e: co::ERROR) -> io::Error {
    io::Error::other(format!("native token query failed: {e}"))
}

/// Read user, elevation and enabled-group facts without launching a shell or
/// changing/impersonating any token. Any unexpected response is an error.
pub fn current_identity() -> io::Result<WindowsIdentity> {
    // Process identity must not silently stand in for an impersonated thread's
    // effective identity. This CLI boundary does not support impersonation.
    match HTHREAD::GetCurrentThread().OpenThreadToken(co::TOKEN::QUERY, true) {
        Err(co::ERROR::NO_TOKEN) => (),
        Err(other) => return Err(error(other)),
        Ok(_) => return Err(io::Error::other("impersonated thread is unsupported")),
    }
    let token = HACCESSTOKEN::GetCurrentProcessToken();
    let TokenInfo::User(user) = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::User)
        .map_err(error)?
    else {
        return Err(io::Error::other("native user token unavailable"));
    };
    let sid = winsafe::ConvertSidToStringSid(
        user.User
            .Sid()
            .ok_or_else(|| io::Error::other("native user SID missing"))?,
    )
    .map_err(error)?;
    let TokenInfo::Elevation(elevation) = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::Elevation)
        .map_err(error)?
    else {
        return Err(io::Error::other("native token elevation unavailable"));
    };
    let TokenInfo::Groups(groups) = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::Groups)
        .map_err(error)?
    else {
        return Err(io::Error::other("native token groups unavailable"));
    };
    let mut enabled_administrator = false;
    for group in groups.Groups() {
        let group_sid = winsafe::ConvertSidToStringSid(
            group
                .Sid()
                .ok_or_else(|| io::Error::other("native group SID missing"))?,
        )
        .map_err(error)?;
        if super::enabled_admin_group(&group_sid, group.Attributes) {
            enabled_administrator = true;
        }
    }
    Ok(WindowsIdentity {
        sid,
        elevated: elevation.TokenIsElevated(),
        enabled_administrator,
    })
}
