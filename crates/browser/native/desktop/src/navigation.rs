//! CEF adapter for the single pure navigation authority and bounded receipts.
use crate::child::Shared;
use cef::*;
use humanymous_browser::{
    internal_urls::{self, InternalPage, Route},
    navigation::{Navigation, Outcome, ProbeLedger, Ticket},
};
use std::io;

#[derive(Default)]
pub struct State {
    pub transaction: Navigation,
    pub notice: Option<&'static str>,
    pub receipts: Vec<Receipt>,
    pub receipts_dropped: u64,
    pub probe: Option<ProbeLedger>,
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
    fn record(
        &mut self,
        ticket: Ticket,
        phase: &'static str,
        code: i32,
        bytes: usize,
        identity: bool,
    ) {
        if self.receipts.len() < 128 {
            self.receipts.push(Receipt {
                generation: ticket.generation(),
                route: ticket.route().url(),
                phase,
                code,
                bytes,
                identity,
            });
        } else {
            self.receipts_dropped = self.receipts_dropped.saturating_add(1);
        }
    }
    pub fn begin(&mut self, route: Route, phase: &'static str) -> io::Result<Ticket> {
        let superseded = self.transaction.pending();
        let ticket = self.transaction.begin(route).map_err(io::Error::other)?;
        if let Some(old) = superseded {
            self.record(old, "superseded", 0, 0, false);
        }
        self.notice = None;
        self.record(ticket, phase, 0, 0, false);
        Ok(ticket)
    }
    pub fn committed(&self) -> Option<(u64, Route)> {
        self.transaction
            .committed()
            .map(|t| (t.generation(), t.route()))
    }
    fn commit(&mut self, route: Route) {
        if let Some(ticket) = self.transaction.pending().filter(|t| t.route() == route)
            && self.transaction.observe_commit(ticket)
        {
            self.record(ticket, "committed", 0, 0, false);
        }
    }
    fn load_end(&mut self, route: Route, status: i32) -> Option<Ticket> {
        let ticket = self.transaction.pending().filter(|t| t.route() == route)?;
        if self.transaction.committed() != Some(ticket) {
            return None;
        }
        self.record(ticket, "load_end", status, 0, false);
        if status != 200 {
            self.finish(ticket, Outcome::LoadStatusRejected, status, 0);
            return None;
        }
        Some(ticket)
    }
    pub fn finish(&mut self, ticket: Ticket, outcome: Outcome, code: i32, bytes: usize) -> bool {
        if !self.transaction.finish(ticket, outcome) {
            return false;
        }
        let (phase, identity, notice) = match outcome {
            Outcome::TaskRejected => (
                "task_rejected",
                false,
                Some("Local navigation task was rejected"),
            ),
            Outcome::FrameUnavailable => (
                "frame_unavailable",
                false,
                Some("Local page frame is unavailable"),
            ),
            Outcome::LoadError => ("load_error", false, Some("Local page could not be loaded")),
            Outcome::LoadStatusRejected => (
                "load_status_rejected",
                false,
                Some("Local page load was not successful"),
            ),
            Outcome::Content { identity: true } => ("content_observed", true, None),
            Outcome::Content { identity: false } => (
                "content_observed",
                false,
                Some("Local page identity could not be verified"),
            ),
            Outcome::SourceBudgetExceeded => (
                "source_budget_exceeded",
                false,
                Some("Local page exceeds the verification budget"),
            ),
            Outcome::ProbeTimeout => (
                "probe_timeout",
                false,
                Some("Local page verification timed out"),
            ),
        };
        self.notice = notice;
        self.record(ticket, phase, code, bytes, identity);
        true
    }
    pub fn revoke(&mut self) {
        if let Some(ticket) = self.transaction.pending() {
            self.record(ticket, "revoked", 0, 0, false);
        }
        self.transaction.revoke();
    }
}
pub fn initialize(state: &Shared) -> io::Result<()> {
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("state poisoned"))?;
    if state.revoked || state.closed {
        return Err(io::Error::other("navigation unavailable"));
    }
    state
        .navigation
        .begin(Route::NewTab, "creation_requested")?;
    Ok(())
}
pub fn queue(browser: Browser, route: Route, state: Shared, reload: bool) -> io::Result<Ticket> {
    // Chrome quit/restart can act before CEF throttle callbacks; exact preflight
    // precedes creation of the browser-initiated native UI task.
    internal_urls::classify(route.url()).map_err(io::Error::other)?;
    let ticket = {
        let mut state = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?;
        if state.revoked || state.closed {
            return Err(io::Error::other("navigation unavailable"));
        }
        state.navigation.begin(route, "request_accepted")?
    };
    let mut task = Navigate::new(browser, state.clone(), ticket, reload);
    if post_task(ThreadId::UI, Some(&mut task)) != 1 {
        if let Ok(mut state) = state.lock()
            && !state.revoked
            && !state.closed
        {
            state.navigation.finish(ticket, Outcome::TaskRejected, 0, 0);
        }
        return Err(io::Error::other("navigation UI task rejected"));
    }
    Ok(ticket)
}
wrap_task! {
    struct Navigate { browser:Browser,state:Shared,ticket:Ticket,reload:bool, }
    impl Task {
        fn execute(&self) {
            if !self.state.lock().is_ok_and(|s|!s.revoked && !s.closed && s.navigation.transaction.pending()==Some(self.ticket)) { return; }
            let Some(frame)=self.browser.main_frame() else {
                if let Ok(mut state)=self.state.lock() && !state.revoked && !state.closed
                    { state.navigation.finish(self.ticket,Outcome::FrameUnavailable,0,0); }
                return;
            };
            if self.reload && internal_urls::classify(&CefString::from(&frame.url()).to_string())==Ok(self.ticket.route()) { self.browser.reload(); }
            else { frame.load_url(Some(&CefString::from(self.ticket.route().url()))); }
        }
    }
}
pub fn handler(state: Shared) -> LoadHandler {
    Loads::new(state)
}
wrap_load_handler! {
    struct Loads { state:Shared, }
    impl LoadHandler {
        fn on_load_start(&self,_browser:Option<&mut Browser>,frame:Option<&mut Frame>,_transition:TransitionType) {
            let Some(frame)=frame.filter(|f|f.is_main()==1) else { return; };
            let Ok(route)=internal_urls::classify(&CefString::from(&frame.url()).to_string()) else { return; };
            if let Ok(mut state)=self.state.lock() && !state.revoked && !state.closed { state.navigation.commit(route); }
        }
        fn on_load_end(&self,_browser:Option<&mut Browser>,frame:Option<&mut Frame>,status:i32) {
            let Some(frame)=frame.filter(|f|f.is_main()==1) else { return; };
            let Ok(route)=internal_urls::classify(&CefString::from(&frame.url()).to_string()) else { return; };
            let ticket=if let Ok(mut state)=self.state.lock() {
                if state.revoked || state.closed { return; }
                let Some(ticket)=state.navigation.load_end(route,status) else { return; }; ticket
            } else { return; };
            let mut visitor=Source::new(self.state.clone(),ticket); frame.source(Some(&mut visitor));
        }
        fn on_load_error(&self,_browser:Option<&mut Browser>,frame:Option<&mut Frame>,error:Errorcode,_text:Option<&CefString>,url:Option<&CefString>) {
            if frame.is_none_or(|f|f.is_main()!=1) { return; }
            let Some(url)=url else { return; };
            let Ok(route)=internal_urls::classify(&url.to_string()) else { return; };
            if let Ok(mut state)=self.state.lock() && !state.revoked && !state.closed
                && let Some(ticket)=state.navigation.transaction.pending().filter(|t|t.route()==route)
                { state.navigation.finish(ticket,Outcome::LoadError,error.get_raw(),0); }
        }
    }
}
wrap_string_visitor! {
    struct Source { state:Shared,ticket:Ticket, }
    impl CefStringVisitor {
        fn visit(&self,source:Option<&CefString>) {
            if !self.state.lock().is_ok_and(|s|!s.revoked && !s.closed && s.navigation.transaction.pending()==Some(self.ticket)) { return; }
            let Some(source)=source else { return; };
            if source.as_slice().is_some_and(|units|units.len()>8*1024*1024) {
                if let Ok(mut state)=self.state.lock() && !state.revoked && !state.closed {
                    state.navigation.finish(self.ticket,Outcome::SourceBudgetExceeded,0,0);
                }
                return;
            }
            // Raw source/paths/device data is transient; only structural identity
            // enters the bounded report. A captured ticket cannot mutate a new request.
            let source=source.to_string(); let bytes=source.len();
            let outcome=if bytes>8*1024*1024 { Outcome::SourceBudgetExceeded }
                else { Outcome::Content {identity:identity(self.ticket.route(),&source)} };
            if let Ok(mut state)=self.state.lock() && !state.revoked && !state.closed {
                state.navigation.finish(self.ticket,outcome,0,bytes);
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
                if frame.as_ref().is_none_or(|f|f.is_main()!=1) { return 1; }
                let permitted=self.state.lock().is_ok_and(|s|!s.revoked && !s.closed && s.navigation.transaction.permits_browse(route));
                if permitted { return 0; }
                // A renderer gesture gets a new host ticket; a delayed browser
                // callback cannot overwrite a newer pending request's authority.
                if gesture==1 && frame.as_ref().is_some_and(|f|internal_urls::classify(&CefString::from(&f.url()).to_string()).is_ok())
                    && let Some(browser)=browser {
                    let _=queue(browser.clone(),route,self.state.clone(),false);
                } else { denied(&self.state); }
                return 1;
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
            if let Ok(route)=internal_urls::classify(&target) {
                if gesture==1 && frame.as_ref().is_some_and(|f|f.is_main()==1 && internal_urls::classify(&CefString::from(&f.url()).to_string()).is_ok())
                    && let Some(browser)=browser {
                    let _=queue(browser.clone(),route,self.state.clone(),false);
                } else { denied(&self.state); }
                return 1;
            }
            if gesture==1 && frame.is_some_and(|f|f.is_main()==1 && CefString::from(&f.url()).to_string()==internal_urls::NEW_TAB_URL)
                && let Ok(route)=internal_urls::newtab_action(&target) && let Some(browser)=browser {
                let _=queue(browser.clone(),route,self.state.clone(),false);
            } else { denied(&self.state); }
            1
        }
        fn resource_request_handler(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, request: Option<&mut Request>, navigation: i32, download: i32, _initiator: Option<&CefString>, disable: Option<&mut i32>) -> Option<ResourceRequestHandler> {
            let snapshot=self.state.lock().ok().filter(|s|!s.revoked && !s.closed)
                .map(|s|s.navigation.committed().map(|(_,route)|route));
            let allowed=download==0 && snapshot.is_some_and(|current|request.is_some_and(|request| {
                let url=CefString::from(&request.url()).to_string();
                if !self.fixture.is_empty() && navigation==1 && url==self.fixture { return true; }
                internal_urls::resource_allowed(&url,navigation==1,current)
            }));
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
