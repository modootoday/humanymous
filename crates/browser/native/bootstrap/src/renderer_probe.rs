//! Bounded opt-in renderer canary. Never accesses personal paths or reports data.
use cef::*;

wrap_app! {
    struct ProbeApp;
    impl App {
        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) { crate::owner_scheme::register(registrar); }
        fn render_process_handler(&self) -> Option<RenderProcessHandler> { Some(ProbeRenderer::new()) }
    }
}
pub fn app() -> App {
    ProbeApp::new()
}

wrap_render_process_handler! {
    struct ProbeRenderer;
    impl RenderProcessHandler {
        fn on_browser_created(&self, browser: Option<&mut Browser>, extra_info: Option<&mut DictionaryValue>) {
            if !crate::owner_scheme::registered() { return; }
            let Some(info) = extra_info else { return; };
            let owner = info.int(Some(&CefString::from("humanymous_probe_owner")));
            if owner <= 0 { return; }
            let Some(browser) = browser else { return; };
            let Some(frame) = browser.main_frame() else { return; };
            let Ok(executable) = std::env::current_exe() else { return; };
            let Some(bundle) = executable.parent() else { return; };
            let read = std::fs::read(bundle.join(format!("sandbox-canary-{owner}.txt")));
            let read_status = status(read);
            let write_status = status(std::fs::OpenOptions::new().write(true).create_new(true)
                .open(bundle.join(format!("sandbox-renderer-write-{}.tmp", std::process::id()))));
            let Some(mut message) = process_message_create(Some(&CefString::from("humanymous.sandbox.v1"))) else { return; };
            let Some(args) = message.argument_list() else { return; };
            args.set_int(0, std::process::id() as i32);
            args.set_int(1, read_status);
            args.set_int(2, write_status);
            frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
        }
    }
}
fn status<T>(result: std::io::Result<T>) -> i32 {
    match result {
        Ok(_) => 0,
        Err(error) => error.raw_os_error().unwrap_or(-1),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn errors_are_not_promoted_to_denial() {
        assert_eq!(super::status(Ok(())), 0);
        assert_eq!(
            super::status::<()>(Err(std::io::Error::from_raw_os_error(5))),
            5
        );
        assert_eq!(
            super::status::<()>(Err(std::io::Error::from_raw_os_error(2))),
            2
        );
    }
}
