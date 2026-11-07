//! Native address/home/reload intents. Target documents never own these controls.
use crate::child::Shared;
use humanymous_browser::{
    internal_urls,
    tabs::Command,
    ui_navigation::{self, ImeGuard, Intent},
};
use meari_native::infrastructure::native_host::NativeDocumentHandle;
use std::io;
use winit::{
    event::{ElementState, Ime, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};
#[path = "internal_probe.rs"]
mod probe;

pub struct Controls {
    document: NativeDocumentHandle,
    state: Shared,
    cursor: usize,
    ime: ImeGuard,
    modifiers: ModifiersState,
    committed: u64,
    notice: Option<&'static str>,
    controls: Option<humanymous_browser::history::Controls>,
    probe: Option<probe::Probe>,
}
impl Controls {
    pub fn bind(&mut self, state: Shared) {
        self.state = state;
        self.ime = ImeGuard::default();
        self.modifiers = ModifiersState::default();
        self.committed = 0;
        self.notice = None;
        self.controls = None;
    }
    pub fn new(document: NativeDocumentHandle, state: Shared) -> Self {
        let probe_enabled = std::env::args().any(|a| a == "--native-internal-probe");
        Self {
            document,
            state,
            cursor: 0,
            ime: ImeGuard::default(),
            modifiers: ModifiersState::default(),
            committed: 0,
            notice: None,
            controls: None,
            probe: probe_enabled.then(probe::Probe::default),
        }
    }
    pub fn event(&mut self, event: &WindowEvent) -> io::Result<()> {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Ime(Ime::Preedit(text, _)) => self.ime.preedit(!text.is_empty()),
            WindowEvent::Ime(Ime::Commit(_)) => self.ime.commit(),
            WindowEvent::Ime(Ime::Disabled) | WindowEvent::Focused(false) => {
                self.ime.cancel();
                self.modifiers = ModifiersState::default();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let vk = match &event.logical_key {
                    Key::Named(NamedKey::Tab) => 0x09,
                    Key::Named(NamedKey::F5) => 0x74,
                    Key::Named(NamedKey::ArrowLeft) => 0x25,
                    Key::Named(NamedKey::ArrowRight) => 0x27,
                    Key::Character(text) if text.len() == 1 => {
                        u32::from(text.as_bytes()[0].to_ascii_uppercase())
                    }
                    _ => 0,
                };
                if let Some(command) = humanymous_browser::tabs::shortcut(
                    vk,
                    self.modifiers.control_key(),
                    self.modifiers.alt_key(),
                    self.modifiers.shift_key(),
                    self.modifiers.super_key(),
                    event.repeat,
                ) {
                    crate::tab_sessions::enqueue_window(&self.state, command)?;
                    return Ok(());
                }
                if event.logical_key == Key::Named(NamedKey::Enter) {
                    let focused = self
                        .document
                        .try_with(ui_navigation::address_focused)
                        .map_err(io::Error::other)?;
                    if self.ime.enter(focused, event.repeat) {
                        let address = self
                            .document
                            .try_with(ui_navigation::address)
                            .map_err(io::Error::other)?;
                        if let Some(address) = address {
                            self.dispatch(Intent::Open(address))?;
                        }
                    }
                } else {
                    self.ime.ordinary_key();
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn dispatch(&mut self, intent: Intent) -> io::Result<()> {
        let command = match intent {
            Intent::Tab(command) => command,
            Intent::Back => Command::Back,
            Intent::Forward => Command::Forward,
            Intent::Stop => Command::Stop,
            Intent::Home => Command::Home,
            Intent::Reload => Command::Reload,
            Intent::Open(address) => match internal_urls::classify(&address) {
                Ok(route) => Command::Open(route),
                Err(_) => {
                    self.state
                        .lock()
                        .map_err(|_| io::Error::other("state poisoned"))?
                        .navigation
                        .notice = Some("Only local newtab and permitted diagnostics are available");
                    return Ok(());
                }
            },
        };
        crate::tab_sessions::enqueue_window(&self.state, command)
    }
    pub fn poll(&mut self) -> io::Result<()> {
        let (cursor, intents) = self
            .document
            .try_with(|doc| ui_navigation::intents_since(doc, self.cursor))
            .map_err(io::Error::other)?;
        if cursor > 4096 {
            return Err(io::Error::other("navigation intent budget exceeded"));
        }
        self.cursor = cursor;
        for intent in intents {
            self.dispatch(intent)?;
        }
        if let Some(probe) = &mut self.probe {
            probe.tick(&self.state)?;
        }
        let (committed, notice, controls) = {
            let state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            let mut controls = state.navigation.controls;
            controls.loading |= state.navigation.transaction.pending().is_some();
            controls.back &= !controls.loading;
            controls.forward &= !controls.loading;
            (
                state.navigation.committed(),
                state
                    .navigation
                    .notice
                    .or(controls.loading.then_some("Loading local page")),
                controls,
            )
        };
        if self.controls != Some(controls) {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::controls(doc, controls);
                    Ok::<_, io::Error>(())
                })
                .map_err(io::Error::other)?;
            self.controls = Some(controls);
        }
        if let Some((generation, route)) = committed
            && generation != self.committed
        {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::committed(doc, route).map_err(io::Error::other)
                })
                .map_err(io::Error::other)?;
            self.committed = generation;
        }
        if notice != self.notice {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::notice(doc, notice.unwrap_or("LOCAL ONLY"));
                    Ok::<_, io::Error>(())
                })
                .map_err(io::Error::other)?;
            self.notice = notice;
        }
        Ok(())
    }
}
