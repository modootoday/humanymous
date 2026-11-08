//! Review candidate: read-only SCM observations, never installation trust/Ready.
//! Intended destination: native_security/process/observer/windows_service.rs.
#![cfg(windows)]
#![forbid(unsafe_code)]

use std::{
    io,
    os::windows::io::{AsRawHandle, RawHandle},
    time::Instant,
};
use windows_permissions::{
    LocalBox, SecurityDescriptor,
    constants::{SeObjectType, SecurityInformation},
    wrappers,
};
use ::windows_service::{
    service::{Service, ServiceAccess, ServiceConfig, ServiceState, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

/// Holds the SAME queried SCM object. Read-only; no start/stop/delete/configure.
pub(super) struct ServiceObservation {
    service: Service,
    deadline: Instant,
}

// AsRawHandle is a safe borrowed-view trait. No BorrowedHandle::from_raw_handle,
// ownership transfer or CloseHandle is used for an SCM service handle.
struct BorrowedService<'a>(&'a Service);
impl AsRawHandle for BorrowedService<'_> {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.raw_handle().cast()
    }
}

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "native service observation refused")
}

fn remaining(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "original startup deadline expired"));
    }
    Ok(())
}

impl ServiceObservation {
    /// Exact owned name is only a locator. This does NOT authenticate its creator.
    pub(super) fn open_locator(name: &str, deadline: Instant) -> io::Result<Self> {
        remaining(deadline)?;
        crate::native_security::process::require_owned_service_name(name)?;
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .map_err(|_| refused())?;
        remaining(deadline)?;
        let service = manager.open_service(
            name,
            ServiceAccess::QUERY_STATUS | ServiceAccess::QUERY_CONFIG | ServiceAccess::READ_CONTROL,
        ).map_err(|_| refused())?;
        remaining(deadline)?;
        Ok(Self { service, deadline })
    }

    /// RUNNING is native TransportPrepared only. The application remains closed.
    /// This profile refuses all other states, including pending-state PID values.
    pub(super) fn running_pid_locator(&self) -> io::Result<u32> {
        remaining(self.deadline)?;
        let status = self.service.query_status().map_err(|_| refused())?;
        remaining(self.deadline)?;
        if status.current_state != ServiceState::Running
            || status.service_type != ServiceType::OWN_PROCESS
        {
            return Err(refused());
        }
        status.process_id.filter(|pid| *pid != 0).ok_or_else(refused)
    }

    /// Returns untrusted DATA for independent exact BASE-plan/argv comparison.
    /// The dependency's executable_path contains SCM command-line configuration;
    /// it must not be interpreted as an already verified bare executable path.
    pub(super) fn configuration_data(&self) -> io::Result<ServiceConfig> {
        remaining(self.deadline)?;
        let config = self.service.query_config().map_err(|_| refused())?;
        remaining(self.deadline)?;
        Ok(config)
    }

    /// Actual same-object descriptor, still requiring explicit installer policy.
    /// Neither descriptor possession nor configured account grants provenance.
    pub(super) fn descriptor_data(&self) -> io::Result<LocalBox<SecurityDescriptor>> {
        remaining(self.deadline)?;
        let descriptor = wrappers::GetSecurityInfo(
            &BorrowedService(&self.service),
            SeObjectType::SE_SERVICE,
            SecurityInformation::Owner | SecurityInformation::Dacl,
        ).map_err(|_| refused())?;
        remaining(self.deadline)?;
        Ok(descriptor)
    }
}
