//! Identical owner-scheme registration in browser and every CEF subprocess App.
use cef::*;
use std::sync::atomic::{AtomicBool, Ordering};

// Each application DLL/process owns its observation; this is not cross-process
// attestation. Never initialize the observation to an assumed success.
static ACCEPTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn registered() -> bool {
    ACCEPTED.load(Ordering::Acquire)
}

pub(crate) fn register(registrar: Option<&mut SchemeRegistrar>) {
    let accepted = registrar.is_some_and(|registrar| {
        let options = SchemeOptions::STANDARD.get_raw()
            | SchemeOptions::SECURE.get_raw()
            | SchemeOptions::DISPLAY_ISOLATED.get_raw();
        registrar.add_custom_scheme(Some(&CefString::from("humanymous")), options) == 1
    });
    ACCEPTED.store(accepted, Ordering::Release);
    if !accepted {
        eprintln!("native: owner scheme registration missing or rejected");
    }
}
