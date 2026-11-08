//! Review candidate: actual SCM/process observations, NOT a native grant issuer.
//! Intended destination: native_security/process/observer/mod.rs.
#![forbid(unsafe_code)]

#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_service;

#[cfg(windows)]
use std::{io, time::Instant};
#[cfg(windows)]
use windows_process::ProcessObservation;
#[cfg(windows)]
use windows_service::ServiceObservation;

/// Retained native observations; no constructor accepts a caller success fact.
/// This cannot convert into VerifiedObserverPreparation or a Ready capability.
#[cfg(windows)]
pub(in crate::native_security) struct ScmInstanceObservation {
    service: ServiceObservation,
    process: ProcessObservation,
    initial_config: ::windows_service::service::ServiceConfig,
    pid: u32,
}

#[cfg(windows)]
impl ScmInstanceObservation {
    /// Query the exact local service, hold the process, then requery the SAME
    /// service object and config. No trusted owner/launch-file claim follows.
    pub(in crate::native_security) fn open_owned_locator(
        name: &str,
        original_deadline: Instant,
    ) -> io::Result<Self> {
        let service = ServiceObservation::open_locator(name, original_deadline)?;
        let initial_config = service.configuration_data()?;
        let pid = service.running_pid_locator()?;
        let process = ProcessObservation::open_locator(pid, original_deadline)?;
        let observed = Self { service, process, initial_config, pid };
        observed.recheck()?;
        Ok(observed)
    }

    /// A successful repeated observation is not an atomic configuration lock.
    /// Prelaunch file guards and original installer authorization remain required.
    pub(in crate::native_security) fn recheck(&self) -> io::Result<()> {
        self.process.recheck()?;
        if self.service.running_pid_locator()? != self.pid
            || self.service.configuration_data()? != self.initial_config
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "native service instance changed",
            ));
        }
        self.process.recheck()
    }

    /// Compare actual observations with untrusted expected DATA only.
    pub(in crate::native_security) fn matches_claim(
        &self,
        pid: u32,
        creation: u64,
        sid: &str,
    ) -> io::Result<bool> {
        self.recheck()?;
        self.process.matches_claim(pid, creation, sid)
    }

    /// Query the held service descriptor without classifying installation trust.
    pub(in crate::native_security) fn descriptor_data(
        &self,
    ) -> io::Result<windows_permissions::LocalBox<windows_permissions::SecurityDescriptor>> {
        self.recheck()?;
        self.service.descriptor_data()
    }
}
