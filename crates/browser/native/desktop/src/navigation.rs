//! Browser-thread typed navigation and bounded privacy-safe content receipts.
use crate::child::Shared;
use cef::*;
use humanymous_browser::internal_urls::{self, InternalPage, Route};
use std::io;

#[derive(Default)]
pub struct State {
    pub generation: u64,
    pub requested: Option<Route>,
    pub committed: Option<(u64, Route)>,
    pub notice: Option<&'static str>,
    pub receipts: Vec<Receipt>,
}
#[derive(Clone, serde::Serialize)]
pub struct Receipt {
    pub generation: u64,
    pub route: &'static str,
    pub phase: &'static str,
    pub code: i32,
    pub bytes: usize,
    pub identity: bool,
}
impl State {
    pub fn record(
        &mut self,
        route: Route,
        phase: &'static str,
        code: i32,
        bytes: usize,
        identity: bool,
    ) {
        if self.receipts.len() < 128 {
            self.receipts.push(Receipt {
                generation: self.generation,
                route: route.url(),
                phase,
                code,
                bytes,
                identity,
            });
        }
    }
}
pub fn initialize(state: &Shared) -> io::Result<()> {
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("state poisoned"))?;
    state.navigation.generation = 1;
    state.navigation.requested = Some(Route::NewTab);
    state
        .navigation
        .record(Route::NewTab, "creation_requested", 0, 0, false);
    Ok(())
}
pub fn queue(browser: Browser, route: Route, state: Shared, reload: bool) -> io::Result<u64> {
    // Both typed route construction and this exact preflight precede LoadURL.
    // Chrome quit/restart can be handled before CEF navigation throttle callbacks.
    internal_urls::classify(route.url()).map_err(io::Error::other)?;
    let generation = {
        let mut state = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?;
        if state.revoked || state.closed || state.navigation.generation >= 64 {
            return Err(io::Error::other("navigation unavailable/budget exhausted"));
        }
        state.navigation.generation += 1;
        state.navigation.requested = Some(route);
        state.navigation.notice = None;
        state
            .navigation
            .record(route, "request_accepted", 0, 0, false);
        state.navigation.generation
    };
    let mut task = Navigate::new(browser, route, state, generation, reload);
    if post_task(ThreadId::UI, Some(&mut task)) != 1 {
        return Err(io::Error::other("navigation UI task rejected"));
    }
    Ok(generation)
}
wrap_task! {
    struct Navigate { browser: Browser, route: Route, state: Shared, generation: u64, reload: bool, }
    impl Task {
        fn execute(&self) {
            if !self.state.lock().is_ok_and(|s| !s.revoked && !s.closed && s.navigation.generation==self.generation) { return; }
            if let Some(frame)=self.browser.main_frame() {
                if self.reload && internal_urls::classify(&CefString::from(&frame.url()).to_string())==Ok(self.route) { self.browser.reload(); }
                else { frame.load_url(Some(&CefString::from(self.route.url()))); }
            }
        }
    }
}
pub fn handler(state: Shared) -> LoadHandler {
    Loads::new(state)
}
wrap_load_handler! {
    struct Loads { state: Shared, }
    impl LoadHandler {
        fn on_load_start(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, _transition: TransitionType) {
            let Some(frame)=frame.filter(|f|f.is_main()==1) else { return; };
            let Ok(route)=internal_urls::classify(&CefString::from(&frame.url()).to_string()) else { return; };
            if let Ok(mut state)=self.state.lock() && !state.revoked && state.navigation.requested==Some(route) {
                state.navigation.committed=Some((state.navigation.generation,route));
                state.navigation.record(route,"committed",0,0,false);
            }
        }
        fn on_load_end(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, status: i32) {
            let Some(frame)=frame.filter(|f|f.is_main()==1) else { return; };
            let Ok(route)=internal_urls::classify(&CefString::from(&frame.url()).to_string()) else { return; };
            let generation=if let Ok(mut state)=self.state.lock() {
                if state.revoked || state.navigation.requested!=Some(route) { return; }
                state.navigation.record(route,"load_end",status,0,false);
                state.navigation.generation
            } else { return; };
            let mut visitor=Source::new(self.state.clone(),route,generation);
            frame.source(Some(&mut visitor));
        }
        fn on_load_error(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, error: Errorcode, _text: Option<&CefString>, url: Option<&CefString>) {
            if frame.is_none_or(|f|f.is_main()!=1) { return; }
            let Some(url)=url else { return; };
            let Ok(route)=internal_urls::classify(&url.to_string()) else { return; };
            if let Ok(mut state)=self.state.lock() && state.navigation.requested==Some(route) {
                state.navigation.record(route,"load_error",error.get_raw(),0,false);
                state.navigation.notice=Some("Local page could not be loaded");
            }
        }
    }
}
wrap_string_visitor! {
    struct Source { state: Shared, route: Route, generation: u64, }
    impl CefStringVisitor {
        fn visit(&self, source: Option<&CefString>) {
            let Some(source)=source else { return; };
            // No source/paths/device data is retained. Only bounded structural
            // identity escapes this callback. Upstream strings are transient.
            if source.as_slice().is_some_and(|units|units.len()>8*1024*1024) {
                if let Ok(mut state)=self.state.lock() && !state.revoked && state.navigation.generation==self.generation {
                    state.navigation.record(self.route,"source_budget_exceeded",0,0,false);
                }
                return;
            }
            let source=source.to_string(); let bytes=source.len();
            let (phase,identity)=if bytes>8*1024*1024 { ("source_budget_exceeded",false) }
                else { let identity=identity(self.route,&source); ("content_observed",identity) };
            if let Ok(mut state)=self.state.lock() && !state.revoked && state.navigation.generation==self.generation {
                state.navigation.record(self.route,phase,0,bytes,identity);
            }
        }
    }
}
fn identity(route: Route, source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    if lower.contains("chromewebdata")
        || lower.contains("err_unknown_url_scheme")
        || lower.contains("err_invalid_url")
    {
        return false;
    }
    match route {
        Route::NewTab => {
            source.contains("Start with a clear view.")
                && source.contains("/open/version")
                && !lower.contains("<script")
        }
        Route::Internal(InternalPage::Version) => {
            lower.contains("version")
                && (lower.contains("about_version") || lower.contains("version-ui"))
        }
        Route::Internal(InternalPage::Gpu) => {
            lower.contains("gpu")
                && (lower.contains("gpu_internals") || lower.contains("info-view"))
        }
        Route::Internal(InternalPage::Credits) => {
            lower.contains("credits") && (lower.contains("chromium") || lower.contains("license"))
        }
        Route::Internal(InternalPage::License) => {
            lower.contains("license") && (lower.contains("chromium") || lower.contains("copyright"))
        }
        Route::Internal(InternalPage::Sandbox) => {
            lower.contains("sandbox")
                && (lower.contains("sandbox_internals") || lower.contains("sandbox-status"))
        }
    }
}

pub fn request_handler(state: Shared, fixture: String) -> RequestHandler {
    Guard::new(state, fixture)
}
fn denied(state: &Shared) {
    if let Ok(mut state) = state.lock() {
        state.navigation.notice = Some("Only local newtab and permitted diagnostics are available");
    }
}
wrap_request_handler! {
    struct Guard { state: Shared, fixture: String, }
    impl RequestHandler {
        fn on_before_browse(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, request: Option<&mut Request>, gesture: i32, _redirect: i32) -> i32 {
            if self.state.lock().map_or(true,|s|s.revoked || s.closed) { return 1; }
            let Some(request)=request else { return 1; };
            let url=CefString::from(&request.url()).to_string();
            if !self.fixture.is_empty() && (url==self.fixture || url=="about:blank") { return 0; }
            if let Ok(route)=internal_urls::classify(&url) {
                if frame.is_none_or(|f|f.is_main()!=1) { return 1; }
                if let Ok(mut state)=self.state.lock() && state.navigation.requested!=Some(route) {
                    if state.navigation.generation>=64 { return 1; }
                    state.navigation.generation+=1; state.navigation.requested=Some(route);
                    state.navigation.record(route,"renderer_request_accepted",0,0,false);
                }
                return 0;
            }
            if gesture==1 && frame.as_ref().is_some_and(|f|f.is_main()==1 && CefString::from(&f.url()).to_string()==internal_urls::NEW_TAB_URL)
                && let Ok(route)=internal_urls::newtab_action(&url) && let Some(browser)=browser {
                let _=queue(browser.clone(),route,self.state.clone(),false); return 1;
            }
            denied(&self.state); 1
        }
        fn on_open_urlfrom_tab(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, target: Option<&CefString>, disposition: WindowOpenDisposition, gesture: i32) -> i32 {
            if self.state.lock().map_or(true,|s|s.revoked || s.closed) || disposition!=WindowOpenDisposition::CURRENT_TAB { return 1; }
            let Some(target)=target else { return 1; }; let target=target.to_string();
            if internal_urls::classify(&target).is_ok() { return 0; }
            if gesture==1 && frame.is_some_and(|f|f.is_main()==1 && CefString::from(&f.url()).to_string()==internal_urls::NEW_TAB_URL)
                && let Ok(route)=internal_urls::newtab_action(&target) && let Some(browser)=browser {
                let _=queue(browser.clone(),route,self.state.clone(),false);
            } else { denied(&self.state); }
            1
        }
        fn resource_request_handler(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, request: Option<&mut Request>, navigation: i32, download: i32, _initiator: Option<&CefString>, disable: Option<&mut i32>) -> Option<ResourceRequestHandler> {
            let active=self.state.lock().is_ok_and(|s|!s.revoked && !s.closed);
            let allowed=active && download==0 && request.is_some_and(|request| {
                let url=CefString::from(&request.url()).to_string();
                if !self.fixture.is_empty() && navigation==1 && url==self.fixture { return true; }
                let current=self.state.lock().ok().and_then(|s|s.navigation.committed.map(|(_,route)|route));
                internal_urls::resource_allowed(&url,navigation==1,current)
            });
            if allowed { return None; }
            if let Some(disable)=disable { *disable=1; }
            Some(BlockResource::new())
        }
    }
}
wrap_resource_request_handler! {
    struct BlockResource;
    impl ResourceRequestHandler {
        fn on_before_resource_load(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _request: Option<&mut Request>, _callback: Option<&mut Callback>) -> ReturnValue { ReturnValue::CANCEL }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commit_or_error_markup_is_not_rendering_support() {
        assert!(!identity(
            Route::Internal(InternalPage::Gpu),
            "<title>chrome://gpu</title>ERR_INVALID_URL"
        ));
        assert!(!identity(
            Route::NewTab,
            "Start with a clear view.<script>bad</script>"
        ));
        assert!(identity(
            Route::NewTab,
            &humanymous_browser::newtab::markup().unwrap()
        ));
    }
}
