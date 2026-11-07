//! Experimental sandbox-required CEF child, owned by a real Meari window.
#![deny(unsafe_code)]
#![cfg(windows)]
mod child;
mod fixture;
mod host;
mod measured_input;
mod native;
use cef::*;

wrap_app! {
    struct PilotApp;
    impl App {
        fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            if let Some(line) = command_line {
                for switch in ["disable-extensions", "disable-background-networking", "disable-component-update", "no-first-run"] {
                    line.append_switch(Some(&CefString::from(switch)));
                }
            }
        }
    }
}
/// Browser-only ABI, called by the minimal pinned-bootstrap client DLL.
///
/// # Safety
/// The sandbox pointer is borrowed from the upstream executable and must remain
/// valid through initialize/shutdown. This function runs once on the main thread.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn RunBrowserDesktop(sandbox_info: *mut u8) -> i32 {
    match std::panic::catch_unwind(|| run(sandbox_info)) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("CEF native pilot: {error}");
            std::process::exit(78)
        }
        Err(_) => std::process::exit(101),
    }
}

fn run(sandbox_info: *mut u8) -> Result<(), Box<dyn std::error::Error>> {
    if sandbox_info.is_null() {
        return Err("sandbox bootstrap pointer absent".into());
    }
    let args = cef::args::Args::new();
    let bundle = std::env::current_exe()?
        .parent()
        .ok_or("missing bundle")?
        .to_owned();
    // Never reuse an existing directory or read an installed/personal profile.
    let profile = bundle.join(format!("synthetic-profile-{}", std::process::id()));
    std::fs::create_dir(&profile)?;
    let settings = cef::Settings {
        no_sandbox: 0,
        multi_threaded_message_loop: 1,
        root_cache_path: cef::CefString::from(profile.to_string_lossy().as_ref()),
        cache_path: cef::CefString::from(profile.join("cache").to_string_lossy().as_ref()),
        log_file: cef::CefString::from(
            bundle
                .join(format!("cef-debug-{}.log", std::process::id()))
                .to_string_lossy()
                .as_ref(),
        ),
        ..Default::default()
    };
    let mut app = PilotApp::new();
    if cef::initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        sandbox_info,
    ) != 1
    {
        return Err("sandbox-required CEF initialization failed".into());
    }
    let outcome = host::run();
    // No shutdown while a browser remains live: host::run only succeeds after
    // actual on_before_close, and errors are fail-stop for this experimental DLL.
    match outcome {
        Ok(()) => {
            cef::shutdown();
            Ok(())
        }
        Err(error) => {
            eprintln!("incomplete native cleanup: {error}");
            std::process::exit(78)
        }
    }
}
