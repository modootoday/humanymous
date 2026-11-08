//! Review candidate: retained Windows observations, never observer authority.
//! Intended destination: native_security/process/observer/windows_process.rs.
#![cfg(windows)]
#![forbid(unsafe_code)]

use std::{io, time::Instant};
use winsafe::{HACCESSTOKEN, HPROCESS, HTHREAD, TokenInfo, co, guard::CloseHandleGuard};

/// An actual retained process and its initial primary token. Not a role proof.
/// No raw-handle getter, public constructor, Clone, Debug or serialization.
pub(super) struct ProcessObservation {
    process: CloseHandleGuard<HPROCESS>,
    token: CloseHandleGuard<HACCESSTOKEN>,
    pid: u32,
    creation: u64,
    sid: String,
    deadline: Instant,
}

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "native process observation refused")
}

fn native_error(_: co::ERROR) -> io::Error {
    // Never expose the selected PID, SID, image path or native error payload.
    denied()
}

fn remaining(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "original startup deadline expired"));
    }
    Ok(())
}

fn token_sid(token: &HACCESSTOKEN) -> io::Result<String> {
    let TokenInfo::User(user) = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::User)
        .map_err(native_error)?
    else {
        return Err(denied());
    };
    winsafe::ConvertSidToStringSid(user.User.Sid().ok_or_else(denied)?)
        .map_err(native_error)
}

fn require_query_context() -> io::Result<()> {
    match HTHREAD::GetCurrentThread().OpenThreadToken(co::TOKEN::QUERY, true) {
        Err(co::ERROR::NO_TOKEN) => (),
        // No effective-thread identity substitution or impersonation fallback.
        Err(_) | Ok(_) => return Err(denied()),
    }
    let debug = winsafe::LookupPrivilegeValue(None, co::SE_PRIV::DEBUG_NAME)
        .map_err(native_error)?;
    let token = HACCESSTOKEN::GetCurrentProcessToken();
    let TokenInfo::Privileges(privileges) = token
        .GetTokenInformation(co::TOKEN_INFORMATION_CLASS::Privileges)
        .map_err(native_error)?
    else {
        return Err(denied());
    };
    if privileges.Privileges().iter().any(|entry| {
        entry.Luid == debug
            && (entry.Attributes & co::SE_PRIV_ATTR::ENABLED) == co::SE_PRIV_ATTR::ENABLED
    }) {
        return Err(denied());
    }
    Ok(())
}

impl ProcessObservation {
    /// Numeric PID is a locator ONLY. Callers still need protected installation,
    /// SCM configuration, launch-file, publication, channel and witness proofs.
    /// Every kernel call can block: an independent original-budget host is required.
    pub(super) fn open_locator(pid: u32, deadline: Instant) -> io::Result<Self> {
        remaining(deadline)?;
        if pid == 0 {
            return Err(denied());
        }
        require_query_context()?;
        remaining(deadline)?;
        let process = HPROCESS::OpenProcess(
            co::PROCESS::QUERY_LIMITED_INFORMATION | co::PROCESS::SYNCHRONIZE,
            false,
            pid,
        )
        .map_err(native_error)?;
        remaining(deadline)?;
        if process.GetProcessId().map_err(native_error)? != pid
            || process.WaitForSingleObject(Some(0)).map_err(native_error)? != co::WAIT::TIMEOUT
        {
            return Err(denied());
        }
        let (creation, _, _, _) = process.GetProcessTimes().map_err(native_error)?;
        let creation = u64::from(creation);
        if creation == 0 {
            return Err(denied());
        }
        remaining(deadline)?;
        let token = process.OpenProcessToken(co::TOKEN::QUERY).map_err(native_error)?;
        let sid = token_sid(&token)?;
        let observation = Self { process, token, pid, creation, sid, deadline };
        observation.recheck()?;
        Ok(observation)
    }

    /// Requery the SAME retained process; never reopen a cached numeric PID.
    pub(super) fn recheck(&self) -> io::Result<()> {
        remaining(self.deadline)?;
        require_query_context()?;
        if self.process.WaitForSingleObject(Some(0)).map_err(native_error)? != co::WAIT::TIMEOUT
            || self.process.GetProcessId().map_err(native_error)? != self.pid
        {
            return Err(denied());
        }
        let (creation, _, _, _) = self.process.GetProcessTimes().map_err(native_error)?;
        if u64::from(creation) != self.creation || token_sid(&self.token)? != self.sid {
            return Err(denied());
        }
        // Also query the currently attached primary token through the held process.
        let current = self.process.OpenProcessToken(co::TOKEN::QUERY).map_err(native_error)?;
        if token_sid(&current)? != self.sid {
            return Err(denied());
        }
        remaining(self.deadline)
    }

    /// DATA comparison only; true is never a NativeProof or management grant.
    pub(super) fn matches_claim(&self, pid: u32, creation: u64, sid: &str) -> io::Result<bool> {
        self.recheck()?;
        Ok(pid == self.pid && creation == self.creation && sid == self.sid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn zero_locator_is_rejected_without_native_open() {
        assert!(ProcessObservation::open_locator(0, Instant::now() + Duration::from_secs(2)).is_err());
    }

    #[test]
    fn expired_original_deadline_is_not_restarted() {
        let result = ProcessObservation::open_locator(std::process::id(), Instant::now());
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::TimedOut);
    }

    // Actual native queries, not pure tests. Hosted ordinary-token execution only;
    // no ignore/automatic privilege enablement or broad same-account fallback.
    #[test]
    fn actual_self_observation_retains_handle_and_rechecks_primary_token() {
        let observation = ProcessObservation::open_locator(
            std::process::id(), Instant::now() + Duration::from_secs(5),
        ).unwrap();
        observation.recheck().unwrap();
        assert!(observation.matches_claim(observation.pid, observation.creation, &observation.sid).unwrap());
        assert!(!observation.matches_claim(0, observation.creation, &observation.sid).unwrap());
        assert!(!observation.matches_claim(observation.pid, 0, &observation.sid).unwrap());
        assert!(!observation.matches_claim(observation.pid, observation.creation, "S-1-0-0").unwrap());
    }
}
