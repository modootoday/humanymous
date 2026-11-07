//! One host owns all real tab children through the existing Meari lifecycle.
use crate::child::{CefChild, Shared, State};
use humanymous_browser::{
    tabs::{Command, Tabs},
    tabs_ui::View,
};
use meari_native::{
    application::window_lifecycle::*,
    infrastructure::{native_host::NativeDocumentHandle, window::*},
};
use std::{
    cell::RefCell,
    io,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub type Queue = Arc<Mutex<Vec<(Option<usize>, Command)>>>;
pub type Inventory = Rc<RefCell<Vec<Shared>>>;
pub struct Active {
    pub state: Shared,
    pub parent: Rc<NativeParentLease>,
    pub address: bool,
}
struct Entry {
    state: Shared,
    parent: Rc<NativeParentLease>,
    lease: ChildLease,
    address: bool,
}
pub struct Sessions {
    model: Tabs,
    entries: Vec<Entry>,
    queue: Queue,
    children: Rc<WindowLifecycle<CefChild>>,
    inventory: Inventory,
    projected: Vec<View>,
    active: Option<usize>,
}
pub fn enqueue(state: &Shared, command: Command) -> io::Result<()> {
    let (id, queue) = {
        let state = state
            .lock()
            .map_err(|_| io::Error::other("tab state poisoned"))?;
        if !state.is_live() || !state.active {
            return Ok(());
        }
        (state.tab_id, state.commands.clone())
    };
    if let Some(queue) = queue {
        let mut queue = queue
            .lock()
            .map_err(|_| io::Error::other("tab queue poisoned"))?;
        if queue.len() >= 64 {
            return Err(io::Error::other("tab command budget exceeded"));
        }
        queue.push((Some(id), command));
    }
    Ok(())
}
pub fn enqueue_window(state: &Shared, command: Command) -> io::Result<()> {
    let queue = state
        .lock()
        .map_err(|_| io::Error::other("tab state poisoned"))?
        .commands
        .clone();
    if let Some(queue) = queue {
        let mut queue = queue
            .lock()
            .map_err(|_| io::Error::other("tab queue poisoned"))?;
        if queue.len() >= 64 {
            return Err(io::Error::other("tab command budget exceeded"));
        }
        queue.push((None, command));
    }
    Ok(())
}
impl Sessions {
    pub fn new(
        children: Rc<WindowLifecycle<CefChild>>,
        initial: &Shared,
        inventory: Inventory,
    ) -> io::Result<Self> {
        let queue = Queue::default();
        {
            let mut state = initial
                .lock()
                .map_err(|_| io::Error::other("tab state poisoned"))?;
            state.active = true;
            state.commands = Some(queue.clone());
        }
        Ok(Self {
            model: Tabs::default(),
            entries: Vec::new(),
            queue,
            children,
            inventory,
            projected: Vec::new(),
            active: Some(0),
        })
    }
    pub fn attach_initial(
        &mut self,
        state: Shared,
        parent: Rc<NativeParentLease>,
        lease: ChildLease,
    ) {
        self.entries.push(Entry {
            state,
            parent,
            lease,
            address: false,
        });
    }
    pub fn final_tab_closing(&self) -> bool {
        self.model.active().is_none()
    }
    fn activate(&self) -> io::Result<()> {
        // Hide every inactive child before showing the selected one.
        for selected in [false, true] {
            for (id, entry) in self.entries.iter().enumerate() {
                let active = self.model.active() == Some(id);
                if active != selected {
                    continue;
                }
                let browser = {
                    let mut state = entry
                        .state
                        .lock()
                        .map_err(|_| io::Error::other("tab state poisoned"))?;
                    state.active = active;
                    if !state.is_live() {
                        None
                    } else {
                        state.browser.clone()
                    }
                };
                if let Some(browser) = browser {
                    crate::tab_windows::visible(&browser, active)?;
                    crate::tab_windows::focus(browser, entry.state.clone(), active)?;
                }
            }
        }
        Ok(())
    }
    pub fn step(
        &mut self,
        registrar: &NativeChildRegistrar<CefChild>,
        document: &NativeDocumentHandle,
    ) -> io::Result<Option<Active>> {
        for (id, entry) in self.entries.iter().enumerate() {
            let state = entry
                .state
                .lock()
                .map_err(|_| io::Error::other("tab state poisoned"))?;
            if let Some(error) = state.surface_error {
                return Err(io::Error::other(error));
            }
            let closed = state.closed;
            drop(state);
            if closed {
                if self.model.close(id) {
                    self.children
                        .release_child(&entry.lease)
                        .map_err(io::Error::other)?;
                }
                self.children.poll().map_err(io::Error::other)?;
                self.model.acknowledge_closed(id);
            }
        }
        let commands = std::mem::take(
            &mut *self
                .queue
                .lock()
                .map_err(|_| io::Error::other("tab queue poisoned"))?,
        );
        for (source, command) in commands {
            let Some(active) = self.model.active() else {
                continue;
            };
            if source.is_some_and(|source| source != active) {
                continue;
            }
            let source = active;
            match command {
                Command::New => {
                    let Ok(id) = self.model.create() else {
                        continue;
                    };
                    self.activate()?;
                    let state = Arc::new(Mutex::new(State {
                        active: true,
                        tab_id: id,
                        commands: Some(self.queue.clone()),
                        process_info_status: "not_run",
                        ..Default::default()
                    }));
                    self.inventory.borrow_mut().push(state.clone());
                    let mut parent = None;
                    let create_state = state.clone();
                    let lease = registrar
                        .register(
                            registrar.stamp().map_err(io::Error::other)?,
                            |scoped, lease| {
                                let child = CefChild::create(scoped, lease, create_state)?;
                                parent = Some(child.parent.clone());
                                Ok(child)
                            },
                        )
                        .map_err(|_| io::Error::other("tab child registration failed"))?;
                    self.entries.push(Entry {
                        state,
                        parent: parent.ok_or_else(|| io::Error::other("tab parent absent"))?,
                        lease,
                        address: true,
                    });
                    self.queue
                        .lock()
                        .map_err(|_| io::Error::other("tab queue poisoned"))?
                        .push((None, Command::Address));
                }
                Command::Close | Command::CloseId(_) => {
                    let id = if let Command::CloseId(id) = command {
                        id
                    } else {
                        source
                    };
                    if self.model.close(id) {
                        self.activate()?;
                        self.children
                            .release_child(&self.entries[id].lease)
                            .map_err(io::Error::other)?;
                    }
                }
                Command::Select(id) => {
                    self.model.select(id);
                    self.activate()?;
                }
                Command::Cycle(reverse) => {
                    self.model.cycle(reverse);
                    self.activate()?;
                }
                Command::Address => {
                    self.entries[source].address = true;
                    let entry = &self.entries[source];
                    if let Some(browser) = entry
                        .state
                        .lock()
                        .map_err(|_| io::Error::other("tab state poisoned"))?
                        .browser
                        .clone()
                    {
                        crate::tab_windows::focus(browser, entry.state.clone(), false)?;
                    }
                    let stamp = document.stamp().map_err(io::Error::other)?;
                    document
                        .try_reproject(&stamp, |doc| {
                            if let Some(address) =
                                doc.renderer.document.get_element_by_id("address")
                            {
                                doc.renderer.document.set_focus_to(address);
                            }
                            Ok::<_, io::Error>(())
                        })
                        .map_err(io::Error::other)?;
                    entry
                        .parent
                        .with_parent(crate::tab_windows::focus_address)
                        .map_err(io::Error::other)??;
                }
                Command::Back
                | Command::Forward
                | Command::Reload
                | Command::Home
                | Command::Open(_)
                | Command::Stop => {
                    self.entries[source].address = false;
                    let entry = &self.entries[source];
                    let (browser, route) = {
                        let state = entry
                            .state
                            .lock()
                            .map_err(|_| io::Error::other("tab state poisoned"))?;
                        (
                            state.browser.clone(),
                            state.navigation.committed().map(|(_, r)| r),
                        )
                    };
                    if let Some(browser) = browser {
                        if matches!(command, Command::Reload | Command::Home | Command::Open(_)) {
                            let route = match command {
                                Command::Home => {
                                    Some(humanymous_browser::internal_urls::Route::NewTab)
                                }
                                Command::Open(route) => Some(route),
                                _ => route,
                            };
                            if let Some(route) = route {
                                crate::navigation::queue(
                                    browser,
                                    route,
                                    entry.state.clone(),
                                    command == Command::Reload,
                                )?;
                            }
                        } else {
                            crate::history::queue(
                                browser,
                                entry.state.clone(),
                                if command == Command::Stop {
                                    crate::history::Action::Stop
                                } else if command == Command::Back {
                                    crate::history::Action::Back
                                } else {
                                    crate::history::Action::Forward
                                },
                            )?;
                        }
                    }
                }
            }
        }
        if self.active != self.model.active() {
            self.activate()?;
        }
        let views = self
            .model
            .open_ids()
            .into_iter()
            .map(|id| {
                let state = self.entries[id]
                    .state
                    .lock()
                    .map_err(|_| io::Error::other("tab state poisoned"))?;
                Ok(View {
                    id,
                    title: state
                        .navigation
                        .committed()
                        .map_or("New tab", |(_, r)| r.title()),
                    selected: self.model.active() == Some(id),
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        if views != self.projected {
            let stamp = document.stamp().map_err(io::Error::other)?;
            document
                .try_reproject(&stamp, |doc| {
                    humanymous_browser::tabs_ui::project(doc, &views).map_err(io::Error::other)?;
                    Ok::<_, io::Error>(())
                })
                .map_err(io::Error::other)?;
            self.projected = views;
        }
        if self.active != self.model.active() {
            self.active = self.model.active();
            return Ok(self.active.map(|id| Active {
                state: self.entries[id].state.clone(),
                parent: self.entries[id].parent.clone(),
                address: self.entries[id].address,
            }));
        }
        Ok(None)
    }
}
