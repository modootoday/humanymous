//! Pure, non-secret host-plan validation; no native calls or secret retrieval.
use humanymous_runtime::authority::configuration::Peer;
use humanymous_runtime::gateway::service::StopBudget;
use serde::Deserialize;
use std::{io, net::SocketAddr, path::PathBuf, time::Duration};

/// Budgets must match the existing native manager adapter's effective bound.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopPlan {
    /// Asynchronous request drain interval.
    pub drain_ms: u64,
    /// Independent reserve for synchronous flush and final reporting.
    pub supervisor_reserve_ms: u64,
    /// External CLI manager stop-wait bound, not an inferred global OS setting.
    pub manager_stop_ms: u64,
}
impl StopPlan {
    /// Reuse the runtime-owned invariant; never duplicate its arithmetic policy.
    pub fn budget(self) -> io::Result<StopBudget> {
        StopBudget::new(
            self.drain_ms,
            self.supervisor_reserve_ms,
            self.manager_stop_ms,
        )
    }
    pub(super) fn watchdog(self) -> io::Result<Duration> {
        self.budget()?;
        Ok(Duration::from_millis(
            self.drain_ms
                .checked_add(self.supervisor_reserve_ms)
                .ok_or_else(|| invalid("watchdog overflow"))?,
        ))
    }
}

/// Non-secret native observer binding and independent OS credential selector.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverPlan {
    /// Native endpoint consumed by the shared authority transport.
    pub endpoint: String,
    /// Exact native peer, never forwarded browser identity.
    pub peer: Peer,
    /// Credential-store entry identifier, not a credential value.
    pub credential_id: String,
}

/// Whether the OS native dispatcher or an explicit hosted foreground actor owns stop.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostMode {
    /// SCM on Windows; ordinary foreground under systemd/launchd on Unix.
    Native,
    /// Owned foreground process qualification; does not claim native registration.
    Foreground,
}

/// This entire document is non-secret and SHA256-pinned in signed launch arguments.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostPlan {
    /// Exact format discriminator.
    pub schema: String,
    /// Owned service namespace, shared with the signed release.
    pub service_name: String,
    /// Windows installer owner of the delegated, digest-pinned launch document.
    #[serde(default)]
    pub installer_owner_sid: Option<String>,
    /// Disposable hosted fixture; absent from production builds and documents.
    #[cfg(all(windows, feature = "runtime-service-pilot"))]
    #[serde(default)]
    pub pilot: Option<super::pilot::PilotPlan>,
    /// Installation binding that the runtime configuration must match.
    pub installation: String,
    /// Explicit opaque generation identifier, never a credential or entropy claim.
    pub generation: String,
    /// Expected currently executing CLI binary bytes.
    pub executable_sha256: String,
    /// Owner-private non-secret configuration document; its passphrase must be empty.
    pub configuration: PathBuf,
    /// Digest binds that exact configuration independently of path spelling.
    pub configuration_sha256: String,
    /// OS store selector for the existing sealed-custody unseal secret.
    pub unseal_credential_id: String,
    /// Optional authenticated observer, distinct from public readiness.
    pub observer: Option<ObserverPlan>,
    /// Loopback diagnostic endpoint, carrying no management credentials.
    pub diagnostic: SocketAddr,
    /// Signed diagnostic answer; this never attests full product health.
    pub ready_response: String,
    /// Sealed startup/credential/bind deadline, enforced by an independent thread.
    pub startup_ms: u64,
    /// Validated shutdown budget.
    pub stop: StopPlan,
    /// Exact process ownership mode.
    pub mode: HostMode,
}

pub(super) fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
pub(super) fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl HostPlan {
    /// Pure preflight. Actual owner/ACL/OS-store and listener facts are separate gates.
    pub fn validate(&self) -> io::Result<()> {
        let suffix = self
            .service_name
            .strip_prefix("humanymous-lab-")
            .unwrap_or("");
        let stop_limit = if cfg!(windows) { 6000 } else { 5000 };
        if self.schema != "humanymous.runtime-host.v1"
            || suffix.is_empty()
            || suffix.len() > 48
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !identifier(&self.installation)
            || !identifier(&self.generation)
            || !hash(&self.executable_sha256)
            || !hash(&self.configuration_sha256)
            || !self.configuration.is_absolute()
            || !self.diagnostic.ip().is_loopback()
            || self.diagnostic.port() == 0
            || self.ready_response.is_empty()
            || self.ready_response.len() > 512
            || self.ready_response.chars().any(char::is_control)
            || !(1..=30_000).contains(&self.startup_ms)
            || self.stop.manager_stop_ms != stop_limit
        {
            return Err(invalid("invalid runtime host plan"));
        }
        self.stop.budget()?;
        #[cfg(all(windows, feature = "runtime-service-pilot"))]
        if let Some(pilot) = &self.pilot {
            pilot.validate(self)?;
        }
        if cfg!(windows)
            && self
                .installer_owner_sid
                .as_ref()
                .is_none_or(|sid| sid.is_empty() || sid.len() > 184 || !sid.starts_with("S-1-"))
        {
            return Err(invalid("explicit launch-document installer SID required"));
        }
        humanymous_runtime::native_security::secret::ServiceSecretReference::new(
            &self.service_name,
            &self.unseal_credential_id,
        )?;
        if let Some(observer) = &self.observer {
            humanymous_runtime::native_security::secret::ServiceSecretReference::new(
                &self.service_name,
                &observer.credential_id,
            )?;
            if observer.credential_id == self.unseal_credential_id
                || observer.endpoint.is_empty()
                || observer.endpoint.len() > 512
                || observer.endpoint.chars().any(char::is_control)
            {
                return Err(invalid("invalid independent observer selector"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_budget_is_shared_strict_and_bounded() {
        assert!(
            StopPlan {
                drain_ms: 4000,
                supervisor_reserve_ms: 1000,
                manager_stop_ms: 5000
            }
            .budget()
            .is_err()
        );
        assert!(
            StopPlan {
                drain_ms: 3000,
                supervisor_reserve_ms: 1000,
                manager_stop_ms: 5000
            }
            .budget()
            .is_ok()
        );
        assert!(
            StopPlan {
                drain_ms: 1,
                supervisor_reserve_ms: u64::MAX,
                manager_stop_ms: u64::MAX
            }
            .watchdog()
            .is_err()
        );
    }
    #[test]
    fn selectors_and_hashes_have_no_wildcards_or_implicit_secrets() {
        for bad in ["", "../entry", "name\n", "name token", "*"] {
            assert!(!identifier(bad));
        }
        assert!(identifier("service-unseal-v1"));
        assert!(hash(&"0".repeat(64)));
        assert!(!hash(&"A".repeat(64)));
    }
}
