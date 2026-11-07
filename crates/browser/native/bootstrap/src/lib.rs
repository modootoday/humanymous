//! Minimal Windows M138+ bootstrap client. UI DLLs never load in subprocesses.
#![deny(unsafe_code)]
#![cfg(windows)]
#[cfg(all(test, feature = "native-pilot-tests"))]
mod negative_pilot;
#[path = "../../shared/scheme.rs"]
mod owner_scheme;
mod renderer_probe;
/// Upstream bootstrapc.exe owns argc/argv and the sandbox pointer for this call.
///
/// # Safety
/// Called only by the matching pinned upstream bootstrap. Arguments and sandbox
/// info must satisfy CEF's native ABI and remain live until this function returns.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn RunConsoleMain(
    _argc: i32,
    _argv: *const *const u8,
    sandbox_info: *mut u8,
    _version_info: *mut std::ffi::c_void,
) -> i32 {
    std::panic::catch_unwind(|| run(sandbox_info)).unwrap_or(101)
}

fn run(sandbox_info: *mut u8) -> i32 {
    if sandbox_info.is_null() {
        return 78;
    }
    if std::env::args().any(|arg| forbidden_switch(&arg)) {
        return 78;
    }
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = cef::args::Args::new();
    // Must precede all application IO and GUI loading in sandboxed subprocesses.
    let mut app = renderer_probe::app();
    let exit = cef::execute_process(Some(args.as_main_args()), Some(&mut app), sandbox_info);
    if exit >= 0 {
        return exit;
    }
    if std::env::args().any(|arg| arg.starts_with("--disable-features")) {
        return 78;
    }
    match load_desktop(sandbox_info) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("native bootstrap unavailable: {error}");
            78
        }
    }
}

fn forbidden_switch(arg: &str) -> bool {
    [
        "--no-sandbox",
        "--disable-web-security",
        "--disable-site-isolation-trials",
        "--single-process",
        "--in-process-gpu",
        "--remote-debugging-port",
        "--remote-debugging-pipe",
        "--allow-file-access-from-files",
        "--disable-gpu-sandbox",
        "--disable-seccomp-filter-sandbox",
        "--disable-setuid-sandbox",
        "--no-zygote-sandbox",
    ]
    .iter()
    .any(|flag| arg == *flag || arg.starts_with(&format!("{flag}=")))
}

#[allow(unsafe_code)]
fn load_desktop(sandbox_info: *mut u8) -> Result<i32, Box<dyn std::error::Error>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or("missing executable directory")?
        .join("humanymous_cef_desktop.dll");
    // SAFETY: fixed sibling in the private pilot bundle; only reached after CEF
    // identifies this as the browser process. Keep the library alive through all
    // UI callbacks and CEF shutdown. No path comes from target content.
    unsafe {
        use libloading::os::windows::{
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, Library,
        };
        let library = Library::load_with_flags(
            path,
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        )?;
        let entry = library.get::<unsafe extern "C" fn(*mut u8) -> i32>(b"RunBrowserDesktop\0")?;
        Ok(entry(sandbox_info))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "native-pilot-tests")]
    fn native_bootstrap_rejects_security_overrides_before_ui_or_profile_creation() {
        super::negative_pilot::run();
    }

    #[test]
    fn security_weakening_switches_are_denied_with_or_without_values() {
        for arg in [
            "--no-sandbox",
            "--no-sandbox=0",
            "--disable-web-security",
            "--single-process",
            "--remote-debugging-port=9222",
            "--remote-debugging-pipe",
        ] {
            assert!(super::forbidden_switch(arg));
        }
        for arg in [
            "--type=renderer",
            "--module=humanymous_cef_bootstrap",
            "--lang=en-US",
        ] {
            assert!(!super::forbidden_switch(arg));
        }
    }
}
