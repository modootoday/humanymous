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
#[path = "host_deadline.rs"]
mod deadline;
use deadline::CloseDeadline;
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
    document: NativeDocumentHandle,
    controls: crate::browser_controls::Controls,
    dock: humanymous_browser::viewport::InspectorDock,
    registrar: NativeChildRegistrar<CefChild>,
    state: Shared,
    tabs: crate::tab_sessions::Sessions,
    timings: crate::shell_timings::Shared,
    parent: Option<Rc<NativeParentLease>>,
    registered: bool,
    ticks: usize,
    window_request_cursor: usize,
    drag: Option<crate::window_frame::Drag>,
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
#[path = "host_report.rs"]
mod report;
use report::Report;
impl Pilot {
    fn synchronize_page_geometry(&mut self) -> io::Result<()> {
        if self.close_requested {
            return Ok(());
        }
        if self.registrar.phase().map_err(io::Error::other)? != WindowPhase::Ready {
            return Ok(());
        }
        let Some(parent) = self.parent.clone() else {
            return Ok(());
        };
        let browser = {
            let state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            if !state.is_live() || !state.active {
                return Ok(());
            }
            state.browser.clone()
        };
        let Some(browser) = browser else {
            return Ok(());
        };
        parent
            .with_parent(|scoped| -> io::Result<()> {
                if crate::window_frame::observe(scoped)?.minimized {
                    return Ok(());
                }
                let (hwnd, bounds, dpi) = native::dock_geometry(scoped, self.dock)?;
                native::validate_parent(&browser, hwnd)?;
                let key = (bounds.x, bounds.y, bounds.width, bounds.height, dpi);
                if self.last_bounds != Some(key) {
                    native::resize_child(&browser, &bounds)?;
                    self.last_bounds = Some(key);
                    let mut report = self.report.borrow_mut();
                    report.resize_readbacks += 1;
                    if report.page_allocations.len() < 128 {
                        report
                            .page_allocations
                            .push(([bounds.x, bounds.y, bounds.width, bounds.height], dpi));
                    }
                }
                Ok(())
            })
            .map_err(io::Error::other)??;
        Ok(())
    }
    fn tick(&mut self) -> io::Result<()> {
        self.ticks += 1;
        if !self.close_requested
            && self
                .state
                .lock()
                .is_ok_and(|s| s.created && !s.revoked && !s.closed)
        {
            self.controls.poll()?;
        }
        if let Ok(dock) = self
            .document
            .try_with(humanymous_browser::ui::inspector_dock)
        {
            self.dock = dock;
        }
        if !self.close_requested
            && self
                .registrar
                .phase()
                .is_ok_and(|phase| phase == WindowPhase::Ready)
            && let Some(parent) = &self.parent
        {
            let (cursor, commands) = self
                .document
                .try_with(|doc| {
                    humanymous_browser::window_chrome::commands_since(
                        doc,
                        self.window_request_cursor,
                    )
                })
                .map_err(io::Error::other)?;
            self.window_request_cursor = cursor;
            if cursor > 4096 {
                return Err(io::Error::other("window intent budget exceeded"));
            }
            for command in commands {
                if self.report.borrow().window_commands.len() < 128 {
                    self.report
                        .borrow_mut()
                        .window_commands
                        .push(format!("{command:?}:requested"));
                }
                if command == humanymous_browser::window_chrome::WindowCommand::Close {
                    self.close_requested = true;
                    self.driver
                        .wake_handle()
                        .request_close()
                        .map_err(io::Error::other)?;
                    break;
                }
                parent
                    .with_parent(|scoped| crate::window_frame::apply(scoped, command))
                    .map_err(io::Error::other)??;
            }
        }
        // Windows CEF owns its dedicated UI thread. Winit remains the Meari
        // parent thread; async creation copies WindowInfo within the lease call.
        if !self.registered
            && self.registrar.phase().map_err(io::Error::other)? == WindowPhase::Ready
        {
            let state = self.state.clone();
            eprintln!("native: registered parent ready; CEF child creation begin");
            let mut parent = None;
            let lease = self
                .registrar
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
            self.tabs.attach_initial(
                self.state.clone(),
                self.parent
                    .clone()
                    .ok_or_else(|| io::Error::other("initial tab parent absent"))?,
                lease,
            );
            eprintln!("native: CEF child creation end");
            self.registered = true;
            self.report.borrow_mut().decorations_removed_readback = true;
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
            && !self.close_requested
            && self.registrar.phase().map_err(io::Error::other)? == WindowPhase::Ready
        {
            if let Some(active) = self.tabs.step(&self.registrar, &self.document)? {
                self.state = active.state;
                self.parent = Some(active.parent);
                self.controls.bind(self.state.clone());
                self.last_bounds = None;
                self.focus_requested = active.address;
            }
            if self.tabs.final_tab_closing() {
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
        self.synchronize_page_geometry()?;
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
                        let observation = crate::window_frame::observe(scoped)?;
                        let minimized = observation.minimized;
                        if self.report.borrow().window_observations.len() < 64
                            && self.report.borrow().window_observations.last() != Some(&observation)
                        {
                            self.report
                                .borrow_mut()
                                .window_observations
                                .push(observation);
                        }
                        // Minimized zero client size is not a drawable viewport; retain
                        // the child and last valid bounds until restoration.
                        if minimized {
                            return Ok(());
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
                if self.ticks.is_multiple_of(10)
                    && report.owned_surface_transitions.len() < 64
                    && std::env::args().any(|a| a == "--native-measured-hanja")
                {
                    let surfaces = crate::measured_input::owned_surfaces()?;
                    if report.owned_surface_transitions.last() != Some(&surfaces) {
                        eprintln!(
                            "native: owned surface geometry {}",
                            serde_json::to_string(&surfaces).map_err(io::Error::other)?
                        );
                        report.owned_surface_transitions.push(surfaces);
                    }
                }
                let snapshot = native::input_snapshot(&browser)?;
                if report.input_snapshots.len() < 64
                    && report.input_snapshots.last() != Some(&snapshot)
                {
                    report.input_snapshots.push(snapshot);
                }
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
        // Geometry is already observable from Win32 when winit delivers this event.
        // Update the owned child before parent rendering, outside diagnostic polling.
        if matches!(
            event,
            WindowEvent::SurfaceResized(_) | WindowEvent::ScaleFactorChanged { .. }
        ) && let Err(error) = self.synchronize_page_geometry()
        {
            self.fail(event_loop, error);
            return;
        }
        if matches!(event, WindowEvent::KeyboardInput { .. }) {
            self.report.borrow_mut().parent_key_events += 1;
        }
        let frame_event = event.clone();
        let stage = match &event {
            WindowEvent::SurfaceResized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                Some(crate::shell_timings::Stage::MeariResizeHandler)
            }
            WindowEvent::RedrawRequested => Some(crate::shell_timings::Stage::MeariRedrawHandler),
            _ => None,
        };
        let started = stage.and_then(|_| self.timings.borrow().start());
        self.driver.window_event(event_loop, id, event);
        if let Some(stage) = stage
            && started.is_some()
        {
            let frame = self
                .driver
                .last_submission()
                .map(|s| s.ticket.scene_generation);
            self.timings
                .borrow_mut()
                .finish(stage, started, None, frame, None);
        }
        if !self.close_requested
            && let Err(error) = self.controls.event(&frame_event)
        {
            self.fail(event_loop, error);
        }
        if let Some(parent) = &self.parent
            && !self.close_requested
            && let Ok(Err(error)) = parent.with_parent(|scoped| {
                crate::window_frame::pointer_event(scoped, &frame_event, &mut self.drag)
            })
        {
            self.fail(event_loop, error);
        }
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

// Only constructed after both native child and parent cleanup receipts.
pub(super) struct CleanExit {
    pub probe_passed: Option<bool>,
}
pub(super) fn run() -> Result<CleanExit, Box<dyn std::error::Error>> {
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
        LifecycleLimits::new(
            humanymous_browser::tabs::Tabs::CAPACITY,
            Duration::from_secs(5),
        )?,
        WindowOptions {
            title: "humanymous | CEF native child qualification | NOT READY".into(),
        },
        None,
    )?;
    let registrar = prepared.registrar();
    let children = prepared.children();
    let runtime = Rc::new(RefCell::new(NativeHostRuntime::new(
        document.clone(),
        ChildWindowHost::new(Host, children.clone()),
    )));
    let driver = prepared
        .attach(runtime.clone())
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    // Joined on success, error and unwind before this DLL can unload.
    let _close_deadline = CloseDeadline::start(driver.wake_handle(), pilot_seconds);
    let internal_probe_requested = std::env::args().any(|a| a == "--native-internal-probe");
    let state = Arc::new(Mutex::new(State {
        navigation: crate::navigation::State {
            probe: internal_probe_requested
                .then(humanymous_browser::navigation::ProbeLedger::default),
            ..Default::default()
        },
        process_info_status: "not_run",
        ..Default::default()
    }));
    let report = Rc::new(RefCell::new(Report {
        host_pid: std::process::id(),
        navigation_schema: 2,
        renderer_file_probe_schema: 2,
        internal_probe_requested,
        ime_candidate_placement: "not_run",
        ..Default::default()
    }));
    let failure = Rc::new(RefCell::new(None));
    crate::child::prepare_canary()?;
    let inventory = Rc::new(RefCell::new(vec![state.clone()]));
    let timings = crate::shell_timings::Trace::shared(
        std::env::args().any(|a| a == "--native-shell-timings"),
    );
    let tabs =
        crate::tab_sessions::Sessions::new(children, &state, inventory.clone(), timings.clone())?;
    event_loop.run_app(Pilot {
        driver,
        controls: crate::browser_controls::Controls::new(document.clone(), state.clone()),
        document,
        dock: humanymous_browser::viewport::InspectorDock::Hidden,
        registrar,
        state: state.clone(),
        tabs,
        timings: timings.clone(),
        parent: None,
        registered: false,
        ticks: 0,
        window_request_cursor: 0,
        drag: None,
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
        let state = state
            .lock()
            .map_err(|_| io::Error::other("state poisoned"))?;
        report.borrow_mut().observe_state(
            &state,
            runtime.borrow().document().lifecycle() == HostLifecycle::Closed,
        );
    }
    let output = std::env::current_exe()?
        .parent()
        .ok_or("missing bundle")?
        .join("native-report.json");
    report.borrow_mut().observe_tabs(&inventory.borrow())?;
    report.borrow_mut().shell_timings = timings.borrow().snapshot()?;
    std::fs::write(output, serde_json::to_vec_pretty(&*report.borrow())?)?;
    if let Some(error) = failure.borrow().as_ref() {
        return Err(error.clone().into());
    }
    if !report.borrow().before_close
        || !report.borrow().host_closed
        || report.borrow().tabs_incomplete != 0
    {
        return Err("native cleanup receipt incomplete".into());
    }
    Ok(CleanExit {
        probe_passed: report
            .borrow()
            .internal_probe_requested
            .then_some(report.borrow().internal_probe_passed),
    })
}
