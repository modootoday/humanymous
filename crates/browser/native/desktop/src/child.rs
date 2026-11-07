use cef::*;
use meari_native::{application::window_lifecycle::*, infrastructure::window::*};
use std::{
    io,
    rc::Rc as LocalRc,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct State {
    pub browser: Option<Browser>,
    pub navigation: crate::navigation::State,
    pub created: bool,
    pub creation_requested: bool,
    pub closed: bool,
    pub revoked: bool,
    pub late_created_after_cancel: bool,
    pub signal: Option<CompletionSignal>,
    pub observer: Option<Registration>,
    pub process_roles: Vec<ProcessRole>,
    pub process_info_status: &'static str,
    pub key_events: usize,
    pub editable_key_events: usize,
    pub focus_events: usize,
    pub file_probes: Vec<FileProbe>,
    pub ime_setup: Option<String>,
    pub ime_candidate_request: Option<String>,
    pub measured_hanja: Vec<String>,
    pub hanja_key_events: usize,
}
pub type Shared = Arc<Mutex<State>>;

#[derive(Clone, serde::Serialize)]
pub struct FileProbe {
    pub pid: u32,
    pub read_os_error: i32,
    pub create_os_error: i32,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct ProcessRole {
    pub id: u32,
    #[serde(rename = "type")]
    pub kind: String,
}

wrap_dev_tools_message_observer! {
    struct ProcessObserver { state: Shared, }
    impl DevToolsMessageObserver {
        fn on_dev_tools_method_result(&self, _browser: Option<&mut Browser>, message_id: i32, success: i32, result: Option<&[u8]>) {
            if message_id != 7 { return; }
            if success != 1 {
                if let Ok(mut state) = self.state.lock() { state.process_info_status = "unsupported_or_failed"; }
                return;
            }
            let Some(bytes) = result.filter(|b| b.len() <= 65536) else { return; };
            #[derive(serde::Deserialize)] struct Info { #[serde(rename="processInfo")] processes: Vec<ProcessRole> }
            if let Ok(info) = serde_json::from_slice::<Info>(bytes)
                && info.processes.len() <= 64 && info.processes.iter().all(|p| p.kind.len() <= 64)
                && let Ok(mut state) = self.state.lock() {
                state.process_roles = info.processes; state.process_info_status = "observed";
            }
        }
    }
}
wrap_task! {
    struct BrowserAction { browser: Browser, close: bool, state: Shared, }
    impl Task {
        fn execute(&self) {
            eprintln!("native: CEF UI action close={}", self.close);
            let Some(host) = self.browser.host() else { return; };
            if self.close { host.close_browser(1); eprintln!("native: CEF close request delivered"); }
            else {
                host.set_accessibility_state(cef::State::ENABLED);
                host.set_focus(1);
                if std::env::args().any(|a| a == "--native-korean-ime") {
                    let outcome = crate::native::prepare_korean_ime(&self.browser)
                        .map_or_else(|error| error.to_string(), |_| "open_requested_not_qualified".into());
                    if let Ok(mut state) = self.state.lock() { state.ime_setup = Some(outcome); }
                }
                let mut observer = ProcessObserver::new(self.state.clone());
                let registration = host.add_dev_tools_message_observer(Some(&mut observer));
                if let Ok(mut state) = self.state.lock() { state.observer = registration; }
                host.execute_dev_tools_method(7, Some(&CefString::from("SystemInfo.getProcessInfo")), None);
            }
        }
    }
}
pub fn queue_action(browser: Browser, close: bool, state: Shared) -> io::Result<()> {
    let mut task = BrowserAction::new(browser, close, state);
    if post_task(ThreadId::UI, Some(&mut task)) != 1 {
        return Err(io::Error::other("CEF UI task rejected"));
    }
    Ok(())
}
wrap_task! {
    struct DestroyChild { browser: Browser, }
    impl Task {
        fn execute(&self) {
            if let Err(error) = crate::native::destroy_child(&self.browser) { eprintln!("native: child destroy failed: {error}"); }
        }
    }
}

wrap_client! {
    struct PilotClient { state: Shared, uri: String, }
    impl Client {
        fn on_process_message_received(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, source: ProcessId, message: Option<&mut ProcessMessage>) -> i32 {
            if source != ProcessId::RENDERER { return 0; }
            let Some(message) = message else { return 0; };
            if CefString::from(&message.name()).to_string() != "humanymous.sandbox.v1" { return 0; }
            let Some(args) = message.argument_list().filter(|a| a.size() == 3) else { return 1; };
            if (0..3).any(|i| args.get_type(i) != ValueType::INT) { return 1; }
            let pid = args.int(0);
            if pid <= 0 { return 1; }
            if let Ok(mut state) = self.state.lock()
                && state.file_probes.len() < 32 && !state.file_probes.iter().any(|p| p.pid == pid as u32) {
                state.file_probes.push(FileProbe { pid: pid as u32, read_os_error: args.int(1), create_os_error: args.int(2) });
            }
            1
        }
        fn load_handler(&self) -> Option<LoadHandler> { Some(crate::navigation::handler(self.state.clone())) }
        fn keyboard_handler(&self) -> Option<KeyboardHandler> { Some(InputEvidence::new(self.state.clone())) }
        fn focus_handler(&self) -> Option<FocusHandler> { Some(FocusEvidence::new(self.state.clone())) }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> { Some(PilotLife::new(self.state.clone())) }
        fn request_handler(&self) -> Option<RequestHandler> { Some(crate::navigation::request_handler(self.state.clone(), self.uri.clone())) }
        fn permission_handler(&self) -> Option<PermissionHandler> { Some(DenyPermissions::new()) }
        fn dialog_handler(&self) -> Option<DialogHandler> { Some(DenyFiles::new()) }
        fn download_handler(&self) -> Option<DownloadHandler> { Some(DenyDownloads::new()) }
    }
}
// Explicit denial also covers builtin WebUI export actions and both CEF styles.
wrap_download_handler! {
    struct DenyDownloads;
    impl DownloadHandler {
        fn can_download(&self, _browser: Option<&mut Browser>, _url: Option<&CefString>, _method: Option<&CefString>) -> i32 { 0 }
        fn on_before_download(&self, _browser: Option<&mut Browser>, _item: Option<&mut DownloadItem>, _name: Option<&CefString>, _callback: Option<&mut BeforeDownloadCallback>) -> i32 {
            // CEF: handled=true with callback released without continuation cancels.
            1
        }
        fn on_download_updated(&self, _browser: Option<&mut Browser>, _item: Option<&mut DownloadItem>, callback: Option<&mut DownloadItemCallback>) {
            if let Some(callback)=callback { callback.cancel(); }
        }
    }
}
wrap_keyboard_handler! {
    struct InputEvidence { state: Shared, }
    impl KeyboardHandler {
        fn on_pre_key_event(&self, _browser: Option<&mut Browser>, event: Option<&KeyEvent>, _os_event: Option<&mut cef::sys::MSG>, _shortcut: Option<&mut i32>) -> i32 {
            if event.is_some_and(|e| e.windows_key_code == 0x79 && e.type_ == KeyEventType::RAWKEYDOWN)
                && std::env::args().any(|a| a == "--native-measured-hanja")
                && std::env::args().any(|a| a == "--native-korean-ime") {
                if self.state.lock().map_or(true, |s| s.measured_hanja.len() >= 4) { return 1; }
                if let Some(browser) = _browser.as_ref() {
                    let result = crate::measured_input::replay_hanja(browser, event.map_or(0, |e| e.modifiers)).unwrap_or_else(|e| e.to_string());
                    eprintln!("native: measured Hanja {result}");
                    if let Ok(mut state) = self.state.lock() { state.measured_hanja.push(result); }
                }
                return 1;
            }
            if event.is_some_and(|e| e.windows_key_code == 0x78 && e.type_ == KeyEventType::RAWKEYDOWN)
                && std::env::args().any(|a| a == "--native-korean-ime")
                && let Some(browser) = _browser.as_ref() {
                let outcome = crate::native::request_ime_candidates(browser).unwrap_or_else(|error| error.to_string());
                if let Ok(mut state) = self.state.lock() { state.ime_candidate_request = Some(outcome); }
                return 1;
            }
            if event.is_some_and(|e| e.windows_key_code == 0x77 && e.type_ == KeyEventType::RAWKEYDOWN)
                && std::env::args().any(|a| a == "--native-korean-ime")
                && let Some(browser) = _browser {
                let outcome = crate::native::prepare_korean_ime(browser)
                    .map_or_else(|error| error.to_string(), |_| "focused_context_open_requested_not_qualified".into());
                if let Ok(mut state) = self.state.lock() { state.ime_setup = Some(outcome); }
                return 1;
            }
            if let Ok(mut state) = self.state.lock() {
                state.hanja_key_events += usize::from(event.is_some_and(|e| e.windows_key_code == 0x19));
                state.key_events += 1;
                state.editable_key_events += usize::from(event.is_some_and(|e| e.focus_on_editable_field != 0));
            }
            0
        }
    }
}
wrap_focus_handler! {
    struct FocusEvidence { state: Shared, }
    impl FocusHandler {
        fn on_got_focus(&self, _browser: Option<&mut Browser>) {
            if let Ok(mut state) = self.state.lock() { state.focus_events += 1; }
        }
    }
}
wrap_permission_handler! {
    struct DenyPermissions;
    impl PermissionHandler {
        fn on_request_media_access_permission(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            _requesting_origin: Option<&CefString>, _requested_permissions: u32, callback: Option<&mut MediaAccessCallback>) -> i32 {
            if let Some(callback) = callback { callback.cancel(); }
            1
        }
        fn on_show_permission_prompt(&self, _browser: Option<&mut Browser>, _prompt_id: u64,
            _requesting_origin: Option<&CefString>, _requested_permissions: u32, callback: Option<&mut PermissionPromptCallback>) -> i32 {
            if let Some(callback) = callback { callback.cont(PermissionRequestResult::DENY); }
            1
        }
    }
}
wrap_dialog_handler! {
    struct DenyFiles;
    impl DialogHandler {
        fn on_file_dialog(&self, _browser: Option<&mut Browser>, _mode: FileDialogMode,
            _title: Option<&CefString>, _default_file_path: Option<&CefString>,
            _accept_filters: Option<&mut CefStringList>, _accept_extensions: Option<&mut CefStringList>,
            _accept_descriptions: Option<&mut CefStringList>, callback: Option<&mut FileDialogCallback>) -> i32 {
            if let Some(callback) = callback { callback.cancel(); }
            1
        }
    }
}
wrap_life_span_handler! {
    struct PilotLife { state: Shared, }
    impl LifeSpanHandler {
        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            eprintln!("native: CEF DoClose; child-only hierarchy teardown");
            if let Some(browser) = browser {
                let mut task = DestroyChild::new(browser.clone());
                if post_task(ThreadId::UI, Some(&mut task)) != 1 { eprintln!("native: child teardown task rejected"); }
            }
            1
        }
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let browser = browser.cloned();
            let mut close = false;
            if let Ok(mut state) = self.state.lock() {
                state.created = true;
                state.browser = browser.clone();
                close = state.revoked;
                state.late_created_after_cancel = close;
            }
            if close && let Some(host) = browser.and_then(|b| b.host()) { host.close_browser(1); }
        }
        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            eprintln!("native: CEF on_before_close");
            if let Ok(mut state) = self.state.lock() {
                state.browser = None;
                state.observer = None;
                state.closed = true;
                if let Some(signal) = &state.signal { signal.notify(); }
            }
        }
        fn on_before_popup(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            _popup_id: i32, _target_url: Option<&CefString>, _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition, _user_gesture: i32,
            _popup_features: Option<&PopupFeatures>, _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>, _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>, _no_javascript_access: Option<&mut i32>) -> i32 { 1 }
    }
}
pub struct CefChild {
    pub state: Shared,
    pub parent: LocalRc<NativeParentLease>,
}
impl CefChild {
    pub fn create(
        parent: &NativeScopedParent<'_>,
        lease: NativeParentLease,
        state: Shared,
    ) -> Result<Self, ChildCreationFailure<Self>> {
        let create = || -> io::Result<()> {
            crate::native::prepare_parent(parent)?;
            let (hwnd, bounds, _) = crate::native::parent_geometry(parent)?;
            let info = WindowInfo {
                runtime_style: RuntimeStyle::ALLOY,
                ..Default::default()
            }
            .set_as_child(cef::sys::HWND(hwnd.0.cast()), &bounds);
            let fixture = crate::fixture::input_mode();
            let uri = if fixture {
                crate::fixture::uri().map_err(io::Error::other)?
            } else {
                crate::navigation::initialize(&state)?;
                humanymous_browser::internal_urls::NEW_TAB_URL.to_owned()
            };
            let mut client = PilotClient::new(
                state.clone(),
                if fixture { uri.clone() } else { String::new() },
            );
            let settings = BrowserSettings {
                javascript: if fixture {
                    cef::State::DISABLED
                } else {
                    cef::State::ENABLED
                },
                ..Default::default()
            };
            // Only our synthetic generated canary is offered to the renderer.
            use std::io::Write;
            let executable = std::env::current_exe()?;
            let bundle = executable
                .parent()
                .ok_or_else(|| io::Error::other("bundle absent"))?;
            let mut canary = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(bundle.join(format!("sandbox-canary-{}.txt", std::process::id())))?;
            canary.write_all(b"humanymous synthetic sandbox canary\n")?;
            let mut extra = dictionary_value_create()
                .ok_or_else(|| io::Error::other("CEF dictionary absent"))?;
            extra.set_int(
                Some(&CefString::from("humanymous_probe_owner")),
                std::process::id() as i32,
            );
            if browser_host_create_browser(
                Some(&info),
                Some(&mut client),
                Some(&CefString::from(uri.as_str())),
                Some(&settings),
                Some(&mut extra),
                None,
            ) != 1
            {
                return Err(io::Error::other("CEF child creation request rejected"));
            }
            state
                .lock()
                .map_err(|_| io::Error::other("child state poisoned"))?
                .creation_requested = true;
            Ok(())
        };
        let result = create();
        let child = Self {
            state,
            parent: LocalRc::new(lease),
        };
        match result {
            Ok(()) => Ok(child),
            Err(error) => {
                let has_resource = child.state.lock().map_or(true, |state| {
                    state.creation_requested || state.created || state.browser.is_some()
                });
                Err(ChildCreationFailure {
                    error,
                    partial: has_resource.then_some(child),
                })
            }
        }
    }
}
impl NativeChild for CefChild {
    type Error = io::Error;
    fn parent_lost(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.revoked = true;
        }
    }
    fn begin_cleanup(
        &mut self,
        request: CleanupRequest,
        signal: CompletionSignal,
    ) -> io::Result<CleanupProgress> {
        eprintln!("native: Meari child begin_cleanup");
        self.parent
            .with_cleanup_parent(request, |_| ())
            .map_err(io::Error::other)?;
        let browser = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("child state poisoned"))?;
            state.revoked = true;
            if state.closed {
                return Ok(CleanupProgress::Complete);
            }
            state.signal = Some(signal);
            state.browser.clone()
        };
        if let Some(browser) = browser {
            queue_action(browser, true, self.state.clone())?;
        }
        Ok(CleanupProgress::Pending)
    }
    fn observe_cleanup(&mut self, _request: CleanupRequest) -> io::Result<CleanupProgress> {
        let state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("child state poisoned"))?;
        Ok(if state.closed {
            CleanupProgress::Complete
        } else {
            CleanupProgress::Pending
        })
    }
}
