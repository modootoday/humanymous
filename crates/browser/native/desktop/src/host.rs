use crate::{
    child::{CefChild, Shared, State},
    native,
};
use meari_native::{
    application::{WindowOptions, window_lifecycle::*},
    infrastructure::{native_host::*, window::*},
};
use std::{
    cell::RefCell,
    io,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

struct Host;
struct CloseDeadline {
    cancel: std::sync::mpsc::Sender<()>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl CloseDeadline {
    fn start(wake: NativeWindowWake, seconds: u64) -> Self {
        use std::sync::mpsc::{RecvTimeoutError, channel};
        let (cancel, receiver) = channel();
        let worker = std::thread::spawn(move || {
            if matches!(
                receiver.recv_timeout(Duration::from_secs(seconds)),
                Err(RecvTimeoutError::Timeout)
            ) {
                eprintln!("native: independent native close deadline");
                let _ = wake.request_close();
                if matches!(
                    receiver.recv_timeout(Duration::from_secs(15)),
                    Err(RecvTimeoutError::Timeout)
                ) {
                    eprintln!("native: incomplete cleanup at watchdog deadline");
                    std::process::exit(78);
                }
            }
        });
        Self {
            cancel,
            worker: Some(worker),
        }
    }
}
impl Drop for CloseDeadline {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl NativeWindowHost for Host {
    type Error = io::Error;
    type CloseCheckpoint = ();
    fn admit_callback(
        &mut self,
        _: &HostStamp,
        _: &meari::application::host_callbacks::PendingCallback,
    ) -> IntentAdmission<io::Error> {
        IntentAdmission::Rejected(io::Error::other(
            "qualification UI has no target callback bridge",
        ))
    }
    fn after_dispatch(&mut self, _: HostTurn) -> io::Result<HostEffects> {
        Ok(HostEffects::default())
    }
    fn close(&mut self, _: CloseRequest<()>) -> CloseDecision<(), io::Error> {
        CloseDecision::HostCleanupComplete { checkpoint: () }
    }
}
struct Pilot {
    driver: NativeWindowDriver,
    registrar: NativeChildRegistrar<CefChild>,
    state: Shared,
    parent: Option<Rc<NativeParentLease>>,
    registered: bool,
    ticks: usize,
    last_bounds: Option<(i32, i32, i32, i32, u32)>,
    focus_requested: bool,
    started: Instant,
    last_tick: Instant,
    close_requested: bool,
    pilot_seconds: u64,
    cancel_immediately: bool,
    report: Rc<RefCell<Report>>,
    failure: Rc<RefCell<Option<String>>>,
}
#[derive(Default, serde::Serialize)]
struct Report {
    host_pid: u32,
    cancellation_requested: bool,
    late_created_after_cancel: bool,
    cef_child_created: bool,
    actual_parent_matched: bool,
    resize_readbacks: usize,
    focus_observed: bool,
    before_close: bool,
    host_closed: bool,
    process_tokens: Vec<native::ProcessToken>,
    process_roles: Vec<crate::child::ProcessRole>,
    process_info_status: &'static str,
    renderer_restricted_token_observed: bool,
    sandbox_qualified: bool,
    ime_candidate_placement: &'static str,
    accessibility_qualified: bool,
    target_navigation_enabled: bool,
}
impl Pilot {
    fn tick(&mut self) -> io::Result<()> {
        self.ticks += 1;
        // Windows CEF owns its dedicated UI thread. Winit remains the Meari
        // parent thread; async creation copies WindowInfo within the lease call.
        if !self.registered
            && self.registrar.phase().map_err(io::Error::other)? == WindowPhase::Ready
        {
            let state = self.state.clone();
            eprintln!("native: registered parent ready; CEF child creation begin");
            let mut parent = None;
            self.registrar
                .register(
                    self.registrar.stamp().map_err(io::Error::other)?,
                    |scoped, lease| {
                        let child = CefChild::create(scoped, lease, state)?;
                        parent = Some(child.parent.clone());
                        Ok(child)
                    },
                )
                .map_err(|error| {
                    io::Error::other(format!("native CEF registration: {:?}", error.problem))
                })?;
            self.parent = parent;
            eprintln!("native: CEF child creation end");
            self.registered = true;
            if self.cancel_immediately {
                self.report.borrow_mut().cancellation_requested = true;
                self.close_requested = true;
                self.driver
                    .wake_handle()
                    .request_close()
                    .map_err(io::Error::other)?;
            }
        }
        if self.registered
            && self.registrar.phase().map_err(io::Error::other)? != WindowPhase::Ready
        {
            self.close_requested = true;
        }
        if let Some(parent) = &self.parent {
            let browser = self
                .state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?
                .browser
                .clone();
            if let Some(browser) = browser.filter(|_| !self.close_requested) {
                parent
                    .with_parent(|scoped| -> io::Result<()> {
                        let (hwnd, bounds, dpi) = native::parent_geometry(scoped)?;
                        native::validate_parent(&browser, hwnd)?;
                        let key = (bounds.x, bounds.y, bounds.width, bounds.height, dpi);
                        if self.last_bounds != Some(key) {
                            native::resize_child(&browser, &bounds)?;
                            self.last_bounds = Some(key);
                            self.report.borrow_mut().resize_readbacks += 1;
                        }
                        Ok(())
                    })
                    .map_err(io::Error::other)??;
                let mut report = self.report.borrow_mut();
                report.cef_child_created = self
                    .state
                    .lock()
                    .map_err(|_| io::Error::other("state poisoned"))?
                    .created;
                report.actual_parent_matched = true;
                if !self.focus_requested {
                    crate::child::queue_action(browser.clone(), false, self.state.clone())?;
                    self.focus_requested = true;
                }
                report.focus_observed |= native::focus_observed(&browser)?;
                // Native process tokens are independent OS observations, but process
                // type/complete sandbox policy still needs separate attestation.
                if self.started.elapsed() > Duration::from_secs(2)
                    && report.process_tokens.is_empty()
                {
                    report.process_tokens = native::owned_process_tokens()?;
                }
            }
        }
        if self.started.elapsed() > Duration::from_secs(self.pilot_seconds) && !self.close_requested
        {
            self.close_requested = true;
            self.driver
                .wake_handle()
                .request_close()
                .map_err(io::Error::other)?;
        }
        if self.started.elapsed() > Duration::from_secs(self.pilot_seconds + 15) {
            return Err(io::Error::other("native child close timed out"));
        }
        Ok(())
    }
    fn fail(&self, event_loop: &dyn ActiveEventLoop, error: io::Error) {
        *self.failure.borrow_mut() = Some(error.to_string());
        event_loop.exit();
    }
    fn service(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.last_tick.elapsed() < Duration::from_millis(10) {
            return;
        }
        self.last_tick = Instant::now();
        if let Err(error) = self.tick() {
            self.fail(event_loop, error);
        }
        if self
            .registrar
            .phase()
            .is_ok_and(|phase| phase == WindowPhase::Closed)
        {
            event_loop.exit();
        }
    }
}
impl ApplicationHandler for Pilot {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        eprintln!("native: Meari surface creation begin");
        self.driver.can_create_surfaces(event_loop);
        eprintln!("native: Meari surface creation end");
    }
    fn destroy_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.driver.destroy_surfaces(event_loop);
    }
    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.driver.window_event(event_loop, id, event);
        self.service(event_loop);
    }
    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.driver.proxy_wake_up(event_loop);
        self.service(event_loop);
        if self
            .registrar
            .phase()
            .is_ok_and(|phase| phase == WindowPhase::Closed)
        {
            event_loop.exit();
        }
    }
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.ticks == 0 {
            eprintln!("native: first Meari about-to-wait begin");
        }
        self.driver.about_to_wait(event_loop);
        if self.ticks == 0 {
            eprintln!("native: first Meari about-to-wait end");
        }
        self.service(event_loop);
        if self
            .registrar
            .phase()
            .is_ok_and(|phase| phase == WindowPhase::Closed)
        {
            event_loop.exit();
            return;
        }
        let next = Instant::now() + Duration::from_millis(10);
        match event_loop.control_flow() {
            ControlFlow::WaitUntil(earlier) if earlier < next => (),
            _ => event_loop.set_control_flow(ControlFlow::WaitUntil(next)),
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let pilot_seconds = std::env::args()
        .find_map(|arg| {
            arg.strip_prefix("--native-pilot-seconds=")
                .map(str::to_owned)
        })
        .map_or(Ok(25), |value| value.parse::<u64>())?;
    if !(5..=180).contains(&pilot_seconds) {
        return Err("native pilot duration must be5..180".into());
    }
    let cancel_immediately = std::env::args().any(|arg| arg == "--native-cancel-before-created");
    let event_loop = EventLoop::builder().build()?;
    let document = NativeDocumentHandle::new(crate::fixture::management_document()?, 1)?;
    let prepared = NativeWindowPreparation::new(
        &event_loop,
        document.clone(),
        LifecycleLimits::new(1, Duration::from_secs(5))?,
        WindowOptions {
            title: "humanymous | CEF native child qualification | NOT READY".into(),
        },
        None,
    )?;
    let registrar = prepared.registrar();
    let runtime = Rc::new(RefCell::new(NativeHostRuntime::new(
        document,
        ChildWindowHost::new(Host, prepared.children()),
    )));
    let driver = prepared
        .attach(runtime.clone())
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    // Joined on success, error and unwind before this DLL can unload.
    let _close_deadline = CloseDeadline::start(driver.wake_handle(), pilot_seconds);
    let state = Arc::new(Mutex::new(State {
        process_info_status: "not_run",
        ..Default::default()
    }));
    let report = Rc::new(RefCell::new(Report {
        host_pid: std::process::id(),
        ime_candidate_placement: "not_run",
        ..Default::default()
    }));
    let failure = Rc::new(RefCell::new(None));
    event_loop.run_app(Pilot {
        driver,
        registrar,
        state: state.clone(),
        parent: None,
        registered: false,
        ticks: 0,
        last_bounds: None,
        focus_requested: false,
        started: Instant::now(),
        last_tick: Instant::now(),
        close_requested: false,
        pilot_seconds,
        cancel_immediately,
        report: report.clone(),
        failure: failure.clone(),
    })?;
    {
        let mut report = report.borrow_mut();
        report.cef_child_created = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?
            .created;
        report.before_close = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?
            .closed;
        report.host_closed = runtime.borrow().document().lifecycle() == HostLifecycle::Closed;
        report.late_created_after_cancel = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?
            .late_created_after_cancel;
        report.process_roles = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?
            .process_roles
            .clone();
        report.process_info_status = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?
            .process_info_status;
        let renderers: Vec<_> = report
            .process_tokens
            .iter()
            .filter(|p| p.role.as_deref() == Some("renderer"))
            .collect();
        report.renderer_restricted_token_observed = !renderers.is_empty()
            && renderers
                .iter()
                .all(|t| t.restricted && t.integrity_rid == 0);
    }
    let output = std::env::current_exe()?
        .parent()
        .ok_or("missing bundle")?
        .join("native-report.json");
    std::fs::write(output, serde_json::to_vec_pretty(&*report.borrow())?)?;
    if let Some(error) = failure.borrow().as_ref() {
        return Err(error.clone().into());
    }
    if !report.borrow().before_close || !report.borrow().host_closed {
        return Err("native cleanup receipt incomplete".into());
    }
    Ok(())
}
