//! Opt-in, non-product owned service for native lifecycle qualification.
#[cfg(windows)]
use std::sync::Arc;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

fn configuration() -> std::io::Result<(String, SocketAddr, String, bool)> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(3..=4).contains(&args.len())
        || !args[0].starts_with("humanymous-lab-")
        || args[2].len() > 512
    {
        return Err(std::io::Error::other(
            "owned fixture name/address/response required",
        ));
    }
    let address: SocketAddr = args[1]
        .parse()
        .map_err(|_| std::io::Error::other("fixture socket address"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(std::io::Error::other("loopback fixture only"));
    }
    let scm = args.get(3).is_some_and(|s| s == "--scm");
    if args.len() == 4 && !scm {
        return Err(std::io::Error::other("unsupported fixture option"));
    }
    Ok((args[0].clone(), address, args[2].clone(), scm))
}
fn listen(address: SocketAddr) -> std::io::Result<TcpListener> {
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}
fn serve(listener: TcpListener, response: &str, stop: &AtomicBool) -> std::io::Result<()> {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                stream.set_read_timeout(Some(Duration::from_millis(200)))?;
                stream.set_write_timeout(Some(Duration::from_millis(200)))?;
                let mut input = [0; 64];
                let expected = b"HUMANYMOUS-READY/1\n";
                if stream.read_exact(&mut input[..expected.len()]).is_ok()
                    && &input[..expected.len()] == expected
                {
                    let _ = stream.write_all(response.as_bytes());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(windows)]
mod scm {
    use super::*;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };
    define_windows_service!(entry, service_main);
    pub(super) fn dispatch(name: &str) -> std::io::Result<()> {
        service_dispatcher::start(name, entry).map_err(std::io::Error::other)
    }
    fn service_main(_: Vec<std::ffi::OsString>) {
        let _ = run();
    }
    fn run() -> std::io::Result<()> {
        let (name, address, response, _) = configuration()?;
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let handle = service_control_handler::register(name, move |control| match control {
            ServiceControl::Stop => {
                signal.store(true, Ordering::Relaxed);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })
        .map_err(std::io::Error::other)?;
        let status = |state, code| ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: if state == ServiceState::Running {
                ServiceControlAccept::STOP
            } else {
                ServiceControlAccept::empty()
            },
            exit_code: ServiceExitCode::Win32(code),
            checkpoint: 0,
            wait_hint: Duration::from_secs(2),
            process_id: None,
        };
        handle
            .set_service_status(status(ServiceState::StartPending, 0))
            .map_err(std::io::Error::other)?;
        let result = (|| {
            let listener = listen(address)?;
            handle
                .set_service_status(status(ServiceState::Running, 0))
                .map_err(std::io::Error::other)?;
            serve(listener, &response, &stop)
        })();
        handle
            .set_service_status(status(
                ServiceState::Stopped,
                if result.is_ok() { 0 } else { 1 },
            ))
            .map_err(std::io::Error::other)?;
        result
    }
}

fn run() -> std::io::Result<()> {
    #[cfg(feature = "runtime-service-pilot")]
    {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        if args
            .first()
            .is_some_and(|value| value == "--watchdog-boundary")
        {
            if args.len() != 2 {
                return Err(std::io::Error::other(
                    "exact watchdog boundary case required",
                ));
            }
            return humanymous_cli::runtime_host::watchdog_fixture::run(&args[1]);
        }
    }
    let (name, address, response, scm) = configuration()?;
    #[cfg(windows)]
    if scm {
        return scm::dispatch(&name);
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        if scm {
            return Err(std::io::Error::other("SCM unsupported on this OS"));
        }
    }
    serve(listen(address)?, &response, &AtomicBool::new(false))
}
fn main() {
    if let Err(e) = run() {
        eprintln!("fixture failed: {e}");
        std::process::exit(1);
    }
}
