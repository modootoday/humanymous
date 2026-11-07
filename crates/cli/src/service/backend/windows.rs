use super::*;
use std::ffi::{OsStr, OsString};
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceSidType, ServiceStartType,
        ServiceState, ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn io(error: windows_service::Error) -> std::io::Error {
    std::io::Error::other(format!("native SCM operation failed: {error}"))
}
fn manager(create: bool) -> Result<ServiceManager> {
    ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT
            | if create {
                ServiceManagerAccess::CREATE_SERVICE
            } else {
                ServiceManagerAccess::empty()
            },
    )
    .map_err(io)
}
fn open(d: &Descriptor, rights: ServiceAccess) -> Result<Service> {
    manager(false)?
        .open_service(&d.service_name, rights)
        .map_err(io)
}
fn fingerprint(service: &Service) -> Result<String> {
    Ok(crate::release::digest(
        format!("{:?}", service.query_config().map_err(io)?).as_bytes(),
    ))
}

impl Manager for Native {
    fn absent(&self, d: &Descriptor) -> Result<()> {
        match manager(false)?.open_service(&d.service_name, ServiceAccess::QUERY_CONFIG) {
            Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1060) => Ok(()),
            Err(e) => Err(io(e)),
            Ok(_) => Err(error("SCM service already exists")),
        }
    }
    fn register(&self, g: &Generation, root: &Path, d: &Descriptor) -> Result<Identity> {
        self.absent(d)?;
        let service = manager(true)?
            .create_service(
                &ServiceInfo {
                    name: d.service_name.clone().into(),
                    display_name: d.service_name.clone().into(),
                    service_type: ServiceType::OWN_PROCESS,
                    start_type: ServiceStartType::OnDemand,
                    error_control: ServiceErrorControl::Normal,
                    executable_path: g.executable(root),
                    launch_arguments: d.arguments.iter().map(OsString::from).collect(),
                    dependencies: vec![],
                    account_name: Some(OsString::from(format!("NT SERVICE\\{}", d.service_name))),
                    account_password: None,
                },
                ServiceAccess::QUERY_CONFIG | ServiceAccess::CHANGE_CONFIG | ServiceAccess::DELETE,
            )
            .map_err(io)?;
        let result = (|| {
            service
                .set_config_service_sid_info(ServiceSidType::Unrestricted)
                .map_err(io)?;
            let sid = humanymous_runtime::native_security::storage::resolve_service_sid(&d.service_name)?;
            Ok(Identity {
                configuration: fingerprint(&service)?,
                service_sid: Some(sid),
            })
        })();
        if let Err(ref cause) = result {
            service
                .delete()
                .map_err(|e| error(format!("{cause}; SCM create cleanup failed: {e}")))?;
        }
        result
    }
    fn verify(&self, g: &Generation, _root: &Path, d: &Descriptor) -> Result<()> {
        let service = open(d, ServiceAccess::QUERY_CONFIG)?;
        let config = service.query_config().map_err(io)?;
        let expected = OsString::from(format!("NT SERVICE\\{}", d.service_name));
        let observed_fingerprint = fingerprint(&service)?;
        let observed_sid =
            humanymous_runtime::native_security::storage::resolve_service_sid(&d.service_name)?;
        if config.account_name.as_deref() != Some(expected.as_os_str())
            || config.service_type != ServiceType::OWN_PROCESS
            || config.start_type != ServiceStartType::OnDemand
            || g.identity.as_ref().is_none_or(|i| {
                i.configuration != observed_fingerprint
                    || i.service_sid.as_deref() != Some(observed_sid.as_str())
            })
        {
            return Err(error("SCM registration/account changed"));
        }
        Ok(())
    }
    fn start(&self, d: &Descriptor) -> Result<()> {
        open(d, ServiceAccess::START)?
            .start::<&OsStr>(&[])
            .map_err(io)
    }
    fn running(&self, d: &Descriptor) -> Result<bool> {
        Ok(open(d, ServiceAccess::QUERY_STATUS)?
            .query_status()
            .map_err(io)?
            .current_state
            == ServiceState::Running)
    }
    fn unregister(&self, d: &Descriptor) -> Result<()> {
        let service = open(
            d,
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
        )?;
        if service.query_status().map_err(io)?.current_state != ServiceState::Stopped {
            service.stop().map_err(io)?;
            let until = Instant::now() + Duration::from_secs(6);
            while service.query_status().map_err(io)?.current_state != ServiceState::Stopped {
                if Instant::now() >= until {
                    return Err(error("SCM graceful stop deadline"));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        service.delete().map_err(io)?;
        drop(service);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if self.absent(d).is_ok() {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err(error("SCM deletion pending; recovery required"));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
