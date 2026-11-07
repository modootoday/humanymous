//! Identical owner-scheme registration in browser and every CEF subprocess App.
use cef::*;

pub(crate) fn register(registrar: Option<&mut SchemeRegistrar>) {
    if let Some(registrar) = registrar {
        let options = SchemeOptions::STANDARD.get_raw()
            | SchemeOptions::SECURE.get_raw()
            | SchemeOptions::DISPLAY_ISOLATED.get_raw();
        if registrar.add_custom_scheme(Some(&CefString::from("humanymous")), options) != 1 {
            // Registration failure must never be reported as an admitted page.
            eprintln!("native: owner scheme registration failed");
        }
    }
}
