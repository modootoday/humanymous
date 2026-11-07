//! OS control adapters. No service registration, privilege or account mutation.
#[cfg(windows)]
use super::HostMode;
use super::{HostPlan, NativeSecrets, invalid};
#[cfg(not(windows))]
use super::{HostReadiness, control::StopHandle};
use std::io;
#[cfg(not(windows))]
use std::io::Write;

pub(super) fn execute(plan: HostPlan) -> io::Result<()> {
    #[cfg(windows)]
    {
        if plan.mode == HostMode::Native {
            return windows::dispatch(plan);
        }
        return Err(invalid(
            "protected Windows service input requires the native SCM dispatcher",
        ));
    }
    #[cfg(not(windows))]
    foreground(plan)
}
#[cfg(not(windows))]
fn output(value: &impl serde::Serialize) -> io::Result<()> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value)
        .map_err(|_| invalid("non-secret lifecycle report"))?;
    output.write_all(b"\n")?;
    output.flush()
}
#[cfg(not(windows))]
fn foreground(plan: HostPlan) -> io::Result<()> {
    let generation = plan.generation.clone();
    let executable = plan.executable_sha256.clone();
    let (supervisor, shutdown) = super::start(&plan)?;
    let stop = supervisor.stop_handle();
    let runtime = super::runtime()?;
    runtime.block_on(async move {
        let signal = signals(stop)?;
        let result = super::run(
            plan,
            &NativeSecrets,
            None,
            None,
            &supervisor,
            shutdown,
            output::<HostReadiness>,
        )
        .await;
        signal.abort();
        let _ = signal.await;
        match result {
            Ok(report) => {
                supervisor.stopped(&report.report())?;
                output(&report)
            }
            Err(error) => {
                let _ = output(&super::describe_failure(&error, &generation, &executable));
                Err(error)
            }
        }
    })
}
#[cfg(unix)]
fn signals(stop: StopHandle) -> io::Result<tokio::task::JoinHandle<()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    Ok(tokio::spawn(async move {
        tokio::select! {_=terminate.recv()=>{},_=interrupt.recv()=>{}}
        let _ = stop.request();
    }))
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        sync::{Arc, Mutex, OnceLock},
        time::Duration,
    };
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };
    static PLAN: OnceLock<Mutex<Option<HostPlan>>> = OnceLock::new();
    define_windows_service!(entry, service_main);
    pub(super) fn dispatch(plan: HostPlan) -> io::Result<()> {
        let name = plan.service_name.clone();
        PLAN.set(Mutex::new(Some(plan)))
            .map_err(|_| invalid("SCM dispatcher already initialized"))?;
        service_dispatcher::start(name, entry).map_err(io::Error::other)
    }
    fn status(state: ServiceState, exit: u32, wait: Duration) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: if state == ServiceState::Running {
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
            } else {
                ServiceControlAccept::empty()
            },
            exit_code: ServiceExitCode::Win32(exit),
            checkpoint: 0,
            wait_hint: wait,
            process_id: None,
        }
    }
    fn service_main(_: Vec<std::ffi::OsString>) {
        let _ = run_service();
    }
    fn run_service() -> io::Result<()> {
        let plan = PLAN
            .get()
            .ok_or_else(|| invalid("SCM host plan unavailable"))?
            .lock()
            .map_err(|_| invalid("SCM plan poisoned"))?
            .take()
            .ok_or_else(|| invalid("SCM host plan already consumed"))?;
        let (supervisor, shutdown) = super::super::start(&plan)?;
        let stop = supervisor.stop_handle();
        let wait = plan.stop.budget()?.manager_stop();
        let published = Arc::new(OnceLock::<service_control_handler::ServiceStatusHandle>::new());
        let observer = published.clone();
        let handle =
            service_control_handler::register(&plan.service_name, move |control| match control {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    if let Some(handle) = observer.get() {
                        let _ =
                            handle.set_service_status(status(ServiceState::StopPending, 0, wait));
                    }
                    let _ = stop.request();
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            })
            .map_err(io::Error::other)?;
        published
            .set(handle)
            .map_err(|_| invalid("SCM status handle already set"))?;
        let handle = published
            .get()
            .ok_or_else(|| invalid("SCM status handle unavailable"))?;
        handle
            .set_service_status(status(
                ServiceState::StartPending,
                0,
                Duration::from_millis(plan.startup_ms),
            ))
            .map_err(io::Error::other)?;
        #[cfg(feature = "runtime-service-pilot")]
        let mut plan = plan;
        #[cfg(feature = "runtime-service-pilot")]
        let mut pilot = plan
            .pilot
            .take()
            .map(|metadata| super::super::pilot::Session::new(metadata, &plan));
        let ready = std::cell::Cell::new(false);
        let result = (|| {
            #[cfg(feature = "runtime-service-pilot")]
            if let Some(pilot) = &mut pilot {
                pilot.prepare(&mut plan)?;
            }
            super::super::runtime()?.block_on(super::super::run(
                plan,
                &NativeSecrets,
                None,
                None,
                &supervisor,
                shutdown,
                |_| {
                    ready.set(true);
                    handle
                        .set_service_status(status(ServiceState::Running, 0, Duration::ZERO))
                        .map_err(io::Error::other)
                },
            ))
        })();
        #[cfg(feature = "runtime-service-pilot")]
        let result = match &mut pilot {
            Some(pilot) => pilot.finish(result, ready.get()),
            None => result,
        };
        if let Ok(report) = &result {
            supervisor.stopped(&report.report())?;
        }
        let exit = if result.is_ok() { 0 } else { 1 };
        handle
            .set_service_status(status(ServiceState::Stopped, exit, wait))
            .map_err(io::Error::other)?;
        result.map(|_| ())
    }
}
